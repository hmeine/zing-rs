use zing_game::game::unicode;
use zing_game::{table::Table, zing_game::ZingGame};
use zing_game::zing_ai::{ZingAI, RandomPlayer};

fn show_state(game: &ZingGame) {
    for stack in &game.state().stacks {
        if stack.cards.len() > 8 {
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
    let table = Table {
        players: vec![
            zing_game::table::Player {
                name: "Hans".into(),
            },
            zing_game::table::Player {
                name: "Darko".into(),
            },
        ],
    };
    let mut game = ZingGame::new_from_table(table, 1);
    game.setup_game();

    let players = [RandomPlayer::new(0), RandomPlayer::new(1)];

    while !game.finished()
    {
        show_state(&game);

        players[game.current_player()].auto_play(&mut game);
    }

    show_state(&game);

    let scores = game.total_points();
    //println!(game.game_state.player[0].name)
    println!("Scores: {} vs. {}", scores.0, scores.1);
}
