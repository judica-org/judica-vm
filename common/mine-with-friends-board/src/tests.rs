use tracing::{debug, info};

use crate::{
    game::{
        game_move::{GameMove, Heartbeat},
        FinishReason, GameBoard, GameSetup,
    },
    sanitize::Unsanitized,
    MoveEnvelope,
};

const ALICE: &str = "alice";
const BOB: &str = "bob";
type PostCondition = &'static dyn Fn(&GameBoard);

const NO_POST: PostCondition = (&|g: &GameBoard| {}) as PostCondition;
#[test_log::test]
fn test_game_termination_time() {
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
                time_millis: 650,
            },
            NO_POST,
        ),
        (
            ALICE,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 2,
                time_millis: 3000000250,
            },
            NO_POST,
        ),
        (
            BOB,
            MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
                sequence: 2,
                time_millis: 700,
            },
            &|game: &GameBoard| {
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

fn run_game<I>(moves: I, mut game: GameBoard)
where
    I: IntoIterator<Item = (&'static str, MoveEnvelope, &'static dyn Fn(&GameBoard))>,
{
    for (by, mv, f) in moves {
        match game.play(mv.clone(), by.into()) {
            Ok(s) => {
                info!(move_=?mv, by, "Success");
            }
            Err(e) => {
                debug!(error=?e, "Failed (Non Catastrophic)")
            }
        }
        f(&game);
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
