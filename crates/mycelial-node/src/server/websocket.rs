//! WebSocket connection handling
//!
//! This module handles WebSocket connections from dashboard clients,
//! including support for economics protocol messages.

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
use mycelial_protocol::{
    topics,
    VouchMessage, VouchRequest, VouchAck as ProtocolVouchAck,
    CreditMessage, CreateCreditLine as ProtocolCreateCreditLine, CreditTransfer as ProtocolCreditTransfer,
    GovernanceMessage, CreateProposal as ProtocolCreateProposal, CastVote as ProtocolCastVote, Vote,
    ResourceMessage, ResourceContribution as ProtocolResourceContribution, ResourceType,
};

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
        ClientMessage::SendChat { content, to, room_id } => {
            info!("SendChat: content='{}', to={:?}, room_id={:?}", content, to, room_id);

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
                    // Determine topic based on message target
                    let topic = if room_id.is_some() {
                        format!("/mycelial/1.0.0/room/{}", room_id.as_ref().unwrap())
                    } else if to.is_some() {
                        "/mycelial/1.0.0/direct".to_string()
                    } else {
                        "/mycelial/1.0.0/chat".to_string()
                    };

                    info!("Publishing to topic: {}", topic);

                    if let Err(e) = state.network.publish(&topic, data).await {
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
                            room_id: room_id.clone(),
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

        // ============ Economics Protocol Handlers ============

        ClientMessage::SendVouch { vouchee, weight, message } => {
            info!("SendVouch: vouchee='{}', weight={}", vouchee, weight);

            let timestamp = chrono::Utc::now().timestamp_millis();

            // Create vouch request message (uses stake, not weight)
            let mut vouch_req = VouchRequest::new(
                state.local_peer_id.to_string(),
                vouchee.clone(),
                weight, // VouchRequest calls this 'stake'
            );
            if let Some(msg) = message {
                vouch_req = vouch_req.with_message(msg);
            }
            let request_id = vouch_req.id.to_string();
            let vouch_msg = VouchMessage::VouchRequest(vouch_req);

            // Serialize and publish to network
            match serde_json::to_vec(&vouch_msg) {
                Ok(data) => {
                    if let Err(e) = state.network.publish(topics::VOUCH, data).await {
                        error!("Failed to publish vouch request: {}", e);
                    } else {
                        info!("Vouch request published successfully");

                        // Local echo for the sender
                        let echo_msg = WsMessage::VouchRequest {
                            id: request_id,
                            voucher: state.local_peer_id.to_string(),
                            vouchee,
                            weight,
                            timestamp,
                        };
                        let _ = state.event_tx.send(echo_msg);
                    }
                }
                Err(e) => {
                    error!("Failed to serialize vouch request: {}", e);
                }
            }
        }

        ClientMessage::RespondVouch { request_id, accept } => {
            info!("RespondVouch: request_id='{}', accept={}", request_id, accept);

            let timestamp = chrono::Utc::now().timestamp_millis();

            // Parse request_id as UUID
            let vouch_id = match Uuid::parse_str(&request_id) {
                Ok(id) => id,
                Err(e) => {
                    error!("Invalid vouch request ID: {}", e);
                    return;
                }
            };

            // Create vouch ack message with correct fields
            let ack_msg = VouchMessage::VouchAck(ProtocolVouchAck {
                vouch_id,
                from: state.local_peer_id.to_string(),
                accepted: accept,
                reason: None,
                timestamp: chrono::Utc::now(),
            });

            match serde_json::to_vec(&ack_msg) {
                Ok(data) => {
                    if let Err(e) = state.network.publish(topics::VOUCH, data).await {
                        error!("Failed to publish vouch ack: {}", e);
                    } else {
                        let echo_msg = WsMessage::VouchAck {
                            id: Uuid::new_v4().to_string(),
                            request_id,
                            accepted: accept,
                            new_reputation: None,
                            timestamp,
                        };
                        let _ = state.event_tx.send(echo_msg);
                    }
                }
                Err(e) => {
                    error!("Failed to serialize vouch ack: {}", e);
                }
            }
        }

        ClientMessage::CreateCreditLine { debtor, limit } => {
            info!("CreateCreditLine: debtor='{}', limit={}", debtor, limit);

            let timestamp = chrono::Utc::now().timestamp_millis();

            let credit_msg = CreditMessage::CreateLine(ProtocolCreateCreditLine::new(
                state.local_peer_id.to_string(),
                debtor.clone(),
                limit,
            ));

            match serde_json::to_vec(&credit_msg) {
                Ok(data) => {
                    if let Err(e) = state.network.publish(topics::CREDIT, data).await {
                        error!("Failed to publish credit line: {}", e);
                    } else {
                        let echo_msg = WsMessage::CreditLine {
                            id: Uuid::new_v4().to_string(),
                            creditor: state.local_peer_id.to_string(),
                            debtor,
                            limit,
                            balance: 0.0,
                            timestamp,
                        };
                        let _ = state.event_tx.send(echo_msg);
                    }
                }
                Err(e) => {
                    error!("Failed to serialize credit line: {}", e);
                }
            }
        }

        ClientMessage::TransferCredit { to, amount, memo } => {
            info!("TransferCredit: to='{}', amount={}", to, amount);

            let timestamp = chrono::Utc::now().timestamp_millis();

            // For transfers, we use a placeholder line_id - in practice, the client should
            // provide the actual credit line ID they want to use for the transfer
            let line_id = Uuid::new_v4(); // Placeholder - real impl would look up active credit line
            let mut transfer = ProtocolCreditTransfer::new(
                line_id,
                state.local_peer_id.to_string(),
                to.clone(),
                amount,
            );
            if let Some(ref m) = memo {
                transfer = transfer.with_memo(m);
            }
            let transfer_msg = CreditMessage::Transfer(transfer);

            match serde_json::to_vec(&transfer_msg) {
                Ok(data) => {
                    if let Err(e) = state.network.publish(topics::CREDIT, data).await {
                        error!("Failed to publish credit transfer: {}", e);
                    } else {
                        let echo_msg = WsMessage::CreditTransfer {
                            id: Uuid::new_v4().to_string(),
                            from: state.local_peer_id.to_string(),
                            to,
                            amount,
                            memo,
                            timestamp,
                        };
                        let _ = state.event_tx.send(echo_msg);
                    }
                }
                Err(e) => {
                    error!("Failed to serialize credit transfer: {}", e);
                }
            }
        }

        ClientMessage::CreateProposal { title, description, proposal_type } => {
            info!("CreateProposal: title='{}'", title);

            let timestamp = chrono::Utc::now().timestamp_millis();

            let proposal_msg = GovernanceMessage::CreateProposal(ProtocolCreateProposal::new(
                state.local_peer_id.to_string(),
                title.clone(),
                description.clone(),
            ));

            match serde_json::to_vec(&proposal_msg) {
                Ok(data) => {
                    if let Err(e) = state.network.publish(topics::GOVERNANCE, data).await {
                        error!("Failed to publish proposal: {}", e);
                    } else {
                        let echo_msg = WsMessage::Proposal {
                            id: Uuid::new_v4().to_string(),
                            proposer: state.local_peer_id.to_string(),
                            title,
                            description,
                            proposal_type,
                            status: "active".to_string(),
                            yes_votes: 0,
                            no_votes: 0,
                            quorum: 3,
                            deadline: timestamp + 86400000, // 24 hours
                            timestamp,
                        };
                        let _ = state.event_tx.send(echo_msg);
                    }
                }
                Err(e) => {
                    error!("Failed to serialize proposal: {}", e);
                }
            }
        }

        ClientMessage::CastVote { proposal_id, vote } => {
            info!("CastVote: proposal_id='{}', vote='{}'", proposal_id, vote);

            let timestamp = chrono::Utc::now().timestamp_millis();

            // Parse proposal_id as UUID
            let prop_uuid = match Uuid::parse_str(&proposal_id) {
                Ok(id) => id,
                Err(e) => {
                    error!("Invalid proposal ID: {}", e);
                    return;
                }
            };

            let vote_enum = match vote.as_str() {
                "yes" => Vote::For,
                "no" => Vote::Against,
                _ => Vote::Abstain,
            };

            // CastVote::new takes (proposal_id: Uuid, voter, vote, weight)
            let vote_msg = GovernanceMessage::CastVote(ProtocolCastVote::new(
                prop_uuid,
                state.local_peer_id.to_string(),
                vote_enum,
                1.0, // Default weight, could be based on reputation
            ));

            match serde_json::to_vec(&vote_msg) {
                Ok(data) => {
                    if let Err(e) = state.network.publish(topics::GOVERNANCE, data).await {
                        error!("Failed to publish vote: {}", e);
                    } else {
                        let echo_msg = WsMessage::VoteCast {
                            id: Uuid::new_v4().to_string(),
                            proposal_id,
                            voter: state.local_peer_id.to_string(),
                            vote,
                            weight: 1.0,
                            timestamp,
                        };
                        let _ = state.event_tx.send(echo_msg);
                    }
                }
                Err(e) => {
                    error!("Failed to serialize vote: {}", e);
                }
            }
        }

        ClientMessage::ReportResource { resource_type, amount, unit } => {
            info!("ReportResource: type='{}', amount={}", resource_type, amount);

            let timestamp = chrono::Utc::now().timestamp_millis();

            let res_type = match resource_type.as_str() {
                "bandwidth" => ResourceType::Bandwidth,
                "storage" => ResourceType::Storage,
                "compute" => ResourceType::Compute,
                _ => ResourceType::Other(resource_type.clone()),
            };

            let resource_msg = ResourceMessage::Contribution(ProtocolResourceContribution::new(
                state.local_peer_id.to_string(),
                res_type,
                amount,
                unit.clone(),
            ));

            match serde_json::to_vec(&resource_msg) {
                Ok(data) => {
                    if let Err(e) = state.network.publish(topics::RESOURCE, data).await {
                        error!("Failed to publish resource contribution: {}", e);
                    } else {
                        let echo_msg = WsMessage::ResourceContribution {
                            id: Uuid::new_v4().to_string(),
                            peer_id: state.local_peer_id.to_string(),
                            resource_type,
                            amount,
                            unit,
                            timestamp,
                        };
                        let _ = state.event_tx.send(echo_msg);
                    }
                }
                Err(e) => {
                    error!("Failed to serialize resource contribution: {}", e);
                }
            }
        }

        // ============ Room/Seance Handlers ============

        ClientMessage::CreateRoom { room_id, room_name, description, is_public } => {
            info!("CreateRoom: name='{}', is_public={:?}", room_name, is_public);

            let timestamp = chrono::Utc::now().timestamp_millis();
            let id = room_id.unwrap_or_else(|| Uuid::new_v4().to_string());
            let topic = format!("/mycelial/1.0.0/room/{}", id);
            let is_public = is_public.unwrap_or(true);

            // Subscribe to the room topic
            if let Err(e) = state.network.subscribe(&topic).await {
                error!("Failed to subscribe to room topic {}: {}", topic, e);
                let error_msg = WsMessage::Error {
                    message: format!("Failed to create room: {}", e),
                };
                let _ = state.event_tx.send(error_msg);
                return;
            }

            info!("Room created and subscribed to topic: {}", topic);

            // Send room joined confirmation
            let room_msg = WsMessage::RoomJoined {
                id: id.clone(),
                name: room_name,
                description,
                topic,
                members: vec![state.local_peer_id.to_string()],
                created_by: state.local_peer_id.to_string(),
                created_at: timestamp,
                is_public,
            };
            let _ = state.event_tx.send(room_msg);
        }

        ClientMessage::JoinRoom { room_id, room_name: _ } => {
            info!("JoinRoom: room_id='{}'", room_id);

            let timestamp = chrono::Utc::now().timestamp_millis();
            let topic = format!("/mycelial/1.0.0/room/{}", room_id);

            // Subscribe to the room topic
            if let Err(e) = state.network.subscribe(&topic).await {
                error!("Failed to subscribe to room topic {}: {}", topic, e);
                let error_msg = WsMessage::Error {
                    message: format!("Failed to join room: {}", e),
                };
                let _ = state.event_tx.send(error_msg);
                return;
            }

            info!("Joined room and subscribed to topic: {}", topic);

            // Send room joined confirmation
            // Note: In a full implementation, we'd fetch room details from state/network
            let room_msg = WsMessage::RoomJoined {
                id: room_id.clone(),
                name: format!("Room {}", &room_id[..8.min(room_id.len())]),
                description: None,
                topic: topic.clone(),
                members: vec![state.local_peer_id.to_string()],
                created_by: "unknown".to_string(),
                created_at: timestamp,
                is_public: true,
            };
            let _ = state.event_tx.send(room_msg);

            // Notify other room members (broadcast to room topic)
            let peer_joined_msg = WsMessage::RoomPeerJoined {
                room_id: room_id.clone(),
                peer_id: state.local_peer_id.to_string(),
                peer_name: Some(state.node_name.clone()),
            };
            if let Ok(data) = serde_json::to_vec(&peer_joined_msg) {
                if let Err(e) = state.network.publish(&topic, data).await {
                    warn!("Failed to announce room join: {}", e);
                }
            }
        }

        ClientMessage::LeaveRoom { room_id } => {
            info!("LeaveRoom: room_id='{}'", room_id);

            let topic = format!("/mycelial/1.0.0/room/{}", room_id);

            // Notify other room members before leaving
            let peer_left_msg = WsMessage::RoomPeerLeft {
                room_id: room_id.clone(),
                peer_id: state.local_peer_id.to_string(),
            };
            if let Ok(data) = serde_json::to_vec(&peer_left_msg) {
                if let Err(e) = state.network.publish(&topic, data).await {
                    warn!("Failed to announce room leave: {}", e);
                }
            }

            // Unsubscribe from the room topic
            if let Err(e) = state.network.unsubscribe(&topic).await {
                error!("Failed to unsubscribe from room topic {}: {}", topic, e);
            }

            info!("Left room and unsubscribed from topic: {}", topic);

            // Send room left confirmation
            let left_msg = WsMessage::RoomLeft { room_id };
            let _ = state.event_tx.send(left_msg);
        }

        ClientMessage::GetRooms => {
            info!("GetRooms requested");

            // For now, send an empty list
            // In a full implementation, we'd query a room registry or DHT
            let rooms_msg = WsMessage::RoomList { rooms: vec![] };
            let _ = state.event_tx.send(rooms_msg);
        }

        // ============ ENR Bridge Handlers ============

        ClientMessage::ReportGradient { cpu_available, memory_available, bandwidth_available, storage_available } => {
            info!("ReportGradient: cpu={}, mem={}, bw={}, storage={}", cpu_available, memory_available, bandwidth_available, storage_available);

            let timestamp = chrono::Utc::now().timestamp_millis();

            // Broadcast gradient update to all clients
            let gradient_msg = WsMessage::GradientUpdate {
                source: state.local_peer_id.to_string(),
                cpu_available,
                memory_available,
                bandwidth_available,
                storage_available,
                timestamp,
            };
            let _ = state.event_tx.send(gradient_msg);
        }

        ClientMessage::StartElection { region_id } => {
            info!("StartElection: region_id='{}'", region_id);

            let timestamp = chrono::Utc::now().timestamp_millis();
            let election_id = timestamp as u64; // Simple ID generation

            let election_msg = WsMessage::ElectionAnnouncement {
                election_id,
                initiator: state.local_peer_id.to_string(),
                region_id,
                timestamp,
            };
            let _ = state.event_tx.send(election_msg);
        }

        ClientMessage::RegisterCandidacy { election_id, uptime, cpu_available, memory_available, reputation } => {
            info!("RegisterCandidacy: election_id={}", election_id);

            let timestamp = chrono::Utc::now().timestamp_millis();

            let candidacy_msg = WsMessage::ElectionCandidacy {
                election_id,
                candidate: state.local_peer_id.to_string(),
                uptime,
                cpu_available,
                memory_available,
                reputation,
                timestamp,
            };
            let _ = state.event_tx.send(candidacy_msg);
        }

        ClientMessage::VoteElection { election_id, candidate } => {
            info!("VoteElection: election_id={}, candidate='{}'", election_id, candidate);

            let timestamp = chrono::Utc::now().timestamp_millis();

            let vote_msg = WsMessage::ElectionVote {
                election_id,
                voter: state.local_peer_id.to_string(),
                candidate,
                timestamp,
            };
            let _ = state.event_tx.send(vote_msg);
        }

        ClientMessage::SendEnrCredit { to, amount } => {
            info!("SendEnrCredit: to='{}', amount={}", to, amount);

            let timestamp = chrono::Utc::now().timestamp_millis();
            static NONCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let nonce = NONCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let tax = amount / 100; // 1% tax

            let transfer_msg = WsMessage::EnrCreditTransfer {
                from: state.local_peer_id.to_string(),
                to: to.clone(),
                amount,
                tax,
                nonce,
                timestamp,
            };
            let _ = state.event_tx.send(transfer_msg);

            // Update sender's balance (decrease)
            let balance_msg = WsMessage::EnrBalanceUpdate {
                node_id: state.local_peer_id.to_string(),
                balance: 0, // Placeholder - real impl would track actual balance
                timestamp,
            };
            let _ = state.event_tx.send(balance_msg);
        }
    }
}
