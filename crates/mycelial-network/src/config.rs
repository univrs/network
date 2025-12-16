//! Network configuration types

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Addresses to listen on
    pub listen_addresses: Vec<String>,
    /// Bootstrap peers to connect to
    pub bootstrap_peers: Vec<String>,
    /// Enable mDNS for local peer discovery
    pub enable_mdns: bool,
    /// Enable Kademlia DHT
    pub enable_kademlia: bool,
    /// Maximum number of connections
    pub max_connections: u32,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Connection idle timeout in seconds
    pub idle_timeout_secs: u64,
    /// Enable TCP transport
    pub enable_tcp: bool,
    /// Enable QUIC transport
    pub enable_quic: bool,
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
            enable_kademlia: true,
            max_connections: 100,
            max_message_size: 1024 * 1024, // 1 MB
            idle_timeout_secs: 30,
            enable_tcp: true,
            enable_quic: true,
        }
    }
}

impl NetworkConfig {
    /// Create a configuration for local testing
    pub fn local_test(port: u16) -> Self {
        Self {
            listen_addresses: vec![format!("/ip4/127.0.0.1/tcp/{}", port)],
            bootstrap_peers: Vec::new(),
            enable_mdns: true,
            enable_kademlia: true,
            max_connections: 50,
            max_message_size: 1024 * 1024,
            idle_timeout_secs: 30,
            enable_tcp: true,
            enable_quic: false, // Simpler for testing
        }
    }

    /// Get the idle timeout as a Duration
    pub fn idle_timeout(&self) -> Duration {
        Duration::from_secs(self.idle_timeout_secs)
    }
}
