use rand::{thread_rng, Rng};
use zing_rs::game::{unicode, GameState};
use zing_rs::{table::Table, zing_game::ZingGame};

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
    println!("");
}

fn main() {
    let table = Table {
        players: vec![
            zing_rs::table::Player {
                name: "Hans".into(),
            },
            zing_rs::table::Player {
                name: "Darko".into(),
            },
        ],
    };
    let mut game = ZingGame::new_from_table(table, 1);

    while game
        .state()
        .players
        .iter()
        .any(|player| player.hand.len() > 0)
    {
        show_state(&game);

        let player = game.current_player();
        game.play_card(
            player,
            thread_rng().gen_range(0..game.state().players[player].hand.len()),
        );
    }

    show_state(&game);

    let scores = game.total_points();
    //println!(game.game_state.player[0].name)
    println!("Scores: {} vs. {}", scores.0, scores.1);
}
