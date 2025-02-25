use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

mod app_state;
mod card_sprite;
mod constants;
mod debug;
mod game_logic;
mod zing_layout;

// The login_id is actually unused in wasm builds (because the .wasm re-uses the
// browser cookies), but I don't know how to elegantly handle that here (so I
// just pass some random string from JS for now).
#[cfg_attr(target_family = "wasm", wasm_bindgen)]
pub fn start_remote_game(login_id: String, table_id: String, base_url: String) {
    App::new()
        .insert_resource(ClearColor(Color::srgb_u8(0x33, 0x69, 0x1d)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Zing".to_string(),
                canvas: Some("#gamecanvas".into()),
                prevent_default_event_handling: false,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(TweeningPlugin)
        .add_plugins(zing_layout::LayoutPlugin)
        .add_plugins(game_logic::GameLogicPlugin {
            base_url,
            login_id,
            table_id,
        })
        .add_plugins(debug::DebugPlugin)
        .run();
}
