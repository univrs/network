//! Mycelial Node - P2P network node with dashboard server
//!
//! This binary runs a full mycelial network node with:
//! - P2P networking via libp2p (gossipsub, kademlia, mDNS)
//! - WebSocket server for real-time dashboard updates
//! - REST API for peer and network information

mod server;

use clap::Parser;
use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Instant;
use tokio::sync::broadcast;
use tracing::{info, warn, error, Level};
use tracing_subscriber::FmtSubscriber;

use mycelial_core::peer::{PeerId, PeerInfo};
use mycelial_core::reputation::Reputation;
use mycelial_network::{NetworkService, NetworkHandle, NetworkConfig, NetworkEvent, Keypair, Libp2pPeerId};
use mycelial_network::{is_economics_topic, parse_economics_message, EconomicsEvent};
use mycelial_network::enr_bridge::{
    EnrMessage, GRADIENT_TOPIC, CREDIT_TOPIC, ELECTION_TOPIC, SEPTAL_TOPIC,
};
use mycelial_state::SqliteStore;
use server::messages::{WsMessage, ContributorEntry};
use server::economics_state::{
    EconomicsStateManager, CreditLine, Proposal, ProposalStatus, Vouch, Vote, VoteType,
    ResourceContribution,
};

#[derive(Parser)]
#[command(name = "mycelial-node")]
#[command(about = "Mycelial P2P network node with dashboard server")]
struct Args {
    /// Run as bootstrap node (defaults to ports 9000/8080)
    #[arg(long)]
    bootstrap: bool,

    /// Connect to existing node (multiaddr format)
    #[arg(long, short)]
    connect: Option<String>,

    /// P2P listen port (0 = auto-assign, bootstrap default: 9000, peer default: 0)
    #[arg(long)]
    port: Option<u16>,

    /// Dashboard HTTP server port (0 = auto-assign, bootstrap default: 8080, peer default: 0)
    #[arg(long)]
    http_port: Option<u16>,

    /// Display name for this node
    #[arg(long, short, default_value = "Anonymous")]
    name: String,

    /// Database path
    #[arg(long, default_value = "mycelial.db")]
    db: String,

    /// Enable verbose logging
    #[arg(long, short)]
    verbose: bool,
}

/// Application state shared across handlers
pub struct AppState {
    /// Local peer ID (mycelial-core format)
    pub local_peer_id: PeerId,
    /// Network handle for sending commands
    pub network: NetworkHandle,
    /// State storage
    pub store: SqliteStore,
    /// Broadcast channel for WebSocket events
    pub event_tx: broadcast::Sender<WsMessage>,
    /// Message counter
    pub message_count: AtomicU64,
    /// Node start time
    pub start_time: Instant,
    /// Node name
    pub node_name: String,
    /// Subscribed topics
    pub subscribed_topics: RwLock<Vec<String>>,
    /// Economics state manager for tracking credit lines, proposals, vouches, resources
    pub economics: EconomicsStateManager,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let level = if args.verbose { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Determine ports based on bootstrap flag and user input
    // Bootstrap nodes: default to 9000/8080 for predictable addresses
    // Peer nodes: default to 0 (OS auto-assigns) for easy multi-node testing
    let p2p_port = args.port.unwrap_or(if args.bootstrap { 9000 } else { 0 });
    let http_port = args.http_port.unwrap_or(if args.bootstrap { 8080 } else { 0 });

    info!("Starting Mycelial Node: {}", args.name);
    if args.bootstrap {
        info!("Running as BOOTSTRAP node");
    }

    // Generate keypair
    let keypair = Keypair::generate_ed25519();
    let libp2p_peer_id = keypair.public().to_peer_id();

    // Convert to mycelial-core PeerId (base58 encoded)
    let local_peer_id = PeerId(libp2p_peer_id.to_base58());

    info!("Local peer ID: {}", local_peer_id);

    // Initialize state store
    let db_url = format!("sqlite:{}?mode=rwc", args.db);
    let store = SqliteStore::new(&db_url).await?;
    info!("Database initialized: {}", args.db);

    // Configure network
    // Port 0 tells the OS to assign an available port automatically
    let mut config = NetworkConfig::default();
    config.listen_addresses = vec![
        format!("/ip4/0.0.0.0/tcp/{}", p2p_port),
        format!("/ip4/0.0.0.0/udp/{}/quic-v1", if p2p_port == 0 { 0 } else { p2p_port + 1 }),
    ];

    if p2p_port == 0 {
        info!("P2P port: auto-assign (OS will select available port)");
    } else {
        info!("P2P port: {} (TCP), {} (QUIC)", p2p_port, p2p_port + 1);
    }

    if let Some(ref addr) = args.connect {
        config.bootstrap_peers.push(addr.clone());
        info!("Will connect to bootstrap peer: {}", addr);
    }

    // Create network service
    let (network_service, network_handle, mut event_rx) = NetworkService::new(keypair.clone(), config)?;

    info!("Network service created");

    // Create broadcast channel for WebSocket events
    let (event_tx, _) = broadcast::channel(256);

    // Create shared state
    let state = Arc::new(AppState {
        local_peer_id: local_peer_id.clone(),
        network: network_handle.clone(),
        store,
        event_tx: event_tx.clone(),
        message_count: AtomicU64::new(0),
        start_time: Instant::now(),
        node_name: args.name.clone(),
        subscribed_topics: RwLock::new(Vec::new()),
        economics: EconomicsStateManager::new(),
    });

    // Spawn network service
    tokio::spawn(async move {
        if let Err(e) = network_service.run().await {
            error!("Network error: {}", e);
        }
    });

    // Spawn network event handler
    let event_state = state.clone();
    let peer_id_for_events = libp2p_peer_id;
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            handle_network_event(event, &event_state, peer_id_for_events).await;
        }
    });

    // Start HTTP server - bind to requested port (0 = auto-assign)
    let http_bind_addr = format!("0.0.0.0:{}", http_port);
    let listener = tokio::net::TcpListener::bind(&http_bind_addr).await?;

    // Get the actual bound address (important when port was 0)
    let actual_http_addr = listener.local_addr()?;
    let actual_http_port = actual_http_addr.port();

    info!("═══════════════════════════════════════════════════════════");
    info!("  Dashboard server listening on http://127.0.0.1:{}", actual_http_port);
    info!("  WebSocket endpoint: ws://127.0.0.1:{}/ws", actual_http_port);
    info!("  REST API: http://127.0.0.1:{}/api/", actual_http_port);
    info!("═══════════════════════════════════════════════════════════");

    let app = server::create_router(state);
    axum::serve(listener, app).await?;

    Ok(())
}

/// Handle events from the P2P network
async fn handle_network_event(event: NetworkEvent, state: &AppState, local_peer_id: Libp2pPeerId) {
    match event {
        NetworkEvent::PeerConnected { peer_id, num_connections } => {
            info!("Peer connected: {} (total: {})", peer_id, num_connections);

            let core_peer_id = PeerId(peer_id.to_base58());
            let short_id = &peer_id.to_base58()[..8.min(peer_id.to_base58().len())];

            // Create peer info
            // Use peer_id's base58 as public_key (PeerId is derived from public key)
            let peer_info = PeerInfo {
                id: core_peer_id.clone(),
                public_key: peer_id.to_base58(),
                addresses: vec![],
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                name: Some(format!("Peer-{}", short_id)),
            };

            // Store peer with default reputation
            if let Err(e) = state.store.upsert_peer(&peer_info, Some(&Reputation::default())).await {
                warn!("Failed to store peer: {}", e);
            }

            // Broadcast to dashboard
            let _ = state.event_tx.send(WsMessage::PeerJoined {
                peer_id: peer_id.to_base58(),
                name: peer_info.name.clone(),
            });
        }

        NetworkEvent::PeerDisconnected { peer_id, num_connections } => {
            info!("Peer disconnected: {} (remaining: {})", peer_id, num_connections);
            let _ = state.event_tx.send(WsMessage::PeerLeft {
                peer_id: peer_id.to_base58(),
            });
        }

        NetworkEvent::MessageReceived { message_id, topic, source, data, timestamp } => {
            // Update message count
            state.message_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            let from_id = source.map(|p| p.to_base58()).unwrap_or_else(|| "unknown".to_string());
            let ts = timestamp.timestamp_millis();

            // Check if this is an economics protocol message
            if is_economics_topic(&topic) {
                if let Some(econ_event) = parse_economics_message(&topic, &data) {
                    match econ_event {
                        EconomicsEvent::Vouch(vouch_msg) => {
                            use mycelial_protocol::VouchMessage;
                            match vouch_msg {
                                VouchMessage::VouchRequest(req) => {
                                    // Track vouch in state
                                    state.economics.add_vouch(Vouch {
                                        id: req.id.to_string(),
                                        voucher: req.voucher.clone(),
                                        vouchee: req.vouchee.clone(),
                                        weight: req.stake,
                                        accepted: false, // Pending until ack
                                        created_at: ts,
                                    });

                                    let _ = state.event_tx.send(WsMessage::VouchRequest {
                                        id: req.id.to_string(),
                                        voucher: req.voucher,
                                        vouchee: req.vouchee,
                                        weight: req.stake,
                                        timestamp: ts,
                                    });
                                }
                                VouchMessage::VouchAck(ack) => {
                                    // Update vouch state and get new reputation
                                    let vouch_id = ack.vouch_id.to_string();
                                    let new_rep = state.economics.respond_to_vouch(&vouch_id, ack.accepted)
                                        .map(|v| state.economics.get_reputation(&v.vouchee));

                                    let _ = state.event_tx.send(WsMessage::VouchAck {
                                        id: message_id.to_string(),
                                        request_id: vouch_id,
                                        accepted: ack.accepted,
                                        new_reputation: new_rep,
                                        timestamp: ts,
                                    });
                                }
                                VouchMessage::ReputationUpdate(update) => {
                                    let _ = state.event_tx.send(WsMessage::ReputationUpdate {
                                        peer_id: update.peer_id,
                                        new_score: update.score,
                                    });
                                }
                            }
                        }
                        EconomicsEvent::Credit(credit_msg) => {
                            use mycelial_protocol::CreditMessage;
                            match credit_msg {
                                CreditMessage::CreateLine(line) => {
                                    let line_id = line.id.to_string();

                                    // Track credit line in state
                                    state.economics.upsert_credit_line(CreditLine {
                                        id: line_id.clone(),
                                        creditor: line.creditor.clone(),
                                        debtor: line.debtor.clone(),
                                        limit: line.limit,
                                        balance: 0.0,
                                        created_at: ts,
                                        updated_at: ts,
                                    });

                                    let _ = state.event_tx.send(WsMessage::CreditLine {
                                        id: line_id,
                                        creditor: line.creditor,
                                        debtor: line.debtor,
                                        limit: line.limit,
                                        balance: 0.0,
                                        timestamp: ts,
                                    });
                                }
                                CreditMessage::Transfer(transfer) => {
                                    // Update credit line balance if exists
                                    // Transfer from debtor to creditor decreases balance
                                    // Transfer from creditor to debtor increases balance
                                    if let Some(line) = state.economics.get_credit_line_between(&transfer.to, &transfer.from) {
                                        // transfer.from is debtor, transfer.to is creditor
                                        // Debtor paying back - decrease balance
                                        let new_balance = (line.balance - transfer.amount).max(0.0);
                                        state.economics.update_credit_balance(&line.id, new_balance);
                                    } else if let Some(line) = state.economics.get_credit_line_between(&transfer.from, &transfer.to) {
                                        // transfer.from is creditor, transfer.to is debtor
                                        // Extending credit - increase balance
                                        let new_balance = (line.balance + transfer.amount).min(line.limit);
                                        state.economics.update_credit_balance(&line.id, new_balance);
                                    }

                                    let _ = state.event_tx.send(WsMessage::CreditTransfer {
                                        id: transfer.id.to_string(),
                                        from: transfer.from,
                                        to: transfer.to,
                                        amount: transfer.amount,
                                        memo: transfer.memo,
                                        timestamp: ts,
                                    });
                                }
                                CreditMessage::LineAck(ack) => {
                                    // LineAck doesn't have creditor/debtor/limit - it's just an ack
                                    // We can skip or send a minimal message
                                    info!("Credit line {} {}", ack.line_id, if ack.accepted { "accepted" } else { "rejected" });
                                }
                                CreditMessage::TransferAck(_) | CreditMessage::LineUpdate(_) => {
                                    // Handle additional credit events if needed
                                }
                            }
                        }
                        EconomicsEvent::Governance(gov_msg) => {
                            use mycelial_protocol::GovernanceMessage;
                            match gov_msg {
                                GovernanceMessage::CreateProposal(proposal) => {
                                    let proposal_id = proposal.id.to_string();
                                    let deadline_ms = proposal.deadline.timestamp_millis();
                                    let quorum_pct = (proposal.quorum * 100.0) as u32;

                                    // Track proposal in state
                                    state.economics.add_proposal(Proposal {
                                        id: proposal_id.clone(),
                                        proposer: proposal.proposer.clone(),
                                        title: proposal.title.clone(),
                                        description: proposal.description.clone(),
                                        proposal_type: format!("{:?}", proposal.proposal_type),
                                        status: ProposalStatus::Active,
                                        yes_votes: 0.0,
                                        no_votes: 0.0,
                                        quorum: proposal.quorum,
                                        deadline: deadline_ms,
                                        created_at: ts,
                                        votes: std::collections::HashMap::new(),
                                    });

                                    let _ = state.event_tx.send(WsMessage::Proposal {
                                        id: proposal_id,
                                        proposer: proposal.proposer,
                                        title: proposal.title,
                                        description: proposal.description,
                                        proposal_type: format!("{:?}", proposal.proposal_type),
                                        status: "active".to_string(),
                                        yes_votes: 0,
                                        no_votes: 0,
                                        quorum: quorum_pct,
                                        deadline: deadline_ms,
                                        timestamp: ts,
                                    });
                                }
                                GovernanceMessage::CastVote(vote) => {
                                    let proposal_id = vote.proposal_id.to_string();

                                    // Parse vote type
                                    let vote_type = match format!("{:?}", vote.vote).to_lowercase().as_str() {
                                        "yes" => VoteType::Yes,
                                        "no" => VoteType::No,
                                        _ => VoteType::Abstain,
                                    };

                                    // Record vote in state
                                    state.economics.record_vote(&proposal_id, Vote {
                                        voter: vote.voter.clone(),
                                        vote_type,
                                        weight: vote.weight,
                                        timestamp: ts,
                                    });

                                    let _ = state.event_tx.send(WsMessage::VoteCast {
                                        id: message_id.to_string(),
                                        proposal_id,
                                        voter: vote.voter,
                                        vote: format!("{:?}", vote.vote),
                                        weight: vote.weight,
                                        timestamp: ts,
                                    });
                                }
                                GovernanceMessage::ProposalUpdate(update) => {
                                    // votes_for/against are f64 (weighted), convert to u32 counts
                                    let _ = state.event_tx.send(WsMessage::Proposal {
                                        id: update.proposal_id.to_string(),
                                        proposer: "".to_string(),
                                        title: "".to_string(),
                                        description: "".to_string(),
                                        proposal_type: "".to_string(),
                                        status: format!("{:?}", update.status),
                                        yes_votes: update.votes_for as u32,
                                        no_votes: update.votes_against as u32,
                                        quorum: 0,
                                        deadline: 0,
                                        timestamp: ts,
                                    });
                                }
                                GovernanceMessage::ProposalExecuted(_) => {
                                    // Handle proposal execution if needed
                                }
                            }
                        }
                        EconomicsEvent::Resource(res_msg) => {
                            use mycelial_protocol::ResourceMessage;
                            match res_msg {
                                ResourceMessage::Contribution(contrib) => {
                                    let resource_type = format!("{:?}", contrib.resource_type);

                                    // Record contribution in state
                                    state.economics.record_resource_contribution(ResourceContribution {
                                        peer_id: contrib.peer_id.clone(),
                                        resource_type: resource_type.clone(),
                                        amount: contrib.amount,
                                        unit: contrib.unit.clone(),
                                        timestamp: ts,
                                    });

                                    let _ = state.event_tx.send(WsMessage::ResourceContribution {
                                        id: contrib.id.to_string(),
                                        peer_id: contrib.peer_id,
                                        resource_type,
                                        amount: contrib.amount,
                                        unit: contrib.unit,
                                        timestamp: ts,
                                    });
                                }
                                ResourceMessage::PoolUpdate(pool) => {
                                    let contributors: Vec<ContributorEntry> = pool.top_contributors
                                        .iter()
                                        .map(|c| ContributorEntry {
                                            peer_id: c.peer_id.clone(),
                                            contribution: c.contribution_score,
                                            percentage: 0.0, // Not available in protocol type
                                        })
                                        .collect();
                                    let _ = state.event_tx.send(WsMessage::ResourcePoolUpdate {
                                        resource_type: "pool".to_string(),
                                        total_available: pool.total_bandwidth + pool.total_compute,
                                        total_used: 0.0, // Not tracked in protocol
                                        contributors,
                                        timestamp: ts,
                                    });
                                }
                                ResourceMessage::Metrics(_) => {
                                    // Handle resource metrics if needed
                                }
                            }
                        }
                    }
                }
            }
            // Check if this is an ENR bridge message
            else if topic == GRADIENT_TOPIC || topic == CREDIT_TOPIC || topic == ELECTION_TOPIC || topic == SEPTAL_TOPIC {
                match EnrMessage::decode(&data) {
                    Ok(enr_msg) => {
                        use mycelial_network::enr_bridge::messages::*;
                        match enr_msg {
                            EnrMessage::GradientUpdate(update) => {
                                let _ = state.event_tx.send(WsMessage::GradientUpdate {
                                    source: update.source.to_string(),
                                    cpu_available: update.gradient.cpu_available,
                                    memory_available: update.gradient.memory_available,
                                    bandwidth_available: update.gradient.bandwidth_available,
                                    storage_available: update.gradient.storage_available,
                                    timestamp: update.timestamp.millis as i64,
                                });
                            }
                            EnrMessage::CreditTransfer(transfer_msg) => {
                                let _ = state.event_tx.send(WsMessage::EnrCreditTransfer {
                                    from: format!("{}", transfer_msg.transfer.from.node),
                                    to: format!("{}", transfer_msg.transfer.to.node),
                                    amount: transfer_msg.transfer.amount.amount,
                                    tax: transfer_msg.transfer.entropy_cost.amount,
                                    nonce: transfer_msg.nonce,
                                    timestamp: ts,
                                });
                            }
                            EnrMessage::BalanceQuery(_) => {
                                // Balance queries are internal, no dashboard broadcast
                            }
                            EnrMessage::BalanceResponse(resp) => {
                                let _ = state.event_tx.send(WsMessage::EnrBalanceUpdate {
                                    node_id: "query_response".to_string(),
                                    balance: resp.balance.amount,
                                    timestamp: resp.as_of.millis as i64,
                                });
                            }
                            EnrMessage::Election(election_msg) => {
                                match election_msg {
                                    ElectionMessage::Announcement(ann) => {
                                        let _ = state.event_tx.send(WsMessage::ElectionAnnouncement {
                                            election_id: ann.election_id,
                                            initiator: ann.initiator.to_string(),
                                            region_id: ann.region_id,
                                            timestamp: ann.timestamp.millis as i64,
                                        });
                                    }
                                    ElectionMessage::Candidacy(candidacy) => {
                                        let _ = state.event_tx.send(WsMessage::ElectionCandidacy {
                                            election_id: candidacy.election_id,
                                            candidate: candidacy.candidate.node.to_string(),
                                            uptime: (candidacy.candidate.uptime * 1000.0) as u64, // Convert f64 to millis
                                            cpu_available: 0.0, // Not in NexusCandidate, use default
                                            memory_available: 0.0, // Not in NexusCandidate, use default
                                            reputation: candidacy.candidate.reputation,
                                            timestamp: ts,
                                        });
                                    }
                                    ElectionMessage::Vote(vote) => {
                                        let _ = state.event_tx.send(WsMessage::ElectionVote {
                                            election_id: vote.election_id,
                                            voter: vote.voter.to_string(),
                                            candidate: vote.candidate.to_string(),
                                            timestamp: vote.timestamp.millis as i64,
                                        });
                                    }
                                    ElectionMessage::Result(result) => {
                                        let _ = state.event_tx.send(WsMessage::ElectionResult {
                                            election_id: result.election_id,
                                            winner: result.winner.to_string(),
                                            region_id: result.region_id,
                                            vote_count: result.vote_count,
                                            timestamp: result.timestamp.millis as i64,
                                        });
                                    }
                                }
                            }
                            EnrMessage::Septal(septal_msg) => {
                                match septal_msg {
                                    SeptalMessage::StateChange(change) => {
                                        let _ = state.event_tx.send(WsMessage::SeptalStateChange {
                                            node_id: change.node.to_string(),
                                            from_state: format!("{:?}", change.from_state),
                                            to_state: format!("{:?}", change.to_state),
                                            reason: change.reason,
                                            timestamp: change.timestamp.millis as i64,
                                        });
                                    }
                                    SeptalMessage::HealthProbe(_) => {
                                        // Health probes are internal, no dashboard broadcast
                                    }
                                    SeptalMessage::HealthResponse(resp) => {
                                        let _ = state.event_tx.send(WsMessage::SeptalHealthStatus {
                                            node_id: resp.node.to_string(),
                                            is_healthy: resp.is_healthy,
                                            failure_count: resp.failure_count,
                                            timestamp: resp.timestamp.millis as i64,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to decode ENR message on {}: {}", topic, e);
                    }
                }
            }
            // Try to parse as chat message (handles chat, content, direct, and room topics)
            else if topic.contains("chat") || topic.contains("content") || topic.contains("direct") || topic.contains("room") {
                if let Ok(content) = String::from_utf8(data.clone()) {
                    let short_from = &from_id[..8.min(from_id.len())];

                    // Extract room_id from topic if it's a room message
                    // Topic format: /mycelial/1.0.0/room/{room_id}
                    let room_id = if topic.contains("/room/") {
                        topic.split("/room/").nth(1).map(|s| s.to_string())
                    } else {
                        None
                    };

                    let _ = state.event_tx.send(WsMessage::ChatMessage {
                        id: message_id.to_string(),
                        from: from_id.clone(),
                        from_name: format!("Peer-{}", short_from),
                        to: None,
                        room_id,
                        content,
                        timestamp: ts,
                    });
                }
            }
        }

        NetworkEvent::ListeningOn { address } => {
            // Print full multiaddr with peer ID so users know how to connect
            let full_multiaddr = format!("{}/p2p/{}", address, local_peer_id);
            info!("═══════════════════════════════════════════════════════════");
            info!("  P2P Listening on: {}", address);
            info!("  Full multiaddr (use this to connect):");
            info!("    {}", full_multiaddr);
            info!("═══════════════════════════════════════════════════════════");
        }

        NetworkEvent::Subscribed { topic } => {
            info!("Subscribed to topic: {}", topic);
            state.subscribed_topics.write().push(topic);
        }

        NetworkEvent::Unsubscribed { topic } => {
            info!("Unsubscribed from topic: {}", topic);
            state.subscribed_topics.write().retain(|t| t != &topic);
        }

        NetworkEvent::Started { peer_id, listen_addresses: _ } => {
            info!("Network started for peer: {}", peer_id);
            info!("Listen addresses will be reported as they become available");
        }

        NetworkEvent::Stopped => {
            info!("Network stopped");
        }

        NetworkEvent::DialFailed { peer_id, error } => {
            if let Some(pid) = peer_id {
                warn!("Failed to dial {}: {}", pid, error);
            }
        }

        NetworkEvent::MdnsDiscovered { peers } => {
            for (peer_id, addr) in &peers {
                info!("mDNS discovered: {} at {}", peer_id, addr);
            }
        }

        _ => {}
    }
}
