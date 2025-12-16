//! WebSocket connection handling

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::AppState;
use super::messages::{WsMessage, ClientMessage, PeerListEntry};

/// Handle WebSocket upgrade
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    info!("New WebSocket connection established");
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast events
    let mut event_rx = state.event_tx.subscribe();

    // Send initial peer list
    match state.store.list_peers().await {
        Ok(peers) => {
            let entries: Vec<PeerListEntry> = peers.into_iter().map(Into::into).collect();
            let init_msg = WsMessage::PeersList { peers: entries };
            if let Ok(json) = serde_json::to_string(&init_msg) {
                let _ = sender.send(Message::Text(json.into())).await;
            }
        }
        Err(e) => {
            warn!("Failed to get initial peer list: {}", e);
        }
    }

    // Spawn task to forward broadcast events to this client
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages from client
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    info!("Received WebSocket text: {}", text);
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(client_msg) => {
                            handle_client_message(client_msg, &state_clone).await;
                        }
                        Err(e) => {
                            warn!("Failed to parse client message: {} - raw: {}", e, text);
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    info!("WebSocket connection closed");
}

/// Handle messages from the client
async fn handle_client_message(msg: ClientMessage, state: &AppState) {
    info!("Received client message: {:?}", msg);

    match msg {
        ClientMessage::SendChat { content, to } => {
            info!("SendChat: content='{}', to={:?}", content, to);

            // Generate message ID and timestamp for local echo
            let message_id = Uuid::new_v4().to_string();
            let timestamp = chrono::Utc::now().timestamp_millis();

            // Create chat message using core Message type
            let chat_msg = mycelial_core::message::Message::new(
                mycelial_core::message::MessageType::Content,
                state.local_peer_id.clone(),
                content.as_bytes().to_vec(),
            );

            // Serialize and publish to network
            match serde_json::to_vec(&chat_msg) {
                Ok(data) => {
                    let topic = if to.is_some() {
                        "/mycelial/1.0.0/direct"
                    } else {
                        "/mycelial/1.0.0/chat"
                    };

                    info!("Publishing to topic: {}", topic);

                    if let Err(e) = state.network.publish(topic, data).await {
                        error!("Failed to publish chat: {}", e);
                    } else {
                        info!("Chat message published successfully");

                        // LOCAL ECHO: Send the message back to the sender immediately
                        // Gossipsub doesn't deliver messages back to the sender, so we
                        // need to broadcast to all WebSocket clients including the sender
                        let echo_msg = WsMessage::ChatMessage {
                            id: message_id,
                            from: state.local_peer_id.to_string(),
                            from_name: state.node_name.clone(),
                            to: to.clone(),
                            content: content.clone(),
                            timestamp,
                        };

                        if let Err(e) = state.event_tx.send(echo_msg) {
                            error!("Failed to broadcast local echo: {}", e);
                        } else {
                            info!("Local echo sent to WebSocket clients");
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to serialize chat message: {}", e);
                }
            }
        }

        ClientMessage::GetPeers => {
            // Peer list is sent on connect, but can be requested again
            if let Ok(peers) = state.store.list_peers().await {
                let entries: Vec<PeerListEntry> = peers.into_iter().map(Into::into).collect();
                let msg = WsMessage::PeersList { peers: entries };
                let _ = state.event_tx.send(msg);
            }
        }

        ClientMessage::GetStats => {
            let stats = WsMessage::Stats {
                peer_count: state.store.list_peers().await.map(|p| p.len()).unwrap_or(0),
                message_count: state.message_count.load(std::sync::atomic::Ordering::Relaxed),
                uptime_seconds: state.start_time.elapsed().as_secs(),
            };
            let _ = state.event_tx.send(stats);
        }

        ClientMessage::Subscribe { topic } => {
            if let Err(e) = state.network.subscribe(&topic).await {
                error!("Failed to subscribe to topic {}: {}", topic, e);
            }
        }
    }
}
