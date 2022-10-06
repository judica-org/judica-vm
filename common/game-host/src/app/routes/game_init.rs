//! Module Game Init defineds HTTP Handlers for registering and creating a new game.
//!
//! It contains hacks nescessary for an MVP.
//!
//! Basic Flow is:
//!
//! create_new_game_instance() -> NewGame{password: JoinCode, join: JoinCode}
//!
//! for each player:
//!     add_player(join, genesis envelope) -> ()
//!
//! finish_setup(password, join, ... params) -> CreatedNewChain {genesis: Envelope, name: String}

use crate::app::{create_new_attestation_chain, CreatedNewChain};
use attest_database::connection::MsgDB;
use attest_messages::{AuthenticationError, GenericEnvelope};
use axum::{
    http::{Response, StatusCode},
    Extension, Json,
};
use game_player_messages::ParticipantAction;
use mine_with_friends_board::{
    game::{game_move::GameMove, GameSetup},
    sanitize::Unsanitized,
    MoveEnvelope,
};
use sapio_bitcoin::{
    hashes::hex::{FromHex, ToHex},
    secp256k1::{
        rand::{thread_rng, Rng},
        All, Secp256k1,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    error::Error,
    fmt::{Debug, Display},
    sync::{Arc, Weak},
};
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Hash, Clone, Copy)]
#[serde(into = "String")]
#[serde(try_from = "String")]
pub struct JoinCode([u8; 16]);
impl TryFrom<String> for JoinCode {
    type Error = sapio_bitcoin::hashes::hex::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        FromHex::from_hex(&value).map(JoinCode)
    }
}
impl From<JoinCode> for String {
    fn from(s: JoinCode) -> Self {
        s.0.to_hex()
    }
}
impl JoinCode {
    fn new() -> Self {
        let mut rng = thread_rng();
        JoinCode(rng.gen())
    }
}

impl Default for JoinCode {
    fn default() -> Self {
        Self::new()
    }
}

struct Metadata {
    state: Mutex<GameStartingState>,
    code: Arc<JoinCode>,
    admin: JoinCode,
}

type MetadataRc = Arc<Metadata>;
type MetadataWRc = Weak<Metadata>;
enum GameStartingState {
    AddingPlayers(Vec<GenericEnvelope<ParticipantAction>>),
    WaitingForSetup(Vec<GenericEnvelope<ParticipantAction>>),
    Setup(Vec<GenericEnvelope<ParticipantAction>>, GameSetup),
}
pub struct NewGameDB {
    states: HashMap<Arc<JoinCode>, MetadataRc>,
    by_time: VecDeque<MetadataWRc>,
}

impl NewGameDB {
    pub fn new() -> NewGameDB {
        Self {
            states: HashMap::with_capacity(1000),
            by_time: VecDeque::with_capacity(1000),
        }
    }
}

impl NewGameDB {
    fn add_new_game(&mut self) -> (JoinCode, Arc<JoinCode>) {
        let m = Arc::new(Metadata {
            state: Mutex::new(GameStartingState::new()),
            code: Arc::new(Default::default()),
            admin: Default::default(),
        });
        let code = (m.admin, m.code.clone());

        if self.by_time.capacity() == self.by_time.len() {
            if let Some(remove) = self.by_time.pop_front() {
                if let Some(remove) = remove.upgrade() {
                    self.states.remove(&remove.code);
                }
            }
        }

        self.by_time.push_back(Arc::downgrade(&m));
        self.states.insert(m.code.clone(), m);
        code
    }
}
const MAX_PLAYERS: usize = 10;

#[derive(Debug, Serialize, Deserialize)]
enum AddPlayerError {
    AlreadySetup,
    NoMorePlayers,
    NotGenesisEnvelope,
    WrongFirstMessage,
}
impl Display for AddPlayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}
impl Error for AddPlayerError {}
impl GameStartingState {
    fn new() -> GameStartingState {
        GameStartingState::AddingPlayers(vec![])
    }
    fn add_player(&mut self, p: GenericEnvelope<ParticipantAction>) -> Result<(), AddPlayerError> {
        if p.get_genesis_hash() != p.canonicalized_hash_ref() {
            return Err(AddPlayerError::NotGenesisEnvelope);
        }
        match p.msg() {
            ParticipantAction::MoveEnvelope(MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(_)),
                sequence,
                time_millis,
            }) => {}
            _ => return Err(AddPlayerError::WrongFirstMessage),
        };
        match self {
            GameStartingState::AddingPlayers(ref mut v) => {
                v.push(p);
                if v.len() == MAX_PLAYERS as usize {
                    let mut clr = vec![];
                    std::mem::swap(&mut clr, v);
                    *self = GameStartingState::WaitingForSetup(clr);
                    Ok(())
                } else {
                    Ok(())
                }
            }
            GameStartingState::Setup(_, _) => Err(AddPlayerError::AlreadySetup),
            GameStartingState::WaitingForSetup(_) => Err(AddPlayerError::NoMorePlayers),
        }
    }
    fn finalize_setup(
        &mut self,
        finish_time: u64,
        start_amount: u64,
    ) -> Result<(), AddPlayerError> {
        match self {
            GameStartingState::AddingPlayers(v) | GameStartingState::WaitingForSetup(v) => {
                let players = v.iter().map(|i| i.header().key().to_string()).collect();
                let game = GameSetup {
                    players,
                    start_amount,
                    finish_time,
                };

                let mut clr = vec![];
                std::mem::swap(&mut clr, v);
                *self = GameStartingState::Setup(clr, game);
                Ok(())
            }
            GameStartingState::Setup(_, _) => Err(AddPlayerError::AlreadySetup),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct NewGame {
    password: JoinCode,
    join: JoinCode,
}
pub async fn create_new_game_instance(
    Extension(db): Extension<Arc<Mutex<NewGameDB>>>,
) -> Result<(Response<()>, Json<NewGame>), (StatusCode, &'static str)> {
    let code = db.lock().await.add_new_game();
    let new = NewGame {
        password: code.0,
        join: *code.1,
    };
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(new),
    ))
}

pub async fn add_player(
    Json((code, envelope)): Json<(JoinCode, GenericEnvelope<ParticipantAction>)>,
    Extension(db): Extension<Arc<Mutex<NewGameDB>>>,
) -> Result<(Response<()>, Json<()>), (StatusCode, String)> {
    if let Some(v) = db.lock().await.states.get(&code) {
        v.state
            .lock()
            .await
            .add_player(envelope)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(()),
    ))
}

#[derive(Deserialize)]
pub struct FinishArgs {
    passcode: JoinCode,
    code: JoinCode,
    finish_time: u64,
    start_amount: u64,
}

pub async fn finish_setup(
    msgdb: Extension<MsgDB>,
    secp: Extension<Secp256k1<All>>,
    Json(FinishArgs {
        passcode,
        code,
        finish_time,
        start_amount,
    }): Json<FinishArgs>,
    Extension(db): Extension<Arc<Mutex<NewGameDB>>>,
) -> Result<(Response<()>, Json<CreatedNewChain>), (StatusCode, String)> {
    if let Some(v) = db.lock().await.states.get(&code) {
        if passcode == v.admin {
            let mut game = v.state.lock().await;
            game.finalize_setup(finish_time, start_amount)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            match &*game {
                GameStartingState::AddingPlayers(_) | GameStartingState::WaitingForSetup(_) => {
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, "Unreachable".into()))
                }
                GameStartingState::Setup(envelopes, gs) => {
                    let authed: Vec<_> = envelopes
                        .iter()
                        .map(|e| e.self_authenticate(&secp.0))
                        .collect::<Result<_, AuthenticationError>>()
                        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
                    {
                        let mut handle = msgdb.get_handle().await;
                        for env in authed {
                            handle
                                .try_insert_authenticated_envelope(env, false)
                                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, "".to_string()))?
                                // These errors are OK here
                                .ok();
                        }
                    }
                    return create_new_attestation_chain(
                        Json((
                            envelopes
                                .iter()
                                .map(|m| m.canonicalized_hash_ref())
                                .collect(),
                            gs.clone(),
                        )),
                        msgdb,
                        secp,
                    )
                    .await
                    .map_err(|e| (e.0, e.1.to_owned()));
                }
            }
        } else {
            return Err((StatusCode::UNAUTHORIZED, "Wrong Passcode".into()));
        }
    }

    Err((StatusCode::NOT_FOUND, "No Such Game".into()))
}
