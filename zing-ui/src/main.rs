use bevy::prelude::*;

mod constants;
mod card_sprite;
mod zing_layout;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(ClearColor(Color::rgb_u8(0x33, 0x69, 0x1d)))
        .insert_resource(WindowDescriptor {
            title: "Zing".to_string(),
            width: 1200.,
            height: 900.,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        //.insert_resource(game.state())
        .add_plugin(zing_layout::LayoutPlugin)
        .run();
}
