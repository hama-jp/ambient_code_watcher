use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AmbientEvent {
    Analysis(String),
    UserQuery(String),
    QueryResponse(String), // 質問への回答を区別
    System(String),
}

impl AmbientEvent {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<AmbientEvent>,
}

pub async fn run_server(tx: broadcast::Sender<AmbientEvent>, port: u16) {
    let app_state = Arc::new(AppState { tx });

    // Serve static files from the `ambient_ui` directory.
    // The `index.html` file will be served at the root.
    let serve_dir = tower_http::services::ServeDir::new("cli/src/ambient_ui")
        .append_index_html_on_directories(true);

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .nest_service("/", serve_dir)
        .with_state(app_state);

    // 指定されたポートを試し、失敗したら次のポートを試す
    let mut try_port = port;
    let listener = loop {
        match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", try_port)).await {
            Ok(l) => break l,
            Err(_) if try_port < port + 10 => {
                // 最大10ポート試す
                eprintln!("ポート{}は使用中です。次のポートを試します...", try_port);
                try_port += 1;
            }
            Err(e) => {
                eprintln!("ポート{}へのバインドに失敗しました: {}", try_port, e);
                return;
            }
        }
    };

    let actual_port = listener.local_addr().unwrap().port();
    if actual_port == port {
        println!(
            "Ambient Watcherが http://127.0.0.1:{} で動作中です",
            actual_port
        );
    } else {
        println!(
            "Ambient Watcherが http://127.0.0.1:{} で動作中です (設定ポート{}は使用中)",
            actual_port, port
        );
    }

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Server error: {e}");
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    // Send a welcome message.
    let welcome_msg = AmbientEvent::System("Ambient Watcherに接続しました".to_string());
    if sender
        .send(Message::Text(welcome_msg.to_json()))
        .await
        .is_err()
    {
        return; // Client disconnected.
    }

    // This task will forward broadcast messages to the client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender
                .send(Message::Text(msg.to_json()))
                .await
                .is_err()
            {
                break; // Client disconnected.
            }
        }
    });

    // This task will receive messages from the client and broadcast them.
    let tx = state.tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                // A message from the client is treated as a user query.
                let query_event = AmbientEvent::UserQuery(text);
                // The receiver of this event is in the main ambient loop.
                let _ = tx.send(query_event);
            }
        }
    });

    // Wait for either task to complete, and abort the other when one finishes.
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
