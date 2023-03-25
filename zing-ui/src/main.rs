use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;
use layout_state::LayoutState;
use zing_game::zing_game::ZingGame;

mod card_sprite;
mod constants;
mod layout_state;
mod zing_layout;

fn main() {
    App::new()
        .insert_resource(Msaa::default())
        .insert_resource(ClearColor(Color::rgb_u8(0x33, 0x69, 0x1d)))
        .insert_resource(LayoutState::new(
            ZingGame::new_with_player_names(vec!["Hans".into(), "Darko".into()], 1),
            0,
        ))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Zing".to_string(),
                resolution: (1200., 900.).into(),
                fit_canvas_to_parent: true,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugin(TweeningPlugin)
        .add_plugin(zing_layout::LayoutPlugin)
        .run();
}
