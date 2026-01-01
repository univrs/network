//! Network service - main entry point for network operations
//!
//! The NetworkService manages the libp2p swarm, handles events,
//! and provides a high-level API for network operations.

use futures::StreamExt;
use libp2p::{gossipsub, identify, kad, mdns, swarm::SwarmEvent, Multiaddr, PeerId, Swarm};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn};

use crate::behaviour::{MycelialBehaviour, MycelialBehaviourEvent};
use crate::config::NetworkConfig;
#[cfg(feature = "univrs-compat")]
use crate::enr_bridge::{EnrBridge, CREDIT_TOPIC, ELECTION_TOPIC, GRADIENT_TOPIC, SEPTAL_TOPIC};
use crate::error::{NetworkError, Result};
use crate::event::{NetworkEvent, NetworkStats};
use crate::peer::{ConnectionState, PeerManager};
use crate::transport::{self, TransportConfig};
#[cfg(feature = "univrs-compat")]
use univrs_enr::core::NodeId;

/// Commands sent to the network service
#[derive(Debug)]
pub enum NetworkCommand {
    /// Dial a peer
    Dial { address: Multiaddr },
    /// Disconnect from a peer
    Disconnect { peer_id: PeerId },
    /// Subscribe to a topic
    Subscribe { topic: String },
    /// Unsubscribe from a topic
    Unsubscribe { topic: String },
    /// Publish a message
    Publish { topic: String, data: Vec<u8> },
    /// Store a value in the DHT
    PutRecord { key: Vec<u8>, value: Vec<u8> },
    /// Get a value from the DHT
    GetRecord { key: Vec<u8> },
    /// Get connected peers
    GetPeers {
        response: tokio::sync::oneshot::Sender<Vec<PeerId>>,
    },
    /// Get network stats
    GetStats {
        response: tokio::sync::oneshot::Sender<NetworkStats>,
    },
    /// Shutdown
    Shutdown,
}

/// Handle for interacting with the network service
#[derive(Clone)]
pub struct NetworkHandle {
    command_tx: mpsc::Sender<NetworkCommand>,
    local_peer_id: PeerId,
}

impl NetworkHandle {
    /// Get the local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Dial a peer by multiaddr
    pub async fn dial(&self, address: Multiaddr) -> Result<()> {
        self.command_tx
            .send(NetworkCommand::Dial { address })
            .await
            .map_err(|_| NetworkError::Channel("Failed to send dial command".into()))
    }

    /// Disconnect from a peer
    pub async fn disconnect(&self, peer_id: PeerId) -> Result<()> {
        self.command_tx
            .send(NetworkCommand::Disconnect { peer_id })
            .await
            .map_err(|_| NetworkError::Channel("Failed to send disconnect command".into()))
    }

    /// Subscribe to a gossipsub topic
    pub async fn subscribe(&self, topic: impl Into<String>) -> Result<()> {
        self.command_tx
            .send(NetworkCommand::Subscribe {
                topic: topic.into(),
            })
            .await
            .map_err(|_| NetworkError::Channel("Failed to send subscribe command".into()))
    }

    /// Unsubscribe from a gossipsub topic
    pub async fn unsubscribe(&self, topic: impl Into<String>) -> Result<()> {
        self.command_tx
            .send(NetworkCommand::Unsubscribe {
                topic: topic.into(),
            })
            .await
            .map_err(|_| NetworkError::Channel("Failed to send unsubscribe command".into()))
    }

    /// Publish a message to a gossipsub topic
    pub async fn publish(&self, topic: impl Into<String>, data: Vec<u8>) -> Result<()> {
        self.command_tx
            .send(NetworkCommand::Publish {
                topic: topic.into(),
                data,
            })
            .await
            .map_err(|_| NetworkError::Channel("Failed to send publish command".into()))
    }

    /// Store a value in the DHT
    pub async fn put_record(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.command_tx
            .send(NetworkCommand::PutRecord { key, value })
            .await
            .map_err(|_| NetworkError::Channel("Failed to send put_record command".into()))
    }

    /// Get a value from the DHT
    pub async fn get_record(&self, key: Vec<u8>) -> Result<()> {
        self.command_tx
            .send(NetworkCommand::GetRecord { key })
            .await
            .map_err(|_| NetworkError::Channel("Failed to send get_record command".into()))
    }

    /// Get list of connected peers
    pub async fn get_peers(&self) -> Result<Vec<PeerId>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(NetworkCommand::GetPeers { response: tx })
            .await
            .map_err(|_| NetworkError::Channel("Failed to send get_peers command".into()))?;

        rx.await
            .map_err(|_| NetworkError::Channel("Failed to receive peers".into()))
    }

    /// Get network statistics
    pub async fn get_stats(&self) -> Result<NetworkStats> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(NetworkCommand::GetStats { response: tx })
            .await
            .map_err(|_| NetworkError::Channel("Failed to send get_stats command".into()))?;

        rx.await
            .map_err(|_| NetworkError::Channel("Failed to receive stats".into()))
    }

    /// Shutdown the network service
    pub async fn shutdown(&self) -> Result<()> {
        self.command_tx
            .send(NetworkCommand::Shutdown)
            .await
            .map_err(|_| NetworkError::Channel("Failed to send shutdown command".into()))
    }
}

/// The network service manages all P2P networking
pub struct NetworkService {
    /// The libp2p swarm
    swarm: Swarm<MycelialBehaviour>,
    /// Configuration
    config: NetworkConfig,
    /// Peer manager
    peer_manager: Arc<PeerManager>,
    /// Event broadcaster
    event_tx: broadcast::Sender<NetworkEvent>,
    /// Command receiver
    command_rx: mpsc::Receiver<NetworkCommand>,
    /// Command sender (for creating handles)
    #[allow(dead_code)]
    command_tx: mpsc::Sender<NetworkCommand>,
    /// Subscribed topics
    subscribed_topics: HashSet<String>,
    /// Statistics
    stats: Arc<RwLock<NetworkStats>>,
    /// Start time
    start_time: Instant,
    /// Running flag
    running: bool,
    /// ENR bridge for economic primitives (requires univrs-compat feature)
    #[cfg(feature = "univrs-compat")]
    enr_bridge: Arc<EnrBridge>,
}

impl NetworkService {
    /// Create a new network service
    pub fn new(
        keypair: libp2p::identity::Keypair,
        config: NetworkConfig,
    ) -> Result<(Self, NetworkHandle, broadcast::Receiver<NetworkEvent>)> {
        let local_peer_id = keypair.public().to_peer_id();
        info!("Local peer ID: {}", local_peer_id);

        // Create transport
        let transport_config = TransportConfig {
            enable_tcp: config.enable_tcp,
            enable_quic: config.enable_quic,
            ..Default::default()
        };
        let transport = transport::create_transport(&keypair, &transport_config)?;

        // Create behaviour
        let behaviour = MycelialBehaviour::new(&keypair, &config)?;

        // Create swarm
        let swarm = Swarm::new(
            transport,
            behaviour,
            local_peer_id,
            libp2p::swarm::Config::with_tokio_executor(),
        );

        // Create channels
        let (event_tx, event_rx) = broadcast::channel(1024);
        let (command_tx, command_rx) = mpsc::channel(256);

        let handle = NetworkHandle {
            command_tx: command_tx.clone(),
            local_peer_id,
        };

        // Create ENR bridge with publish callback (requires univrs-compat feature)
        #[cfg(feature = "univrs-compat")]
        let enr_bridge = {
            // Convert PeerId to NodeId (use peer_id bytes, padded/truncated to 32)
            let peer_id_bytes = local_peer_id.to_bytes();
            let mut node_id_bytes = [0u8; 32];
            let len = peer_id_bytes.len().min(32);
            node_id_bytes[..len].copy_from_slice(&peer_id_bytes[..len]);
            let local_node_id = NodeId::from_bytes(node_id_bytes);

            // Create publish callback that uses the command channel
            let publish_tx = command_tx.clone();
            let publish_fn = move |topic: String, data: Vec<u8>| {
                // Use try_send which is non-blocking and works in any context
                // This may fail if the channel is full, but that's acceptable
                // for gossip messages which can be retried
                publish_tx
                    .try_send(NetworkCommand::Publish { topic, data })
                    .map_err(|e| e.to_string())
            };

            Arc::new(EnrBridge::new(local_node_id, publish_fn))
        };

        let service = Self {
            swarm,
            config,
            peer_manager: Arc::new(PeerManager::default()),
            event_tx,
            command_rx,
            command_tx,
            subscribed_topics: HashSet::new(),
            stats: Arc::new(RwLock::new(NetworkStats::default())),
            start_time: Instant::now(),
            running: false,
            #[cfg(feature = "univrs-compat")]
            enr_bridge,
        };

        Ok((service, handle, event_rx))
    }

    /// Get a reference to the peer manager
    pub fn peer_manager(&self) -> &Arc<PeerManager> {
        &self.peer_manager
    }

    /// Get a reference to the ENR bridge for economic operations (requires univrs-compat feature)
    #[cfg(feature = "univrs-compat")]
    pub fn enr_bridge(&self) -> &Arc<EnrBridge> {
        &self.enr_bridge
    }

    /// Start the network service
    pub async fn run(mut self) -> Result<()> {
        info!("Starting network service");

        // Start listening on configured addresses
        for addr_str in &self.config.listen_addresses.clone() {
            let addr: Multiaddr = addr_str
                .parse()
                .map_err(|e| NetworkError::InvalidMultiaddr(format!("{}: {}", addr_str, e)))?;

            self.swarm
                .listen_on(addr.clone())
                .map_err(|e| NetworkError::ListenFailed {
                    address: addr_str.clone(),
                    reason: e.to_string(),
                })?;

            info!("Listening on {}", addr);
        }

        // Subscribe to gossipsub topics
        // Note: mesh_n=2, mesh_n_low=1 configured for small test networks
        info!("Gossipsub config: mesh_outbound_min=0, mesh_n=2, mesh_n_low=1, mesh_n_high=4 (optimized for small networks)");

        // Core topics always subscribed
        let core_topics = [
            // Core messaging topics
            "/mycelial/1.0.0/chat",
            "/mycelial/1.0.0/announce",
            "/mycelial/1.0.0/reputation",
            "/mycelial/1.0.0/direct",
            // Economics protocol topics (Phase 7)
            "/mycelial/1.0.0/vouch",      // Vouch/reputation delegation
            "/mycelial/1.0.0/credit",     // Mutual credit transactions
            "/mycelial/1.0.0/governance", // Proposals and voting
            "/mycelial/1.0.0/resource",   // Resource sharing metrics
        ];

        // ENR bridge topics (only with univrs-compat feature)
        #[cfg(feature = "univrs-compat")]
        let enr_topics = [
            GRADIENT_TOPIC, // Resource gradient broadcasts
            CREDIT_TOPIC,   // Credit transfers
            ELECTION_TOPIC, // Nexus election
            SEPTAL_TOPIC,   // Septal gate (circuit breaker)
        ];
        #[cfg(not(feature = "univrs-compat"))]
        let enr_topics: [&str; 0] = [];

        // Combine all topics
        let topics: Vec<&str> = core_topics
            .iter()
            .copied()
            .chain(enr_topics.iter().copied())
            .collect();
        for topic_str in topics {
            let topic = libp2p::gossipsub::IdentTopic::new(topic_str);
            match self.swarm.behaviour_mut().gossipsub.subscribe(&topic) {
                Ok(true) => {
                    info!(
                        "Subscribed to topic: {} (awaiting mesh formation)",
                        topic_str
                    );
                    self.subscribed_topics.insert(topic_str.to_string());
                    // Emit event so AppState gets updated
                    let _ = self.event_tx.send(NetworkEvent::Subscribed {
                        topic: topic_str.to_string(),
                    });
                }
                Ok(false) => debug!("Already subscribed to: {}", topic_str),
                Err(e) => warn!("Failed to subscribe to {}: {:?}", topic_str, e),
            }
        }

        // Connect to bootstrap peers
        for addr_str in &self.config.bootstrap_peers.clone() {
            let addr: Multiaddr = match addr_str.parse() {
                Ok(a) => a,
                Err(e) => {
                    warn!("Invalid bootstrap address {}: {}", addr_str, e);
                    continue;
                }
            };

            if let Err(e) = self.swarm.dial(addr.clone()) {
                warn!("Failed to dial bootstrap peer {}: {:?}", addr, e);
            } else {
                info!("Dialing bootstrap peer {}", addr);
            }
        }

        self.running = true;

        // Emit started event
        let _ = self.event_tx.send(NetworkEvent::Started {
            peer_id: *self.swarm.local_peer_id(),
            listen_addresses: self.swarm.listeners().cloned().collect(),
        });

        // Main event loop
        loop {
            tokio::select! {
                // Handle swarm events
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await;
                }

                // Handle commands
                Some(cmd) = self.command_rx.recv() => {
                    if !self.handle_command(cmd).await {
                        break;
                    }
                }
            }

            // Update stats
            {
                let mut stats = self.stats.write();
                stats.connected_peers = self.peer_manager.connected_count();
                stats.subscribed_topics = self.subscribed_topics.len();
                stats.uptime_secs = self.start_time.elapsed().as_secs();
            }
        }

        self.running = false;
        let _ = self.event_tx.send(NetworkEvent::Stopped);
        info!("Network service stopped");

        Ok(())
    }

    /// Handle a swarm event
    async fn handle_swarm_event(&mut self, event: SwarmEvent<MycelialBehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(behaviour_event) => {
                self.handle_behaviour_event(behaviour_event).await;
            }

            SwarmEvent::ConnectionEstablished {
                peer_id,
                num_established,
                endpoint,
                ..
            } => {
                debug!("Connection established with {}", peer_id);

                self.peer_manager
                    .set_state(peer_id, ConnectionState::Connected);

                let addr = endpoint.get_remote_address();
                self.peer_manager.add_address(peer_id, addr.clone());

                let _ = self.event_tx.send(NetworkEvent::ConnectionEstablished {
                    peer_id,
                    num_established: num_established.get(),
                    outbound: endpoint.is_dialer(),
                });

                if num_established.get() == 1 {
                    let _ = self.event_tx.send(NetworkEvent::PeerConnected {
                        peer_id,
                        num_connections: self.peer_manager.connected_count(),
                    });
                }
            }

            SwarmEvent::ConnectionClosed {
                peer_id,
                num_established,
                cause,
                ..
            } => {
                debug!("Connection closed with {}: {:?}", peer_id, cause);

                if num_established == 0 {
                    self.peer_manager
                        .set_state(peer_id, ConnectionState::Disconnected);

                    let _ = self.event_tx.send(NetworkEvent::PeerDisconnected {
                        peer_id,
                        num_connections: self.peer_manager.connected_count(),
                    });
                }

                let _ = self.event_tx.send(NetworkEvent::ConnectionClosed {
                    peer_id,
                    num_established,
                    cause: cause.map(|e| e.to_string()),
                });
            }

            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);
                let _ = self.event_tx.send(NetworkEvent::ListeningOn { address });
            }

            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                if let Some(peer_id) = peer_id {
                    // Only mark as failed if not already connected or connecting
                    // Dial errors for secondary addresses shouldn't affect existing connections,
                    // and dial errors during connection setup shouldn't override Connecting state
                    let current_state = self.peer_manager.get_state(&peer_id);
                    match current_state {
                        Some(ConnectionState::Connected) | Some(ConnectionState::Connecting) => {
                            debug!(
                                "Ignoring dial error for peer {} (state: {:?})",
                                peer_id, current_state
                            );
                        }
                        _ => {
                            // Check if we're actually connected to this peer via the swarm
                            if self.swarm.is_connected(&peer_id) {
                                debug!("Ignoring dial error for swarm-connected peer {}", peer_id);
                            } else {
                                warn!("Dial error for {}: {:?}", peer_id, error);
                                self.peer_manager
                                    .set_state(peer_id, ConnectionState::Failed);
                            }
                        }
                    }
                }

                let _ = self.event_tx.send(NetworkEvent::DialFailed {
                    peer_id,
                    error: error.to_string(),
                });
            }

            SwarmEvent::Dialing {
                peer_id: Some(peer_id),
                ..
            } => {
                debug!("Dialing {}", peer_id);
                self.peer_manager
                    .set_state(peer_id, ConnectionState::Connecting);
                let _ = self.event_tx.send(NetworkEvent::Dialing { peer_id });
            }
            SwarmEvent::Dialing { peer_id: None, .. } => {}

            _ => {}
        }
    }

    /// Handle a behaviour event
    async fn handle_behaviour_event(&mut self, event: MycelialBehaviourEvent) {
        match event {
            MycelialBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source: _,
                message_id,
                message,
            }) => {
                let topic_str = message.topic.to_string();
                debug!(
                    "Received message on topic {} from {:?}",
                    topic_str, message.source
                );

                {
                    let mut stats = self.stats.write();
                    stats.messages_received += 1;
                    stats.bytes_received += message.data.len() as u64;
                }

                // Route ENR messages to the bridge handler (requires univrs-compat feature)
                #[cfg(feature = "univrs-compat")]
                if topic_str == GRADIENT_TOPIC
                    || topic_str == CREDIT_TOPIC
                    || topic_str == ELECTION_TOPIC
                    || topic_str == SEPTAL_TOPIC
                {
                    let bridge = self.enr_bridge.clone();
                    let data = message.data.clone();
                    tokio::spawn(async move {
                        if let Err(e) = bridge.handle_message(&data).await {
                            warn!("Failed to handle ENR message: {}", e);
                        }
                    });
                }

                let _ = self.event_tx.send(NetworkEvent::MessageReceived {
                    message_id,
                    topic: topic_str,
                    source: message.source,
                    data: message.data,
                    timestamp: chrono::Utc::now(),
                });
            }

            MycelialBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic }) => {
                // Log at info level with mesh peer count for debugging mesh formation
                let topic_str = topic.to_string();
                let mesh_peers = self.swarm.behaviour().mesh_peers(&topic_str);
                let all_peers = self.swarm.behaviour().all_peers_on_topic(&topic_str);

                info!(
                    "Peer {} subscribed to '{}' | Mesh peers: {} | Total subscribed: {}",
                    peer_id,
                    topic_str,
                    mesh_peers.len(),
                    all_peers.len()
                );

                if !mesh_peers.is_empty() {
                    debug!("Current mesh peers for '{}': {:?}", topic_str, mesh_peers);
                }

                let _ = self.event_tx.send(NetworkEvent::PeerSubscribed {
                    peer_id,
                    topic: topic_str,
                });
            }

            MycelialBehaviourEvent::Gossipsub(gossipsub::Event::Unsubscribed {
                peer_id,
                topic,
            }) => {
                let topic_str = topic.to_string();
                let mesh_peers = self.swarm.behaviour().mesh_peers(&topic_str);

                info!(
                    "Peer {} unsubscribed from '{}' | Remaining mesh peers: {}",
                    peer_id,
                    topic_str,
                    mesh_peers.len()
                );

                let _ = self.event_tx.send(NetworkEvent::PeerUnsubscribed {
                    peer_id,
                    topic: topic_str,
                });
            }

            MycelialBehaviourEvent::Identify(identify::Event::Received {
                peer_id, info, ..
            }) => {
                debug!("Identified peer {}: {:?}", peer_id, info.agent_version);

                self.peer_manager.set_identify_info(
                    peer_id,
                    info.agent_version.clone(),
                    info.protocol_version.clone(),
                    info.protocols.iter().map(|p| p.to_string()).collect(),
                );

                // Add addresses to Kademlia (filter to only routable addresses)
                // This avoids adding unreachable Docker/WSL addresses in test environments
                for addr in &info.listen_addrs {
                    if is_routable_address(addr) {
                        self.swarm
                            .behaviour_mut()
                            .add_address(&peer_id, addr.clone());
                    } else {
                        debug!("Skipping non-routable address for {}: {}", peer_id, addr);
                    }
                }

                let _ = self.event_tx.send(NetworkEvent::PeerIdentified {
                    peer_id,
                    agent_version: info.agent_version,
                    protocol_version: info.protocol_version,
                    protocols: info.protocols.iter().map(|p| p.to_string()).collect(),
                    observed_addr: info.observed_addr,
                });
            }

            MycelialBehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed {
                result: kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))),
                ..
            }) => {
                debug!("Found DHT record: {:?}", record.record.key);
                let _ = self.event_tx.send(NetworkEvent::RecordFound {
                    key: record.record.key.to_vec(),
                    value: record.record.value,
                });
            }

            MycelialBehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed {
                result: kad::QueryResult::PutRecord(Ok(kad::PutRecordOk { key })),
                ..
            }) => {
                debug!("Stored DHT record: {:?}", key);
                let _ = self
                    .event_tx
                    .send(NetworkEvent::RecordStored { key: key.to_vec() });
            }

            MycelialBehaviourEvent::Mdns(mdns::Event::Discovered(peers)) => {
                debug!("mDNS discovered {} peers", peers.len());

                let discovered: Vec<_> = peers
                    .into_iter()
                    .map(|(peer_id, addr)| {
                        self.peer_manager.add_address(peer_id, addr.clone());
                        self.swarm
                            .behaviour_mut()
                            .add_address(&peer_id, addr.clone());
                        (peer_id, addr)
                    })
                    .collect();

                let _ = self
                    .event_tx
                    .send(NetworkEvent::MdnsDiscovered { peers: discovered });
            }

            MycelialBehaviourEvent::Mdns(mdns::Event::Expired(peers)) => {
                debug!("mDNS expired {} peers", peers.len());
                let expired: Vec<_> = peers.into_iter().map(|(peer_id, _)| peer_id).collect();
                let _ = self
                    .event_tx
                    .send(NetworkEvent::MdnsExpired { peers: expired });
            }

            _ => {}
        }
    }

    /// Handle a command, returns false if should shutdown
    async fn handle_command(&mut self, cmd: NetworkCommand) -> bool {
        match cmd {
            NetworkCommand::Dial { address } => {
                if let Err(e) = self.swarm.dial(address.clone()) {
                    warn!("Failed to dial {}: {:?}", address, e);
                } else {
                    debug!("Dialing {}", address);
                }
            }

            NetworkCommand::Disconnect { peer_id } => {
                let _ = self.swarm.disconnect_peer_id(peer_id);
            }

            NetworkCommand::Subscribe { topic } => {
                if let Err(e) = self.swarm.behaviour_mut().subscribe(&topic) {
                    warn!("Failed to subscribe to {}: {:?}", topic, e);
                } else {
                    self.subscribed_topics.insert(topic.clone());
                    let _ = self.event_tx.send(NetworkEvent::Subscribed { topic });
                }
            }

            NetworkCommand::Unsubscribe { topic } => {
                if let Err(e) = self.swarm.behaviour_mut().unsubscribe(&topic) {
                    warn!("Failed to unsubscribe from {}: {:?}", topic, e);
                } else {
                    self.subscribed_topics.remove(&topic);
                    let _ = self.event_tx.send(NetworkEvent::Unsubscribed { topic });
                }
            }

            NetworkCommand::Publish { topic, data } => {
                // Log mesh status before publishing for debugging
                let mesh_peers = self.swarm.behaviour().mesh_peers(&topic);
                let all_peers = self.swarm.behaviour().all_peers_on_topic(&topic);

                info!(
                    "Publishing to '{}' | {} bytes | Mesh peers: {} | Total subscribers: {}",
                    topic,
                    data.len(),
                    mesh_peers.len(),
                    all_peers.len()
                );

                if mesh_peers.is_empty() && !all_peers.is_empty() {
                    warn!(
                        "Warning: Publishing to '{}' with 0 mesh peers but {} subscribed peers. \
                        Mesh may not have formed yet (check mesh_n/mesh_n_low config).",
                        topic,
                        all_peers.len()
                    );
                }

                if !mesh_peers.is_empty() {
                    debug!("Mesh peers for '{}': {:?}", topic, mesh_peers);
                }

                match self.swarm.behaviour_mut().publish(&topic, data.clone()) {
                    Ok(msg_id) => {
                        info!(
                            "Published message {} to '{}' via {} mesh peers",
                            msg_id,
                            topic,
                            mesh_peers.len()
                        );
                        let mut stats = self.stats.write();
                        stats.messages_sent += 1;
                        stats.bytes_sent += data.len() as u64;
                    }
                    Err(e) => {
                        warn!(
                            "Failed to publish to '{}': {:?} | Mesh peers: {} | Consider waiting for mesh formation",
                            topic, e, mesh_peers.len()
                        );
                    }
                }
            }

            NetworkCommand::PutRecord { key, value } => {
                if let Err(e) = self.swarm.behaviour_mut().put_record(key, value) {
                    warn!("Failed to put DHT record: {:?}", e);
                }
            }

            NetworkCommand::GetRecord { key } => {
                self.swarm.behaviour_mut().get_record(key);
            }

            NetworkCommand::GetPeers { response } => {
                let peers = self.peer_manager.connected_peers();
                let _ = response.send(peers);
            }

            NetworkCommand::GetStats { response } => {
                let stats = self.stats.read().clone();
                let _ = response.send(stats);
            }

            NetworkCommand::Shutdown => {
                info!("Shutdown requested");
                return false;
            }
        }

        true
    }
}

/// Check if a multiaddr is routable/usable
///
/// Filters out:
/// - 10.255.255.254 (WSL magic IP)
/// - 172.17.x.x (Docker bridge)
/// - 172.28.x.x, 172.29.x.x (WSL internal bridges)
///
/// Allows:
/// - 127.0.0.1 (localhost)
/// - Public IPs
/// - Standard private ranges used intentionally (192.168.x.x, 10.x.x.x except 10.255.255.254)
fn is_routable_address(addr: &Multiaddr) -> bool {
    use std::net::Ipv4Addr;

    for protocol in addr.iter() {
        if let libp2p::multiaddr::Protocol::Ip4(ip) = protocol {
            // Always allow localhost
            if ip == Ipv4Addr::new(127, 0, 0, 1) {
                return true;
            }

            // Block WSL magic IP
            if ip == Ipv4Addr::new(10, 255, 255, 254) {
                return false;
            }

            // Block Docker bridge (172.17.x.x)
            if ip.octets()[0] == 172 && ip.octets()[1] == 17 {
                return false;
            }

            // Block WSL internal bridges (172.28.x.x, 172.29.x.x)
            if ip.octets()[0] == 172 && (ip.octets()[1] == 28 || ip.octets()[1] == 29) {
                return false;
            }

            // Allow other addresses (public IPs, 192.168.x.x, 10.x.x.x)
            return true;
        }
    }

    // Allow non-IPv4 addresses (IPv6, QUIC, etc.)
    true
}
