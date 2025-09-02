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

pub async fn run_server(tx: broadcast::Sender<AmbientEvent>) {
    let app_state = Arc::new(AppState { tx });

    // Serve static files from the `ambient_ui` directory.
    // The `index.html` file will be served at the root.
    let serve_dir = tower_http::services::ServeDir::new("codex-rs/cli/src/ambient_ui")
        .append_index_html_on_directories(true);

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .nest_service("/", serve_dir)
        .with_state(app_state);

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to port: {e}");
            return;
        }
    };

    println!(
        "Codex Ambient UI is running at http://{}",
        listener.local_addr().unwrap()
    );

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
    let welcome_msg = AmbientEvent::System("Connected to Codex Ambient server.".to_string());
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
