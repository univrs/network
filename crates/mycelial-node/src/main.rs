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
use mycelial_state::SqliteStore;
use server::messages::WsMessage;

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
            let peer_info = PeerInfo {
                id: core_peer_id.clone(),
                public_key: vec![],
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

            // Try to parse as chat message (handles chat, content, and direct topics)
            if topic.contains("chat") || topic.contains("content") || topic.contains("direct") {
                if let Ok(content) = String::from_utf8(data.clone()) {
                    let from_id = source.map(|p| p.to_base58()).unwrap_or_else(|| "unknown".to_string());
                    let short_from = &from_id[..8.min(from_id.len())];

                    let _ = state.event_tx.send(WsMessage::ChatMessage {
                        id: message_id.to_string(),
                        from: from_id.clone(),
                        from_name: format!("Peer-{}", short_from),
                        to: None,
                        content,
                        timestamp: timestamp.timestamp_millis(),
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
