use zing_game::game::unicode;
use zing_game::zing_ai::{RandomPlayer, ZingAI};
use zing_game::zing_game::ZingGame;

fn show_state(game: &ZingGame) {
    for stack in &game.state().stacks {
        if stack.cards.is_empty() {
            continue;
        } else if stack.cards.len() > 8 {
            println!("{}: {} cards", stack.id, stack.cards.len());
        } else {
            println!("{}: {}", stack.id, unicode(&stack.cards));
        }
    }
    for (i, player) in game.state().players.iter().enumerate() {
        println!(
            "{}: {}{}",
            player.name,
            unicode(&player.hand),
            if i == game.current_player() {
                " <= turn"
            } else {
                ""
            }
        );
    }
    println!();
}

fn main() {
    let mut game = ZingGame::new_with_player_names(vec!["Hans".into(), "Darko".into()], 1);
    game.setup_game();

    let players = [RandomPlayer::new(0), RandomPlayer::new(1)];

    while !game.finished() {
        show_state(&game);

        players[game.current_player()].auto_play(&mut game);
    }

    show_state(&game);

    let scores = game.points().total_points();
    //println!(game.game_state.player[0].name)
    println!("Scores: {} vs. {}", scores.0, scores.1);
}
