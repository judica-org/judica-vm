// Copyright Judica, Inc 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! Payment Pool Contract for Sapio Studio Advent Calendar Entry
#[deny(missing_docs)]
use crate::sapio_base::Clause;

use bitcoin::hashes::sha256;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::secp256k1::SecretKey;
use bitcoin::util::amount::Amount;

use bitcoin::XOnlyPublicKey;
use mine_with_friends_board::game::GameBoard;
use mine_with_friends_board::MoveEnvelope;
use sapio::contract::object::ObjectMetadata;
use sapio::contract::*;
use sapio::util::amountrange::AmountF64;
use sapio::*;
use sapio_base::simp::{CompiledObjectLT, SIMPAttachableAt, SIMP};
use sapio_base::timelocks::RelHeight;
use schemars::*;
use serde::*;
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
struct PK(#[schemars(with = "sha256::Hash")] XOnlyPublicKey);

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
struct GameKernel {
    #[schemars(with = "sha256::Hash")]
    game_host: PK,
    players: BTreeMap<PK, AmountF64>,
    timeout: u64,
}
impl GameKernel {}
impl SIMP for GameKernel {
    fn get_protocol_number(&self) -> i64 {
        -119
    }
    fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value::<Self>(self.clone())
    }
    fn from_json(v: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(v)
    }
}
impl SIMPAttachableAt<CompiledObjectLT> for GameKernel {}

struct GameStarted {
    kernel: GameKernel,
}
impl GameStarted {
    #[guard]
    fn all_players_signed(self, _ctx: Context) {
        Clause::And(
            self.kernel
                .players
                .iter()
                .map(|x| Clause::Key(x.0 .0.clone()))
                .collect(),
        )
    }

    #[guard]
    fn degraded_quorum(self, _ctx: Context) {
        let degrade_interval = 6; // every hour
        let total = self.kernel.players.len();
        let keys: Vec<Clause> = self
            .kernel
            .players
            .keys()
            .map(|x| Clause::Key(x.0))
            .collect();
        Clause::Or(
            // with host
            (total - 1..1)
                .map(|n| {
                    (
                        2 * n,
                        Clause::And(vec![
                            RelHeight::from(((2 * (total - n) - 1) * degrade_interval) as u16)
                                .into(),
                            Clause::Key(self.kernel.game_host.0),
                            Clause::Threshold(n, keys.clone()),
                        ]),
                    )
                })
                // without host
                .chain((total - 1..1).map(|n| {
                    (
                        2 * n - 1,
                        Clause::And(vec![
                            RelHeight::from((2 * (total - n) * degrade_interval) as u16).into(),
                            Clause::Threshold(n, keys.clone()),
                        ]),
                    )
                }))
                .collect(),
        )
    }

    #[continuation(
        web_api,
        coerce_args = "coerce_host_key",
        guarded_by = "[Self::all_players_signed]"
    )]
    fn host_cheat_equivocate(self, ctx: Context, proof: Option<HostKey>) {
        match proof {
            Some(k) => {
                let secp = Secp256k1::new();
                if k.0.x_only_public_key(&secp).0 == self.kernel.game_host.0 {
                    let mut tmpl = ctx.template();
                    for (player, balance) in &self.kernel.players {
                        tmpl = tmpl.add_output(balance.clone().into(), &player.0, None)?
                    }
                    tmpl.into()
                } else {
                    Err(CompilationError::Custom(
                        "The Secret Key Provided does not match the Public Key of the Game Host"
                            .into(),
                    ))
                }
            }
            None => empty(),
        }
    }

    #[continuation(
        web_api,
        coerce_args = "coerce_censorship_proof",
        guarded_by = "[Self::all_players_signed]"
    )]
    fn host_cheat_censor(self, _ctx: Context, _proof: Option<CensorshipProof>) {
        todo!()
    }

    #[continuation(
        web_api,
        coerce_args = "coerce_players_win",
        guarded_by = "[Self::all_players_signed]"
    )]
    fn game_end_players_win(self, ctx: Context, game_trace: Option<MoveSequence>) {
        match game_trace {
            None => empty(),
            Some(trace) => {
                let mut game = GameBoard::new();
                for (mv, pk) in trace.sequence {
                    match game.play(mv, pk) {
                        Ok(()) => {
                            continue;
                        }
                        Err(()) => {
                            return Err(CompilationError::TerminateWith(
                                "GameBoard corrupted".into(),
                            ));
                        }
                    }
                }
                // validate that the game has actually timed out
                if game.current_time() < self.kernel.timeout {
                    return empty();
                }

                // calculate payouts for each player
                let total_bitcoin = ctx.funds();
                let mut tmpl = ctx.template();
                let (total_game_coin, users) = game.user_shares();
                let user_data = game.user_data();
                for (eid, game_coin) in users {
                    let total_bitcoin_atomic = total_bitcoin.as_sat() as u128;
                    let player_share = Amount::from_sat(
                        ((game_coin * total_bitcoin_atomic) as f64 / total_game_coin as f64) as u64,
                    );
                    let player_key = match user_data.get(&eid) {
                        None => {
                            // TODO: this is possibly a corruption event and warrants aborting?
                            // If this has occurred it means there is a user in the user shares map
                            // that is absent from the key map. This implies inconsistent state.
                            continue;
                        }
                        Some(ud) => match XOnlyPublicKey::from_str(&ud.key) {
                            Err(_) => {
                                return Err(CompilationError::TerminateWith(
                                    "GameBoard corrupted: invalid user key".into(),
                                ));
                            }
                            Ok(pk) => pk,
                        },
                    };
                    tmpl = tmpl.add_output(player_share, &player_key, None)?;
                }
                tmpl.into()
            }
        }
    }

    #[continuation(
        web_api,
        coerce_args = "coerce_players_lose",
        guarded_by = "[Self::all_players_signed]"
    )]
    fn game_end_players_lose(self, _ctx: Context, game_trace: Option<MoveSequence>) {
        match game_trace {
            None => empty(),
            Some(trace) => {
                let mut game = GameBoard::new();
                for (mv, pk) in trace.sequence {
                    match game.play(mv, pk) {
                        Ok(()) => {
                            continue;
                        }
                        Err(()) => {
                            return Err(CompilationError::TerminateWith(
                                "GameBoard corrupted".into(),
                            ));
                        }
                    }
                }

                // TODO: verify that one player possesses over 50% of the hash rate, else abort with empty

                // TODO: if there is a player that possesses that hash rate, allocate assets according to the game
                // semantics of the players having lost
                todo!();
            }
        }
    }

    #[continuation(
        web_api,
        coerce_args = "coerce_degrade",
        guarded_by = "[Self::degraded_quorum]"
    )]
    fn degrade(self, ctx: Context, unit: Option<()>) {
        match unit {
            None => empty(),
            Some(_) => {
                let mut tmpl = ctx.template();
                for (k, v) in self.kernel.players.iter() {
                    tmpl = tmpl.add_output(v.clone().into(), &k.0, None)?;
                }
                tmpl.into()
            }
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MoveSequence {
    sequence: Vec<(MoveEnvelope, String)>,
}

#[derive(JsonSchema)]
struct GameStart {
    #[serde(with = "Vec::<sha256::Hash>")]
    players: Vec<XOnlyPublicKey>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct HostKey(SecretKey);
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CensorshipProof {}

pub enum GameEnd {
    HostCheatEquivocate(HostKey),
    HostCheatCensor(CensorshipProof),
    PlayersWin(MoveSequence),
    PlayersLose(MoveSequence),
    Degrade,
}

impl Contract for GameStarted {
    declare! {
        updatable<Option<GameEnd>>,
        Self::host_cheat_equivocate,
        Self::host_cheat_censor,
        Self::game_end_players_win,
        Self::game_end_players_lose,
        Self::degrade
    }

    fn metadata(&self, _ctx: Context) -> Result<object::ObjectMetadata, CompilationError> {
        Ok(ObjectMetadata::default().add_simp(self.kernel.clone())?)
    }
}

// Coercions
fn coerce_host_key(
    k: <GameStarted as Contract>::StatefulArguments,
) -> Result<Option<HostKey>, CompilationError> {
    match k {
        Some(GameEnd::HostCheatEquivocate(x)) => Ok(Some(x)),
        Some(_) => Err(CompilationError::ContinuationCoercion(
            "Failed to coerce GameEnd into HostKey".into(),
        )),
        None => Ok(None),
    }
}

fn coerce_censorship_proof(
    k: <GameStarted as Contract>::StatefulArguments,
) -> Result<Option<CensorshipProof>, CompilationError> {
    match k {
        Some(GameEnd::HostCheatCensor(x)) => Ok(Some(x)),
        Some(_) => Err(CompilationError::ContinuationCoercion(
            "Failed to coerce GameEnd into CensorshipProof".into(),
        )),
        None => Ok(None),
    }
}

fn coerce_players_win(
    k: <GameStarted as Contract>::StatefulArguments,
) -> Result<Option<MoveSequence>, CompilationError> {
    match k {
        Some(GameEnd::PlayersWin(ms)) => Ok(Some(ms)),
        Some(_) => Err(CompilationError::ContinuationCoercion(
            "Failed to coerce GameEnd into PlayersWin".into(),
        )),
        None => Ok(None),
    }
}

fn coerce_players_lose(
    k: <GameStarted as Contract>::StatefulArguments,
) -> Result<Option<MoveSequence>, CompilationError> {
    match k {
        Some(GameEnd::PlayersLose(ms)) => Ok(Some(ms)),
        Some(_) => Err(CompilationError::ContinuationCoercion(
            "Failed to coerce GameEnd into MoveSequence".into(),
        )),
        None => Ok(None),
    }
}

fn coerce_degrade(
    k: <GameStarted as Contract>::StatefulArguments,
) -> Result<Option<()>, CompilationError> {
    match k {
        Some(GameEnd::Degrade) => Ok(Some(())),
        Some(_) => Err(CompilationError::ContinuationCoercion(
            "Failed to coerce GameEnd into Degrade".into(),
        )),
        None => Ok(None),
    }
}
