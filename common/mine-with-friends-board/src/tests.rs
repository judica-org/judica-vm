// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    game::{
        game_move::{
            GameMove, Heartbeat, ListNFTForSale, MintPowerPlant, PurchaseNFT, RemoveTokens,
            SendTokens, Trade,
        },
        FinishReason, GameBoard, GameSetup, MoveRejectReason,
    },
    sanitize::Unsanitized,
    tokens::token_swap::TradingPairID,
    MoveEnvelope,
};
use tracing::{debug, info, trace};

const ALICE: &str = "alice";
const BOB: &str = "bob";
type PostCondition = &'static dyn Fn(&GameBoard, Result<(), MoveRejectReason>);
const NO_POST: PostCondition =
    (&|_g: &GameBoard, r: Result<(), MoveRejectReason>| assert!(r.is_ok())) as PostCondition;

#[test]
fn test_game_termination_time() {
    let _ = tracing_subscriber::fmt::try_init();
    let mut game = setup_game();
    let moves = [
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 0,
                time_millis: 123,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 0,
                time_millis: 455,
            },
            NO_POST,
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 1,
                time_millis: 3000000000,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 1,
                time_millis: 700,
            },
            &|game, _r| {
                info!("Time: {}", game.elapsed_time);
                assert!(matches!(
                    game.game_is_finished(),
                    Some(FinishReason::TimeExpired)
                ))
            },
        ),
    ];
    run_game(moves, &mut game);
}

#[test]
fn test_game_swaps() {
    let _ = tracing_subscriber::fmt::try_init();
    let mut game = setup_game();
    let moves = [
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 0,
                time_millis: 123,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 0,
                time_millis: 1232,
            },
            NO_POST,
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Trade(Trade {
                    pair: TradingPairID {
                        asset_a: game.bitcoin_token_id,
                        asset_b: game.asic_token_id,
                    },
                    amount_a: 0,
                    amount_b: 1,
                    sell: false,
                    cap: None,
                })),
                sequence: 1,
                time_millis: 1000,
            },
            &|game, r| {
                trace!(?r);
                println!("{:?}", r);
                assert!(r.is_ok());
                let id = game.get_user_id(ALICE).unwrap();
                assert_eq!(game.tokens[game.asic_token_id].balance_check(&id), 1);
                assert_eq!(game.tokens[game.steel_token_id].balance_check(&id), 0);
                assert_eq!(game.tokens[game.silicon_token_id].balance_check(&id), 0);
                assert_eq!(game.tokens[game.concrete_token_id].balance_check(&id), 0);
            },
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Trade(Trade {
                    pair: TradingPairID {
                        asset_a: game.bitcoin_token_id,
                        asset_b: game.asic_token_id,
                    },
                    amount_a: 0,
                    amount_b: 1,
                    sell: true,
                    cap: None,
                })),
                sequence: 2,
                time_millis: 2000,
            },
            &|game, r| {
                assert!(r.is_ok());
                let id = game.get_user_id(ALICE).unwrap();
                assert_eq!(game.tokens[game.asic_token_id].balance_check(&id), 0);
            },
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Trade(Trade {
                    pair: TradingPairID {
                        asset_a: game.bitcoin_token_id,
                        asset_b: game.asic_token_id,
                    },
                    amount_a: 0,
                    amount_b: 25,
                    sell: false,
                    cap: None,
                })),
                sequence: 3,
                time_millis: 3000,
            },
            &|game, r| {
                assert!(r.is_ok());
                let id = game.get_user_id(ALICE).unwrap();
                assert_eq!(game.tokens[game.asic_token_id].balance_check(&id), 25);
            },
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Trade(Trade {
                    pair: TradingPairID {
                        asset_a: game.bitcoin_token_id,
                        asset_b: game.asic_token_id,
                    },
                    amount_a: 0,
                    amount_b: 10,
                    sell: true,
                    cap: None,
                })),
                sequence: 4,
                time_millis: 4000,
            },
            &|game, r| {
                assert!(r.is_ok());
                let id = game.get_user_id(ALICE).unwrap();
                assert_eq!(game.tokens[game.asic_token_id].balance_check(&id), 15);
            },
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Trade(Trade {
                    pair: TradingPairID {
                        asset_a: game.bitcoin_token_id,
                        asset_b: game.asic_token_id,
                    },
                    amount_a: 0,
                    amount_b: 2000,
                    sell: true,
                    cap: None,
                })),
                sequence: 5,
                time_millis: 5000,
            },
            &|game, r| {
                assert!(matches!(r, Err(MoveRejectReason::TradeRejected(_))));
                let id = game.get_user_id(ALICE).unwrap();
                assert_eq!(game.tokens[game.asic_token_id].balance_check(&id), 15);
            },
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Trade(Trade {
                    pair: TradingPairID {
                        asset_a: game.bitcoin_token_id,
                        asset_b: game.asic_token_id,
                    },
                    amount_a: 0,
                    amount_b: 200000000000000,
                    sell: false,
                    cap: None,
                })),
                sequence: 6,
                time_millis: 6000,
            },
            &|game, r| {
                assert!(matches!(r, Err(MoveRejectReason::TradeRejected(_))));
                let id = game.get_user_id(ALICE).unwrap();
                assert_eq!(game.tokens[game.asic_token_id].balance_check(&id), 15);
            },
        ),
    ];
    run_game(moves, &mut game);
}

#[test]
fn test_super_mint() {
    let _ = tracing_subscriber::fmt::try_init();
    let mut game = setup_game();
    let moves = [
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 0,
                time_millis: 123,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 0,
                time_millis: 1232,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 1,
                time_millis: 30000,
            },
            NO_POST,
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::SuperMintPowerPlant(MintPowerPlant {
                    scale: 1,
                    plant_type: crate::nfts::instances::powerplant::PlantType::Solar,
                    location: (15, 15),
                })),
                sequence: 1,
                time_millis: 1000,
            },
            &|game, r| {
                trace!(?r);
                println!("{:?}", r);
                assert!(r.is_ok());
                let id = game.get_user_id(ALICE).unwrap();
                let plants = game.get_user_power_plants(id).unwrap();
                let plant = plants.power_plant_data.iter().next().unwrap().1;
                assert_eq!(
                    plant.plant_type,
                    crate::nfts::instances::powerplant::PlantType::Solar
                );
            },
        ),
    ];
    run_game(moves, &mut game);
}

#[test]
fn test_send_tokens_to_plant() {
    let _ = tracing_subscriber::fmt::try_init();
    let mut game = setup_game();

    let mut alice_seq = 0;
    let mut alice_seq_next = || {
        alice_seq += 1;
        alice_seq - 1
    };
    let moves = [
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: alice_seq_next(),
                time_millis: 123,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 0,
                time_millis: 1232,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 1,
                time_millis: 30000,
            },
            NO_POST,
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::SuperMintPowerPlant(MintPowerPlant {
                    scale: 1,
                    plant_type: crate::nfts::instances::powerplant::PlantType::Solar,
                    location: (15, 15),
                })),
                sequence: alice_seq_next(),
                time_millis: 1000,
            },
            NO_POST,
        ),
    ];
    run_game(moves, &mut game);

    let id = game.get_user_id(ALICE).unwrap();
    let plants = game.get_user_power_plants(id).unwrap();
    let plant_id = *plants.power_plant_data.iter().next().unwrap().0;

    let btc_balance_before_mining = game.tokens[game.bitcoin_token_id].balance_check(&id);

    let moves2 = [
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Trade(Trade {
                    pair: TradingPairID {
                        asset_a: game.bitcoin_token_id,
                        asset_b: game.asic_token_id,
                    },
                    amount_a: 0,
                    amount_b: 1,
                    sell: false,
                    cap: None,
                })),
                sequence: alice_seq_next(),
                time_millis: 2000,
            },
            NO_POST,
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::SendTokens(SendTokens {
                    to: plant_id.inner(),
                    amount: 1,
                    currency: game.asic_token_id,
                })),
                sequence: alice_seq_next(),
                time_millis: 3000,
            },
            &|g, v| {
                let id = g.get_user_id(ALICE).unwrap();
                let plants = &g.get_user_power_plants(id).unwrap();
                let plant_id = plants.power_plant_data.iter().next().unwrap().0;
                assert!(v.is_ok());
                let t = &g.tokens[g.asic_token_id];
                assert_eq!(1, t.balance_check(&plant_id.inner()));
            },
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: alice_seq_next(),
                time_millis: 100000,
            },
            NO_POST,
        ),
    ];

    run_game(moves2, &mut game);

    let btc_balance_after_mining = game.tokens[game.bitcoin_token_id].balance_check(&id);
    trace!(btc_balance_after_mining, btc_balance_before_mining);
    assert!(btc_balance_after_mining > btc_balance_before_mining);

    run_game(
        [
            (
                ALICE,
                MoveEnvelope {
                    d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                    sequence: alice_seq_next(),
                    /// median time should be averaged to >= 750k
                    time_millis: 1_500_000,
                },
                NO_POST,
            ),
            (
                ALICE,
                MoveEnvelope {
                    d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                    sequence: alice_seq_next(),
                    time_millis: 1_500_200,
                },
                &|g, v| {
                    let shares = g.get_user_hashrate_share();
                    trace!(?shares);
                    trace!(game_is_finished=?g.game_is_finished());
                    let timeout = g.elapsed_time >= (g.finish_time / 4);
                    trace!(finish_time=?g.finish_time, elapsed_time=?g.elapsed_time, timeout);
                    assert!(matches!(
                        v,
                        Err(MoveRejectReason::GameIsFinished(
                            FinishReason::DominatingPlayer(id)
                        )) if id == *g.users_by_key.get(ALICE).unwrap()
                    ))
                },
            ),
        ],
        &mut game,
    );
}

#[test]
fn test_sell_plant() {
    let _ = tracing_subscriber::fmt::try_init();
    let mut game = setup_game();

    let mut alice_seq = 0;
    let mut alice_seq_next = || {
        alice_seq += 1;
        alice_seq - 1
    };

    let mut bob_seq = 0;
    let mut bob_seq_next = || {
        bob_seq += 1;
        bob_seq - 1
    };
    // moves to make a plant
    let moves = [
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: alice_seq_next(),
                time_millis: 123,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: bob_seq_next(),
                time_millis: 500,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: bob_seq_next(),
                time_millis: 900,
            },
            NO_POST,
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::SuperMintPowerPlant(MintPowerPlant {
                    scale: 1,
                    plant_type: crate::nfts::instances::powerplant::PlantType::Solar,
                    location: (15, 15),
                })),
                sequence: alice_seq_next(),
                time_millis: 1000,
            },
            NO_POST,
        ),
    ];

    run_game(moves, &mut game);

    let a_id = game.get_user_id(ALICE).unwrap();
    let plants = game.get_user_power_plants(a_id).unwrap();
    let plant_id = *plants.power_plant_data.iter().next().unwrap().0;
    let btc_token_id = game.bitcoin_token_id;

    let moves2 = [
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::ListNFTForSale(ListNFTForSale {
                    nft_id: plant_id,
                    price: 1,
                    currency: btc_token_id,
                })),
                sequence: alice_seq_next(),
                time_millis: 2000,
            },
            NO_POST,
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: alice_seq_next(),
                time_millis: 3000,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::PurchaseNFT(PurchaseNFT {
                    nft_id: plant_id,
                    currency: btc_token_id,
                    limit_price: 2,
                })),
                sequence: bob_seq_next(),
                time_millis: 10000,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: bob_seq_next(),
                time_millis: 12000,
            },
            NO_POST,
        ),
    ];
    run_game(moves2, &mut game);
    let b_id = game.get_user_id(BOB).unwrap();
    let new_owner = game.nfts.nfts.get(&plant_id).unwrap().owner();
    assert_eq!(new_owner, b_id);
}

#[test]
fn test_remove_tokens_from_plant() {
    // setup, build plant, tx tokens (assert it happened), tx tokens back
    let _ = tracing_subscriber::fmt::try_init();
    let mut game = setup_game();

    let mut alice_seq = 0;
    let mut alice_seq_next = || {
        alice_seq += 1;
        alice_seq - 1
    };

    let mut bob_seq = 0;
    let mut bob_seq_next = || {
        bob_seq += 1;
        bob_seq - 1
    };
    // moves to make a plant
    let moves = [
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: alice_seq_next(),
                time_millis: 123,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: bob_seq_next(),
                time_millis: 1232,
            },
            NO_POST,
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::SuperMintPowerPlant(MintPowerPlant {
                    scale: 1,
                    plant_type: crate::nfts::instances::powerplant::PlantType::Solar,
                    location: (15, 15),
                })),
                sequence: alice_seq_next(),
                time_millis: 1000,
            },
            NO_POST,
        ),
    ];
    run_game(moves, &mut game);

    let id = game.get_user_id(ALICE).unwrap();
    let plants = game.get_user_power_plants(id).unwrap();
    let plant_id = *plants.power_plant_data.iter().next().unwrap().0;
    let _btc_balance_before_mining = game.tokens[game.bitcoin_token_id].balance_check(&id);

    let moves2 = [
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Trade(Trade {
                    pair: TradingPairID {
                        asset_a: game.bitcoin_token_id,
                        asset_b: game.asic_token_id,
                    },
                    amount_a: 0,
                    amount_b: 1,
                    sell: false,
                    cap: None,
                })),
                sequence: alice_seq_next(),
                time_millis: 2000,
            },
            NO_POST,
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::SendTokens(SendTokens {
                    to: plant_id.inner(),
                    amount: 1,
                    currency: game.asic_token_id,
                })),
                sequence: alice_seq_next(),
                time_millis: 3000,
            },
            &|g, v| {
                let id = g.get_user_id(ALICE).unwrap();
                let plants = &g.get_user_power_plants(id).unwrap();
                let plant_id = plants.power_plant_data.iter().next().unwrap().0;
                assert!(v.is_ok());
                let t = &g.tokens[g.asic_token_id];
                assert_eq!(1, t.balance_check(&plant_id.inner()));
            },
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: alice_seq_next(),
                time_millis: 10000,
            },
            NO_POST,
        ),
    ];

    run_game(moves2, &mut game);

    let asics_before_removal = game.tokens[game.asic_token_id].balance_check(&id);
    // transfer tokens back to A
    let moves3 = [
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::RemoveTokens(RemoveTokens {
                    nft_id: plant_id,
                    amount: 1,
                    currency: game.asic_token_id,
                })),
                sequence: alice_seq_next(),
                time_millis: 12000,
            },
            NO_POST,
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: alice_seq_next(),
                time_millis: 13000,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: bob_seq_next(),
                time_millis: 100_000,
            },
            NO_POST,
        ),
    ];

    run_game(moves3, &mut game);
    let asics_after_removal = game.tokens[game.asic_token_id].balance_check(&id);
    trace!(asics_after_removal, asics_before_removal);
    assert!(asics_after_removal > asics_before_removal);
}

fn run_game<I>(moves: I, game: &mut GameBoard)
where
    I: IntoIterator<
        Item = (
            &'static str,
            MoveEnvelope,
            &'static dyn Fn(&GameBoard, Result<(), MoveRejectReason>),
        ),
    >,
{
    for (by, mv, f) in moves {
        let r = game.play(mv.clone(), by.into());
        match &r {
            Ok(_s) => {
                info!(move_=?mv, by, "Success");
            }
            Err(e) => {
                debug!(error=?e, "Failed (Non Catastrophic)")
            }
        }
        f(game, r);
    }
}

fn setup_game() -> GameBoard {
    let setup = GameSetup {
        players: vec![ALICE.into(), BOB.into()],
        start_amount: 10_000_000,
        finish_time: 1_000_000,
    };

    GameBoard::new(&setup)
}
