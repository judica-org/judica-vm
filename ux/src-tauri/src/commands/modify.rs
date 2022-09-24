use crate::tasks::GameServer;
use crate::Database;
use crate::DatabaseInner;
use crate::Game;
use crate::GameState;
use crate::SigningKeyInner;
use attest_database::db_handle::create::TipControl;
use attest_database::generate_new_user;
use game_host_messages::BroadcastByHost;
use game_host_messages::Channelized;
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
use tauri::State;
use tokio::spawn;
use tokio::sync::Notify;

pub(crate) trait ErrToString<E> {
    fn err_to_string(self) -> Result<E, String>;
}

impl<E, T: std::fmt::Debug> ErrToString<E> for Result<E, T> {
    fn err_to_string(self) -> Result<E, String> {
        self.map_err(|e| format!("{:?}", e))
    }
}

pub(crate) async fn make_new_chain_inner(
    nickname: String,
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
) -> Result<String, String> {
    let (kp, next_nonce, genesis) = generate_new_user::<_, ParticipantAction, _>(
        secp.inner(),
        MoveEnvelope {
            d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
            sequence: 0,
            /// The player who is making the move, myst be figured out somewhere...
            time: attest_util::now() as u64,
        },
    )
    .err_to_string()?;
    let msgdb = db.get().await.err_to_string()?;
    let mut handle = msgdb.get_handle().await;
    // TODO: Transaction?
    handle.save_keypair(kp).err_to_string()?;
    let k = kp.public_key().x_only_public_key().0;
    handle.save_nonce_for_user_by_key(next_nonce, secp.inner(), k);
    handle.insert_user_by_genesis_envelope(
        nickname,
        genesis.self_authenticate(secp.inner()).err_to_string()?,
    );
    Ok(k.to_hex())
}

pub(crate) async fn make_move_inner_inner(
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
    sk: State<'_, SigningKeyInner>,
    next_move: GameMove,
    _from: EntityID,
) -> Result<(), &'static str> {
    let xpubkey = sk.inner().lock().await.ok_or("No Key Selected")?;
    let msgdb = db.get().await.map_err(|_e| "No DB Available")?;
    let mut handle = msgdb.get_handle().await;
    // Seek the last game move -- in *most* cases should be the immediate prior
    // message, but this isn't quite ideal.
    let last = {
        let mut h = None;
        loop {
            let tip = if let Some(prev) = h {
                let mut v = handle
                    .messages_by_hash::<_, _, ParticipantAction>([prev].iter())
                    .or(Err("No Tip Found"))?;
                v.pop().unwrap()
            } else {
                handle
                    .get_tip_for_user_by_key::<ParticipantAction>(xpubkey)
                    .or(Err("No Tip Found"))?
            };
            match tip.msg() {
                ParticipantAction::MoveEnvelope(m) => break m.sequence,
                ParticipantAction::PsbtSigningCoordination(_) | ParticipantAction::Custom(_) => {
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
        time: attest_util::now() as u64,
    };
    let keys = handle.get_keymap().or(Err("Could not get keys"))?;
    let sk = keys.get(&xpubkey).ok_or("Unknown Secret Key for PK")?;
    let keypair = KeyPair::from_secret_key(secp.inner(), sk);
    // TODO: Runa tipcache
    let msg = handle
        .wrap_message_in_envelope_for_user_by_key::<_, ParticipantAction, _>(
            mve,
            &keypair,
            secp.inner(),
            None,
            None,
            TipControl::AllTips,
        )
        .or(Err("Could Not Wrap Message"))?
        .or(Err("Signing Failed"))?;
    let authenticated = msg
        .self_authenticate(secp.inner())
        .ok()
        .ok_or("Signature Incorrect")?;
    let _ = handle
        .try_insert_authenticated_envelope(authenticated)
        .ok()
        .ok_or("Could Not Insert Message")?;
    Ok::<(), _>(())
}

pub(crate) async fn switch_to_game_inner(
    db: State<'_, Database>,
    game: GameState<'_>,
    key: XOnlyPublicKey,
) -> Result<(), ()> {
    let db = db.inner().clone();
    let game = game.inner().clone();
    spawn(async move {
        let genesis = {
            let db = db.state.lock().await;
            let db: &DatabaseInner = db.as_ref().ok_or("No Database Set Up")?;
            let handle = db.db.get_handle().await;
            handle
                .get_message_at_height_for_user::<Channelized<BroadcastByHost>>(key, 0)
                .map_err(|e| "Internal Databse Error")?
                .ok_or("No Genesis found for selected Key")?
        };
        let game_setup = {
            let m: &Channelized<BroadcastByHost> = genesis.msg();
            match &m.data {
                BroadcastByHost::GameSetup(g) => g,
                _ => return Err("First Message was not a GameSetup"),
            }
        };

        let game2 = game.clone();
        let mut g = game2.lock().await;
        g.as_mut()
            .map(|game| game.server.as_ref().map(|s| s.shutdown()));
        let new_game = Game {
            board: GameBoard::new(game_setup),
            should_notify: Arc::new(Notify::new()),
            host_key: key,
            server: None,
        };
        *g = Some(new_game);
        GameServer::start(&db, g, game).await?;
        Ok::<(), &'static str>(())
    });
    Ok(())
}

pub(crate) async fn set_signing_key_inner(
    s: GameState<'_>,
    selected: Option<XOnlyPublicKey>,
    sk: State<'_, SigningKeyInner>,
) -> Result<(), ()> {
    tracing::info!(?selected, "Selecting Key");
    {
        let mut l = sk.inner().lock().await;
        *l = selected;
    }
    {
        let l = s.lock().await;
        if let Some(g) = l.as_ref() {
            g.should_notify.notify_one()
        }
    }

    Ok(())
}
