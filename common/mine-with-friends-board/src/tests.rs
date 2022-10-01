use tracing::{debug, info, trace};

use crate::{
    game::{
        game_move::{GameMove, Heartbeat, Trade},
        FinishReason, GameBoard, GameSetup, MoveRejectReason,
    },
    sanitize::Unsanitized,
    tokens::token_swap::TradingPairID,
    MoveEnvelope,
};

const ALICE: &str = "alice";
const BOB: &str = "bob";
type PostCondition = &'static dyn Fn(&GameBoard, Result<(), MoveRejectReason>);

const NO_POST: PostCondition =
    (&|g: &GameBoard, r: Result<(), MoveRejectReason>| assert!(r.is_ok())) as PostCondition;
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
            &|game, r| {
                info!("Time: {}", game.elapsed_time);
                assert!(matches!(
                    game.game_is_finished(),
                    Some(FinishReason::TimeExpired)
                ))
            },
        ),
    ];
    run_game(moves, game);
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
                println!("{:?}",r);
                assert!(r.is_ok());
                let id = game.get_user_id(ALICE.into()).unwrap();
                assert_eq!(game.tokens[game.asic_token_id].balance_check(&id), 1);
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
                let id = game.get_user_id(ALICE.into()).unwrap();
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
                let id = game.get_user_id(ALICE.into()).unwrap();
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
                let id = game.get_user_id(ALICE.into()).unwrap();
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
                let id = game.get_user_id(ALICE.into()).unwrap();
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
                let id = game.get_user_id(ALICE.into()).unwrap();
                assert_eq!(game.tokens[game.asic_token_id].balance_check(&id), 15);
            },
        ),
    ];
    run_game(moves, game);
}

fn run_game<I>(moves: I, mut game: GameBoard)
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
            Ok(s) => {
                info!(move_=?mv, by, "Success");
            }
            Err(e) => {
                debug!(error=?e, "Failed (Non Catastrophic)")
            }
        }
        f(&game, r);
    }
}

fn setup_game() -> GameBoard {
    let setup = GameSetup {
        players: vec![ALICE.into(), BOB.into()],
        start_amount: 1_000_000,
        finish_time: 1_000_000,
    };
    let mut game = GameBoard::new(&setup);
    game
}
