use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;
use clap::Parser;
use futures::sink::SinkExt;
use futures_util::StreamExt;
use std::{sync::mpsc::Sender, time::Duration};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::IntoClientRequest,
        http::{HeaderValue, Uri},
        Message,
    },
};
use zing_game::client_notification::ClientNotification;

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
async fn tokio_main(
    args: Cli,
    notification_sender: Sender<ClientNotification>,
    mut card_receiver: tokio::sync::mpsc::Receiver<usize>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_uri: Uri =
        format!("{}/table/{}/game/ws", args.base_url, args.table_id).parse::<Uri>()?;

    let mut request = ws_uri.into_client_request()?;
    request.headers_mut().insert(
        "Cookie",
        HeaderValue::from_str(&format!("login_id={}", args.login_id))?,
    );

    let (ws_stream, _response) = connect_async(request).await?;

    let (mut ws_tx, ws_rx) = ws_stream.split();
    //let mut card_receiver = Arc::new(card_receiver);

    tokio::spawn(async move {
        loop {
            if let Ok(card) = card_receiver.try_recv() {
                if let Err(err) = ws_tx.send(Message::Text(card.to_string())).await {
                    println!(
                        "could not send WebSocket notification about card being played: {}",
                        err
                    );
                };
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });

    ws_rx
        .for_each(|message| async {
            let json = message.unwrap().into_text().unwrap();
            let client_notification: Option<ClientNotification> = serde_json::from_str(&json).ok();
            if let Some(client_notification) = client_notification {
                if let Err(err) = notification_sender.send(client_notification) {
                    println!("error sending notification to Bevy thread: {}", err);
                }
            }
        })
        .await;

    println!("WebSocket for_each endet.");

    Ok(())
}

fn main() {
    let args = Cli::parse();

    let (notification_tx, notification_rx) = std::sync::mpsc::channel();
    let (card_tx, card_rx) = tokio::sync::mpsc::channel(4);

    let _thread_handle = std::thread::spawn(|| tokio_main(args, notification_tx, card_rx));
    let game_logic = game_logic::GameLogic::new(notification_rx, card_tx);

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
