use zing_rs::{table::Table, zing_game::ZingGame};
use zing_rs::game::{unicode, GameState};

fn show_state(game: &GameState) {
    for stack in &game.stacks {
        if stack.cards.len() > 8 {
            println!("{}: {} cards", stack.id, stack.cards.len());
        } else {
            println!("{}: {}", stack.id, unicode(&stack.cards));
        }
    }
    for player in &game.players {
        println!("{}: {}", player.name, unicode(&player.hand));
    }
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

    while game.state().stacks[0].cards.len() > 0 {
        show_state(&game.state());

        game.play_card(1, 0);
        break;
    
    }

}
