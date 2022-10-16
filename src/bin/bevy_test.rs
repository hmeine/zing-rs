use bevy::prelude::*;
use bevy_svg::prelude::*;

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
        .add_startup_system(setup_system)
        .add_system(svg_loaded_system)
        .run();
}

fn setup_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
//    mut windows: Res<Windows>,
) {
    commands.spawn_bundle(Camera2dBundle::default());

    let svg = asset_server.load("svg_cards_1.3/king_of_clubs2.svg");

    commands.spawn_bundle(Svg2dBundle {
        svg,
        origin: Origin::Center, // Origin::TopLeft is the default
        ..Default::default()
    });

//    let window = windows.get_primary().unwrap();
//    commands.insert_resource(WindowSize::new(window));
}

fn svg_loaded_system(mut ev_asset: EventReader<AssetEvent<Svg>>, assets: Res<Assets<Svg>>) {
    for ev in ev_asset.iter() {
        if let AssetEvent::Created { handle } = ev {
            let svg = assets.get(&handle.clone()).unwrap();
            println!("svg loaded, size {} and view box {:?}", svg.size, svg.view_box);
        }
    }
}
