// Copyright Judica, Inc 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! Payment Pool Contract for Sapio Studio Advent Calendar Entry
#[deny(missing_docs)]
use crate::sapio_base::Clause;

use bitcoin::hashes::sha256;
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::ffi::SECP256K1_START_NONE;
use bitcoin::secp256k1::schnorr::Signature;
use bitcoin::secp256k1::Message;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::secp256k1::SecretKey;
use bitcoin::util::amount::Amount;
use bitcoin::Address;

use bitcoin::XOnlyPublicKey;
use mine_with_friends_board::game::game_move::GameMove;
use sapio::contract::actions::conditional_compile::ConditionalCompileType;
use sapio::contract::object::ObjectMetadata;
use sapio::contract::*;
use sapio::util::amountrange::{AmountF64, AmountU64};
use sapio::*;
use sapio_base::simp::SIMP;
use sapio_contrib::contracts::treepay::{Payment, TreePay};
use sapio_wasm_plugin::client::*;
use sapio_wasm_plugin::*;
use schemars::*;
use serde::*;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::io::Write;
use std::marker::PhantomData;
use std::str::FromStr;

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
struct GameKernel {
    #[schemars(with = "sha256::Hash")]
    game_host: XOnlyPublicKey,
    players: BTreeMap<XOnlyPublicKey, AmountF64>,
}
impl GameKernel {}
impl SIMP for GameKernel {
    fn get_protocol_number() -> i64 {
        -119
    }
}

struct GameStarted {
    kernel: GameKernel,
    game_witnesses: Vec<XOnlyPublicKey>,
}
impl GameStarted {
    #[guard]
    fn all_players_signed(self, ctx: Context) {
        Clause::And(
            self.kernel
                .players
                .iter()
                .map(|x| Clause::Key(x.0.clone()))
                .collect(),
        )
    }

    #[continuation(
        coerce_args = "coerce_host_key",
        guarded_by = "[Self::all_players_signed]"
    )]
    fn host_cheat_equivocate(self, ctx: Context, proof: Option<HostKey>) {
        match proof {
            Some(k) => {
                let secp = Secp256k1::new();
                if k.0.x_only_public_key(&secp).0 == self.kernel.game_host {
                    let mut tmpl = ctx.template();
                    for (player, balance) in &self.kernel.players {
                        tmpl = tmpl.add_output(balance.clone().into(), player, None)?
                    }
                    tmpl.into()
                } else {
                    Err(CompilationError::Custom(
                        ("The Secret Key Provided does not match the Public Key of the Game Host"
                            .into()),
                    ))
                }
            }
            None => empty(),
        }
    }

    #[continuation(
        coerce_args = "coerce_censorship_proof",
        guarded_by = "[Self::all_players_signed]"
    )]
    fn host_cheat_censor(self, ctx: Context, proof: Option<CensorshipProof>) {
        todo!()
    }

    #[continuation(coerce_args = "coerce_move_sequence")]
    fn game_end_players_win(self, ctx: Context, game_trace: Option<MoveSequence>) {
        todo!()
    }

    #[continuation(coerce_args = "coerce_move_sequence")]
    fn game_end_players_lose(self, ctx: Context, game_trace: Option<MoveSequence>) {
        todo!()
    }

    #[continuation(coerce_args = "coerce_degrade")]
    fn degrade(self, ctx: Context, _unit: Option<()>) {
        todo!()
    }
}

struct Degraded(usize);

struct MoveSequence {
    sequence: Vec<GameMove>,
    signature_hex: String,
}

struct MiningGame {}

#[derive(JsonSchema)]
struct GameStart {
    #[serde(with = "Vec::<sha256::Hash>")]
    players: Vec<XOnlyPublicKey>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
struct HostKey(SecretKey);
#[derive(Serialize, Deserialize, JsonSchema)]
struct CensorshipProof {}

enum GameEnd {
    HostCheatEquivocate(HostKey),
    HostCheatCensor(CensorshipProof),
    PlayersWin,
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
        Some(_) => Err(todo!()),
        None => Ok(None),
    }
}

fn coerce_censorship_proof(
    k: <GameStarted as Contract>::StatefulArguments,
) -> Result<Option<CensorshipProof>, CompilationError> {
    match k {
        Some(GameEnd::HostCheatCensor(x)) => Ok(Some(x)),
        Some(_) => Err(todo!()),
        None => Ok(None),
    }
}

fn coerce_players_win(
    k: <GameStarted as Contract>::StatefulArguments,
) -> Result<Option<()>, CompilationError> {
    match k {
        Some(GameEnd::PlayersWin) => Ok(Some(())),
        Some(_) => Err(todo!()),
        None => Ok(None),
    }
}

fn coerce_move_sequence(
    k: <GameStarted as Contract>::StatefulArguments,
) -> Result<Option<MoveSequence>, CompilationError> {
    todo!()
}

fn coerce_degrade(
    k: <GameStarted as Contract>::StatefulArguments,
) -> Result<Option<()>, CompilationError> {
    match k {
        Some(GameEnd::Degrade) => Ok(Some(())),
        Some(_) => Err(todo!()),
        None => Ok(None),
    }
}

// #[derive(Deserialize, JsonSchema, Clone)]
// struct PaymentPool {
//     /// # Pool Members
//     /// map of all initial balances as PK to BTC
//     members: BTreeMap<XOnlyPublicKey, AmountF64>,
//     /// The current sequence number (for authenticating state updates)
//     sequence: u64,
//     /// If to require signatures or not (debugging, should be true)
//     sig_needed: bool,
// }

// impl Contract for PaymentPool {
//     declare! {then, Self::ejection}
//     declare! {updatable<DoTx>, Self::do_tx}
// }
// /// Payment Request
// #[derive(Deserialize, JsonSchema, Serialize)]
// struct PaymentRequest {
//     /// # Signature
//     /// hex encoded signature of the fee, sequence number, and payments
//     hex_sig: String,
//     /// # Fees
//     /// Fees for this participant to pay in Satoshis
//     fee: AmountU64,
//     /// # Payments
//     /// Mapping of Address to Bitcoin Amount (btc)
//     payments: BTreeMap<Address, AmountF64>,
// }
// /// New Update message for generating a transaction from.
// #[derive(Deserialize, JsonSchema, Serialize)]
// struct DoTx {
//     /// # Payments
//     /// A mapping of public key in members to signed list of payouts with a fee rate.
//     payments: BTreeMap<XOnlyPublicKey, PaymentRequest>,
// }
// /// required...
// impl Default for DoTx {
//     fn default() -> Self {
//         DoTx {
//             payments: BTreeMap::new(),
//         }
//     }
// }
// impl StatefulArgumentsTrait for DoTx {}

// /// helper for rust type system issue
// fn default_coerce(
//     k: <PaymentPool as Contract>::StatefulArguments,
// ) -> Result<DoTx, CompilationError> {
//     Ok(k)
// }

// impl PaymentPool {
//     /// Sum Up all the balances
//     fn total(&self) -> Amount {
//         self.members
//             .values()
//             .cloned()
//             .map(Amount::from)
//             .fold(Amount::from_sat(0), |a, b| a + b)
//     }
//     /// Only compile an ejection if the pool has other users in it, otherwise
//     /// it's base case.
//     #[compile_if]
//     fn has_eject(self, _ctx: Context) {
//         if self.members.len() > 1 {
//             ConditionalCompileType::Required
//         } else {
//             ConditionalCompileType::Never
//         }
//     }
//     /// Split the pool in two -- users can eject multiple times to fully eject.
//     #[then(compile_if = "[Self::has_eject]")]
//     fn ejection(self, ctx: Context) {
//         let t = ctx.template();
//         let mid = (self.members.len() + 1) / 2;
//         // find the middle
//         let key = self.members.keys().nth(mid).expect("must be present");
//         let mut pool_one: PaymentPool = self.clone();
//         pool_one.sequence += 1;
//         let pool_two = PaymentPool {
//             // removes the back half including key
//             members: pool_one.members.split_off(&key),
//             sequence: self.sequence + 1,
//             sig_needed: self.sig_needed,
//         };
//         let amt_one = pool_one.total();
//         let amt_two = pool_two.total();
//         t.add_output(amt_one, &pool_one, None)?
//             .add_output(amt_two, &pool_two, None)?
//             .into()
//     }

//     /// all signed the transaction!
//     #[guard]
//     fn all_signed(self, _ctx: Context) {
//         Clause::Threshold(
//             self.members.len(),
//             self.members.keys().cloned().map(Clause::Key).collect(),
//         )
//     }
//     /// This Function will create a proposed transaction that is safe to sign
//     /// given a list of data from participants.
//     #[continuation(
//         web_api,
//         guarded_by = "[Self::all_signed]",
//         coerce_args = "default_coerce"
//     )]
//     fn do_tx(self, ctx: Context, update: DoTx) {
//         let _effects = unsafe { ctx.get_effects_internal() };
//         // don't allow empty updates.
//         if update.payments.is_empty() {
//             return empty();
//         }
//         // collect members with updated balances here
//         let mut new_members = self.members.clone();
//         // verification context
//         let secp = Secp256k1::new();
//         // collect all the payments
//         let mut all_payments = vec![];
//         let mut spent = Amount::from_sat(0);
//         // for each payment...
//         for (
//             from,
//             PaymentRequest {
//                 hex_sig,
//                 fee,
//                 payments,
//             },
//         ) in update.payments.iter()
//         {
//             // every from must be in the members
//             let balance = self
//                 .members
//                 .get(from)
//                 .ok_or(CompilationError::TerminateCompilation)?;
//             let new_balance = Amount::from(*balance)
//                 - (payments
//                     .values()
//                     .cloned()
//                     .map(Amount::from)
//                     .fold(Amount::from_sat(0), |a, b| a + b)
//                     + Amount::from(*fee));
//             // check for no underflow
//             if new_balance.as_sat() < 0 {
//                 return Err(CompilationError::TerminateCompilation);
//             }
//             // updates the balance or remove if empty
//             if new_balance.as_sat() > 0 {
//                 new_members.insert(from.clone(), new_balance.into());
//             } else {
//                 new_members.remove(from);
//             }

//             // collect all the payment
//             for (address, amt) in payments.iter() {
//                 spent += Amount::from(*amt);
//                 all_payments.push(Payment {
//                     address: address.clone(),
//                     amount: Amount::from(*amt).into(),
//                 })
//             }
//             // Check the signature for this request
//             // came from this user
//             if self.sig_needed {
//                 let mut hasher = sha256::Hash::engine();
//                 hasher.write(&self.sequence.to_le_bytes());
//                 hasher.write(&Amount::from(*fee).as_sat().to_le_bytes());
//                 for (address, amt) in payments.iter() {
//                     hasher.write(&Amount::from(*amt).as_sat().to_le_bytes());
//                     hasher.write(address.script_pubkey().as_bytes());
//                 }
//                 let h = sha256::Hash::from_engine(hasher);
//                 let m = Message::from_slice(&h.as_inner()[..]).expect("Correct Size");
//                 let sig = Signature::from_str(&hex_sig)
//                     .map_err(|_| CompilationError::TerminateCompilation)?;
//                 let _: () = secp
//                     .verify_schnorr(&sig, &m, &from)
//                     .map_err(|_| CompilationError::TerminateCompilation)?;
//             }
//         }
//         // Send any leftover funds to a new pool
//         let change = PaymentPool {
//             members: new_members,
//             sequence: self.sequence + 1,
//             sig_needed: self.sig_needed,
//         };
//         let mut tmpl = ctx.template().add_output(change.total(), &change, None)?;
//         if all_payments.len() > 4 {
//             // We'll use the contract from our last post to make the state
//             // transitions more efficient!
//             // Think about what else could be fun here though...
//             tmpl = tmpl.add_output(
//                 spent,
//                 // TODO: Fix this treepay
//                 &TreePay {
//                     participants: all_payments,
//                     radix: 4,
//                 },
//                 None,
//             )?;
//         } else {
//             for p in all_payments {
//                 tmpl = tmpl.add_output(
//                     p.amount.try_into()?,
//                     &Compiled::from_address(p.address, None),
//                     None,
//                 )?;
//             }
//         }
//         tmpl.into()
//     }
// }
// REGISTER![PaymentPool, "logo.png"];
