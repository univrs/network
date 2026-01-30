//! Integration tests for the Meshtastic-libp2p bridge
//!
//! These tests verify end-to-end functionality of the bridge including:
//! - Full message flow between mock LoRa device and gossipsub
//! - Economics protocol bridging (vouch, credit, governance, resource)
//! - Compression and chunking for large messages
//! - Deduplication across both directions
//! - Error handling and recovery

use bytes::Bytes;
use mycelial_meshtastic::{
    BridgeConfig, BridgeDirection, BridgeHandle, BridgeStats, CacheStats, ChannelConfig,
    ChannelIndexMapper, DeduplicationCache, DeduplicationKey, EconomicsMessageCodec,
    GossipsubMessage, MeshtasticBridge, MeshtasticConfig, MeshtasticConfigBuilder, MeshtasticError,
    MeshtasticPacket, MeshtasticPort, MessageChunk, MessageChunker, MessageCompressor,
    MessageDirection, MessageReassembler, MessageTranslator, NodeIdMapper, PublishCallback,
    TopicMapper, BRIDGE_PROTOCOL_VERSION, DEFAULT_BAUD_RATE, LORA_MAX_PAYLOAD, MESHTASTIC_MAGIC,
    VERSION,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ============================================================================
// Mock Infrastructure for Integration Testing
// ============================================================================

/// Mock interface that simulates a Meshtastic device
struct MockMeshtasticDevice {
    connected: bool,
    incoming_packets: Vec<Vec<u8>>,
    outgoing_packets: Arc<Mutex<Vec<Vec<u8>>>>,
    simulate_disconnect: bool,
}

impl MockMeshtasticDevice {
    fn new() -> Self {
        Self {
            connected: false,
            incoming_packets: Vec::new(),
            outgoing_packets: Arc::new(Mutex::new(Vec::new())),
            simulate_disconnect: false,
        }
    }

    fn queue_incoming_packet(&mut self, packet: Vec<u8>) {
        self.incoming_packets.push(packet);
    }

    fn get_outgoing_packets(&self) -> Vec<Vec<u8>> {
        self.outgoing_packets.lock().unwrap().clone()
    }

    fn set_simulate_disconnect(&mut self, disconnect: bool) {
        self.simulate_disconnect = disconnect;
    }
}

#[async_trait::async_trait]
impl mycelial_meshtastic::MeshtasticInterface for MockMeshtasticDevice {
    async fn connect(&mut self) -> mycelial_meshtastic::Result<()> {
        if self.simulate_disconnect {
            return Err(MeshtasticError::ConnectionTimeout { duration_ms: 5000 });
        }
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> mycelial_meshtastic::Result<()> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn read_packet(&mut self) -> mycelial_meshtastic::Result<Option<Bytes>> {
        if self.simulate_disconnect {
            return Err(MeshtasticError::Disconnected);
        }
        if self.incoming_packets.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Bytes::from(self.incoming_packets.remove(0))))
        }
    }

    async fn write_packet(&mut self, data: &[u8]) -> mycelial_meshtastic::Result<()> {
        if self.simulate_disconnect {
            return Err(MeshtasticError::WriteError("Disconnected".to_string()));
        }
        self.outgoing_packets.lock().unwrap().push(data.to_vec());
        Ok(())
    }

    fn name(&self) -> &str {
        "MockMeshtasticDevice"
    }
}

/// Mock gossipsub network that captures published messages
struct MockGossipsubNetwork {
    published_messages: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
}

impl MockGossipsubNetwork {
    fn new() -> Self {
        Self {
            published_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_publish_callback(&self) -> PublishCallback {
        let messages = self.published_messages.clone();
        Arc::new(move |topic, data| {
            messages.lock().unwrap().push((topic, data));
            Ok(())
        })
    }

    fn get_published_messages(&self) -> Vec<(String, Vec<u8>)> {
        self.published_messages.lock().unwrap().clone()
    }
}

// ============================================================================
// Integration Tests: Bridge Lifecycle
// NOTE: These tests are ignored because they require running a full event loop
// with a mock device that properly simulates async behavior. The bridge
// functionality is tested in the bridge module's unit tests instead.
// ============================================================================

#[tokio::test]
#[ignore = "Requires hardware or properly async mock - see bridge module unit tests"]
async fn test_bridge_connect_and_disconnect() {
    let device = MockMeshtasticDevice::new();
    let network = MockGossipsubNetwork::new();
    let config = MeshtasticConfigBuilder::new().build();

    let (bridge, handle) = MeshtasticBridge::new(device, &config, network.get_publish_callback());

    // Shutdown the bridge immediately
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        handle.shutdown().await.unwrap();
    });

    // Run the bridge (should connect, then shutdown)
    let result = bridge.run().await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "Requires hardware or properly async mock - see bridge module unit tests"]
async fn test_bridge_stats_tracking() {
    let device = MockMeshtasticDevice::new();
    let network = MockGossipsubNetwork::new();
    let config = MeshtasticConfigBuilder::new().build();

    let (bridge, handle) = MeshtasticBridge::new(device, &config, network.get_publish_callback());

    // Get stats before running
    let handle_clone = handle.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let stats = handle_clone.stats().await.unwrap();
        assert_eq!(stats.lora_to_gossipsub, 0);
        assert_eq!(stats.gossipsub_to_lora, 0);
        handle_clone.shutdown().await.unwrap();
    });

    let _ = bridge.run().await;
}

// ============================================================================
// Integration Tests: Full Message Flow
// ============================================================================

#[tokio::test]
async fn test_full_lora_to_gossipsub_flow() {
    // Test a complete message flow from LoRa device to gossipsub
    let node_mapper = NodeIdMapper::new();
    let _topic_mapper = TopicMapper::new();
    let translator = MessageTranslator::new(node_mapper.clone());
    let dedup_cache = DeduplicationCache::new();

    // Create a text message packet from LoRa
    let packet = MeshtasticPacket {
        from: 0x12345678,
        to: 0xFFFFFFFF, // Broadcast
        packet_id: 12345,
        channel: 0,
        port_num: MeshtasticPort::TextMessage,
        payload: Bytes::from("Hello from LoRa mesh!"),
        hop_limit: 3,
        want_ack: false,
        rx_time: Some(chrono::Utc::now()),
    };

    // Verify it's not a duplicate
    let dedup_key = DeduplicationKey::from_meshtastic(packet.from, packet.packet_id);
    assert!(!dedup_cache.is_duplicate(&dedup_key, MessageDirection::FromLora));

    // Translate to Mycelial message
    let message = translator.meshtastic_to_mycelial(&packet).unwrap();
    assert_eq!(
        message.sender,
        node_mapper.node_to_peer(0x12345678).unwrap()
    );

    // Mark as seen to prevent echo
    dedup_cache.mark_seen(&dedup_key, MessageDirection::FromLora);
    assert!(dedup_cache.is_duplicate(&dedup_key, MessageDirection::FromLora));
}

#[tokio::test]
async fn test_full_gossipsub_to_lora_flow() {
    // Test a complete message flow from gossipsub to LoRa device
    let _node_mapper = NodeIdMapper::new();
    let topic_mapper = TopicMapper::new();
    let dedup_cache = DeduplicationCache::new();

    // Create a gossipsub message
    let msg = GossipsubMessage {
        topic: "/mycelial/1.0.0/chat".to_string(),
        source: Some("QmTestPeer123".to_string()),
        data: b"Hello from gossipsub!".to_vec(),
        message_id: "msg-abc-123".to_string(),
    };

    // Verify topic is bridgeable to LoRa
    assert!(topic_mapper.should_bridge_to_lora(&msg.topic));

    // Verify deduplication
    let dedup_key =
        DeduplicationKey::from_libp2p(msg.source.as_deref().unwrap_or("unknown"), &msg.message_id);
    assert!(!dedup_cache.is_duplicate(&dedup_key, MessageDirection::FromLibp2p));

    // Mark as seen
    dedup_cache.mark_seen(&dedup_key, MessageDirection::FromLibp2p);
    assert!(dedup_cache.is_duplicate(&dedup_key, MessageDirection::FromLibp2p));

    // Verify hop limit calculation
    let hop_limit = topic_mapper.get_hop_limit(&msg.topic);
    assert!(hop_limit >= 1 && hop_limit <= 7);
}

// ============================================================================
// Integration Tests: Economics Protocol Port Recognition
// NOTE: These tests verify that economics ports are recognized correctly.
// The actual binary format encoding/decoding and full translation roundtrips
// are tested in the translator module unit tests (translator::tests::*).
// ============================================================================

#[tokio::test]
async fn test_vouch_protocol_port_recognition() {
    // Verify vouch port is correctly identified
    assert_eq!(MeshtasticPort::from(512), MeshtasticPort::MycelialVouch);
    assert_eq!(u32::from(MeshtasticPort::MycelialVouch), 512);
}

#[tokio::test]
async fn test_credit_protocol_port_recognition() {
    // Verify credit port is correctly identified
    assert_eq!(MeshtasticPort::from(513), MeshtasticPort::MycelialCredit);
    assert_eq!(u32::from(MeshtasticPort::MycelialCredit), 513);
}

#[tokio::test]
async fn test_governance_protocol_port_recognition() {
    // Verify governance port is correctly identified
    assert_eq!(
        MeshtasticPort::from(514),
        MeshtasticPort::MycelialGovernance
    );
    assert_eq!(u32::from(MeshtasticPort::MycelialGovernance), 514);
}

#[tokio::test]
async fn test_resource_protocol_port_recognition() {
    // Verify resource port is correctly identified
    assert_eq!(MeshtasticPort::from(515), MeshtasticPort::MycelialResource);
    assert_eq!(u32::from(MeshtasticPort::MycelialResource), 515);
}

#[tokio::test]
async fn test_text_message_translation() {
    let node_mapper = NodeIdMapper::new();
    let translator = MessageTranslator::new(node_mapper.clone());

    // Create a simple text message packet
    let packet = MeshtasticPacket {
        from: 0x12345678,
        to: 0xFFFFFFFF,
        packet_id: 9999,
        channel: 0,
        port_num: MeshtasticPort::TextMessage,
        payload: Bytes::from("Hello from LoRa!"),
        hop_limit: 3,
        want_ack: false,
        rx_time: Some(chrono::Utc::now()),
    };

    // Text messages should translate successfully
    let result = translator.meshtastic_to_mycelial(&packet);
    assert!(result.is_ok());

    let _message = result.unwrap();
    // Verify the sender was mapped
    assert!(node_mapper.is_node_known(0x12345678));
}

// ============================================================================
// Integration Tests: Compression and Chunking
// ============================================================================

#[tokio::test]
async fn test_large_message_compression() {
    let mut codec = EconomicsMessageCodec::new();

    // Create a large governance proposal message
    let proposal_content: Vec<u8> = (0u32..800).map(|i| (i % 256) as u8).collect();

    // Encode (should compress and potentially chunk)
    let encoded = codec.encode(&proposal_content).unwrap();

    // Verify we got chunks if needed
    if proposal_content.len() > LORA_MAX_PAYLOAD {
        // Large message may require chunking after compression
        println!("Encoded into {} packets", encoded.len());
    }

    // Decode all chunks
    let mut decoder = EconomicsMessageCodec::new();
    let mut result = None;
    for packet in encoded {
        result = decoder.decode(&packet).unwrap();
    }

    // Verify round-trip
    assert!(result.is_some());
    assert_eq!(result.unwrap(), proposal_content);
}

#[tokio::test]
async fn test_chunk_reassembly_out_of_order() {
    let mut reassembler = MessageReassembler::new();

    // Create 3 chunks out of order
    let chunk0 = MessageChunk {
        message_id: 999,
        chunk_index: 0,
        total_chunks: 3,
        is_first: true,
        is_last: false,
        is_compressed: false,
        payload: Bytes::from(vec![1, 2, 3]),
    };

    let chunk1 = MessageChunk {
        message_id: 999,
        chunk_index: 1,
        total_chunks: 3,
        is_first: false,
        is_last: false,
        is_compressed: false,
        payload: Bytes::from(vec![4, 5, 6]),
    };

    let chunk2 = MessageChunk {
        message_id: 999,
        chunk_index: 2,
        total_chunks: 3,
        is_first: false,
        is_last: true,
        is_compressed: false,
        payload: Bytes::from(vec![7, 8, 9]),
    };

    // Add chunks in reverse order
    assert!(reassembler.add_chunk(chunk2.clone()).unwrap().is_none());
    assert_eq!(reassembler.pending_count(), 1);

    assert!(reassembler.add_chunk(chunk0.clone()).unwrap().is_none());
    assert_eq!(reassembler.pending_count(), 1);

    // Adding the last chunk completes the message
    let result = reassembler.add_chunk(chunk1.clone()).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(reassembler.pending_count(), 0);
}

#[tokio::test]
async fn test_compression_threshold() {
    let compressor = MessageCompressor::new();

    // Small data should not be compressed
    let small_data = vec![1, 2, 3, 4, 5];
    assert!(!compressor.should_compress(&small_data));

    // Large data should be compressed
    let large_data: Vec<u8> = (0u32..300).map(|i| (i % 256) as u8).collect();
    assert!(compressor.should_compress(&large_data));

    // Verify compression actually reduces size for compressible data
    let compressible: Vec<u8> = (0u32..500).map(|i| (i % 10) as u8).collect();
    let compressed = compressor.compress(&compressible).unwrap();
    assert!(compressed.len() < compressible.len());
}

// ============================================================================
// Integration Tests: Deduplication
// ============================================================================

#[tokio::test]
async fn test_deduplication_bidirectional() {
    let cache = DeduplicationCache::with_capacity_and_ttl(100, Duration::from_secs(60));

    // LoRa direction
    let lora_key = DeduplicationKey::from_meshtastic(0x12345678, 1234);
    assert!(!cache.is_duplicate(&lora_key, MessageDirection::FromLora));
    cache.mark_seen(&lora_key, MessageDirection::FromLora);
    assert!(cache.is_duplicate(&lora_key, MessageDirection::FromLora));

    // libp2p direction
    let libp2p_key = DeduplicationKey::from_libp2p("QmPeer123", "msg-456");
    assert!(!cache.is_duplicate(&libp2p_key, MessageDirection::FromLibp2p));
    cache.mark_seen(&libp2p_key, MessageDirection::FromLibp2p);
    assert!(cache.is_duplicate(&libp2p_key, MessageDirection::FromLibp2p));
}

#[tokio::test]
async fn test_deduplication_expiry() {
    let cache = DeduplicationCache::with_capacity_and_ttl(100, Duration::from_millis(50));

    let key = DeduplicationKey::from_meshtastic(0xAAAAAAAA, 9999);
    cache.mark_seen(&key, MessageDirection::FromLora);
    assert!(cache.is_duplicate(&key, MessageDirection::FromLora));

    // Wait for expiry
    tokio::time::sleep(Duration::from_millis(100)).await;
    cache.expire_old_entries();

    // Should no longer be duplicate after expiry
    assert!(!cache.is_duplicate(&key, MessageDirection::FromLora));
}

// ============================================================================
// Integration Tests: Node ID Mapping
// ============================================================================

#[tokio::test]
async fn test_node_id_peer_id_round_trip() {
    let mapper = NodeIdMapper::new();

    // Create a PeerId and register it
    let peer_id = mycelial_core::PeerId("test_peer_12345678".to_string());
    mapper.register(0x12345678, peer_id.clone());

    // Verify round trip
    let retrieved = mapper.node_to_peer(0x12345678).unwrap();
    assert_eq!(retrieved, peer_id);

    // Verify reverse lookup
    let node_id = mapper.peer_to_node(&peer_id).unwrap();
    assert_eq!(node_id, 0x12345678);
}

#[tokio::test]
async fn test_virtual_peer_id_generation() {
    let mapper = NodeIdMapper::new();

    // Get PeerId for unknown node (should generate virtual)
    let peer_id = mapper.node_to_peer(0xDEADBEEF).unwrap();

    // Same node should return same virtual PeerId
    let peer_id2 = mapper.node_to_peer(0xDEADBEEF).unwrap();
    assert_eq!(peer_id, peer_id2);
}

// ============================================================================
// Integration Tests: Topic Mapping
// ============================================================================

#[tokio::test]
async fn test_topic_to_channel_mapping() {
    let mapper = TopicMapper::new();

    // Verify default mappings (resource is not in default mapping)
    assert!(mapper.should_bridge_to_lora("/mycelial/1.0.0/chat"));
    assert!(mapper.should_bridge_to_lora("/mycelial/1.0.0/vouch"));
    assert!(mapper.should_bridge_to_lora("/mycelial/1.0.0/credit"));
    assert!(mapper.should_bridge_to_lora("/mycelial/1.0.0/governance"));
    assert!(mapper.should_bridge_to_lora("/mycelial/1.0.0/direct"));
    // announce is LoRa-to-libp2p only, so it should NOT bridge TO LoRa
    assert!(!mapper.should_bridge_to_lora("/mycelial/1.0.0/announce"));
}

#[tokio::test]
async fn test_channel_index_mapping() {
    let mut mapper = ChannelIndexMapper::new();

    // Default channels are pre-configured, but we can override
    mapper.set_channel(0, "Primary");
    mapper.set_channel(1, "Secondary");

    // Lookup
    assert_eq!(mapper.index_to_name(0), Some("Primary"));
    assert_eq!(mapper.name_to_index("Secondary"), Some(1));
    assert_eq!(mapper.index_to_name(99), None);
}

// ============================================================================
// Integration Tests: Error Handling
// ============================================================================

#[tokio::test]
async fn test_error_is_retriable() {
    assert!(MeshtasticError::Disconnected.is_retriable());
    assert!(MeshtasticError::ConnectionTimeout { duration_ms: 5000 }.is_retriable());
    assert!(MeshtasticError::ReadError("test".to_string()).is_retriable());
    assert!(MeshtasticError::WriteError("test".to_string()).is_retriable());

    assert!(!MeshtasticError::InvalidMagic { got: 0x1234 }.is_retriable());
    assert!(!MeshtasticError::MessageTooLarge {
        size: 300,
        max: 237
    }
    .is_retriable());
}

#[tokio::test]
async fn test_error_is_protocol_error() {
    assert!(MeshtasticError::InvalidMagic { got: 0x1234 }.is_protocol_error());
    assert!(MeshtasticError::ProtobufDecode("test".to_string()).is_protocol_error());
    assert!(MeshtasticError::InvalidPacket("test".to_string()).is_protocol_error());
    assert!(MeshtasticError::UnknownPort(999).is_protocol_error());

    assert!(!MeshtasticError::Disconnected.is_protocol_error());
}

#[tokio::test]
async fn test_message_too_large_error() {
    let data = vec![0u8; 500]; // Larger than LORA_MAX_PAYLOAD

    let err = MeshtasticError::MessageTooLarge {
        size: data.len(),
        max: LORA_MAX_PAYLOAD,
    };

    assert_eq!(err.error_code(), "MESSAGE_TOO_LARGE");
    assert!(err.to_string().contains("500"));
    assert!(err.to_string().contains("237"));
}

// ============================================================================
// Integration Tests: Configuration
// ============================================================================

#[tokio::test]
async fn test_config_builder() {
    let config = MeshtasticConfigBuilder::new()
        .serial_port_with_baud("/dev/ttyUSB0", 115200)
        .max_hops(5)
        .dedup_cache_size(2000)
        .build();

    // Check interface is Serial variant with correct values
    match &config.interface {
        mycelial_meshtastic::InterfaceConfig::Serial { port, baud_rate } => {
            assert_eq!(port.to_string_lossy(), "/dev/ttyUSB0");
            assert_eq!(baud_rate, &115200);
        }
        #[allow(unreachable_patterns)]
        _ => panic!("Expected Serial interface config"),
    }
    assert_eq!(config.bridge.max_hops, 5);
    assert_eq!(config.bridge.dedup_cache_size, 2000);
}

#[tokio::test]
async fn test_config_default_values() {
    let config = MeshtasticConfigBuilder::new().build();

    // Check default interface uses DEFAULT_BAUD_RATE
    match &config.interface {
        mycelial_meshtastic::InterfaceConfig::Serial { baud_rate, .. } => {
            assert_eq!(baud_rate, &DEFAULT_BAUD_RATE);
        }
        #[allow(unreachable_patterns)]
        _ => panic!("Expected Serial interface config by default"),
    }
    assert!(config.bridge.max_hops > 0 && config.bridge.max_hops <= 7);
}

// ============================================================================
// Integration Tests: Constants and Versioning
// ============================================================================

#[test]
fn test_protocol_constants() {
    assert_eq!(MESHTASTIC_MAGIC, 0x94C3);
    assert_eq!(LORA_MAX_PAYLOAD, 237);
    assert_eq!(DEFAULT_BAUD_RATE, 115200);
}

#[test]
fn test_version_info() {
    assert!(!VERSION.is_empty());
    assert_eq!(BRIDGE_PROTOCOL_VERSION, "1.0.0");
}
