//! Raft type definitions for ENR credit ledger
//!
//! Sprint 1 scaffold - simple types for local state with broadcasting.
//! Full OpenRaft types will be added in Sprint 2.

use serde::{Deserialize, Serialize};
use univrs_enr::core::{AccountId, Credits, CreditTransfer, NodeId, Timestamp};

/// Commands that can be proposed to the Raft cluster
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CreditCommand {
    /// Transfer credits between accounts
    Transfer(CreditTransfer),
    /// Grant initial credits to a new node
    GrantCredits {
        node: NodeId,
        amount: Credits,
    },
    /// Record a peer failure (for septal gate integration)
    RecordFailure {
        node: NodeId,
        reason: String,
        timestamp: Timestamp,
    },
    /// No-op command (for testing/heartbeat)
    Noop,
}

/// Responses from applying commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreditResponse {
    /// Response for a transfer command (Ok or error message)
    Transfer(Result<(), String>),
    /// Response for a grant command
    Grant,
    /// Response for a failure record
    FailureRecorded,
    /// Response for no-op
    Noop,
}

/// Snapshot data for the credit state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditSnapshot {
    /// Account balances
    pub balances: std::collections::HashMap<AccountId, Credits>,
    /// Revival pool balance
    pub revival_pool: Credits,
    /// Last applied log ID
    pub last_applied: Option<u64>,
}

impl Default for CreditSnapshot {
    fn default() -> Self {
        Self {
            balances: std::collections::HashMap::new(),
            revival_pool: Credits::ZERO,
            last_applied: None,
        }
    }
}

/// Convert ENR NodeId to u64 (uses first 8 bytes)
pub fn node_id_to_u64(node_id: NodeId) -> u64 {
    let bytes = node_id.to_bytes();
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

/// Convert u64 back to ENR NodeId (lossy - for display only)
pub fn u64_to_node_id(raft_id: u64) -> NodeId {
    let mut bytes = [0u8; 32];
    bytes[0..8].copy_from_slice(&raft_id.to_le_bytes());
    NodeId::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_conversion() {
        let node = NodeId::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        let raft_id = node_id_to_u64(node);
        assert_eq!(raft_id, 0x0807060504030201);
    }

    #[test]
    fn test_command_serialization() {
        let cmd = CreditCommand::GrantCredits {
            node: NodeId::from_bytes([1u8; 32]),
            amount: Credits::new(1000),
        };

        let serialized = bincode::serialize(&cmd).unwrap();
        let deserialized: CreditCommand = bincode::deserialize(&serialized).unwrap();

        assert_eq!(cmd, deserialized);
    }
}
