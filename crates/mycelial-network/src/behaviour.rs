//! Network behaviour combining multiple libp2p protocols
//!
//! This module provides the composite network behaviour that combines
//! gossipsub, kademlia, identify, and mDNS protocols.

use libp2p::{
    gossipsub::{self, IdentTopic, MessageAuthenticity, MessageId, ValidationMode},
    identify,
    identity::Keypair,
    kad::{self, store::MemoryStore},
    mdns,
    swarm::NetworkBehaviour,
    PeerId,
};
use sha2::{Digest, Sha256};
use std::time::Duration;

use crate::config::NetworkConfig;
use crate::error::NetworkError;

/// Combined network behaviour for the mycelial network
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "MycelialBehaviourEvent")]
pub struct MycelialBehaviour {
    /// Gossipsub for pub/sub messaging
    pub gossipsub: gossipsub::Behaviour,
    /// Kademlia DHT for peer discovery and content routing
    pub kademlia: kad::Behaviour<MemoryStore>,
    /// Identify protocol for peer identification
    pub identify: identify::Behaviour,
    /// mDNS for local peer discovery
    pub mdns: mdns::tokio::Behaviour,
}

/// Events emitted by the network behaviour
#[derive(Debug)]
pub enum MycelialBehaviourEvent {
    /// Gossipsub event
    Gossipsub(gossipsub::Event),
    /// Kademlia event
    Kademlia(kad::Event),
    /// Identify event
    Identify(identify::Event),
    /// mDNS event
    Mdns(mdns::Event),
}

impl From<gossipsub::Event> for MycelialBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        MycelialBehaviourEvent::Gossipsub(event)
    }
}

impl From<kad::Event> for MycelialBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        MycelialBehaviourEvent::Kademlia(event)
    }
}

impl From<identify::Event> for MycelialBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        MycelialBehaviourEvent::Identify(event)
    }
}

impl From<mdns::Event> for MycelialBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        MycelialBehaviourEvent::Mdns(event)
    }
}

impl MycelialBehaviour {
    /// Create a new network behaviour
    pub fn new(keypair: &Keypair, config: &NetworkConfig) -> crate::error::Result<Self> {
        let local_peer_id = keypair.public().to_peer_id();

        // Create gossipsub behaviour
        let gossipsub = create_gossipsub(keypair, config)?;

        // Create Kademlia behaviour
        let kademlia = create_kademlia(local_peer_id, config);

        // Create Identify behaviour
        let identify = create_identify(keypair);

        // Create mDNS behaviour
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)
            .map_err(|e| NetworkError::Config(e.to_string()))?;

        Ok(Self {
            gossipsub,
            kademlia,
            identify,
            mdns,
        })
    }

    /// Subscribe to a gossipsub topic
    pub fn subscribe(&mut self, topic: &str) -> crate::error::Result<()> {
        let topic = IdentTopic::new(topic);
        self.gossipsub
            .subscribe(&topic)
            .map_err(|e| NetworkError::Gossipsub(format!("Failed to subscribe: {:?}", e)))?;
        Ok(())
    }

    /// Unsubscribe from a gossipsub topic
    pub fn unsubscribe(&mut self, topic: &str) -> crate::error::Result<()> {
        let topic = IdentTopic::new(topic);
        self.gossipsub
            .unsubscribe(&topic)
            .map_err(|e| NetworkError::Gossipsub(format!("Failed to unsubscribe: {:?}", e)))?;
        Ok(())
    }

    /// Publish a message to a gossipsub topic
    pub fn publish(&mut self, topic: &str, data: Vec<u8>) -> crate::error::Result<MessageId> {
        let topic = IdentTopic::new(topic);
        self.gossipsub
            .publish(topic, data)
            .map_err(|e| NetworkError::Gossipsub(format!("Failed to publish: {:?}", e)))
    }

    /// Get the mesh peers for a specific topic
    /// Returns the list of peer IDs that are in the gossipsub mesh for this topic
    pub fn mesh_peers(&self, topic: &str) -> Vec<PeerId> {
        let topic_hash = IdentTopic::new(topic).hash();
        self.gossipsub.mesh_peers(&topic_hash).cloned().collect()
    }

    /// Get all peers subscribed to a topic (includes non-mesh peers)
    pub fn all_peers_on_topic(&self, topic: &str) -> Vec<PeerId> {
        let topic_hash = IdentTopic::new(topic).hash();
        self.gossipsub.all_peers()
            .filter(|(_, topics)| topics.contains(&&topic_hash))
            .map(|(peer_id, _)| *peer_id)
            .collect()
    }

    /// Log mesh status for debugging
    pub fn log_mesh_status(&self, topic: &str) {
        let topic_hash = IdentTopic::new(topic).hash();
        let mesh_peers: Vec<_> = self.gossipsub.mesh_peers(&topic_hash).collect();
        let all_topic_peers: Vec<_> = self.gossipsub.all_peers()
            .filter(|(_, topics)| topics.contains(&&topic_hash))
            .map(|(peer_id, _)| peer_id)
            .collect();

        tracing::info!(
            "Mesh status for '{}': {} mesh peers, {} total subscribed peers",
            topic, mesh_peers.len(), all_topic_peers.len()
        );
        for peer in &mesh_peers {
            tracing::debug!("  Mesh peer: {}", peer);
        }
    }

    /// Add a peer to the Kademlia routing table
    pub fn add_address(&mut self, peer_id: &PeerId, addr: libp2p::Multiaddr) {
        self.kademlia.add_address(peer_id, addr);
    }

    /// Bootstrap the Kademlia DHT
    pub fn bootstrap(&mut self) -> crate::error::Result<kad::QueryId> {
        self.kademlia
            .bootstrap()
            .map_err(|e| NetworkError::Kademlia(format!("Bootstrap failed: {:?}", e)))
    }

    /// Get closest peers to a key
    pub fn get_closest_peers(&mut self, key: Vec<u8>) -> kad::QueryId {
        self.kademlia.get_closest_peers(key)
    }

    /// Store a value in the DHT
    pub fn put_record(&mut self, key: Vec<u8>, value: Vec<u8>) -> crate::error::Result<kad::QueryId> {
        let record = kad::Record::new(key, value);
        self.kademlia
            .put_record(record, kad::Quorum::One)
            .map_err(|e| NetworkError::Kademlia(format!("Put record failed: {:?}", e)))
    }

    /// Get a value from the DHT
    pub fn get_record(&mut self, key: Vec<u8>) -> kad::QueryId {
        let key = kad::RecordKey::new(&key);
        self.kademlia.get_record(key)
    }
}

/// Create a gossipsub behaviour with the given configuration
fn create_gossipsub(keypair: &Keypair, config: &NetworkConfig) -> crate::error::Result<gossipsub::Behaviour> {
    // Message ID function based on content hash
    let message_id_fn = |message: &gossipsub::Message| {
        let mut hasher = Sha256::new();
        hasher.update(&message.data);
        MessageId::from(hasher.finalize().to_vec())
    };

    // Build gossipsub config
    // Use smaller mesh parameters suitable for small test networks (2-3 nodes)
    // Constraint: mesh_outbound_min <= mesh_n_low <= mesh_n <= mesh_n_high
    // mesh_outbound_min: minimum outbound mesh peers (default=2, set to 0 for flexibility)
    // mesh_n: target number of peers in the mesh (default=6, lowered to 2)
    // mesh_n_low: minimum mesh peers before trying to add more (default=4, lowered to 1)
    // mesh_n_high: maximum mesh peers before pruning (default=12, lowered to 4)
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(1))
        .validation_mode(ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .max_transmit_size(config.max_message_size)
        .mesh_outbound_min(0)  // Allow 0 outbound (for 2-node networks)
        .mesh_n(2)             // Target 2 mesh peers
        .mesh_n_low(1)         // Minimum 1 peer to maintain mesh
        .mesh_n_high(4)        // Maximum 4 before pruning
        .gossip_lazy(2)        // Reduced for smaller networks
        .fanout_ttl(Duration::from_secs(60))
        .history_length(5)
        .history_gossip(3)
        .duplicate_cache_time(Duration::from_secs(60))
        .build()
        .map_err(|e| NetworkError::Config(format!("Gossipsub config error: {}", e)))?;

    // Create behaviour with signing using the keypair
    gossipsub::Behaviour::new(
        MessageAuthenticity::Signed(keypair.clone()),
        gossipsub_config,
    )
    .map_err(|e| NetworkError::Config(format!("Gossipsub creation error: {}", e)))
}

/// Create a Kademlia behaviour
fn create_kademlia(local_peer_id: PeerId, _config: &NetworkConfig) -> kad::Behaviour<MemoryStore> {
    let store = MemoryStore::new(local_peer_id);
    let mut kademlia = kad::Behaviour::new(local_peer_id, store);

    // Set Kademlia to server mode for full participation
    kademlia.set_mode(Some(kad::Mode::Server));

    kademlia
}

/// Create an Identify behaviour
fn create_identify(keypair: &Keypair) -> identify::Behaviour {
    let config = identify::Config::new(
        "/mycelia/1.0.0".to_string(),
        keypair.public(),
    )
    .with_agent_version(format!("mycelia/{}", env!("CARGO_PKG_VERSION")));

    identify::Behaviour::new(config)
}

/// Standard gossipsub topics for the Mycelial network
pub mod topics {
    /// Chat messages between peers
    pub const CHAT: &str = "/mycelial/1.0.0/chat";
    /// Peer announcements (join/leave, status updates)
    pub const ANNOUNCE: &str = "/mycelial/1.0.0/announce";
    /// Reputation updates
    pub const REPUTATION: &str = "/mycelial/1.0.0/reputation";
    /// Social content (posts, media)
    pub const CONTENT: &str = "/mycelial/1.0.0/content";
    /// Orchestration (scheduling, workload events)
    pub const ORCHESTRATION: &str = "/mycelial/1.0.0/orchestration";
    /// Economics (credit, transactions)
    pub const ECONOMICS: &str = "/mycelial/1.0.0/economics";
    /// Governance (proposals, votes)
    pub const GOVERNANCE: &str = "/mycelial/1.0.0/governance";
    /// System messages (peer discovery, health)
    pub const SYSTEM: &str = "/mycelial/1.0.0/system";

    /// Get all standard topics
    pub fn all() -> Vec<&'static str> {
        vec![CHAT, ANNOUNCE, REPUTATION, CONTENT, ORCHESTRATION, ECONOMICS, GOVERNANCE, SYSTEM]
    }
}
