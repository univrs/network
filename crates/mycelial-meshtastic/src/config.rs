//! Configuration types for Meshtastic bridge
//!
//! This module provides configuration structures for the Meshtastic
//! bridge including serial port settings, channel mappings, and behavior options.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Maximum payload size for Meshtastic LoRa packets
pub const LORA_MAX_PAYLOAD: usize = 237;

/// Meshtastic protocol magic number (first 2 bytes)
pub const MESHTASTIC_MAGIC: u16 = 0x94C3;

/// Default baud rate for Meshtastic serial devices
pub const DEFAULT_BAUD_RATE: u32 = 115200;

/// Default connection timeout
pub const DEFAULT_TIMEOUT_MS: u64 = 10000;

/// Default maximum hop limit for LoRa messages
pub const DEFAULT_MAX_HOPS: u8 = 3;

/// Maximum allowed hops in Meshtastic protocol
pub const MAX_HOP_LIMIT: u8 = 7;

/// Main configuration for Meshtastic bridge
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MeshtasticConfig {
    /// Interface configuration (serial, BLE, or TCP)
    #[serde(default)]
    pub interface: InterfaceConfig,

    /// Channel mapping configuration
    #[serde(default)]
    pub channels: ChannelConfig,

    /// Bridge behavior settings
    #[serde(default)]
    pub bridge: BridgeConfig,

    /// Reconnection settings
    #[serde(default)]
    pub reconnect: ReconnectConfig,
}

/// Interface type for connecting to Meshtastic device
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InterfaceConfig {
    /// Serial port connection (most common)
    Serial {
        /// Path to serial port (e.g., /dev/ttyUSB0, COM3)
        port: PathBuf,
        /// Baud rate (default: 115200)
        #[serde(default = "default_baud_rate")]
        baud_rate: u32,
    },
    /// TCP connection (for devices with network)
    #[cfg(feature = "tcp")]
    Tcp {
        /// Host address
        host: String,
        /// Port number
        port: u16,
    },
    /// Bluetooth Low Energy connection
    #[cfg(feature = "ble")]
    Ble {
        /// Device name or address
        device: String,
    },
}

fn default_baud_rate() -> u32 {
    DEFAULT_BAUD_RATE
}

impl Default for InterfaceConfig {
    fn default() -> Self {
        InterfaceConfig::Serial {
            port: PathBuf::from("/dev/ttyUSB0"),
            baud_rate: DEFAULT_BAUD_RATE,
        }
    }
}

/// Channel mapping between Meshtastic and gossipsub topics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Default channel for unmapped topics
    pub default_channel: String,

    /// Topic to channel mappings
    pub topic_mappings: HashMap<String, ChannelMapping>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        let mut mappings = HashMap::new();

        // Default topic mappings based on ADR-002
        mappings.insert(
            "/mycelial/1.0.0/chat".to_string(),
            ChannelMapping {
                channel: "Primary".to_string(),
                direction: BridgeDirection::Bidirectional,
                priority: MessagePriority::Normal,
            },
        );
        mappings.insert(
            "/mycelial/1.0.0/announce".to_string(),
            ChannelMapping {
                channel: "LongFast".to_string(),
                direction: BridgeDirection::LoraToLibp2p,
                priority: MessagePriority::Low,
            },
        );
        mappings.insert(
            "/mycelial/1.0.0/vouch".to_string(),
            ChannelMapping {
                channel: "Primary".to_string(),
                direction: BridgeDirection::Bidirectional,
                priority: MessagePriority::High,
            },
        );
        mappings.insert(
            "/mycelial/1.0.0/credit".to_string(),
            ChannelMapping {
                channel: "Primary".to_string(),
                direction: BridgeDirection::Bidirectional,
                priority: MessagePriority::High,
            },
        );
        mappings.insert(
            "/mycelial/1.0.0/governance".to_string(),
            ChannelMapping {
                channel: "Primary".to_string(),
                direction: BridgeDirection::Bidirectional,
                priority: MessagePriority::High,
            },
        );
        mappings.insert(
            "/mycelial/1.0.0/direct".to_string(),
            ChannelMapping {
                channel: "Direct".to_string(),
                direction: BridgeDirection::Bidirectional,
                priority: MessagePriority::Normal,
            },
        );

        Self {
            default_channel: "Primary".to_string(),
            topic_mappings: mappings,
        }
    }
}

/// Mapping configuration for a single topic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMapping {
    /// Meshtastic channel name
    pub channel: String,

    /// Bridge direction
    pub direction: BridgeDirection,

    /// Message priority (affects hop limit)
    pub priority: MessagePriority,
}

/// Direction of message bridging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeDirection {
    /// Bridge messages in both directions
    Bidirectional,
    /// Only bridge LoRa messages to libp2p
    LoraToLibp2p,
    /// Only bridge libp2p messages to LoRa
    Libp2pToLora,
}

/// Message priority affecting transmission parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessagePriority {
    /// Low priority (fewer hops, may be dropped under load)
    Low,
    /// Normal priority
    Normal,
    /// High priority (more hops, prioritized transmission)
    High,
}

impl MessagePriority {
    /// Get hop limit for this priority
    pub fn hop_limit(&self) -> u8 {
        match self {
            MessagePriority::Low => 2,
            MessagePriority::Normal => DEFAULT_MAX_HOPS,
            MessagePriority::High => 5,
        }
    }
}

/// Bridge behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Maximum hop limit for outgoing LoRa messages
    #[serde(default = "default_max_hops")]
    pub max_hops: u8,

    /// Size of deduplication cache (number of messages)
    #[serde(default = "default_dedup_cache_size")]
    pub dedup_cache_size: usize,

    /// TTL for deduplication cache entries
    #[serde(with = "humantime_serde", default = "default_dedup_ttl")]
    pub dedup_ttl: Duration,

    /// Enable message compression for economics messages
    #[serde(default = "default_compression")]
    pub enable_compression: bool,

    /// Queue size for outgoing LoRa messages
    #[serde(default = "default_queue_size")]
    pub outgoing_queue_size: usize,
}

fn default_max_hops() -> u8 {
    DEFAULT_MAX_HOPS
}

fn default_dedup_cache_size() -> usize {
    1000
}

fn default_dedup_ttl() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_compression() -> bool {
    true
}

fn default_queue_size() -> usize {
    100
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            max_hops: DEFAULT_MAX_HOPS,
            dedup_cache_size: 1000,
            dedup_ttl: Duration::from_secs(300),
            enable_compression: true,
            outgoing_queue_size: 100,
        }
    }
}

/// Reconnection behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectConfig {
    /// Enable automatic reconnection
    #[serde(default = "default_auto_reconnect")]
    pub enabled: bool,

    /// Initial delay before first reconnection attempt
    #[serde(with = "humantime_serde", default = "default_initial_delay")]
    pub initial_delay: Duration,

    /// Maximum delay between reconnection attempts
    #[serde(with = "humantime_serde", default = "default_max_delay")]
    pub max_delay: Duration,

    /// Maximum number of reconnection attempts (0 = infinite)
    #[serde(default)]
    pub max_attempts: u32,
}

fn default_auto_reconnect() -> bool {
    true
}

fn default_initial_delay() -> Duration {
    Duration::from_secs(1)
}

fn default_max_delay() -> Duration {
    Duration::from_secs(60)
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            max_attempts: 0, // Infinite
        }
    }
}

/// Builder for MeshtasticConfig
#[derive(Debug, Default)]
pub struct MeshtasticConfigBuilder {
    config: MeshtasticConfig,
}

impl MeshtasticConfigBuilder {
    /// Create a new builder with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set serial port path
    pub fn serial_port(mut self, port: impl Into<PathBuf>) -> Self {
        self.config.interface = InterfaceConfig::Serial {
            port: port.into(),
            baud_rate: DEFAULT_BAUD_RATE,
        };
        self
    }

    /// Set serial port with baud rate
    pub fn serial_port_with_baud(mut self, port: impl Into<PathBuf>, baud_rate: u32) -> Self {
        self.config.interface = InterfaceConfig::Serial {
            port: port.into(),
            baud_rate,
        };
        self
    }

    /// Set maximum hop limit
    pub fn max_hops(mut self, hops: u8) -> Self {
        self.config.bridge.max_hops = hops.min(MAX_HOP_LIMIT);
        self
    }

    /// Set deduplication cache size
    pub fn dedup_cache_size(mut self, size: usize) -> Self {
        self.config.bridge.dedup_cache_size = size;
        self
    }

    /// Enable or disable compression
    pub fn compression(mut self, enabled: bool) -> Self {
        self.config.bridge.enable_compression = enabled;
        self
    }

    /// Enable or disable auto-reconnect
    pub fn auto_reconnect(mut self, enabled: bool) -> Self {
        self.config.reconnect.enabled = enabled;
        self
    }

    /// Add a topic mapping
    pub fn map_topic(
        mut self,
        topic: impl Into<String>,
        channel: impl Into<String>,
        direction: BridgeDirection,
    ) -> Self {
        self.config.channels.topic_mappings.insert(
            topic.into(),
            ChannelMapping {
                channel: channel.into(),
                direction,
                priority: MessagePriority::Normal,
            },
        );
        self
    }

    /// Build the configuration
    pub fn build(self) -> MeshtasticConfig {
        self.config
    }
}

// Custom serde module for Duration with humantime
mod humantime_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = humantime::format_duration(*duration).to_string();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        humantime::parse_duration(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MeshtasticConfig::default();
        assert_eq!(config.bridge.max_hops, DEFAULT_MAX_HOPS);
        assert!(config.reconnect.enabled);
    }

    #[test]
    fn test_config_builder() {
        let config = MeshtasticConfigBuilder::new()
            .serial_port("/dev/ttyACM0")
            .max_hops(5)
            .compression(false)
            .build();

        assert_eq!(config.bridge.max_hops, 5);
        assert!(!config.bridge.enable_compression);
    }

    #[test]
    fn test_priority_hop_limits() {
        assert_eq!(MessagePriority::Low.hop_limit(), 2);
        assert_eq!(MessagePriority::Normal.hop_limit(), 3);
        assert_eq!(MessagePriority::High.hop_limit(), 5);
    }

    #[test]
    fn test_default_topic_mappings() {
        let config = ChannelConfig::default();
        assert!(config.topic_mappings.contains_key("/mycelial/1.0.0/chat"));
        assert!(config.topic_mappings.contains_key("/mycelial/1.0.0/vouch"));
        assert!(config.topic_mappings.contains_key("/mycelial/1.0.0/credit"));
    }

    #[test]
    fn test_max_hops_clamping() {
        let config = MeshtasticConfigBuilder::new()
            .max_hops(10) // Should be clamped to MAX_HOP_LIMIT (7)
            .build();

        assert_eq!(config.bridge.max_hops, MAX_HOP_LIMIT);
    }
}
