use bevy::prelude::*;

use bevy_tweening::TweeningPlugin;

mod card_sprite;
mod constants;
mod game_logic;
mod zing_layout;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn start_remote_game(login_id: String, table_id: String, base_url: String) {
    App::new()
        .insert_resource(Msaa::default())
        .insert_resource(ClearColor(Color::rgb_u8(0x33, 0x69, 0x1d)))
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
        .add_plugin(game_logic::GameLogicPlugin {
            base_url,
            login_id,
            table_id,
        })
        .run();
}
