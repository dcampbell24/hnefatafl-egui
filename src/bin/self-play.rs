use std::{error::Error, time::Duration};

use hnefatafl::{
    board::state::BasicBoardState,
    game::GameStatus,
    pieces::Side,
    play::ValidPlay,
    preset::{boards, rules},
};
use hnefatafl_egui::ai::Ai;

fn main() -> Result<(), Box<dyn Error>> {
    let mut game: hnefatafl::game::Game<BasicBoardState<u128>> =
        hnefatafl::game::Game::new(rules::COPENHAGEN, boards::COPENHAGEN).unwrap();

    println!("{}", game.state.board);

    let mut ai_attacker =
        hnefatafl_egui::ai::BasicAi::new(game.logic, Side::Attacker, Duration::from_secs(15));

    let mut ai_defender =
        hnefatafl_egui::ai::BasicAi::new(game.logic, Side::Defender, Duration::from_secs(15));

    loop {
        let (ValidPlay { play }, info) = ai_attacker.next_play(&game.state).unwrap();
        println!("play: {play}");
        println!("{info:?}\n");

        let status = game.do_play(play).unwrap();
        if status != GameStatus::Ongoing {
            println!("{status:?}");
            return Ok(());
        }

        println!("{}", game.state.board);

        let (ValidPlay { play }, info) = ai_defender.next_play(&game.state).unwrap();
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
