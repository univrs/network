//! OpenRaft Consensus Layer for ENR Credit Ledger
//!
//! This module provides distributed consensus for the credit synchronization
//! system using OpenRaft. It replaces the MVP's optimistic local ledger with
//! a strongly consistent replicated state machine.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                  RaftCreditLedger                   │
//! │                                                     │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
//! │  │ RaftNetwork │  │RaftLogStore │  │RaftStateMac │ │
//! │  │ (gossipsub) │  │ (sled/mem)  │  │  (credits)  │ │
//! │  └─────────────┘  └─────────────┘  └─────────────┘ │
//! │                                                     │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Status: Sprint 1 Scaffold
//!
//! This is the initial scaffold for OpenRaft integration.
//! Full implementation in progress per docs/OpenRaft/README.md

mod config;
mod types;

pub use config::RaftConfig;
pub use types::{CreditCommand, CreditResponse};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use univrs_enr::core::{AccountId, CreditTransfer, Credits, NodeId};

use crate::enr_bridge::credits::TransferError;

/// Callback type for publishing to gossipsub
pub type PublishFn = Box<dyn Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync>;

/// Gossipsub topic for Raft protocol messages
pub const RAFT_TOPIC: &str = "/vudo/enr/raft/1.0.0";

/// Raft-based credit ledger with distributed consensus
///
/// Sprint 1 implementation uses local state with Raft message broadcasting.
/// Full OpenRaft integration will be completed in Sprint 2.
pub struct RaftCreditLedger {
    /// Local node ID
    local_node: NodeId,
    /// Account balances (local state, will be replicated via Raft)
    balances: Arc<RwLock<HashMap<AccountId, Credits>>>,
    /// Revival pool balance
    revival_pool: Arc<RwLock<Credits>>,
    /// Publish function for Raft messages
    publish_fn: PublishFn,
    /// Configuration
    config: RaftConfig,
    /// Is this node the leader?
    is_leader: Arc<RwLock<bool>>,
    /// Current term
    current_term: Arc<RwLock<u64>>,
    /// Log index
    log_index: Arc<RwLock<u64>>,
}

impl RaftCreditLedger {
    /// Create a new single-node Raft cluster (for testing)
    pub async fn new_single_node(
        node_id: NodeId,
        publish_fn: impl Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync + 'static,
    ) -> Result<Self, RaftError> {
        let config = RaftConfig::default();
        Self::new_with_config(node_id, publish_fn, config, true).await
    }

    /// Create a new Raft node with custom configuration
    pub async fn new_with_config(
        node_id: NodeId,
        publish_fn: impl Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync + 'static,
        config: RaftConfig,
        bootstrap: bool,
    ) -> Result<Self, RaftError> {
        info!(node = %node_id, bootstrap, "Creating RaftCreditLedger");

        let ledger = Self {
            local_node: node_id,
            balances: Arc::new(RwLock::new(HashMap::new())),
            revival_pool: Arc::new(RwLock::new(Credits::ZERO)),
            publish_fn: Box::new(publish_fn),
            config,
            is_leader: Arc::new(RwLock::new(bootstrap)), // Bootstrap node starts as leader
            current_term: Arc::new(RwLock::new(1)),
            log_index: Arc::new(RwLock::new(0)),
        };

        Ok(ledger)
    }

    /// Propose a credit command to the Raft cluster
    pub async fn propose(&self, command: CreditCommand) -> Result<CreditResponse, RaftError> {
        // Check if we're the leader
        if !*self.is_leader.read().await {
            return Err(RaftError::NotLeader);
        }

        debug!(?command, "Proposing command");

        // Increment log index
        let log_idx = {
            let mut idx = self.log_index.write().await;
            *idx += 1;
            *idx
        };

        // Apply command locally
        let response = self.apply_command(&command).await;

        // Broadcast to followers (in full implementation, wait for quorum)
        let msg = RaftLogEntry {
            term: *self.current_term.read().await,
            index: log_idx,
            command: command.clone(),
        };

        if let Ok(bytes) = bincode::serialize(&msg) {
            if let Err(e) = (self.publish_fn)(RAFT_TOPIC.to_string(), bytes) {
                warn!("Failed to broadcast Raft entry: {}", e);
            }
        }

        Ok(response)
    }

    /// Apply a command to the state machine
    async fn apply_command(&self, command: &CreditCommand) -> CreditResponse {
        match command {
            CreditCommand::Transfer(transfer) => {
                let result = self.apply_transfer(transfer).await;
                CreditResponse::Transfer(result.map_err(|e| e.to_string()))
            }
            CreditCommand::GrantCredits { node, amount } => {
                let account = AccountId::node_account(*node);
                let mut balances = self.balances.write().await;
                let current = balances.get(&account).copied().unwrap_or(Credits::ZERO);
                balances.insert(account, current.saturating_add(*amount));
                info!(node = %node, amount = amount.amount, "Granted credits");
                CreditResponse::Grant
            }
            CreditCommand::RecordFailure { node, reason, .. } => {
                debug!(node = %node, reason = %reason, "Recorded failure");
                CreditResponse::FailureRecorded
            }
            CreditCommand::Noop => CreditResponse::Noop,
        }
    }

    /// Apply a credit transfer
    async fn apply_transfer(&self, transfer: &CreditTransfer) -> Result<(), TransferError> {
        let mut balances = self.balances.write().await;

        let from_balance = balances
            .get(&transfer.from)
            .copied()
            .unwrap_or(Credits::ZERO);
        let total_cost = transfer.amount.saturating_add(transfer.entropy_cost);

        if from_balance.amount < total_cost.amount {
            return Err(TransferError::InsufficientCredits {
                available: from_balance,
                required: total_cost,
            });
        }

        // Debit sender
        balances.insert(
            transfer.from.clone(),
            from_balance.saturating_sub(total_cost),
        );

        // Credit receiver
        let to_balance = balances.get(&transfer.to).copied().unwrap_or(Credits::ZERO);
        balances.insert(
            transfer.to.clone(),
            to_balance.saturating_add(transfer.amount),
        );

        drop(balances);

        // Add tax to revival pool
        let mut pool = self.revival_pool.write().await;
        *pool = pool.saturating_add(transfer.entropy_cost);

        debug!(
            from = ?transfer.from,
            to = ?transfer.to,
            amount = transfer.amount.amount,
            tax = transfer.entropy_cost.amount,
            "Applied transfer"
        );

        Ok(())
    }

    /// Transfer credits (convenience method)
    pub async fn transfer(&self, to: NodeId, amount: Credits) -> Result<(), TransferError> {
        if amount.is_zero() {
            return Err(TransferError::ZeroAmount);
        }

        if to == self.local_node {
            return Err(TransferError::SelfTransfer);
        }

        let transfer = CreditTransfer::new(
            AccountId::node_account(self.local_node),
            AccountId::node_account(to),
            amount,
            univrs_enr::revival::calculate_entropy_tax(amount),
        );

        let response = self
            .propose(CreditCommand::Transfer(transfer))
            .await
            .map_err(|e| TransferError::Publish(e.to_string()))?;

        match response {
            CreditResponse::Transfer(Ok(())) => Ok(()),
            CreditResponse::Transfer(Err(msg)) => Err(TransferError::Publish(msg)),
            _ => Err(TransferError::Publish("Unexpected response".into())),
        }
    }

    /// Get balance for an account
    pub async fn get_balance(&self, account: &AccountId) -> Credits {
        let balances = self.balances.read().await;
        balances.get(account).copied().unwrap_or(Credits::ZERO)
    }

    /// Get local node's balance
    pub async fn local_balance(&self) -> Credits {
        self.get_balance(&AccountId::node_account(self.local_node))
            .await
    }

    /// Grant initial credits to a node
    pub async fn grant_credits(&self, node: NodeId, amount: Credits) -> Result<(), RaftError> {
        self.propose(CreditCommand::GrantCredits { node, amount })
            .await?;
        Ok(())
    }

    /// Check if this node is the Raft leader
    pub async fn is_leader(&self) -> bool {
        *self.is_leader.read().await
    }

    /// Get the current Raft leader (self if leader, None otherwise for now)
    pub async fn leader(&self) -> Option<NodeId> {
        if self.is_leader().await {
            Some(self.local_node)
        } else {
            None
        }
    }

    /// Get all known account balances
    pub async fn all_balances(&self) -> HashMap<AccountId, Credits> {
        self.balances.read().await.clone()
    }

    /// Get total credits in circulation
    pub async fn total_supply(&self) -> Credits {
        let balances = self.balances.read().await;
        balances
            .values()
            .fold(Credits::ZERO, |acc, c| acc.saturating_add(*c))
    }

    /// Get revival pool balance
    pub async fn revival_pool(&self) -> Credits {
        *self.revival_pool.read().await
    }

    /// Handle incoming Raft message from gossipsub
    pub async fn handle_message(&self, bytes: &[u8]) -> Result<(), RaftError> {
        let entry: RaftLogEntry =
            bincode::deserialize(bytes).map_err(|e| RaftError::Decode(e.to_string()))?;

        debug!(
            term = entry.term,
            index = entry.index,
            "Received Raft entry"
        );

        // If we're not the leader, apply the entry
        if !self.is_leader().await {
            self.apply_command(&entry.command).await;
        }

        Ok(())
    }
}

/// A Raft log entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RaftLogEntry {
    /// Term when entry was created
    pub term: u64,
    /// Log index
    pub index: u64,
    /// The command to apply
    pub command: CreditCommand,
}

/// Errors that can occur in Raft operations
#[derive(Debug, thiserror::Error)]
pub enum RaftError {
    #[error("Not the leader")]
    NotLeader,
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Initialization error: {0}")]
    Init(String),
    #[error("Bootstrap error: {0}")]
    Bootstrap(String),
    #[error("Propose error: {0}")]
    Propose(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Decode error: {0}")]
    Decode(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Initial credits for test nodes (matches INITIAL_NODE_CREDITS)
    const TEST_INITIAL_CREDITS: u64 = 1000;

    fn mock_publish() -> (
        impl Fn(String, Vec<u8>) -> Result<(), String> + Clone,
        Arc<AtomicUsize>,
    ) {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        let f = move |_topic: String, _bytes: Vec<u8>| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        };
        (f, counter)
    }

    #[tokio::test]
    async fn test_single_node_creation() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, _) = mock_publish();

        let ledger = RaftCreditLedger::new_single_node(node, publish).await;
        assert!(ledger.is_ok());

        let ledger = ledger.unwrap();
        assert!(ledger.is_leader().await);
    }

    #[tokio::test]
    async fn test_grant_and_transfer() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, counter) = mock_publish();

        let ledger = RaftCreditLedger::new_single_node(node1, publish)
            .await
            .unwrap();

        // Grant initial credits
        ledger
            .grant_credits(node1, Credits::new(TEST_INITIAL_CREDITS))
            .await
            .unwrap();

        let balance = ledger.local_balance().await;
        assert_eq!(balance.amount, TEST_INITIAL_CREDITS);

        // Transfer to node2
        ledger.transfer(node2, Credits::new(100)).await.unwrap();

        // Check balances: node1 should have 898 (1000 - 100 - 2 tax)
        let balance = ledger.local_balance().await;
        assert_eq!(balance.amount, 898);

        // node2 should have 100
        let balance = ledger.get_balance(&AccountId::node_account(node2)).await;
        assert_eq!(balance.amount, 100);

        // Revival pool should have 2 (tax)
        assert_eq!(ledger.revival_pool().await.amount, 2);

        // Should have broadcast 2 messages (grant + transfer)
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_insufficient_balance() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();

        let ledger = RaftCreditLedger::new_single_node(node1, publish)
            .await
            .unwrap();

        // Grant 50 credits
        ledger.grant_credits(node1, Credits::new(50)).await.unwrap();

        // Try to transfer 100 (insufficient - error message contains "Insufficient")
        let result = ledger.transfer(node2, Credits::new(100)).await;
        assert!(matches!(result, Err(TransferError::Publish(msg)) if msg.contains("Insufficient")));
    }

    #[tokio::test]
    async fn test_self_transfer_rejected() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, _) = mock_publish();

        let ledger = RaftCreditLedger::new_single_node(node, publish)
            .await
            .unwrap();

        ledger
            .grant_credits(node, Credits::new(1000))
            .await
            .unwrap();

        let result = ledger.transfer(node, Credits::new(100)).await;
        assert!(matches!(result, Err(TransferError::SelfTransfer)));
    }
}
