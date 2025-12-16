//! Network-specific error types

use libp2p::TransportError;
use thiserror::Error;

/// Network-specific errors
#[derive(Error, Debug)]
pub enum NetworkError {
    /// Transport layer error
    #[error("Transport error: {0}")]
    Transport(String),

    /// Failed to dial peer
    #[error("Failed to dial peer {peer}: {reason}")]
    DialFailed { peer: String, reason: String },

    /// Connection closed
    #[error("Connection closed: {0}")]
    ConnectionClosed(String),

    /// Failed to listen on address
    #[error("Failed to listen on {address}: {reason}")]
    ListenFailed { address: String, reason: String },

    /// Gossipsub error
    #[error("Gossipsub error: {0}")]
    Gossipsub(String),

    /// Kademlia error
    #[error("Kademlia error: {0}")]
    Kademlia(String),

    /// Message too large
    #[error("Message too large: {size} bytes (max: {max})")]
    MessageTooLarge { size: usize, max: usize },

    /// Topic not subscribed
    #[error("Not subscribed to topic: {0}")]
    NotSubscribed(String),

    /// Peer not found
    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    /// Already connected to peer
    #[error("Already connected to peer: {0}")]
    AlreadyConnected(String),

    /// Invalid multiaddr
    #[error("Invalid multiaddr: {0}")]
    InvalidMultiaddr(String),

    /// Network not started
    #[error("Network service not started")]
    NotStarted,

    /// Network already started
    #[error("Network service already started")]
    AlreadyStarted,

    /// Timeout
    #[error("Operation timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// Channel error
    #[error("Channel error: {0}")]
    Channel(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl<T> From<TransportError<T>> for NetworkError
where
    T: std::fmt::Debug,
{
    fn from(err: TransportError<T>) -> Self {
        NetworkError::Transport(format!("{:?}", err))
    }
}

impl From<std::io::Error> for NetworkError {
    fn from(err: std::io::Error) -> Self {
        NetworkError::Transport(err.to_string())
    }
}

impl From<libp2p::swarm::ConnectionDenied> for NetworkError {
    fn from(err: libp2p::swarm::ConnectionDenied) -> Self {
        NetworkError::ConnectionClosed(err.to_string())
    }
}

/// Result type for network operations
pub type Result<T> = std::result::Result<T, NetworkError>;
