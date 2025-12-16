//! Network events
//!
//! Events emitted by the network service for consumption by other parts
//! of the application.

use chrono::{DateTime, Utc};
use libp2p::{gossipsub::MessageId, Multiaddr, PeerId};
use serde::{Deserialize, Serialize};

/// Events emitted by the network service
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// Network service started
    Started {
        /// Local peer ID
        peer_id: PeerId,
        /// Addresses we're listening on
        listen_addresses: Vec<Multiaddr>,
    },

    /// Network service stopped
    Stopped,

    /// Started listening on an address
    ListeningOn {
        /// The address we're listening on
        address: Multiaddr,
    },

    /// A new peer connected
    PeerConnected {
        /// The connected peer's ID
        peer_id: PeerId,
        /// Number of current connections
        num_connections: usize,
    },

    /// A peer disconnected
    PeerDisconnected {
        /// The disconnected peer's ID
        peer_id: PeerId,
        /// Number of remaining connections
        num_connections: usize,
    },

    /// Peer identification received
    PeerIdentified {
        /// The peer's ID
        peer_id: PeerId,
        /// Agent version
        agent_version: String,
        /// Protocol version
        protocol_version: String,
        /// Supported protocols
        protocols: Vec<String>,
        /// Observed address
        observed_addr: Multiaddr,
    },

    /// A gossipsub message was received
    MessageReceived {
        /// Message ID
        message_id: MessageId,
        /// Topic the message was received on
        topic: String,
        /// Source peer (if known)
        source: Option<PeerId>,
        /// Message data
        data: Vec<u8>,
        /// When the message was received
        timestamp: DateTime<Utc>,
    },

    /// Successfully subscribed to a topic
    Subscribed {
        /// The topic subscribed to
        topic: String,
    },

    /// Unsubscribed from a topic
    Unsubscribed {
        /// The topic unsubscribed from
        topic: String,
    },

    /// A peer subscribed to a topic we're subscribed to
    PeerSubscribed {
        /// The peer's ID
        peer_id: PeerId,
        /// The topic
        topic: String,
    },

    /// A peer unsubscribed from a topic
    PeerUnsubscribed {
        /// The peer's ID
        peer_id: PeerId,
        /// The topic
        topic: String,
    },

    /// DHT record found
    RecordFound {
        /// The key
        key: Vec<u8>,
        /// The value
        value: Vec<u8>,
    },

    /// DHT record stored
    RecordStored {
        /// The key
        key: Vec<u8>,
    },

    /// Peer discovered via mDNS
    MdnsDiscovered {
        /// Discovered peers
        peers: Vec<(PeerId, Multiaddr)>,
    },

    /// Peer expired from mDNS
    MdnsExpired {
        /// Expired peers
        peers: Vec<PeerId>,
    },

    /// Dialing a peer
    Dialing {
        /// The peer being dialed
        peer_id: PeerId,
    },

    /// Dial failed
    DialFailed {
        /// The peer we failed to dial
        peer_id: Option<PeerId>,
        /// Error message
        error: String,
    },

    /// Connection established (inbound or outbound)
    ConnectionEstablished {
        /// The peer's ID
        peer_id: PeerId,
        /// Number of established connections to this peer
        num_established: u32,
        /// Whether we initiated the connection
        outbound: bool,
    },

    /// Connection closed
    ConnectionClosed {
        /// The peer's ID
        peer_id: PeerId,
        /// Number of remaining connections to this peer
        num_established: u32,
        /// Reason for closure
        cause: Option<String>,
    },
}

impl NetworkEvent {
    /// Check if this is a peer connection event
    pub fn is_peer_event(&self) -> bool {
        matches!(
            self,
            NetworkEvent::PeerConnected { .. }
                | NetworkEvent::PeerDisconnected { .. }
                | NetworkEvent::PeerIdentified { .. }
                | NetworkEvent::ConnectionEstablished { .. }
                | NetworkEvent::ConnectionClosed { .. }
        )
    }

    /// Check if this is a message event
    pub fn is_message_event(&self) -> bool {
        matches!(self, NetworkEvent::MessageReceived { .. })
    }

    /// Check if this is a discovery event
    pub fn is_discovery_event(&self) -> bool {
        matches!(
            self,
            NetworkEvent::MdnsDiscovered { .. }
                | NetworkEvent::MdnsExpired { .. }
                | NetworkEvent::RecordFound { .. }
        )
    }

    /// Get the peer ID associated with this event, if any
    pub fn peer_id(&self) -> Option<&PeerId> {
        match self {
            NetworkEvent::PeerConnected { peer_id, .. } => Some(peer_id),
            NetworkEvent::PeerDisconnected { peer_id, .. } => Some(peer_id),
            NetworkEvent::PeerIdentified { peer_id, .. } => Some(peer_id),
            NetworkEvent::PeerSubscribed { peer_id, .. } => Some(peer_id),
            NetworkEvent::PeerUnsubscribed { peer_id, .. } => Some(peer_id),
            NetworkEvent::Dialing { peer_id } => Some(peer_id),
            NetworkEvent::ConnectionEstablished { peer_id, .. } => Some(peer_id),
            NetworkEvent::ConnectionClosed { peer_id, .. } => Some(peer_id),
            NetworkEvent::MessageReceived { source, .. } => source.as_ref(),
            _ => None,
        }
    }
}

/// Statistics about the network
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Number of connected peers
    pub connected_peers: usize,
    /// Total messages received
    pub messages_received: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Number of subscribed topics
    pub subscribed_topics: usize,
    /// Uptime in seconds
    pub uptime_secs: u64,
}
