use crate::config::Globals;
use crate::tasks::GameServer;
use crate::tor::GameHost;
use crate::tor::TorClient;
use crate::Database;
use crate::DatabaseInner;
use crate::Game;
use crate::GameInitState;
use crate::GameState;
use crate::Pending;
use crate::SigningKeyInner;
use attest_database::db_handle::create::TipControl;
use attest_database::generate_new_user;
use attest_messages::Authenticated;
use attest_messages::GenericEnvelope;
use game_host_messages::BroadcastByHost;
use game_host_messages::Channelized;
use game_host_messages::JoinCode;
use game_player_messages::ParticipantAction;
use mine_with_friends_board::entity::EntityID;
use mine_with_friends_board::game::game_move::GameMove;
use mine_with_friends_board::game::game_move::Heartbeat;
use mine_with_friends_board::game::GameBoard;
use mine_with_friends_board::sanitize::Unsanitized;
use mine_with_friends_board::MoveEnvelope;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::secp256k1::All;
use sapio_bitcoin::secp256k1::Secp256k1;
use sapio_bitcoin::KeyPair;
use sapio_bitcoin::XOnlyPublicKey;
use std::sync::Arc;
use tauri::async_runtime::spawn_blocking;
use tauri::async_runtime::Mutex;
use tauri::State;
use tokio::spawn;
use tokio::sync::Notify;
use tokio::task::spawn_blocking;
use tracing::debug;
use tracing::info;

pub(crate) trait ErrToString<E> {
    fn err_to_string(self) -> Result<E, String>;
}

impl<E, T: std::fmt::Debug> ErrToString<E> for Result<E, T> {
    fn err_to_string(self) -> Result<E, String> {
        self.map_err(|e| format!("{:?}", e))
    }
}

pub(crate) async fn make_new_game(
    nickname: String,
    secp: State<'_, Arc<Secp256k1<All>>>,
    db: State<'_, Database>,
    globals: State<'_, Arc<Globals>>,
    game_host_state: State<'_, Arc<Mutex<Option<GameHost>>>>,
    game: GameState<'_>,
) -> Result<(), String> {
    let game_host = game_host_state.inner().lock().await.clone();
    let game_host = game_host.ok_or("Must be connected to some host")?;
    let mut game = game.lock().await;
    if game.game_mut().is_some() {
        Err("Must not be in Game State")?;
    }
    let client = globals
        .inner()
        .get_client()
        .await
        .map_err(|e| e.to_string())?;

    let new_game = client
        .create_new_game_instance(&game_host)
        .await
        .map_err(|e| e.to_string())?;
    info!(?new_game, "New Game Set Up");
    let new_chain = make_new_chain_genesis(nickname, secp, db).await?;
    client
        .add_player(&game_host, (new_game.join, new_chain))
        .await
        .map_err(|e| {
            debug!(error=?e, "Adding Player Failed");
            e.to_string()
        })?;

    info!("Setting GameInitState to Pending");
    *game = GameInitState::Pending(Pending {
        join_code: new_game.join,
        password: Some(new_game.password),
    });
    Ok(())
}
pub(crate) async fn make_new_chain_genesis(
    nickname: String,
    secp: State<'_, Arc<Secp256k1<All>>>,
    db: State<'_, Database>,
) -> Result<Authenticated<GenericEnvelope<ParticipantAction>>, String> {
    let (kp, next_nonce, genesis) = generate_new_user::<_, ParticipantAction, _>(
        secp.inner(),
        MoveEnvelope {
            d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
            sequence: 0,
            /// The player who is making the move, myst be figured out somewhere...
            time_millis: attest_util::now() as u64,
        },
    )
    .err_to_string()?;
    let msgdb = db.get().await.err_to_string()?;
    let mut handle = msgdb.get_handle_all().await;
    spawn_blocking(move || {
        // TODO: Transaction?
        handle.save_keypair(kp).err_to_string()?;
        let k = kp.public_key().x_only_public_key().0;
        handle.save_nonce_for_user_by_key(next_nonce, secp.inner(), k);

        let envelope = genesis.self_authenticate(secp.inner()).err_to_string()?;
        handle.insert_user_by_genesis_envelope(nickname, envelope.clone());
        envelope
    })
    .await
    .map_err(|e| e.to_string())
}
pub(crate) async fn make_new_chain_inner(
    nickname: String,
    code: JoinCode,
    secp: State<'_, Arc<Secp256k1<All>>>,
    db: State<'_, Database>,
    globals: State<'_, Arc<Globals>>,
    game_host_state: State<'_, Arc<Mutex<Option<GameHost>>>>,
    game: GameState<'_>,
) -> Result<(), String> {
    let game_host = game_host_state.inner().lock().await.clone();
    let game_host = game_host.ok_or("Must be connected to some host")?;
    let k = make_new_chain_genesis(nickname, secp, db).await?;
    let client = globals.get_client().await.map_err(|_| "No Client")?;
    let mut game = game.lock().await;
    if game.game_mut().is_some() {
        Err("Must not be in Game State")?;
    }
    client
        .add_player(&game_host, (code, k))
        .await
        .map_err(|e| {
            debug!(error=?e, "Adding Player Failed");
            e.to_string()
        })?;

    info!("Setting GameInitState to Pending");
    *game = GameInitState::Pending(Pending {
        join_code: code,
        password: None,
    });
    Ok(())
}

pub(crate) async fn make_move_inner_inner(
    secp: Arc<Secp256k1<All>>,
    db: Database,
    sk: SigningKeyInner,
    next_move: GameMove,
) -> Result<(), &'static str> {
    let xpubkey = sk.lock().await.ok_or("No Key Selected")?;
    let msgdb = db.get().await.map_err(|_e| "No DB Available")?;
    let mut handle = msgdb.get_handle_all().await;
    spawn_blocking(move || {
        // Seek the last game move -- in *most* cases should be the immediate prior
        // message, but this isn't quite ideal.
        let last = {
            let mut h = None;
            loop {
                let tip = if let Some(prev) = h {
                    let mut v = handle
                        .messages_by_hash::<_, _, ParticipantAction>([prev].iter())
                        .map_err(|e| {
                            tracing::trace!(error=?e, "Error Finding Predecessor");
                            "No Tip Found"
                        })?;
                    v.pop().unwrap()
                } else {
                    handle
                        .get_tip_for_user_by_key::<ParticipantAction>(xpubkey)
                        .map_err(|e| {
                            tracing::trace!(error=?e, "Error First Tip");
                            "No Tip Found"
                        })?
                };
                match tip.msg() {
                    ParticipantAction::MoveEnvelope(m) => break m.sequence,
                    ParticipantAction::PsbtSigningCoordination(_)
                    | ParticipantAction::Custom(_) => {
                        if tip.header().ancestors().is_none() {
                            return Err("No MoveEnvelope Found");
                        }
                        h = tip.header().ancestors().map(|a| a.prev_msg())
                    }
                }
            }
        };
        let mve = MoveEnvelope {
            d: Unsanitized(next_move),
            sequence: last + 1,
            time_millis: attest_util::now() as u64,
        };
        let keys = handle.get_keymap().or(Err("Could not get keys"))?;
        let sk = keys.get(&xpubkey).ok_or("Unknown Secret Key for PK")?;
        let keypair = KeyPair::from_secret_key(&secp, sk);
        // TODO: Runa tipcache
        handle
            .retry_insert_authenticated_envelope_atomic::<ParticipantAction, _, _>(
                mve,
                &keypair,
                &secp,
                None,
                TipControl::AllTips,
            )
            .or(Err("Could Not Wrap/Insert Message"))
            .into()
    })
    .await
    .or(Err("DB Error"))?
}

pub(crate) async fn switch_to_game_inner(
    secp: Arc<Secp256k1<All>>,
    singing_key: SigningKeyInner,
    db: Database,
    game: GameState<'_>,
    key: XOnlyPublicKey,
) -> Result<(), ()> {
    info!(?key, "Switching to Sequencer Key");
    let game = game.inner().clone();
    spawn(async move {
        info!("Spawned Game switching Task");
        let genesis = {
            let db = db.state.lock().await;
            let db: &DatabaseInner = db.as_ref().ok_or("No Database Set Up")?;
            let handle = db.db.get_handle_read().await;
            spawn_blocking(move || {
                handle
                    .get_message_at_height_for_user::<Channelized<BroadcastByHost>>(key, 0)
                    .map_err(|e| "Internal Databse Error")?
                    .ok_or("No Genesis found for selected Key")
            })
            .await
            .or(Err("DB Panic"))?
        };
        tracing::trace!(?genesis, "Found Genesis");
        let game_setup = {
            let m: &Channelized<BroadcastByHost> = genesis.msg();
            match &m.data {
                BroadcastByHost::GameSetup(g) => g,
                _ => return Err("First Message was not a GameSetup"),
            }
        };
        tracing::trace!(?game_setup, "Found GameSetup");

        let game2 = game.clone();
        let mut g = game2.lock().await;
        match &*g {
            GameInitState::Game(g) => {
                g.server.as_ref().map(|s| s.shutdown());
            }
            GameInitState::Pending(_) | GameInitState::None => {}
        }
        let new_game = Game {
            board: GameBoard::new(game_setup),
            host_key: key,
            server: None,
        };
        *g = GameInitState::Game(new_game);
        GameServer::start(secp, singing_key, db, g, game).await?;
        Ok::<(), &'static str>(())
    });
    Ok(())
}

pub(crate) async fn set_signing_key_inner(
    s: GameState<'_>,
    selected: Option<XOnlyPublicKey>,
    sk: State<'_, SigningKeyInner>,
) -> Result<(), ()> {
    info!(?selected, "Selecting Key");
    {
        let mut l = sk.inner().lock().await;
        *l = selected;
    }

    Ok(())
}
