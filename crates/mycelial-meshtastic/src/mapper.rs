//! Topic and Node ID mapping for Meshtastic-libp2p bridge
//!
//! This module provides mapping between:
//!
//! - **TopicMapper**: Gossipsub topics ↔ Meshtastic channels
//! - **NodeIdMapper**: Meshtastic NodeId (u32) ↔ libp2p PeerId
//!
//! # Topic Mapping
//!
//! Meshtastic uses numeric channel indices (0-7), while libp2p gossipsub
//! uses string topic names. The TopicMapper maintains this bidirectional
//! mapping with support for direction filtering.
//!
//! # Node ID Mapping
//!
//! Meshtastic identifies nodes with 4-byte node IDs, while libp2p uses
//! Ed25519 public keys (PeerId). The NodeIdMapper maintains a registry
//! of known mappings, learning new associations as messages arrive.

use lru::LruCache;
use mycelial_core::PeerId;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};
use tracing::{debug, trace, warn};

use crate::config::{BridgeDirection, ChannelConfig, ChannelMapping, MessagePriority};
use crate::error::{MeshtasticError, Result};

// ============================================================================
// Topic Mapper
// ============================================================================

/// Maps between gossipsub topics and Meshtastic channels
///
/// The TopicMapper provides bidirectional translation between libp2p
/// gossipsub topic names (like `/mycelial/1.0.0/chat`) and Meshtastic
/// channel names (like "Primary").
#[derive(Debug, Clone)]
pub struct TopicMapper {
    /// Topic to channel mappings
    topic_to_channel: HashMap<String, ChannelMapping>,
    /// Channel to topic reverse mappings
    channel_to_topics: HashMap<String, Vec<String>>,
    /// Default channel for unmapped topics
    default_channel: String,
}

impl TopicMapper {
    /// Create a new TopicMapper with default mappings
    pub fn new() -> Self {
        Self::from_config(&ChannelConfig::default())
    }

    /// Create from channel configuration
    pub fn from_config(config: &ChannelConfig) -> Self {
        let topic_to_channel = config.topic_mappings.clone();

        // Build reverse mapping
        let mut channel_to_topics: HashMap<String, Vec<String>> = HashMap::new();
        for (topic, mapping) in &topic_to_channel {
            channel_to_topics
                .entry(mapping.channel.clone())
                .or_default()
                .push(topic.clone());
        }

        Self {
            topic_to_channel,
            channel_to_topics,
            default_channel: config.default_channel.clone(),
        }
    }

    /// Get the Meshtastic channel for a gossipsub topic
    ///
    /// Returns the channel name and mapping configuration.
    pub fn topic_to_channel(&self, topic: &str) -> Option<&ChannelMapping> {
        self.topic_to_channel.get(topic)
    }

    /// Get gossipsub topics for a Meshtastic channel
    ///
    /// Returns all topics that should receive messages from this channel.
    pub fn channel_to_topics(&self, channel: &str) -> Vec<&str> {
        self.channel_to_topics
            .get(channel)
            .map(|topics| topics.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Check if a topic should be bridged to LoRa
    ///
    /// Returns true if the topic has a mapping and the direction allows
    /// libp2p → LoRa bridging.
    pub fn should_bridge_to_lora(&self, topic: &str) -> bool {
        self.topic_to_channel.get(topic).is_some_and(|mapping| {
            matches!(
                mapping.direction,
                BridgeDirection::Bidirectional | BridgeDirection::Libp2pToLora
            )
        })
    }

    /// Check if a channel should be bridged to libp2p
    ///
    /// Returns true if any topic mapped to this channel allows
    /// LoRa → libp2p bridging.
    pub fn should_bridge_to_libp2p(&self, channel: &str) -> bool {
        self.channel_to_topics.get(channel).is_some_and(|topics| {
            topics.iter().any(|topic| {
                self.topic_to_channel.get(topic).is_some_and(|mapping| {
                    matches!(
                        mapping.direction,
                        BridgeDirection::Bidirectional | BridgeDirection::LoraToLibp2p
                    )
                })
            })
        })
    }

    /// Get the hop limit for a topic based on its priority
    pub fn get_hop_limit(&self, topic: &str) -> u8 {
        self.topic_to_channel
            .get(topic)
            .map(|mapping| mapping.priority.hop_limit())
            .unwrap_or(3)
    }

    /// Get the message priority for a topic
    pub fn get_priority(&self, topic: &str) -> MessagePriority {
        self.topic_to_channel
            .get(topic)
            .map(|mapping| mapping.priority)
            .unwrap_or(MessagePriority::Normal)
    }

    /// Get the default channel name
    pub fn default_channel(&self) -> &str {
        &self.default_channel
    }

    /// Add a custom topic mapping
    pub fn add_mapping(&mut self, topic: String, mapping: ChannelMapping) {
        self.channel_to_topics
            .entry(mapping.channel.clone())
            .or_default()
            .push(topic.clone());
        self.topic_to_channel.insert(topic, mapping);
    }

    /// List all configured topics
    pub fn topics(&self) -> impl Iterator<Item = &str> {
        self.topic_to_channel.keys().map(String::as_str)
    }

    /// List all configured channels
    pub fn channels(&self) -> impl Iterator<Item = &str> {
        self.channel_to_topics.keys().map(String::as_str)
    }
}

impl Default for TopicMapper {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Node ID Mapper
// ============================================================================

/// Maps between Meshtastic NodeId (u32) and libp2p PeerId
///
/// This mapper maintains a bidirectional registry of known node/peer
/// associations. When a LoRa message arrives, we learn the mapping
/// from the Meshtastic NodeId to a virtual PeerId. When a libp2p
/// message needs to be sent to LoRa, we look up the target NodeId.
///
/// # Thread Safety
///
/// The NodeIdMapper uses interior mutability (Arc<RwLock>) to allow
/// concurrent reads and synchronized writes.
#[derive(Debug, Clone)]
pub struct NodeIdMapper {
    /// Node ID to Peer ID mappings
    node_to_peer: Arc<RwLock<HashMap<u32, PeerId>>>,
    /// Peer ID to Node ID reverse mappings
    peer_to_node: Arc<RwLock<HashMap<String, u32>>>,
    /// This node's Meshtastic NodeId
    local_node_id: Option<u32>,
    /// This node's libp2p PeerId
    local_peer_id: Option<PeerId>,
}

impl NodeIdMapper {
    /// Create a new empty NodeIdMapper
    pub fn new() -> Self {
        Self {
            node_to_peer: Arc::new(RwLock::new(HashMap::new())),
            peer_to_node: Arc::new(RwLock::new(HashMap::new())),
            local_node_id: None,
            local_peer_id: None,
        }
    }

    /// Create with local node information
    pub fn with_local(node_id: u32, peer_id: PeerId) -> Self {
        let mapper = Self::new();
        mapper.register(node_id, peer_id.clone());

        Self {
            local_node_id: Some(node_id),
            local_peer_id: Some(peer_id),
            ..mapper
        }
    }

    /// Register a node/peer mapping
    ///
    /// This is called when we learn about a new association, either
    /// from receiving a LoRa message or from configuration.
    pub fn register(&self, node_id: u32, peer_id: PeerId) {
        debug!(
            node_id = format!("0x{:08X}", node_id),
            peer_id = %peer_id.short(),
            "Registering node/peer mapping"
        );

        {
            let mut node_to_peer = self.node_to_peer.write().unwrap();
            node_to_peer.insert(node_id, peer_id.clone());
        }
        {
            let mut peer_to_node = self.peer_to_node.write().unwrap();
            peer_to_node.insert(peer_id.0.clone(), node_id);
        }
    }

    /// Convert a Meshtastic NodeId to a libp2p PeerId
    ///
    /// If the mapping is not known, generates a deterministic virtual
    /// PeerId from the NodeId.
    pub fn node_to_peer(&self, node_id: u32) -> Result<PeerId> {
        // Check for broadcast address
        if node_id == 0xFFFFFFFF {
            return Err(MeshtasticError::InvalidNodeId(
                "Broadcast address cannot be converted to PeerId".to_string(),
            ));
        }

        // Check cached mapping
        {
            let node_to_peer = self.node_to_peer.read().unwrap();
            if let Some(peer_id) = node_to_peer.get(&node_id) {
                return Ok(peer_id.clone());
            }
        }

        // Generate deterministic virtual PeerId
        // Format: "lora:{node_id_hex}" to distinguish from real peers
        let virtual_id = format!("lora:{:08x}", node_id);
        let peer_id = PeerId(virtual_id);

        // Cache the mapping for consistency
        self.register(node_id, peer_id.clone());

        trace!(
            node_id = format!("0x{:08X}", node_id),
            peer_id = %peer_id.short(),
            "Generated virtual PeerId for unknown LoRa node"
        );

        Ok(peer_id)
    }

    /// Convert a libp2p PeerId to a Meshtastic NodeId
    ///
    /// If the mapping is not known, generates a deterministic NodeId
    /// from the PeerId.
    pub fn peer_to_node(&self, peer_id: &PeerId) -> Result<u32> {
        // Check if this is a virtual LoRa PeerId
        if peer_id.0.starts_with("lora:") {
            // Parse the node ID from the virtual PeerId
            let hex_str = peer_id.0.strip_prefix("lora:").unwrap();
            return u32::from_str_radix(hex_str, 16)
                .map_err(|_| MeshtasticError::InvalidNodeId(peer_id.0.clone()));
        }

        // Check cached mapping
        {
            let peer_to_node = self.peer_to_node.read().unwrap();
            if let Some(&node_id) = peer_to_node.get(&peer_id.0) {
                return Ok(node_id);
            }
        }

        // Generate deterministic NodeId from PeerId
        // Use hash of the PeerId string to generate a stable NodeId
        let node_id = Self::hash_peer_id(peer_id);

        // Cache the mapping
        self.register(node_id, peer_id.clone());

        trace!(
            peer_id = %peer_id.short(),
            node_id = format!("0x{:08X}", node_id),
            "Generated NodeId for unknown libp2p peer"
        );

        Ok(node_id)
    }

    /// Get the local Meshtastic NodeId
    pub fn local_node_id(&self) -> Option<u32> {
        self.local_node_id
    }

    /// Get the local libp2p PeerId
    pub fn local_peer_id(&self) -> Option<&PeerId> {
        self.local_peer_id.as_ref()
    }

    /// Check if a NodeId is known (has been seen before)
    pub fn is_node_known(&self, node_id: u32) -> bool {
        let node_to_peer = self.node_to_peer.read().unwrap();
        node_to_peer.contains_key(&node_id)
    }

    /// Check if a PeerId is known (has been mapped to a NodeId)
    pub fn is_peer_known(&self, peer_id: &PeerId) -> bool {
        let peer_to_node = self.peer_to_node.read().unwrap();
        peer_to_node.contains_key(&peer_id.0)
    }

    /// Get the number of known mappings
    pub fn mapping_count(&self) -> usize {
        let node_to_peer = self.node_to_peer.read().unwrap();
        node_to_peer.len()
    }

    /// Clear all mappings
    pub fn clear(&self) {
        {
            let mut node_to_peer = self.node_to_peer.write().unwrap();
            node_to_peer.clear();
        }
        {
            let mut peer_to_node = self.peer_to_node.write().unwrap();
            peer_to_node.clear();
        }
    }

    /// Generate a deterministic NodeId from a PeerId using FNV-1a hash
    fn hash_peer_id(peer_id: &PeerId) -> u32 {
        // FNV-1a hash (32-bit)
        const FNV_PRIME: u32 = 16777619;
        const FNV_OFFSET: u32 = 2166136261;

        let mut hash = FNV_OFFSET;
        for byte in peer_id.0.as_bytes() {
            hash ^= *byte as u32;
            hash = hash.wrapping_mul(FNV_PRIME);
        }

        // Ensure we don't collide with broadcast address
        if hash == 0xFFFFFFFF {
            hash = 0xFFFFFFFE;
        }

        hash
    }
}

impl Default for NodeIdMapper {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Channel Index Mapper
// ============================================================================

/// Maps between channel names and Meshtastic channel indices (0-7)
///
/// Meshtastic devices support up to 8 channels, each identified by a
/// numeric index. This mapper maintains a consistent mapping between
/// human-readable channel names and their indices.
#[derive(Debug, Clone)]
pub struct ChannelIndexMapper {
    /// Channel name to index
    name_to_index: HashMap<String, u8>,
    /// Index to channel name
    index_to_name: [Option<String>; 8],
}

impl ChannelIndexMapper {
    /// Create a new ChannelIndexMapper with default Meshtastic channels
    pub fn new() -> Self {
        let mut mapper = Self {
            name_to_index: HashMap::new(),
            index_to_name: Default::default(),
        };

        // Default Meshtastic channel setup
        mapper.set_channel(0, "Primary");
        mapper.set_channel(1, "LongFast");
        mapper.set_channel(2, "MediumSlow");
        mapper.set_channel(3, "ShortSlow");

        mapper
    }

    /// Set a channel name for an index
    pub fn set_channel(&mut self, index: u8, name: &str) {
        if index >= 8 {
            return;
        }

        // Remove old mapping if exists
        if let Some(old_name) = &self.index_to_name[index as usize] {
            self.name_to_index.remove(old_name);
        }

        self.name_to_index.insert(name.to_string(), index);
        self.index_to_name[index as usize] = Some(name.to_string());
    }

    /// Get the channel index for a name
    pub fn name_to_index(&self, name: &str) -> Option<u8> {
        self.name_to_index.get(name).copied()
    }

    /// Get the channel name for an index
    pub fn index_to_name(&self, index: u8) -> Option<&str> {
        if index >= 8 {
            return None;
        }
        self.index_to_name[index as usize].as_deref()
    }

    /// Get the primary channel index (always 0)
    pub fn primary_index(&self) -> u8 {
        0
    }
}

impl Default for ChannelIndexMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TopicMapper tests
    #[test]
    fn test_topic_mapper_default_mappings() {
        let mapper = TopicMapper::new();

        // Check that default mappings exist
        assert!(mapper.topic_to_channel("/mycelial/1.0.0/chat").is_some());
        assert!(mapper.topic_to_channel("/mycelial/1.0.0/vouch").is_some());
        assert!(mapper.topic_to_channel("/mycelial/1.0.0/credit").is_some());
        assert!(mapper
            .topic_to_channel("/mycelial/1.0.0/governance")
            .is_some());
    }

    #[test]
    fn test_topic_mapper_channel_lookup() {
        let mapper = TopicMapper::new();

        let mapping = mapper.topic_to_channel("/mycelial/1.0.0/chat").unwrap();
        assert_eq!(mapping.channel, "Primary");
    }

    #[test]
    fn test_topic_mapper_reverse_lookup() {
        let mapper = TopicMapper::new();

        let topics = mapper.channel_to_topics("Primary");
        assert!(!topics.is_empty());
        assert!(topics.contains(&"/mycelial/1.0.0/chat"));
    }

    #[test]
    fn test_topic_mapper_bridge_directions() {
        let mapper = TopicMapper::new();

        // Bidirectional topics
        assert!(mapper.should_bridge_to_lora("/mycelial/1.0.0/chat"));
        assert!(mapper.should_bridge_to_libp2p("Primary"));

        // Announce is LoRa → libp2p only
        let announce_mapping = mapper.topic_to_channel("/mycelial/1.0.0/announce").unwrap();
        assert!(matches!(
            announce_mapping.direction,
            BridgeDirection::LoraToLibp2p
        ));
        assert!(!mapper.should_bridge_to_lora("/mycelial/1.0.0/announce"));
    }

    #[test]
    fn test_topic_mapper_hop_limits() {
        let mapper = TopicMapper::new();

        // High priority gets more hops
        assert!(mapper.get_hop_limit("/mycelial/1.0.0/vouch") > 3);

        // Normal priority gets default
        assert_eq!(mapper.get_hop_limit("/mycelial/1.0.0/chat"), 3);
    }

    #[test]
    fn test_topic_mapper_add_custom() {
        let mut mapper = TopicMapper::new();

        mapper.add_mapping(
            "/custom/topic".to_string(),
            ChannelMapping {
                channel: "Custom".to_string(),
                direction: BridgeDirection::Bidirectional,
                priority: MessagePriority::High,
            },
        );

        assert!(mapper.topic_to_channel("/custom/topic").is_some());
        assert_eq!(mapper.get_priority("/custom/topic"), MessagePriority::High);
    }

    // NodeIdMapper tests
    #[test]
    fn test_node_id_mapper_register() {
        let mapper = NodeIdMapper::new();
        let peer_id = PeerId("test_peer".to_string());

        mapper.register(0x12345678, peer_id.clone());

        assert!(mapper.is_node_known(0x12345678));
        assert!(mapper.is_peer_known(&peer_id));
        assert_eq!(mapper.mapping_count(), 1);
    }

    #[test]
    fn test_node_id_mapper_node_to_peer() {
        let mapper = NodeIdMapper::new();
        let peer_id = PeerId("known_peer".to_string());
        mapper.register(0xDEADBEEF, peer_id.clone());

        // Known node
        let result = mapper.node_to_peer(0xDEADBEEF).unwrap();
        assert_eq!(result.0, "known_peer");

        // Unknown node - generates virtual PeerId
        let virtual_peer = mapper.node_to_peer(0x12345678).unwrap();
        assert!(virtual_peer.0.starts_with("lora:"));
    }

    #[test]
    fn test_node_id_mapper_peer_to_node() {
        let mapper = NodeIdMapper::new();
        let peer_id = PeerId("known_peer".to_string());
        mapper.register(0xDEADBEEF, peer_id.clone());

        // Known peer
        let result = mapper.peer_to_node(&peer_id).unwrap();
        assert_eq!(result, 0xDEADBEEF);

        // Unknown peer - generates deterministic NodeId
        let unknown_peer = PeerId("unknown_peer".to_string());
        let node_id = mapper.peer_to_node(&unknown_peer).unwrap();
        assert_ne!(node_id, 0xFFFFFFFF); // Not broadcast
    }

    #[test]
    fn test_node_id_mapper_virtual_peer_roundtrip() {
        let mapper = NodeIdMapper::new();

        // Start with a NodeId
        let original_node_id = 0x12345678u32;

        // Convert to virtual PeerId
        let peer_id = mapper.node_to_peer(original_node_id).unwrap();
        assert!(peer_id.0.starts_with("lora:"));

        // Convert back to NodeId
        let recovered_node_id = mapper.peer_to_node(&peer_id).unwrap();
        assert_eq!(recovered_node_id, original_node_id);
    }

    #[test]
    fn test_node_id_mapper_broadcast_error() {
        let mapper = NodeIdMapper::new();

        // Broadcast address should error
        let result = mapper.node_to_peer(0xFFFFFFFF);
        assert!(result.is_err());
    }

    #[test]
    fn test_node_id_mapper_with_local() {
        let peer_id = PeerId("local_peer".to_string());
        let mapper = NodeIdMapper::with_local(0xABCDEF00, peer_id.clone());

        assert_eq!(mapper.local_node_id(), Some(0xABCDEF00));
        assert_eq!(
            mapper.local_peer_id().map(|p| p.0.as_str()),
            Some("local_peer")
        );

        // Local mapping should be registered
        assert!(mapper.is_node_known(0xABCDEF00));
        assert!(mapper.is_peer_known(&peer_id));
    }

    #[test]
    fn test_node_id_mapper_clear() {
        let mapper = NodeIdMapper::new();
        mapper.register(0x12345678, PeerId("peer1".to_string()));
        mapper.register(0x87654321, PeerId("peer2".to_string()));

        assert_eq!(mapper.mapping_count(), 2);

        mapper.clear();

        assert_eq!(mapper.mapping_count(), 0);
    }

    // ChannelIndexMapper tests
    #[test]
    fn test_channel_index_mapper_defaults() {
        let mapper = ChannelIndexMapper::new();

        assert_eq!(mapper.name_to_index("Primary"), Some(0));
        assert_eq!(mapper.name_to_index("LongFast"), Some(1));
        assert_eq!(mapper.index_to_name(0), Some("Primary"));
    }

    #[test]
    fn test_channel_index_mapper_set_channel() {
        let mut mapper = ChannelIndexMapper::new();

        mapper.set_channel(5, "Custom");

        assert_eq!(mapper.name_to_index("Custom"), Some(5));
        assert_eq!(mapper.index_to_name(5), Some("Custom"));
    }

    #[test]
    fn test_channel_index_mapper_overflow() {
        let mut mapper = ChannelIndexMapper::new();

        // Index 8+ should be ignored
        mapper.set_channel(8, "Invalid");
        assert!(mapper.name_to_index("Invalid").is_none());
    }
}
