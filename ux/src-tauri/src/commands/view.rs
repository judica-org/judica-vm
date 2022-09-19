use crate::{Database, Game, GameState, PrintOnDrop, SigningKeyInner};

use mine_with_friends_board::nfts::{sale::UXForSaleList, NftPtr, UXPlantData};
use sapio_bitcoin::XOnlyPublicKey;

use tauri::{State, Window};
use tokio::sync::futures::Notified;
use tracing::info;

pub(crate) async fn game_synchronizer_inner(
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
        let signing_key: Option<_> = *signing_key.inner().lock().await;
        let (gamestring, wait_on, key, chat_log, user_inventory) = {
            let mut game = s.inner().lock().await;
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
            let user_inventory = game
                .as_mut()
                .map(|g| g.board.get_ux_user_inventory(signing_key.unwrap().to_string()))
                .unwrap()
                .unwrap();
            (
                s,
                w,
                game.as_ref().map(|g| g.host_key),
                chat_log,
                user_inventory,
            )
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
       
        window.emit("chat-log", chat_log);
        window.emit("signing-key", signing_key);
        window.emit("host-key", key).unwrap();
        window.emit("user-keys", user_keys).unwrap();
        window.emit("db-connection", (appName, prefix)).unwrap();
        window.emit("game-board", gamestring).unwrap();
        window.emit("materials-price-data", raw_price_data).unwrap();
        window.emit("power-plants", power_plants).unwrap();
        window.emit("energy-exchange", listings.listings).unwrap();
        window.emit("user-inventory", user_inventory).unwrap();
        if let Some(w) = wait_on {
            w.await;
        } else {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}

pub(crate) async fn list_my_users_inner(
    db: State<'_, Database>,
) -> Result<Vec<(XOnlyPublicKey, String)>, ()> {
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
