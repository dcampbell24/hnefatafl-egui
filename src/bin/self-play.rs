use std::{error::Error, time::Duration};

use hnefatafl::{board::state::{BasicBoardState, BoardState}, game::GameStatus, pieces::Side, play::ValidPlay, preset::{boards, rules}};
use hnefatafl_egui::ai::{Ai, AiError, BasicAi};

fn main() -> Result<(), Box<dyn Error>> {
    loop {
        let game: hnefatafl::game::Game<BasicBoardState<u128>> =
            hnefatafl::game::Game::new(rules::COPENHAGEN, boards::COPENHAGEN).unwrap();

        println!("{}", game.state.board);

        let ai_attacker = hnefatafl_egui::ai::BasicAi::new(
            game.logic,
            Side::Attacker,
            Duration::from_secs(15),
        );

        let ai_defender = hnefatafl_egui::ai::BasicAi::new(
            game.logic,
            Side::Defender,
            Duration::from_secs(15),
        );

        handle_messages(game, ai_attacker, ai_defender)?;
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_messages<T: BoardState>(
    mut game: hnefatafl::game::Game<T>,
    mut ai_attacker: BasicAi<T>,
    mut ai_defender: BasicAi<T>,
) -> Result<(), Box<dyn Error>> {
    loop {
        match ai_attacker.next_play(&game.state) {
            Ok((ValidPlay { play }, info)) => {
                println!("play: {play}");
                println!("{info:?}\n");

                match game.do_play(play) {
                    Err(error) => {
                        println!("invalid_play: {error:?}");
                        return Ok(());
                    }
                    Ok(status) => {
                        if status != GameStatus::Ongoing {
                            return Ok(());
                        }
                    }
                }
            }
            Err(AiError::NoPlayAvailable) => {
                return Ok(());
            }
            Err(AiError::NotMyTurn) => unreachable!(),
        }

        println!("{}", game.state.board);

        match ai_defender.next_play(&game.state) {
            Ok((ValidPlay { play }, info)) => {
                println!("play: {play}");
                println!("{info:?}\n");

                match game.do_play(play) {
                    Err(error) => {
                        println!("invalid_play: {error:?}");
                        return Ok(());
                    }
                    Ok(status) => {
                        if status != GameStatus::Ongoing {
                            return Ok(());
                        }
                    }
                }

            }
            Err(AiError::NoPlayAvailable) => {
                return Ok(());
            }
            Err(AiError::NotMyTurn) => unreachable!(),
        }

        println!("{}", game.state.board);
    }
}
