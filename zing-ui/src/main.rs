use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;
use game_state::GameState;
use zing_game::zing_game::ZingGame;

mod card_sprite;
mod constants;
mod game_state;
mod zing_layout;

fn main() {
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
        .insert_resource(GameState::new(
            ZingGame::new_with_player_names(vec!["Hans".into(), "Darko".into()], 1),
            0,
        ))
        .add_plugins(DefaultPlugins)
        .add_plugin(TweeningPlugin)
        //.insert_resource(game.state())
        .add_plugin(zing_layout::LayoutPlugin)
        .run();
}
