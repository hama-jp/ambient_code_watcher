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
    ProjectRoot(String), // プロジェクトルートパス
}

impl AmbientEvent {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<AmbientEvent>,
    project_root: String,
}

pub async fn run_server(
    tx: broadcast::Sender<AmbientEvent>,
    port: u16,
    shutdown_signal: impl std::future::Future<Output = ()> + Send + 'static,
) {
    let project_root = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let app_state = Arc::new(AppState { tx, project_root });

    // Serve static files from the `ambient_ui` directory.
    // Try multiple possible locations for the UI files
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let ui_paths = vec![
        // When running from the source directory
        "cli/src/ambient_ui".to_string(),
        // When running from cargo target directory
        "../../../cli/src/ambient_ui".to_string(),
        // When installed via install.sh
        format!("{}/.config/ambient/ui", home_dir),
    ];

    let mut serve_dir_path = None;
    for path in &ui_paths {
        if std::path::Path::new(path).exists() {
            serve_dir_path = Some(path.clone());
            break;
        }
    }

    let serve_dir_path = serve_dir_path.unwrap_or_else(|| {
        eprintln!("警告: UIファイルが見つかりません。デフォルトパスを使用します。");
        "cli/src/ambient_ui".to_string()
    });

    let serve_dir =
        tower_http::services::ServeDir::new(serve_dir_path).append_index_html_on_directories(true);

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .nest_service("/", serve_dir)
        .with_state(app_state);

    // 指定されたポートを試し、失敗したら次のポートを試す
    let mut try_port = port;
    let listener = loop {
        match tokio::net::TcpListener::bind(format!("127.0.0.1:{try_port}")).await {
            Ok(l) => break l,
            Err(_) if try_port < port + 10 => {
                // 最大10ポート試す
                eprintln!("ポート{try_port}は使用中です。次のポートを試します...");
                try_port += 1;
            }
            Err(e) => {
                eprintln!("ポート{try_port}へのバインドに失敗しました: {e}");
                return;
            }
        }
    };

    let actual_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
    if actual_port == port {
        println!("Ambient Code Watcherが http://127.0.0.1:{actual_port} で動作中です");
    } else {
        println!(
            "Ambient Code Watcherが http://127.0.0.1:{actual_port} で動作中です (設定ポート{port}は使用中)"
        );
    }

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
    {
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
    let welcome_msg = AmbientEvent::System("Ambient Code Watcherに接続しました".to_string());
    if sender
        .send(Message::Text(welcome_msg.to_json()))
        .await
        .is_err()
    {
        return; // Client disconnected.
    }

    // Send project root path
    let project_root_msg = AmbientEvent::ProjectRoot(state.project_root.clone());
    if sender
        .send(Message::Text(project_root_msg.to_json()))
        .await
        .is_err()
    {
        return; // Client disconnected.
    }

    // This task will forward broadcast messages to the client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.to_json())).await.is_err() {
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
