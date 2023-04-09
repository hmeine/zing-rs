use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;
use clap::Parser;
use futures_util::StreamExt;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::{Uri, HeaderValue}},
};
//use std::sync::mpsc;

mod card_sprite;
mod constants;
mod game_logic;
mod zing_layout;
#[derive(Parser)]
struct Cli {
    login_id: String,
    table_id: String,
    #[arg(default_value = "ws://localhost:3000")]
    base_url: String,
}

#[tokio::main]
async fn tokio_main(args: Cli) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_uri: Uri =
        format!("{}/table/{}/game/ws", args.base_url, args.table_id).parse::<Uri>()?;

    let mut request = ws_uri.into_client_request()?;
    request.headers_mut().insert(
        "Cookie",
        HeaderValue::from_str(&format!("login_id={}", args.login_id))?,
    );

    let (ws_stream, _response) = connect_async(request).await?;

    dbg!(_response);
    //    let (_write, read) = ws_stream.split();

    println!("receiving messages...");
    ws_stream
        .for_each(|message| async {
            println!("got message");
            let json = message.unwrap().into_text().unwrap();
            dbg!(json);
        })
        .await;

    println!("WebSocket for_each endet.");

    Ok(())
}

fn main() {
    let args = Cli::parse();

    let _thread_handle = std::thread::spawn(|| tokio_main(args));

    let game_logic = game_logic::GameLogic::new();
    App::new()
        .insert_resource(Msaa::default())
        .insert_resource(ClearColor(Color::rgb_u8(0x33, 0x69, 0x1d)))
        .insert_resource(game_logic)
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
