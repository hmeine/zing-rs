use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;
use game_state::GameState;
use zing_game::{table::Table, zing_game::ZingGame};

mod card_sprite;
mod constants;
mod game_state;
mod zing_layout;

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

    App::new()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(ClearColor(Color::rgb_u8(0x33, 0x69, 0x1d)))
        .insert_resource(WindowDescriptor {
            title: "Zing".to_string(),
            width: 1200.,
            height: 900.,
            fit_canvas_to_parent: true,
            ..Default::default()
        })
        .insert_resource(GameState::new(ZingGame::new_from_table(table, 0), 0))
        .add_plugins(DefaultPlugins)
        .add_plugin(TweeningPlugin)
        //.insert_resource(game.state())
        .add_plugin(zing_layout::LayoutPlugin)
        .run();
}
