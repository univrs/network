//! Configuration types for the Mycelia network
//!
//! This module provides configuration structures for nodes, modules,
//! and various network parameters.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Main node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Node identity configuration
    pub identity: IdentityConfig,
    /// Network configuration
    pub network: NetworkConfig,
    /// Storage configuration
    pub storage: StorageConfig,
    /// Module configuration
    pub modules: ModulesConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            identity: IdentityConfig::default(),
            network: NetworkConfig::default(),
            storage: StorageConfig::default(),
            modules: ModulesConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Identity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Path to the keypair file (None = generate new)
    pub keypair_path: Option<PathBuf>,
    /// Human-readable node name
    pub name: Option<String>,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            keypair_path: None,
            name: None,
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Listen addresses
    pub listen_addresses: Vec<String>,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<String>,
    /// Enable mDNS discovery
    pub enable_mdns: bool,
    /// Enable Kademlia DHT
    pub enable_dht: bool,
    /// Maximum number of connections
    pub max_connections: u32,
    /// Connection idle timeout
    #[serde(with = "humantime_serde")]
    pub idle_timeout: Duration,
    /// Transport configuration
    pub transport: TransportConfig,
    /// Gossipsub configuration
    pub gossipsub: GossipsubConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec![
                "/ip4/0.0.0.0/tcp/4001".to_string(),
                "/ip4/0.0.0.0/udp/4001/quic-v1".to_string(),
            ],
            bootstrap_peers: Vec::new(),
            enable_mdns: true,
            enable_dht: true,
            max_connections: 100,
            idle_timeout: Duration::from_secs(30),
            transport: TransportConfig::default(),
            gossipsub: GossipsubConfig::default(),
        }
    }
}

/// Transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Enable TCP transport
    pub enable_tcp: bool,
    /// Enable QUIC transport
    pub enable_quic: bool,
    /// Enable WebSocket transport (for browser clients)
    pub enable_websocket: bool,
    /// WebSocket listen port (if enabled)
    pub websocket_port: Option<u16>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            enable_tcp: true,
            enable_quic: true,
            enable_websocket: false,
            websocket_port: None,
        }
    }
}

/// Gossipsub configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipsubConfig {
    /// Heartbeat interval
    #[serde(with = "humantime_serde")]
    pub heartbeat_interval: Duration,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Message validation mode
    pub validation_mode: ValidationMode,
    /// Mesh parameters
    pub mesh_n: usize,
    pub mesh_n_low: usize,
    pub mesh_n_high: usize,
}

impl Default for GossipsubConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(1),
            max_message_size: 1024 * 1024, // 1 MB
            validation_mode: ValidationMode::Strict,
            mesh_n: 6,
            mesh_n_low: 4,
            mesh_n_high: 12,
        }
    }
}

/// Message validation mode for gossipsub
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationMode {
    /// Accept all messages
    Permissive,
    /// Validate message signatures
    Strict,
    /// Anonymous mode (no signatures)
    Anonymous,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Data directory path
    pub data_dir: PathBuf,
    /// Database backend
    pub backend: StorageBackend,
    /// Cache size in MB
    pub cache_size_mb: u32,
    /// Enable content-addressed storage
    pub enable_cas: bool,
    /// Maximum storage size in GB (0 = unlimited)
    pub max_storage_gb: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            backend: StorageBackend::Sqlite,
            cache_size_mb: 64,
            enable_cas: true,
            max_storage_gb: 0,
        }
    }
}

/// Storage backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageBackend {
    /// SQLite (default)
    Sqlite,
    /// In-memory (for testing)
    Memory,
    /// RocksDB (high performance)
    RocksDb,
}

/// Module configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulesConfig {
    /// Enable social module
    pub enable_social: bool,
    /// Enable orchestration module
    pub enable_orchestration: bool,
    /// Enable economics module
    pub enable_economics: bool,
    /// Module tick interval
    #[serde(with = "humantime_serde")]
    pub tick_interval: Duration,
}

impl Default for ModulesConfig {
    fn default() -> Self {
        Self {
            enable_social: true,
            enable_orchestration: false,
            enable_economics: false,
            tick_interval: Duration::from_millis(100),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: LogLevel,
    /// Log format
    pub format: LogFormat,
    /// Log to file
    pub log_file: Option<PathBuf>,
    /// Enable structured logging (JSON)
    pub structured: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Pretty,
            log_file: None,
            structured: false,
        }
    }
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Log format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogFormat {
    /// Human-readable pretty format
    Pretty,
    /// Compact single-line format
    Compact,
    /// JSON format
    Json,
}

/// Reputation system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    /// Initial reputation score for new peers
    pub initial_score: f64,
    /// Trust threshold (below this = untrusted)
    pub trust_threshold: f64,
    /// Alpha coefficient for EMA (weight of previous score)
    pub alpha: f64,
    /// Beta coefficient for EMA (weight of new contribution)
    pub beta: f64,
    /// Decay rate for inactive peers
    pub decay_rate: f64,
    /// Maximum history entries to keep
    pub max_history: usize,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            initial_score: 0.5,
            trust_threshold: 0.4,
            alpha: 0.4,
            beta: 0.6,
            decay_rate: 0.01,
            max_history: 100,
        }
    }
}

/// Credit system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditConfig {
    /// Default credit limit for new relationships
    pub default_credit_limit: f64,
    /// Maximum credit limit allowed
    pub max_credit_limit: f64,
    /// Interest rate (if applicable)
    pub interest_rate: f64,
    /// Grace period for settlement
    #[serde(with = "humantime_serde")]
    pub settlement_grace_period: Duration,
}

impl Default for CreditConfig {
    fn default() -> Self {
        Self {
            default_credit_limit: 100.0,
            max_credit_limit: 10000.0,
            interest_rate: 0.0,
            settlement_grace_period: Duration::from_secs(86400 * 30), // 30 days
        }
    }
}

// Helper module for Duration serialization
mod humantime_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = humantime::format_duration(*duration).to_string();
        s.serialize(serializer)
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
        let config = NodeConfig::default();
        assert!(config.network.enable_mdns);
        assert_eq!(config.storage.backend, StorageBackend::Sqlite);
    }

    #[test]
    fn test_config_serialization() {
        let config = NodeConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let recovered: NodeConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(
            config.network.max_connections,
            recovered.network.max_connections
        );
    }
}
