use bevy::prelude::*;

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
        .add_plugin(bevy_svg::prelude::SvgPlugin)
        //.insert_resource(game.state())
        .add_plugin(zing_layout::LayoutPlugin)
        .add_startup_system(setup_system)
        .run();
}

fn setup_system(
    mut commands: Commands,
) {
    commands.spawn_bundle(Camera2dBundle::default());
}
