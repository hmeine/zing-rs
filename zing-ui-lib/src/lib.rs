use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;
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

fn spawn_card_playing_task(
    base_url: &str,
    login_id: &str,
    table_id: &str,
    mut card_receiver: tokio::sync::mpsc::Receiver<usize>,
) {
    let jar = cookie::Jar::default();
    jar.add_cookie_str(
        &format!("login_id={}", login_id),
        &base_url.parse().unwrap(),
    );
    let client = reqwest::Client::builder()
        .cookie_provider(Arc::new(jar))
        .build()
        .unwrap();

    let play_uri = format!("{}/table/{}/game/play", base_url, table_id);
    tokio::spawn(async move {
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
}

async fn websocket_communication(
    base_url: &str,
    login_id: &str,
    table_id: &str,
    notification_sender: Sender<ClientNotification>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_stream = connect_websocket(&base_url, &login_id, &table_id).await?;

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
    base_url: String,
    login_id: String,
    table_id: String,
    notification_sender: Sender<ClientNotification>,
    card_receiver: tokio::sync::mpsc::Receiver<usize>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    spawn_card_playing_task(&base_url, &login_id, &table_id, card_receiver);

    let result =
        websocket_communication(&base_url, &login_id, &table_id, notification_sender).await;

    info!("WebSocket communication endet.");
    if let Err(error) = result {
        error!("{}", error)
    }

    Ok(())
}

pub fn start_remote_game(login_id: String, table_id: String, base_url: String) {
    let (notification_tx, notification_rx) = std::sync::mpsc::channel();
    let (card_tx, card_rx) = tokio::sync::mpsc::channel(4);

    let _thread_handle =
        std::thread::spawn(|| tokio_main(base_url, login_id, table_id, notification_tx, card_rx));
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
