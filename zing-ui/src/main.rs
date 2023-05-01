use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;
use clap::Parser;
use futures_util::StreamExt;
use reqwest::{cookie, header::CONTENT_TYPE};
use std::{
    sync::{mpsc::Sender, Arc},
    time::Duration,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::IntoClientRequest,
        http::{HeaderValue, Uri},
    },
    MaybeTlsStream, WebSocketStream,
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
    #[arg(default_value = "http://localhost:3000")]
    base_url: String,
}

async fn connect_websocket(
    base_url: &str,
    login_id: &str,
    table_id: &str,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, Box<dyn std::error::Error + Send + Sync>> {
    let ws_uri: Uri = format!(
        "{}/table/{}/game/ws",
        base_url.replace("http", "ws"),
        table_id
    )
    .parse::<Uri>()?;

    let mut request = ws_uri.into_client_request()?;
    request.headers_mut().insert(
        "Cookie",
        HeaderValue::from_str(&format!("login_id={}", login_id))?,
    );

    let (ws_stream, _response) = connect_async(request).await?;

    Ok(ws_stream)
}

async fn websocket_communication(
    args: Cli,
    notification_sender: Sender<ClientNotification>,
    mut card_receiver: tokio::sync::mpsc::Receiver<usize>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let jar = cookie::Jar::default();
    jar.add_cookie_str(
        &format!("login_id={}", args.login_id),
        &args.base_url.parse().unwrap(),
    );
    let client = reqwest::Client::builder()
        .cookie_provider(Arc::new(jar))
        .build()
        .unwrap();

    let ws_stream = connect_websocket(&args.base_url, &args.login_id, &args.table_id).await?;

    tokio::spawn(async move {
        let play_uri = format!("{}/table/{}/game/play", args.base_url, args.table_id);

        loop {
            if let Ok(card_index) = card_receiver.try_recv() {
                match client
                    .post(&play_uri)
                    .header(CONTENT_TYPE, "application/json")
                    .body(format!("{{ \"card_index\": {} }}", card_index))
                    .send()
                    .await
                {
                    Err(err) => println!("Rest API error trying to play card: {}", err),
                    Ok(response) => {
                        println!("{} {}", response.status(), response.text().await.unwrap());
                    }
                };
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });

    ws_stream
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

    Ok(())
}

#[tokio::main]
async fn tokio_main(
    args: Cli,
    notification_sender: Sender<ClientNotification>,
    card_receiver: tokio::sync::mpsc::Receiver<usize>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let result = websocket_communication(args, notification_sender, card_receiver).await;

    info!("WebSocket communication endet.");
    if let Err(error) = result {
        error!("{}", error)
    }

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
