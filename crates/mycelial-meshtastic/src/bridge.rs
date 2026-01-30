//! MeshtasticBridge - Network integration service
//!
//! This module provides the main bridge service that integrates Meshtastic LoRa mesh
//! with libp2p gossipsub network. It handles:
//!
//! - LoRa → libp2p: Reading packets from Meshtastic device and publishing to gossipsub
//! - libp2p → LoRa: Receiving gossipsub messages and sending to LoRa mesh
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     MeshtasticBridge                            │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  ┌─────────────┐    ┌──────────────┐    ┌─────────────────┐    │
//! │  │ LoRa Device │◄──►│ Bridge Core  │◄──►│ NetworkHandle   │    │
//! │  │ (Serial)    │    │              │    │ (gossipsub)     │    │
//! │  └─────────────┘    │ Translator   │    └─────────────────┘    │
//! │                     │ TopicMapper  │                            │
//! │                     │ NodeMapper   │    ┌─────────────────┐    │
//! │                     │ DedupCache   │◄──►│ NetworkEvent rx │    │
//! │                     └──────────────┘    └─────────────────┘    │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use mycelial_meshtastic::bridge::{MeshtasticBridge, BridgeConfig};
//! use mycelial_network::{NetworkHandle, NetworkEvent};
//!
//! let config = BridgeConfig::default();
//! let bridge = MeshtasticBridge::new(config, network_handle, event_rx);
//!
//! // Run the bridge (spawns internal tasks)
//! bridge.run().await?;
//! ```

use bytes::Bytes;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{debug, error, info, trace, warn};

use crate::cache::{DeduplicationCache, DeduplicationKey, MessageDirection};
use crate::compression::{EconomicsMessageCodec, MessageChunk};
use crate::config::{BridgeConfig, MeshtasticConfig, LORA_MAX_PAYLOAD};
use crate::error::{MeshtasticError, Result};
use crate::interface::MeshtasticInterface;
use crate::mapper::{NodeIdMapper, TopicMapper};
use crate::translator::{MeshtasticPacket, MeshtasticPort, MessageTranslator};

#[cfg(feature = "serial")]
use crate::interface::SerialInterface;

/// Events received from libp2p gossipsub that may need bridging to LoRa
#[derive(Debug, Clone)]
pub struct GossipsubMessage {
    /// Topic the message was received on
    pub topic: String,
    /// Source peer ID (base58 encoded)
    pub source: Option<String>,
    /// Message data
    pub data: Vec<u8>,
    /// Message ID for deduplication
    pub message_id: String,
}

/// Commands that can be sent to the bridge
#[derive(Debug)]
pub enum BridgeCommand {
    /// Forward a gossipsub message to LoRa
    ForwardToLora(GossipsubMessage),
    /// Get bridge statistics
    GetStats(oneshot::Sender<BridgeStats>),
    /// Shutdown the bridge
    Shutdown,
}

/// Bridge statistics
#[derive(Debug, Clone, Default)]
pub struct BridgeStats {
    /// Messages forwarded from LoRa to gossipsub
    pub lora_to_gossipsub: u64,
    /// Messages forwarded from gossipsub to LoRa
    pub gossipsub_to_lora: u64,
    /// Messages blocked by deduplication
    pub duplicates_blocked: u64,
    /// Messages too large for LoRa
    pub oversized_messages: u64,
    /// Translation errors
    pub translation_errors: u64,
    /// Interface errors (serial, etc.)
    pub interface_errors: u64,
    /// Economics protocol messages processed
    pub economics_messages: u64,
    /// Compressed messages sent
    pub compressed_messages: u64,
    /// Chunked messages sent (multi-packet)
    pub chunked_messages: u64,
}

/// Callback for publishing messages to gossipsub
pub type PublishCallback =
    Arc<dyn Fn(String, Vec<u8>) -> std::result::Result<(), String> + Send + Sync>;

/// Handle for controlling the MeshtasticBridge
#[derive(Clone)]
pub struct BridgeHandle {
    command_tx: mpsc::Sender<BridgeCommand>,
}

impl BridgeHandle {
    /// Forward a gossipsub message to LoRa mesh
    pub async fn forward_to_lora(&self, msg: GossipsubMessage) -> Result<()> {
        self.command_tx
            .send(BridgeCommand::ForwardToLora(msg))
            .await
            .map_err(|_| MeshtasticError::ChannelClosed)
    }

    /// Get bridge statistics
    pub async fn stats(&self) -> Result<BridgeStats> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(BridgeCommand::GetStats(tx))
            .await
            .map_err(|_| MeshtasticError::ChannelClosed)?;
        rx.await.map_err(|_| MeshtasticError::ChannelClosed)
    }

    /// Shutdown the bridge
    pub async fn shutdown(&self) -> Result<()> {
        self.command_tx
            .send(BridgeCommand::Shutdown)
            .await
            .map_err(|_| MeshtasticError::ChannelClosed)
    }
}

/// Main bridge service connecting Meshtastic LoRa mesh to libp2p gossipsub
pub struct MeshtasticBridge<I: MeshtasticInterface> {
    /// Meshtastic device interface
    interface: I,
    /// Message translator
    translator: MessageTranslator,
    /// Topic mapper
    topic_mapper: TopicMapper,
    /// Node ID mapper
    node_mapper: NodeIdMapper,
    /// Deduplication cache
    dedup_cache: DeduplicationCache,
    /// Callback for publishing to gossipsub
    publish_callback: PublishCallback,
    /// Command receiver
    command_rx: mpsc::Receiver<BridgeCommand>,
    /// Bridge statistics
    stats: BridgeStats,
    /// Default hop limit for outgoing messages
    default_hop_limit: u8,
    /// Running flag
    running: bool,
    /// Economics message codec for compression/chunking
    economics_codec: EconomicsMessageCodec,
}

impl<I: MeshtasticInterface + Send + 'static> MeshtasticBridge<I> {
    /// Create a new bridge with the given interface and publish callback
    pub fn new(
        interface: I,
        config: &MeshtasticConfig,
        publish_callback: PublishCallback,
    ) -> (Self, BridgeHandle) {
        let node_mapper = NodeIdMapper::new();
        let topic_mapper = TopicMapper::from_config(&config.channels);
        let translator = MessageTranslator::new(node_mapper.clone());
        let dedup_cache = DeduplicationCache::from_config(&config.bridge);

        let (command_tx, command_rx) = mpsc::channel(256);
        let handle = BridgeHandle { command_tx };

        let bridge = Self {
            interface,
            translator,
            topic_mapper,
            node_mapper,
            dedup_cache,
            publish_callback,
            command_rx,
            stats: BridgeStats::default(),
            default_hop_limit: config.bridge.max_hops,
            running: false,
            economics_codec: EconomicsMessageCodec::new(),
        };

        (bridge, handle)
    }

    /// Run the bridge service
    ///
    /// This method runs the main event loop, handling:
    /// - Packets received from the LoRa device
    /// - Messages to forward to LoRa from gossipsub
    /// - Control commands (stats, shutdown)
    pub async fn run(mut self) -> Result<()> {
        info!("Starting Meshtastic bridge service");

        // Connect to the device
        self.interface.connect().await?;
        info!("Connected to Meshtastic device");

        self.running = true;

        // Main event loop
        loop {
            tokio::select! {
                // Handle incoming LoRa packets
                packet_result = self.interface.read_packet() => {
                    match packet_result {
                        Ok(Some(data)) => {
                            if let Err(e) = self.handle_lora_packet(&data).await {
                                warn!("Error handling LoRa packet: {}", e);
                                self.stats.interface_errors += 1;
                            }
                        }
                        Ok(None) => {
                            // No packet available, continue
                            trace!("No LoRa packet available");
                        }
                        Err(e) => {
                            warn!("Error reading from LoRa device: {}", e);
                            self.stats.interface_errors += 1;

                            // Try to reconnect on error
                            if let Err(reconnect_err) = self.try_reconnect().await {
                                error!("Failed to reconnect: {}", reconnect_err);
                                break;
                            }
                        }
                    }
                }

                // Handle commands
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        BridgeCommand::ForwardToLora(msg) => {
                            if let Err(e) = self.forward_to_lora(msg).await {
                                debug!("Error forwarding to LoRa: {}", e);
                            }
                        }
                        BridgeCommand::GetStats(tx) => {
                            let _ = tx.send(self.stats.clone());
                        }
                        BridgeCommand::Shutdown => {
                            info!("Bridge shutdown requested");
                            break;
                        }
                    }
                }

                // Periodic housekeeping
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    self.dedup_cache.expire_old_entries();
                    trace!(
                        "Bridge stats: lora->gossip={}, gossip->lora={}, blocked={}",
                        self.stats.lora_to_gossipsub,
                        self.stats.gossipsub_to_lora,
                        self.stats.duplicates_blocked
                    );
                }
            }

            // Check if we should continue running
            if !self.running {
                break;
            }
        }

        // Disconnect from device
        if let Err(e) = self.interface.disconnect().await {
            warn!("Error disconnecting from device: {}", e);
        }

        info!("Meshtastic bridge stopped");
        Ok(())
    }

    /// Handle a packet received from the LoRa device
    ///
    /// This is the LoRa → gossipsub direction:
    /// 1. Parse the packet
    /// 2. Check deduplication
    /// 3. Translate to Mycelial message
    /// 4. Determine gossipsub topic
    /// 5. Publish to gossipsub
    async fn handle_lora_packet(&mut self, data: &[u8]) -> Result<()> {
        // Parse the raw packet into a MeshtasticPacket
        let packet = self.parse_lora_packet(data)?;

        debug!(
            "Received LoRa packet: from=0x{:08X}, to=0x{:08X}, port={:?}, {} bytes",
            packet.from,
            packet.to,
            packet.port_num,
            packet.payload.len()
        );

        // Check for duplicates
        let dedup_key = DeduplicationKey::from_meshtastic(packet.from, packet.packet_id);
        if self
            .dedup_cache
            .is_duplicate(&dedup_key, MessageDirection::FromLora)
        {
            debug!("Dropping duplicate LoRa packet: {}", dedup_key);
            self.stats.duplicates_blocked += 1;
            return Ok(());
        }

        // Translate to Mycelial message
        let message = match self.translator.meshtastic_to_mycelial(&packet) {
            Ok(msg) => msg,
            Err(e) => {
                warn!("Failed to translate LoRa packet: {}", e);
                self.stats.translation_errors += 1;
                return Err(e);
            }
        };

        // Determine the gossipsub topic based on port number
        let topic = self.port_to_topic(packet.port_num, packet.channel);

        // Check if this channel should be bridged to libp2p
        if !self
            .topic_mapper
            .should_bridge_to_libp2p(self.topic_mapper.default_channel())
        {
            debug!("Channel not configured for libp2p bridging, skipping");
            return Ok(());
        }

        // Publish to gossipsub
        let payload = serde_cbor::to_vec(&message)
            .map_err(|e| MeshtasticError::TranslationFailed(e.to_string()))?;

        match (self.publish_callback)(topic.clone(), payload) {
            Ok(()) => {
                info!(
                    "Forwarded LoRa message to gossipsub: topic={}, from=0x{:08X}",
                    topic, packet.from
                );
                self.stats.lora_to_gossipsub += 1;
            }
            Err(e) => {
                warn!("Failed to publish to gossipsub: {}", e);
            }
        }

        Ok(())
    }

    /// Forward a gossipsub message to the LoRa mesh
    ///
    /// This is the gossipsub → LoRa direction:
    /// 1. Check if topic should be bridged to LoRa
    /// 2. Check deduplication
    /// 3. Translate to Meshtastic format
    /// 4. Check size limits
    /// 5. Send to device
    async fn forward_to_lora(&mut self, msg: GossipsubMessage) -> Result<()> {
        debug!(
            "Forwarding gossipsub message to LoRa: topic={}, {} bytes",
            msg.topic,
            msg.data.len()
        );

        // Check if topic should be bridged to LoRa
        if !self.topic_mapper.should_bridge_to_lora(&msg.topic) {
            debug!("Topic '{}' not configured for LoRa bridging", msg.topic);
            return Ok(());
        }

        // Check for duplicates using the message ID
        let source_id = msg.source.as_deref().unwrap_or("unknown");
        let dedup_key = DeduplicationKey::from_libp2p(source_id, &msg.message_id);
        if self
            .dedup_cache
            .is_duplicate(&dedup_key, MessageDirection::FromLibp2p)
        {
            debug!("Dropping duplicate gossipsub message: {}", dedup_key);
            self.stats.duplicates_blocked += 1;
            return Ok(());
        }

        // Determine hop limit based on topic priority
        let hop_limit = self.topic_mapper.get_hop_limit(&msg.topic);

        // Try to decode as a Mycelial Message and translate
        let packet = match serde_cbor::from_slice::<mycelial_core::Message>(&msg.data) {
            Ok(message) => {
                match self.translator.mycelial_to_meshtastic(&message, hop_limit) {
                    Ok(pkt) => pkt,
                    Err(e) => {
                        // If translation fails, try sending as raw text
                        debug!("Translation failed, sending as text: {}", e);
                        self.create_text_packet(&msg.data, hop_limit)?
                    }
                }
            }
            Err(_) => {
                // Not a CBOR message, try to send as raw text
                self.create_text_packet(&msg.data, hop_limit)?
            }
        };

        // Check payload size
        if packet.payload.len() > LORA_MAX_PAYLOAD {
            warn!(
                "Message too large for LoRa: {} bytes (max {})",
                packet.payload.len(),
                LORA_MAX_PAYLOAD
            );
            self.stats.oversized_messages += 1;
            return Err(MeshtasticError::MessageTooLarge {
                size: packet.payload.len(),
                max: LORA_MAX_PAYLOAD,
            });
        }

        // Encode and send to device
        let encoded = self.encode_packet(&packet)?;
        self.interface.write_packet(&encoded).await?;

        // Mark as seen to prevent echo
        self.dedup_cache
            .mark_seen(&dedup_key, MessageDirection::FromLibp2p);

        info!(
            "Forwarded gossipsub message to LoRa: topic={}, {} bytes, hop_limit={}",
            msg.topic,
            encoded.len(),
            hop_limit
        );
        self.stats.gossipsub_to_lora += 1;

        Ok(())
    }

    /// Parse raw bytes into a MeshtasticPacket
    fn parse_lora_packet(&self, data: &[u8]) -> Result<MeshtasticPacket> {
        // Meshtastic packet header format:
        // - Magic: 0x94C3 (2 bytes, big-endian)
        // - Length: u16 (2 bytes, big-endian)
        // - FromRadio protobuf payload

        if data.len() < 4 {
            return Err(MeshtasticError::InvalidPacket(
                "Packet too short".to_string(),
            ));
        }

        // For now, we'll create a simplified packet structure
        // In a full implementation, this would use prost to decode the protobuf
        let from = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let to = if data.len() >= 8 {
            u32::from_be_bytes([data[4], data[5], data[6], data[7]])
        } else {
            0xFFFFFFFF // Broadcast
        };
        let packet_id = if data.len() >= 12 {
            u32::from_be_bytes([data[8], data[9], data[10], data[11]])
        } else {
            rand::random()
        };

        // Determine port from first byte of payload or default to TextMessage
        let (port_num, payload_start) = if data.len() > 12 {
            (MeshtasticPort::from(data[12] as u32), 13)
        } else {
            (MeshtasticPort::TextMessage, 12)
        };

        let payload = if data.len() > payload_start {
            Bytes::copy_from_slice(&data[payload_start..])
        } else {
            Bytes::new()
        };

        Ok(MeshtasticPacket {
            from,
            to,
            packet_id,
            channel: 0,
            port_num,
            payload,
            hop_limit: 3,
            want_ack: false,
            rx_time: Some(chrono::Utc::now()),
        })
    }

    /// Encode a MeshtasticPacket for sending
    fn encode_packet(&self, packet: &MeshtasticPacket) -> Result<Vec<u8>> {
        // Create a simple packet format for sending
        // In a full implementation, this would use prost to encode ToRadio protobuf
        let mut encoded = Vec::with_capacity(packet.payload.len() + 16);

        // Header
        encoded.extend_from_slice(&packet.from.to_be_bytes());
        encoded.extend_from_slice(&packet.to.to_be_bytes());
        encoded.extend_from_slice(&packet.packet_id.to_be_bytes());
        encoded.push(packet.port_num as u8);
        encoded.push(packet.hop_limit);
        encoded.push(packet.channel);
        encoded.push(if packet.want_ack { 1 } else { 0 });

        // Payload
        encoded.extend_from_slice(&packet.payload);

        Ok(encoded)
    }

    /// Create a text message packet from raw data
    fn create_text_packet(&self, data: &[u8], hop_limit: u8) -> Result<MeshtasticPacket> {
        // Truncate text to fit LoRa payload
        let payload = if data.len() > LORA_MAX_PAYLOAD {
            Bytes::copy_from_slice(&data[..LORA_MAX_PAYLOAD])
        } else {
            Bytes::copy_from_slice(data)
        };

        // Get local node ID or generate one
        let from = self
            .node_mapper
            .local_node_id()
            .unwrap_or_else(rand::random);

        Ok(MeshtasticPacket {
            from,
            to: 0xFFFFFFFF, // Broadcast
            packet_id: rand::random(),
            channel: 0,
            port_num: MeshtasticPort::TextMessage,
            payload,
            hop_limit,
            want_ack: false,
            rx_time: Some(chrono::Utc::now()),
        })
    }

    /// Map a Meshtastic port number to a gossipsub topic
    fn port_to_topic(&self, port: MeshtasticPort, _channel: u8) -> String {
        match port {
            MeshtasticPort::TextMessage => "/mycelial/1.0.0/chat".to_string(),
            MeshtasticPort::MycelialVouch => "/mycelial/1.0.0/vouch".to_string(),
            MeshtasticPort::MycelialCredit => "/mycelial/1.0.0/credit".to_string(),
            MeshtasticPort::MycelialGovernance => "/mycelial/1.0.0/governance".to_string(),
            MeshtasticPort::MycelialResource => "/mycelial/1.0.0/resource".to_string(),
            MeshtasticPort::NodeInfo => "/mycelial/1.0.0/announce".to_string(),
            MeshtasticPort::Position => "/mycelial/1.0.0/announce".to_string(),
            _ => "/mycelial/1.0.0/chat".to_string(), // Default to chat
        }
    }

    /// Check if a topic is an economics protocol topic
    fn is_economics_topic(topic: &str) -> bool {
        matches!(
            topic,
            "/mycelial/1.0.0/vouch"
                | "/mycelial/1.0.0/credit"
                | "/mycelial/1.0.0/governance"
                | "/mycelial/1.0.0/resource"
        )
    }

    /// Check if a port is an economics protocol port
    fn is_economics_port(port: MeshtasticPort) -> bool {
        matches!(
            port,
            MeshtasticPort::MycelialVouch
                | MeshtasticPort::MycelialCredit
                | MeshtasticPort::MycelialGovernance
                | MeshtasticPort::MycelialResource
        )
    }

    /// Try to reconnect to the device
    async fn try_reconnect(&mut self) -> Result<()> {
        warn!("Attempting to reconnect to Meshtastic device...");

        // Disconnect first (ignore errors)
        let _ = self.interface.disconnect().await;

        // Wait before reconnecting
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Try to reconnect
        self.interface.connect().await?;

        info!("Successfully reconnected to Meshtastic device");
        Ok(())
    }
}

/// Create a bridge with a mock interface for testing
#[cfg(test)]
pub fn create_test_bridge() -> (MeshtasticBridge<MockInterface>, BridgeHandle) {
    use crate::config::MeshtasticConfigBuilder;

    let config = MeshtasticConfigBuilder::new().build();
    let interface = MockInterface::new();
    let publish_callback: PublishCallback = Arc::new(|topic, data| {
        println!("Mock publish: {} bytes to {}", data.len(), topic);
        Ok(())
    });

    MeshtasticBridge::new(interface, &config, publish_callback)
}

/// Mock interface for testing
#[cfg(test)]
pub struct MockInterface {
    connected: bool,
    incoming: Vec<Vec<u8>>,
    outgoing: Vec<Vec<u8>>,
}

#[cfg(test)]
impl Default for MockInterface {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl MockInterface {
    /// Create a new mock interface for testing.
    pub fn new() -> Self {
        Self {
            connected: false,
            incoming: Vec::new(),
            outgoing: Vec::new(),
        }
    }

    /// Add incoming data to simulate receiving from device.
    pub fn add_incoming(&mut self, data: Vec<u8>) {
        self.incoming.push(data);
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl MeshtasticInterface for MockInterface {
    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn read_packet(&mut self) -> Result<Option<Bytes>> {
        if self.incoming.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Bytes::from(self.incoming.remove(0))))
        }
    }

    async fn write_packet(&mut self, data: &[u8]) -> Result<()> {
        self.outgoing.push(data.to_vec());
        Ok(())
    }

    fn name(&self) -> &str {
        "MockInterface"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_creation() {
        let (bridge, handle) = create_test_bridge();
        assert!(!bridge.running);
        drop(handle);
    }

    #[tokio::test]
    async fn test_bridge_forward_to_lora() {
        let (mut bridge, _handle) = create_test_bridge();

        // Connect the bridge interface
        bridge.interface.connect().await.unwrap();

        let msg = GossipsubMessage {
            topic: "/mycelial/1.0.0/chat".to_string(),
            source: Some("test_peer".to_string()),
            data: b"Hello from gossipsub!".to_vec(),
            message_id: "msg-123".to_string(),
        };

        let result = bridge.forward_to_lora(msg).await;
        assert!(result.is_ok());
        assert_eq!(bridge.stats.gossipsub_to_lora, 1);
    }

    #[tokio::test]
    async fn test_bridge_handle_lora_packet() {
        let (mut bridge, _handle) = create_test_bridge();

        // Create a simple test packet
        let mut packet_data = Vec::new();
        packet_data.extend_from_slice(&0x12345678u32.to_be_bytes()); // from
        packet_data.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes()); // to (broadcast)
        packet_data.extend_from_slice(&0x00000001u32.to_be_bytes()); // packet_id
        packet_data.push(MeshtasticPort::TextMessage as u8); // port
        packet_data.extend_from_slice(b"Hello from LoRa!"); // payload

        let result = bridge.handle_lora_packet(&packet_data).await;
        assert!(result.is_ok());
        assert_eq!(bridge.stats.lora_to_gossipsub, 1);
    }

    #[tokio::test]
    async fn test_deduplication() {
        let (mut bridge, _handle) = create_test_bridge();
        bridge.interface.connect().await.unwrap();

        let msg = GossipsubMessage {
            topic: "/mycelial/1.0.0/chat".to_string(),
            source: Some("test_peer".to_string()),
            data: b"Duplicate test".to_vec(),
            message_id: "dup-msg-456".to_string(),
        };

        // First message should go through
        bridge.forward_to_lora(msg.clone()).await.unwrap();
        assert_eq!(bridge.stats.gossipsub_to_lora, 1);
        assert_eq!(bridge.stats.duplicates_blocked, 0);

        // Second message should be blocked
        bridge.forward_to_lora(msg).await.unwrap();
        assert_eq!(bridge.stats.gossipsub_to_lora, 1);
        assert_eq!(bridge.stats.duplicates_blocked, 1);
    }

    #[test]
    fn test_port_to_topic_mapping() {
        let (bridge, _handle) = create_test_bridge();

        assert_eq!(
            bridge.port_to_topic(MeshtasticPort::TextMessage, 0),
            "/mycelial/1.0.0/chat"
        );
        assert_eq!(
            bridge.port_to_topic(MeshtasticPort::MycelialVouch, 0),
            "/mycelial/1.0.0/vouch"
        );
        assert_eq!(
            bridge.port_to_topic(MeshtasticPort::MycelialCredit, 0),
            "/mycelial/1.0.0/credit"
        );
        assert_eq!(
            bridge.port_to_topic(MeshtasticPort::MycelialGovernance, 0),
            "/mycelial/1.0.0/governance"
        );
    }

    // ========================================================================
    // Phase 4: Economics Protocol Bridging Tests
    // ========================================================================

    #[test]
    fn test_is_economics_topic() {
        assert!(MeshtasticBridge::<MockInterface>::is_economics_topic(
            "/mycelial/1.0.0/vouch"
        ));
        assert!(MeshtasticBridge::<MockInterface>::is_economics_topic(
            "/mycelial/1.0.0/credit"
        ));
        assert!(MeshtasticBridge::<MockInterface>::is_economics_topic(
            "/mycelial/1.0.0/governance"
        ));
        assert!(MeshtasticBridge::<MockInterface>::is_economics_topic(
            "/mycelial/1.0.0/resource"
        ));
        assert!(!MeshtasticBridge::<MockInterface>::is_economics_topic(
            "/mycelial/1.0.0/chat"
        ));
        assert!(!MeshtasticBridge::<MockInterface>::is_economics_topic(
            "/mycelial/1.0.0/announce"
        ));
    }

    #[test]
    fn test_is_economics_port() {
        assert!(MeshtasticBridge::<MockInterface>::is_economics_port(
            MeshtasticPort::MycelialVouch
        ));
        assert!(MeshtasticBridge::<MockInterface>::is_economics_port(
            MeshtasticPort::MycelialCredit
        ));
        assert!(MeshtasticBridge::<MockInterface>::is_economics_port(
            MeshtasticPort::MycelialGovernance
        ));
        assert!(MeshtasticBridge::<MockInterface>::is_economics_port(
            MeshtasticPort::MycelialResource
        ));
        assert!(!MeshtasticBridge::<MockInterface>::is_economics_port(
            MeshtasticPort::TextMessage
        ));
        assert!(!MeshtasticBridge::<MockInterface>::is_economics_port(
            MeshtasticPort::NodeInfo
        ));
    }

    #[tokio::test]
    async fn test_bridge_forward_vouch_to_lora() {
        let (mut bridge, _handle) = create_test_bridge();
        bridge.interface.connect().await.unwrap();

        let msg = GossipsubMessage {
            topic: "/mycelial/1.0.0/vouch".to_string(),
            source: Some("test_peer".to_string()),
            data: b"vouch_data".to_vec(),
            message_id: "vouch-123".to_string(),
        };

        let result = bridge.forward_to_lora(msg).await;
        assert!(result.is_ok());
        assert_eq!(bridge.stats.gossipsub_to_lora, 1);
    }

    #[tokio::test]
    async fn test_bridge_forward_credit_to_lora() {
        let (mut bridge, _handle) = create_test_bridge();
        bridge.interface.connect().await.unwrap();

        let msg = GossipsubMessage {
            topic: "/mycelial/1.0.0/credit".to_string(),
            source: Some("creditor".to_string()),
            data: b"credit_transfer".to_vec(),
            message_id: "credit-456".to_string(),
        };

        let result = bridge.forward_to_lora(msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bridge_forward_governance_to_lora() {
        let (mut bridge, _handle) = create_test_bridge();
        bridge.interface.connect().await.unwrap();

        let msg = GossipsubMessage {
            topic: "/mycelial/1.0.0/governance".to_string(),
            source: Some("proposer".to_string()),
            data: b"proposal_vote".to_vec(),
            message_id: "gov-789".to_string(),
        };

        let result = bridge.forward_to_lora(msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bridge_handle_economics_lora_packet() {
        let (mut bridge, _handle) = create_test_bridge();

        // Create a Vouch economics packet
        let mut packet_data = Vec::new();
        packet_data.extend_from_slice(&0x12345678u32.to_be_bytes()); // from
        packet_data.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes()); // to (broadcast)
        packet_data.extend_from_slice(&0x00000001u32.to_be_bytes()); // packet_id
        packet_data.extend_from_slice(&(MeshtasticPort::MycelialVouch as u32).to_be_bytes()); // vouch port
        packet_data.extend_from_slice(b"vouch_payload"); // payload

        let result = bridge.handle_lora_packet(&packet_data).await;
        assert!(result.is_ok());
        assert_eq!(bridge.stats.lora_to_gossipsub, 1);
    }

    #[test]
    fn test_economics_codec_in_bridge() {
        let (bridge, _handle) = create_test_bridge();
        // Verify economics codec is initialized
        assert_eq!(bridge.economics_codec.pending_count(), 0);
    }
}
