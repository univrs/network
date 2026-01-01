//! Mycelial Core - Foundational types and traits for the P2P network
//!
//! This crate provides the core abstractions used throughout the mycelial network,
//! including identity management, content addressing, module interfaces, and more.
//!
//! # Modules
//!
//! - [`identity`] - Cryptographic identity with Ed25519 keys and DID support
//! - [`content`] - Content-addressed storage using Blake3 hashing
//! - [`peer`] - Peer identity and information
//! - [`reputation`] - Reputation scoring and trust management
//! - [`credit`] - Mutual credit and economic relationships
//! - [`message`] - Network message types
//! - [`module`] - Module trait for substrate architecture
//! - [`event`] - Event types for cross-module communication
//! - [`config`] - Configuration types
//! - [`error`] - Comprehensive error types
//! - [`location`] - Geographic location types
//!
//! # Example
//!
//! ```rust
//! use mycelial_core::{identity::{Keypair, KeypairExt}, content::Content, peer::PeerInfo};
//!
//! // Generate a new identity
//! let keypair = Keypair::generate();
//! let did = keypair.did();  // Uses KeypairExt trait
//! println!("My DID: {}", did);
//!
//! // Create content-addressed data
//! let content = Content::text("Hello, Mycelial Network!");
//! println!("Content ID: {}", content.id);
//! ```

// Core modules
pub mod identity;
pub mod content;
pub mod peer;
pub mod reputation;
pub mod credit;
pub mod message;
pub mod location;

// Infrastructure modules
pub mod module;
pub mod event;
pub mod config;
pub mod error;

// Re-exports for convenience
pub use error::{MycelialError, Result};

// Identity re-exports
pub use identity::{
    Did, Keypair, KeypairExt, PublicKey, PublicKeyExt,
    Signature, SignatureBytes, Signed,
};

// Content re-exports
pub use content::{Content, ContentId, ContentMetadata};

// Peer re-exports
pub use peer::{PeerId, PeerInfo};

// Reputation re-exports
pub use reputation::Reputation;

// Credit re-exports
pub use credit::CreditRelationship;

// Message re-exports
pub use message::{Message, MessageType};

// Module re-exports
pub use module::{ModuleInfo, ModuleMessage, ModuleMetrics, ModuleRegistry, ModuleState, MyceliaModule};

// Event re-exports
pub use event::{Event, EventFilter, EventPayload, EventType};

// Config re-exports
pub use config::{NodeConfig, NetworkConfig, StorageConfig};

// Location re-exports
pub use location::Location;

use async_trait::async_trait;

/// Trait for components that can be started and stopped
#[async_trait]
pub trait Lifecycle: Send + Sync {
    /// Start the component
    async fn start(&mut self) -> Result<()>;

    /// Stop the component gracefully
    async fn stop(&mut self) -> Result<()>;

    /// Check if the component is running
    fn is_running(&self) -> bool;
}

/// Trait for message handlers
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming message
    async fn handle(&self, message: Message, from: PeerId) -> Result<Option<Message>>;
}

/// Trait for peer discovery mechanisms
#[async_trait]
pub trait PeerDiscovery: Send + Sync {
    /// Discover peers in the network
    async fn discover(&self) -> Result<Vec<PeerInfo>>;

    /// Announce this peer to the network
    async fn announce(&self, info: &PeerInfo) -> Result<()>;
}

/// Trait for state persistence
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Store peer information
    async fn store_peer(&self, info: &PeerInfo) -> Result<()>;

    /// Retrieve peer information
    async fn get_peer(&self, id: &PeerId) -> Result<Option<PeerInfo>>;

    /// List all known peers
    async fn list_peers(&self) -> Result<Vec<PeerInfo>>;

    /// Update peer reputation
    async fn update_reputation(&self, id: &PeerId, reputation: &Reputation) -> Result<()>;
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Protocol version
pub const PROTOCOL_VERSION: &str = "1.0.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        // VERSION is set at compile time from Cargo.toml
        assert!(VERSION.contains('.'), "VERSION should be semver format");
    }

    #[test]
    fn test_protocol_version() {
        assert_eq!(PROTOCOL_VERSION, "1.0.0");
    }
}
