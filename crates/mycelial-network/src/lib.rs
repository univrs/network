//! Mycelial Network - P2P networking layer using libp2p
//!
//! This crate provides the networking infrastructure for the mycelial P2P system,
//! including peer discovery, gossipsub messaging, and DHT storage.
//!
//! # Overview
//!
//! The network layer is built on libp2p and provides:
//!
//! - **Gossipsub**: Pub/sub messaging for content propagation
//! - **Kademlia DHT**: Distributed hash table for peer discovery and data storage
//! - **mDNS**: Local network peer discovery
//! - **Identify**: Peer identification protocol
//! - **Noise**: Encryption for all connections
//! - **QUIC/TCP**: Multiple transport options
//!
//! # Example
//!
//! ```rust,no_run
//! use mycelial_network::{NetworkService, NetworkConfig};
//! use libp2p::identity::Keypair;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Generate identity
//!     let keypair = Keypair::generate_ed25519();
//!
//!     // Create network service
//!     let config = NetworkConfig::local_test(4001);
//!     let (service, handle, mut events) = NetworkService::new(keypair, config)?;
//!
//!     // Spawn the service
//!     tokio::spawn(async move {
//!         service.run().await.expect("Network service failed");
//!     });
//!
//!     // Subscribe to a topic
//!     handle.subscribe("/mycelia/1.0.0/content").await?;
//!
//!     // Publish a message
//!     handle.publish("/mycelia/1.0.0/content", b"Hello!".to_vec()).await?;
//!
//!     // Listen for events
//!     while let Ok(event) = events.recv().await {
//!         println!("Event: {:?}", event);
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod behaviour;
pub mod config;
pub mod economics;
pub mod error;
pub mod event;
pub mod peer;
pub mod service;
pub mod transport;

// ENR bridge module (requires univrs-compat feature for full univrs-enr integration)
#[cfg(feature = "univrs-compat")]
pub mod enr_bridge;

// OpenRaft consensus layer (Phase 1)
#[cfg(feature = "openraft")]
pub mod raft;


// Re-exports
pub use behaviour::{MycelialBehaviour, MycelialBehaviourEvent, topics};
pub use config::NetworkConfig;
pub use economics::{EconomicsEvent, EconomicsHandler, economics_topics, is_economics_topic, parse_economics_message};
pub use error::{NetworkError, Result};
pub use event::{NetworkEvent, NetworkStats};
pub use peer::{ConnectionState, PeerInfo, PeerManager};
pub use service::{NetworkCommand, NetworkHandle, NetworkService};
pub use transport::{TransportConfig, create_transport, parse_multiaddr, extract_peer_id};

// Test utilities - available with test-utils feature or in tests
// TODO: Add test_utils module to service when needed
// #[cfg(any(test, feature = "test-utils"))]
// pub use service::test_utils;

// Re-export libp2p types commonly used
pub use libp2p::identity::Keypair;
pub use libp2p::PeerId as Libp2pPeerId;
pub use libp2p::Multiaddr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = NetworkConfig::default();
        assert!(config.enable_mdns);
        assert!(config.enable_kademlia);
        assert_eq!(config.max_message_size, 1024 * 1024);
    }

    #[test]
    fn test_local_test_config() {
        let config = NetworkConfig::local_test(5000);
        assert_eq!(config.listen_addresses[0], "/ip4/127.0.0.1/tcp/5000");
    }
}
