use bevy::prelude::*;
use bevy_tokio_tasks::{TokioTasksPlugin, TokioTasksRuntime};
use bevy_tweening::TweeningPlugin;
use futures_util::StreamExt;
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

async fn websocket_communication(
    base_url: &str,
    login_id: &str,
    table_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_stream = connect_websocket(&base_url, &login_id, &table_id).await?;

    ws_stream
        .for_each(|message| async {
            let json = message.unwrap().into_text().unwrap();
            let client_notification: Option<ClientNotification> = serde_json::from_str(&json).ok();
            if let Some(client_notification) = client_notification {
                //if let Err(err) = notification_sender.send(client_notification) {
                //    println!("error sending notification to Bevy thread: {}", err);
                //}
            }
        })
        .await;

    Ok(())
}

pub fn start_remote_game(login_id: String, table_id: String, base_url: String) {
    let game_logic = game_logic::GameLogic::new(&base_url, &login_id, &table_id);

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
        .add_plugin(TokioTasksPlugin::default())
        .add_plugin(TweeningPlugin)
        .add_plugin(zing_layout::LayoutPlugin)
        .run();
}
