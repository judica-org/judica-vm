use crate::{
    tasks::GameServer, Database, DatabaseInner, Game, GameState, PrintOnDrop, SigningKeyInner,
};
use attest_database::{db_handle::create::TipControl, generate_new_user};
use game_host_messages::{BroadcastByHost, Channelized};
use mine_with_friends_board::{
    entity::EntityID,
    game::{
        game_move::{Chat, GameMove, Heartbeat, PurchaseNFT, Trade},
        GameBoard,
    },
    nfts::{sale::UXForSaleList, NftPtr, UXPlantData},
    sanitize::Unsanitized,
    MoveEnvelope,
};
use sapio_bitcoin::{
    hashes::hex::ToHex,
    secp256k1::{All, Secp256k1},
    KeyPair, XOnlyPublicKey,
};
use schemars::{schema::RootSchema, schema_for};
use std::{path::PathBuf, sync::Arc};
use tauri::{State, Window};
use tokio::{
    spawn,
    sync::{futures::Notified, Notify},
};
use tracing::info;
pub mod bindings;

async fn game_synchronizer_inner(
    window: Window,
    s: GameState<'_>,
    d: State<'_, Database>,
    signing_key: State<'_, SigningKeyInner>,
) -> Result<(), ()> {
    info!("Registering Window for State Updates");
    let _p = PrintOnDrop("Registration Canceled".into());
    loop {
        // No Idea why the borrow checker likes this, but it seems to be the case
        // that because the notified needs to live inside the async state machine
        // hapily, giving a stable reference to it tricks the compiler into thinking
        // that the lifetime is 'static and we can successfully wait on it outside
        // the lock.
        let mut arc_cheat = None;
        let (gamestring, wait_on, key, chat_log) = {
            let game = s.inner().lock().await;
            let s = game
                .as_ref()
                .map(|g| serde_json::to_string(&g.board).unwrap_or("null".into()))
                .unwrap_or("null".into());
            arc_cheat = game.as_ref().map(|g: &Game| g.should_notify.clone());
            let w: Option<Notified> = arc_cheat.as_ref().map(|x| x.notified());
            let chat_log = game
                .as_ref()
                .map(|g| g.board.get_ux_chat_log())
                .unwrap_or_default();
            (s, w, game.as_ref().map(|g| g.host_key), chat_log)
        };
        let (appName, prefix, list_of_chains, user_keys) = {
            let l = d.inner().state.lock().await;
            if let Some(g) = l.as_ref() {
                let handle = g.db.get_handle().await;
                let v = handle.get_all_users().map_err(|_| ())?;
                let keys: Vec<XOnlyPublicKey> = handle.get_keymap().unwrap().into_keys().collect();
                (g.name.clone(), g.prefix.clone(), v, keys)
            } else {
                ("".into(), None, vec![], vec![])
            }
        };

        // TODO: move these under a single held game lock
        // Attempt to get data to show prices
        let raw_price_data = {
            let mut game = s.inner().lock().await;
            let p = game
                .as_mut()
                .map(|g| g.board.get_ux_materials_prices())
                .unwrap_or(Ok(Vec::new()))
                .unwrap();
            p
        };

        let power_plants = {
            let mut game = s.inner().lock().await;
            let plants: Vec<(NftPtr, UXPlantData)> = game
                .as_mut()
                .map(|g| g.board.get_ux_power_plant_data())
                .unwrap_or_else(Vec::new);
            plants
        };

        let listings = {
            let game = s.inner().lock().await;
            let listings = game
                .as_ref()
                .map(|g| g.board.get_ux_energy_market())
                .unwrap_or(Ok(UXForSaleList {
                    listings: Vec::new(),
                }))
                .unwrap();
            listings
        };

        info!("Emitting State Updates");
        window.emit("available-sequencers", list_of_chains);
        let signing_key: Option<_> = *signing_key.inner().lock().await;
        window.emit("chat-log", chat_log);
        window.emit("signing-key", signing_key);
        window.emit("host-key", key).unwrap();
        window.emit("user-keys", user_keys).unwrap();
        window.emit("db-connection", (appName, prefix)).unwrap();
        window.emit("game-board", gamestring).unwrap();
        window.emit("materials-price-data", raw_price_data).unwrap();
        window.emit("power-plants", power_plants).unwrap();
        window.emit("energy-exchange", listings.listings).unwrap();
        if let Some(w) = wait_on {
            w.await;
        } else {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}

async fn list_my_users_inner(db: State<'_, Database>) -> Result<Vec<(XOnlyPublicKey, String)>, ()> {
    let msgdb = db.get().await.map_err(|_| ())?;
    let handle = msgdb.get_handle().await;
    let keys = handle.get_keymap().map_err(|_| ())?;
    let users = keys
        .keys()
        .map(|key| handle.locate_user(key))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ())?;
    let ret: Vec<(XOnlyPublicKey, String)> = users
        .iter()
        .zip(keys.keys())
        .map(|((_a, b), k)| (*k, b.clone()))
        .collect();
    Ok(ret)
}

pub(crate) trait ErrToString<E> {
    fn err_to_string(self) -> Result<E, String>;
}

impl<E, T: std::fmt::Debug> ErrToString<E> for Result<E, T> {
    fn err_to_string(self) -> Result<E, String> {
        self.map_err(|e| format!("{:?}", e))
    }
}

async fn make_new_chain_inner(
    nickname: String,
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
) -> Result<String, String> {
    let (kp, next_nonce, genesis) = generate_new_user(
        secp.inner(),
        Some(MoveEnvelope {
            d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
            sequence: 0,
            /// The player who is making the move, myst be figured out somewhere...
            time: attest_util::now() as u64,
        }),
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

async fn make_move_inner_inner(
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
    sk: State<'_, SigningKeyInner>,
    next_move: GameMove,
    _from: EntityID,
) -> Result<(), &'static str> {
    let xpubkey = sk.inner().lock().await.ok_or("No Key Selected")?;
    let msgdb = db.get().await.map_err(|_e| "No DB Available")?;
    let mut handle = msgdb.get_handle().await;
    let tip = handle
        .get_tip_for_user_by_key(xpubkey)
        .or(Err("No Tip Found"))?;
    let last: MoveEnvelope = serde_json::from_value(tip.msg().to_owned().into())
        .or(Err("Could not Deserialized Old Tip"))?;
    let mve = MoveEnvelope {
        d: Unsanitized(next_move),
        sequence: last.sequence + 1,
        time: attest_util::now() as u64,
    };
    let v = ruma_serde::to_canonical_value(mve).or(Err("Could Not Canonicalize new Enveloper"))?;
    let keys = handle.get_keymap().or(Err("Could not get keys"))?;
    let sk = keys.get(&xpubkey).ok_or("Unknown Secret Key for PK")?;
    let keypair = KeyPair::from_secret_key(secp.inner(), sk);
    // TODO: Runa tipcache
    let msg = handle
        .wrap_message_in_envelope_for_user_by_key(
            v,
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

async fn switch_to_game_inner(
    db: State<'_, Database>,
    game: GameState<'_>,
    key: XOnlyPublicKey,
) -> Result<(), ()> {
    let db = db.inner().clone();
    let game = game.inner().clone();
    spawn(async move {
        let game_setup = {
            let db = db.state.lock().await;
            let db: &DatabaseInner = db.as_ref().ok_or("No Database Set Up")?;
            let handle = db.db.get_handle().await;
            let genesis = handle
                .get_message_at_height_for_user(key, 0)
                .map_err(|_| "No Genesis found for selected Key")?;
            if let Ok(Channelized {
                data: BroadcastByHost::GameSetup(g),
                channel: _,
            }) = serde_json::from_value(genesis.msg().to_owned().into())
            {
                g
            } else {
                return Err("First Message was not a GameSetup");
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

async fn set_signing_key_inner(
    s: GameState<'_>,
    selected: Option<XOnlyPublicKey>,
    sk: State<'_, SigningKeyInner>,
) -> Result<(), ()> {
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
