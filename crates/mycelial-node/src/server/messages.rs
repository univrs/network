//! WebSocket message types

use serde::{Deserialize, Serialize};
use mycelial_core::peer::PeerInfo;

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// A peer joined the network
    PeerJoined {
        peer_id: String,
        name: Option<String>,
    },

    /// A peer left the network
    PeerLeft {
        peer_id: String,
    },

    /// A chat message was received
    ChatMessage {
        id: String,
        from: String,
        from_name: String,
        to: Option<String>,
        content: String,
        timestamp: i64,
    },

    /// A peer's reputation was updated
    ReputationUpdate {
        peer_id: String,
        new_score: f64,
    },

    /// Full list of peers
    PeersList {
        peers: Vec<PeerListEntry>,
    },

    /// Network statistics
    Stats {
        peer_count: usize,
        message_count: u64,
        uptime_seconds: u64,
    },

    /// Error message
    Error {
        message: String,
    },
}

/// Entry in the peers list
#[derive(Debug, Clone, Serialize)]
pub struct PeerListEntry {
    pub id: String,
    pub name: Option<String>,
    pub reputation: f64,
    pub addresses: Vec<String>,
}

impl From<(PeerInfo, mycelial_core::reputation::Reputation)> for PeerListEntry {
    fn from((info, rep): (PeerInfo, mycelial_core::reputation::Reputation)) -> Self {
        Self {
            id: info.id.to_string(),
            name: info.name,
            reputation: rep.score,
            addresses: info.addresses,
        }
    }
}

/// Messages sent from client to server
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Send a chat message
    SendChat {
        content: String,
        to: Option<String>,
    },

    /// Request peer list
    GetPeers,

    /// Request network stats
    GetStats,

    /// Subscribe to a topic
    Subscribe {
        topic: String,
    },
}
