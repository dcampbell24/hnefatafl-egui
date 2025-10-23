use std::{error::Error, time::Duration};

use hnefatafl::{
    board::state::{BasicBoardState, BoardState},
    game::GameStatus,
    pieces::Side,
    play::ValidPlay,
    preset::{boards, rules},
};
use hnefatafl_egui::ai::{Ai, BasicAi};

fn main() -> Result<(), Box<dyn Error>> {
    let game: hnefatafl::game::Game<BasicBoardState<u128>> =
        hnefatafl::game::Game::new(rules::COPENHAGEN, boards::COPENHAGEN).unwrap();

    println!("{}", game.state.board);

    let ai_attacker =
        hnefatafl_egui::ai::BasicAi::new(game.logic, Side::Attacker, Duration::from_secs(15));

    let ai_defender =
        hnefatafl_egui::ai::BasicAi::new(game.logic, Side::Defender, Duration::from_secs(15));

    handle_messages(game, ai_attacker, ai_defender)?;

    Ok(())
}

fn handle_messages<T: BoardState>(
    mut game: hnefatafl::game::Game<T>,
    mut ai_attacker: BasicAi<T>,
    mut ai_defender: BasicAi<T>,
) -> Result<(), Box<dyn Error>> {
    loop {
        let (ValidPlay{ play}, info) = ai_attacker.next_play(&game.state).unwrap();
        println!("play: {play}");
        println!("{info:?}\n");

        let status = game.do_play(play).unwrap();
        if status != GameStatus::Ongoing {
            println!("{status:?}");
            return Ok(());
        }

        println!("{}", game.state.board);

        let (ValidPlay{ play}, info) = ai_defender.next_play(&game.state).unwrap();
        println!("play: {play}");
        println!("{info:?}\n");

        let status = game.do_play(play).unwrap();
        if status != GameStatus::Ongoing {
            println!("{status:?}");
            return Ok(());
        }

        println!("{}", game.state.board);
    }
}
