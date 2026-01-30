//! Meshtastic LoRa Mesh Bridge for Mycelial P2P Network
//!
//! This crate provides a bridge between Meshtastic LoRa mesh networks and
//! the mycelial libp2p gossipsub network, enabling bidirectional message
//! flow between off-grid LoRa devices and IP-connected peers.
//!
//! # Architecture
//!
//! The bridge operates in four layers:
//!
//! 1. **Physical Interface** - Serial/BLE/TCP connection to Meshtastic device
//! 2. **Protocol Translation** - Protobuf ↔ CBOR message conversion
//! 3. **Routing Bridge** - Message routing between networks
//! 4. **Application Integration** - CLI flags and dashboard support
//!
//! # Quick Start
//!
//! ```rust,ignore
//! // Enable the `serial` feature to use SerialInterface
//! // Cargo.toml: mycelial-meshtastic = { version = "0.1", features = ["serial"] }
//!
//! use mycelial_meshtastic::{
//!     SerialInterface, MeshtasticConfig, MeshtasticConfigBuilder,
//!     MessageTranslator, TopicMapper, NodeIdMapper, DeduplicationCache,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create configuration
//!     let config = MeshtasticConfigBuilder::new()
//!         .serial_port("/dev/ttyUSB0")
//!         .max_hops(3)
//!         .build();
//!
//!     // Create bridge components
//!     let node_mapper = NodeIdMapper::new();
//!     let topic_mapper = TopicMapper::new();
//!     let translator = MessageTranslator::new(node_mapper.clone());
//!     let dedup_cache = DeduplicationCache::from_config(&config.bridge);
//!
//!     // Create interface (requires `serial` feature)
//!     let mut interface = SerialInterface::new("/dev/ttyUSB0");
//!
//!     // Connect to device
//!     interface.connect().await?;
//!
//!     // Read packets and translate
//!     while let Ok(Some(packet_bytes)) = interface.read_packet().await {
//!         println!("Received {} bytes", packet_bytes.len());
//!         // Parse and translate packet...
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Features
//!
//! - `serial` - Serial port interface (requires `libudev-dev` on Linux)
//! - `ble` - Bluetooth Low Energy interface (requires `btleplug`)
//! - `tcp` - TCP interface for networked devices
//! - `full` - Enable all interfaces
//!
//! # Message Flow
//!
//! ## LoRa → libp2p
//!
//! 1. Meshtastic device receives LoRa packet
//! 2. SerialInterface reads FromRadio protobuf
//! 3. MessageTranslator converts to mycelial Message
//! 4. TopicMapper determines gossipsub topic
//! 5. DeduplicationCache checks for duplicates
//! 6. NetworkHandle.publish() broadcasts to mesh
//!
//! ## libp2p → LoRa
//!
//! 1. NetworkEvent::MessageReceived from gossipsub
//! 2. TopicMapper checks bridge eligibility
//! 3. DeduplicationCache marks message as seen
//! 4. MessageTranslator converts to Meshtastic protobuf
//! 5. SerialInterface sends ToRadio to device
//!
//! # Protocol Details
//!
//! Meshtastic uses a simple framing protocol over serial:
//! - Bytes 0-1: Magic number `0x94C3`
//! - Bytes 2-3: Payload length (big-endian u16)
//! - Bytes 4+: Protobuf payload
//!
//! Maximum LoRa payload is **237 bytes**, requiring message compression
//! for economics protocol messages.

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

// Phase 1: Core modules
pub mod config;
pub mod error;
pub mod interface;

// Phase 2: Bridge components
pub mod cache;
pub mod mapper;
pub mod translator;

// Phase 3: Network integration
pub mod bridge;

// Phase 4: Economics protocol support
pub mod compression;

// Phase 5: Testing utilities
pub mod test_utils;

// Re-exports for convenience - Phase 1
pub use config::{
    BridgeConfig, BridgeDirection, ChannelConfig, ChannelMapping, InterfaceConfig,
    MeshtasticConfig, MeshtasticConfigBuilder, MessagePriority, ReconnectConfig,
};
pub use error::{MeshtasticError, Result};
pub use interface::{ConnectionState, MeshtasticInterface};

#[cfg(feature = "serial")]
pub use interface::SerialInterface;

// Re-exports for convenience - Phase 2
pub use cache::{CacheStats, DeduplicationCache, DeduplicationKey, MessageDirection};
pub use mapper::{ChannelIndexMapper, NodeIdMapper, TopicMapper};
pub use translator::{MeshtasticPacket, MeshtasticPort, MessageTranslator};

// Re-exports for convenience - Phase 3
pub use bridge::{BridgeHandle, BridgeStats, GossipsubMessage, MeshtasticBridge, PublishCallback};

// Re-exports for convenience - Phase 4
pub use compression::{
    EconomicsMessageCodec, MessageChunk, MessageChunker, MessageCompressor, MessageReassembler,
};

// Re-exports for convenience - Phase 5 (testing)
#[cfg(feature = "serial")]
pub use test_utils::{find_meshtastic_device, list_available_devices, HardwareTestContext};
pub use test_utils::{DeviceInfo, MockInterface, TestFixture};

// Protocol constants re-exports
pub use config::{
    DEFAULT_BAUD_RATE, DEFAULT_MAX_HOPS, DEFAULT_TIMEOUT_MS, LORA_MAX_PAYLOAD, MAX_HOP_LIMIT,
    MESHTASTIC_MAGIC,
};

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Protocol version for bridge messages
pub const BRIDGE_PROTOCOL_VERSION: &str = "1.0.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_protocol_version() {
        assert_eq!(BRIDGE_PROTOCOL_VERSION, "1.0.0");
    }

    #[test]
    fn test_constants() {
        assert_eq!(MESHTASTIC_MAGIC, 0x94C3);
        assert_eq!(LORA_MAX_PAYLOAD, 237);
        assert_eq!(DEFAULT_BAUD_RATE, 115200);
        assert_eq!(MAX_HOP_LIMIT, 7);
    }
}
