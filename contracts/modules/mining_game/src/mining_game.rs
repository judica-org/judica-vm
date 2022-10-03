// Copyright Judica, Inc 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! Payment Pool Contract for Sapio Studio Advent Calendar Entry
use crate::sapio_base::Clause;
use bitcoin::hashes::sha256;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::secp256k1::SecretKey;
use bitcoin::util::amount::Amount;
use bitcoin::XOnlyPublicKey;
use game_sequencer::ExtractedMoveEnvelopes;
use mine_with_friends_board::game::FinishReason;
use mine_with_friends_board::game::GameBoard;
use mine_with_friends_board::game::GameSetup;
use mine_with_friends_board::game::MoveRejectReason;
use sapio::contract::object::ObjectMetadata;
use sapio::contract::*;
use sapio::*;
use sapio_base::timelocks::RelHeight;
use sapio_wasm_plugin::optional_logo;
use sapio_wasm_plugin::REGISTER;
use schemars::*;
use serde::*;
use simps::GameKernel;
use simps::GameStarted as ExtGameStarted;
use simps::PK;
use std::str::FromStr;

#[derive(Deserialize, JsonSchema)]
pub struct GameStarted {
    pub kernel: GameKernel,
}

// Help ensure that types stay synced
impl From<ExtGameStarted> for GameStarted {
    fn from(g: ExtGameStarted) -> Self {
        GameStarted { kernel: g.kernel }
    }
}
impl From<GameStarted> for ExtGameStarted {
    fn from(g: GameStarted) -> Self {
        ExtGameStarted { kernel: g.kernel }
    }
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

    fn get_finished_board(
        &self,
        trace: ExtractedMoveEnvelopes,
    ) -> Result<(FinishReason, GameBoard), GameBoard> {
        let mut game = GameBoard::new(&GameSetup {
            players: self
                .kernel
                .players
                .keys()
                .map(|PK(k)| k.to_string())
                .collect(),
            // TODO: Should this be something else?
            start_amount: 100_000_000,
            finish_time: self.kernel.timeout,
        });

        for (mv, pk) in trace.0 {
            match game.play(mv, pk.to_string()) {
                Ok(()) => {}
                Err(MoveRejectReason::GameIsFinished(g)) => return Ok((g, game)),
                _ => continue,
            }
        }
        Err(game)
    }
    #[continuation(
        web_api,
        coerce_args = "coerce_players_win",
        guarded_by = "[Self::all_players_signed]"
    )]
    fn game_end_players_win(self, ctx: Context, game_trace: Option<ExtractedMoveEnvelopes>) {
        match game_trace {
            None => empty(),
            Some(trace) => {
                match self.get_finished_board(trace) {
                    Ok((FinishReason::TimeExpired, game)) => {
                        // calculate payouts for each player
                        let total_bitcoin = ctx.funds();
                        let mut tmpl = ctx.template();
                        let dist = game
                            .get_close_distribution(
                                total_bitcoin.as_sat(),
                                self.kernel.game_host.0.to_string(),
                            )
                            .map_err(|_| {
                                CompilationError::TerminateWith(
                                    "Game Not Finished, Violating Invariant that was finished"
                                        .into(),
                                )
                            })?;
                        for (strkey, coin) in dist {
                            let key = XOnlyPublicKey::from_str(&strkey).map_err(|_| {
                                CompilationError::TerminateWith(format!(
                                    "Corrupt GameBoard, Invalid Key: {}",
                                    strkey
                                ))
                            })?;
                            tmpl = tmpl.add_output(Amount::from_sat(coin), &key, None)?;
                        }
                        tmpl.into()
                    }
                    _ => empty(),
                }
            }
        }
    }

    #[continuation(
        web_api,
        coerce_args = "coerce_players_lose",
        guarded_by = "[Self::all_players_signed]"
    )]
    fn game_end_players_lose(self, ctx: Context, game_trace: Option<ExtractedMoveEnvelopes>) {
        match game_trace {
            None => empty(),
            Some(trace) => {
                match self.get_finished_board(trace) {
                    Ok((FinishReason::DominatingPlayer(id), game)) => {
                        // TODO: verify that one player possesses over 50% of the hash rate, else abort with empty

                        // TODO: if there is a player that possesses that hash rate, allocate assets according to the game
                        // semantics of the players having lost

                        let total_bitcoin = ctx.funds();
                        let mut tmpl = ctx.template();
                        let dist = game
                            .get_close_distribution(
                                total_bitcoin.as_sat(),
                                self.kernel.game_host.0.to_string(),
                            )
                            .map_err(|_| {
                                CompilationError::TerminateWith(
                                    "Game Not Finished, Violating Invariant that was finished"
                                        .into(),
                                )
                            })?;
                        for (strkey, coin) in dist {
                            let key = XOnlyPublicKey::from_str(&strkey).map_err(|_| {
                                CompilationError::TerminateWith(format!(
                                    "Corrupt GameBoard, Invalid Key: {}",
                                    strkey
                                ))
                            })?;
                            tmpl = tmpl.add_output(Amount::from_sat(coin), &key, None)?;
                        }
                        tmpl.into()
                    }
                    _ => empty(),
                }
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
    PlayersWin(ExtractedMoveEnvelopes),
    PlayersLose(ExtractedMoveEnvelopes),
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
) -> Result<Option<ExtractedMoveEnvelopes>, CompilationError> {
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
) -> Result<Option<ExtractedMoveEnvelopes>, CompilationError> {
    match k {
        Some(GameEnd::PlayersLose(ms)) => Ok(Some(ms)),
        Some(_) => Err(CompilationError::ContinuationCoercion(
            "Failed to coerce GameEnd into ExtractedMoveEnvelopes".into(),
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

REGISTER![GameStarted, "logo.png"];
