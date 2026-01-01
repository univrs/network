//! WebSocket message types
//!
//! This module defines the message types exchanged between the WebSocket server
//! and dashboard clients. Includes support for economics protocols (vouch, credit,
//! governance, resource).

use mycelial_core::peer::PeerInfo;
use serde::{Deserialize, Serialize};

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
    PeerLeft { peer_id: String },

    /// A chat message was received
    ChatMessage {
        id: String,
        from: String,
        from_name: String,
        to: Option<String>,
        room_id: Option<String>,
        content: String,
        timestamp: i64,
    },

    /// A peer's reputation was updated
    ReputationUpdate { peer_id: String, new_score: f64 },

    /// Full list of peers
    PeersList { peers: Vec<PeerListEntry> },

    /// Network statistics
    Stats {
        peer_count: usize,
        message_count: u64,
        uptime_seconds: u64,
    },

    /// Error message
    Error { message: String },

    // ============ Economics Protocol Messages ============
    /// Vouch request received
    VouchRequest {
        id: String,
        voucher: String,
        vouchee: String,
        weight: f64,
        timestamp: i64,
    },

    /// Vouch acknowledgement
    VouchAck {
        id: String,
        request_id: String,
        accepted: bool,
        new_reputation: Option<f64>,
        timestamp: i64,
    },

    /// Credit line created or updated
    CreditLine {
        id: String,
        creditor: String,
        debtor: String,
        limit: f64,
        balance: f64,
        timestamp: i64,
    },

    /// Credit transfer completed
    CreditTransfer {
        id: String,
        from: String,
        to: String,
        amount: f64,
        memo: Option<String>,
        timestamp: i64,
    },

    /// Governance proposal created
    Proposal {
        id: String,
        proposer: String,
        title: String,
        description: String,
        proposal_type: String,
        status: String,
        yes_votes: u32,
        no_votes: u32,
        quorum: u32,
        deadline: i64,
        timestamp: i64,
    },

    /// Vote cast on a proposal
    VoteCast {
        id: String,
        proposal_id: String,
        voter: String,
        vote: String,
        weight: f64,
        timestamp: i64,
    },

    /// Resource contribution reported
    ResourceContribution {
        id: String,
        peer_id: String,
        resource_type: String,
        amount: f64,
        unit: String,
        timestamp: i64,
    },

    /// Resource pool update
    ResourcePoolUpdate {
        resource_type: String,
        total_available: f64,
        total_used: f64,
        contributors: Vec<ContributorEntry>,
        timestamp: i64,
    },

    // ============ Room/Seance Messages ============
    /// Successfully joined a room
    RoomJoined {
        id: String,
        name: String,
        description: Option<String>,
        topic: String,
        members: Vec<String>,
        created_by: String,
        created_at: i64,
        is_public: bool,
    },

    /// Left a room
    RoomLeft { room_id: String },

    /// List of available rooms
    RoomList { rooms: Vec<RoomEntry> },

    /// A peer joined a room
    RoomPeerJoined {
        room_id: String,
        peer_id: String,
        peer_name: Option<String>,
    },

    /// A peer left a room
    RoomPeerLeft { room_id: String, peer_id: String },

    // ============ ENR Bridge Messages ============
    /// Resource gradient update from a node
    GradientUpdate {
        source: String,
        cpu_available: f64,
        memory_available: f64,
        bandwidth_available: f64,
        storage_available: f64,
        timestamp: i64,
    },

    /// ENR credit transfer (different from mutual credit)
    EnrCreditTransfer {
        from: String,
        to: String,
        amount: u64,
        tax: u64,
        nonce: u64,
        timestamp: i64,
    },

    /// ENR balance update
    EnrBalanceUpdate {
        node_id: String,
        balance: u64,
        timestamp: i64,
    },

    /// Nexus election announcement
    ElectionAnnouncement {
        election_id: u64,
        initiator: String,
        region_id: String,
        timestamp: i64,
    },

    /// Nexus election candidacy
    ElectionCandidacy {
        election_id: u64,
        candidate: String,
        uptime: u64,
        cpu_available: f64,
        memory_available: f64,
        reputation: f64,
        timestamp: i64,
    },

    /// Nexus election vote
    ElectionVote {
        election_id: u64,
        voter: String,
        candidate: String,
        timestamp: i64,
    },

    /// Nexus election result
    ElectionResult {
        election_id: u64,
        winner: String,
        region_id: String,
        vote_count: u32,
        timestamp: i64,
    },

    /// Septal gate state change (circuit breaker)
    SeptalStateChange {
        node_id: String,
        from_state: String,
        to_state: String,
        reason: String,
        timestamp: i64,
    },

    /// Septal health probe response
    SeptalHealthStatus {
        node_id: String,
        is_healthy: bool,
        failure_count: u32,
        timestamp: i64,
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

/// Entry for resource pool contributors
#[derive(Debug, Clone, Serialize)]
pub struct ContributorEntry {
    pub peer_id: String,
    pub contribution: f64,
    pub percentage: f64,
}

/// Entry for room list
#[derive(Debug, Clone, Serialize)]
pub struct RoomEntry {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub member_count: usize,
    pub is_public: bool,
    pub created_at: i64,
}

/// Messages sent from client to server
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Send a chat message
    SendChat {
        content: String,
        to: Option<String>,
        room_id: Option<String>,
    },

    /// Request peer list
    GetPeers,

    /// Request network stats
    GetStats,

    /// Subscribe to a topic
    Subscribe { topic: String },

    // ============ Economics Protocol Client Messages ============
    /// Request to vouch for another peer
    SendVouch {
        /// Target peer to vouch for
        vouchee: String,
        /// Weight of the vouch (0.0-1.0)
        weight: f64,
        /// Optional message
        message: Option<String>,
    },

    /// Respond to a vouch request
    RespondVouch {
        /// ID of the vouch request
        request_id: String,
        /// Accept or reject
        accept: bool,
    },

    /// Create a credit line with another peer
    CreateCreditLine {
        /// Peer to extend credit to
        debtor: String,
        /// Credit limit
        limit: f64,
    },

    /// Transfer credit to another peer
    TransferCredit {
        /// Recipient peer
        to: String,
        /// Amount to transfer
        amount: f64,
        /// Optional memo
        memo: Option<String>,
    },

    /// Create a governance proposal
    CreateProposal {
        /// Proposal title
        title: String,
        /// Proposal description
        description: String,
        /// Proposal type (text, parameter_change, treasury_spend)
        proposal_type: String,
    },

    /// Cast a vote on a proposal
    CastVote {
        /// Proposal ID
        proposal_id: String,
        /// Vote (yes, no, abstain)
        vote: String,
    },

    /// Report a resource contribution
    ReportResource {
        /// Resource type (bandwidth, storage, compute)
        resource_type: String,
        /// Amount contributed
        amount: f64,
        /// Unit of measurement
        unit: String,
    },

    // ============ Room/Seance Client Messages ============
    /// Create a new room
    CreateRoom {
        /// Room ID (optional, generated if not provided)
        room_id: Option<String>,
        /// Room name
        room_name: String,
        /// Room description
        description: Option<String>,
        /// Whether room is publicly discoverable
        is_public: Option<bool>,
    },

    /// Join an existing room
    JoinRoom {
        /// Room ID to join
        room_id: String,
        /// Optional room name hint (for discovery)
        room_name: Option<String>,
    },

    /// Leave a room
    LeaveRoom {
        /// Room ID to leave
        room_id: String,
    },

    /// Get list of available rooms
    GetRooms,

    // ============ ENR Bridge Client Messages ============
    /// Report resource gradient (availability)
    ReportGradient {
        /// CPU availability (0.0-1.0)
        cpu_available: f64,
        /// Memory availability (0.0-1.0)
        memory_available: f64,
        /// Bandwidth availability (0.0-1.0)
        bandwidth_available: f64,
        /// Storage availability (0.0-1.0)
        storage_available: f64,
    },

    /// Start a nexus election for a region
    StartElection {
        /// Region identifier
        region_id: String,
    },

    /// Register as an election candidate
    RegisterCandidacy {
        /// Election ID to register for
        election_id: u64,
        /// Node uptime in seconds
        uptime: u64,
        /// CPU availability (0.0-1.0)
        cpu_available: f64,
        /// Memory availability (0.0-1.0)
        memory_available: f64,
        /// Reputation score
        reputation: f64,
    },

    /// Vote for a candidate in an election
    VoteElection {
        /// Election ID
        election_id: u64,
        /// Candidate node ID to vote for
        candidate: String,
    },

    /// Send ENR credits to another node
    SendEnrCredit {
        /// Recipient node ID
        to: String,
        /// Amount of ENR credits to send
        amount: u64,
    },
}
