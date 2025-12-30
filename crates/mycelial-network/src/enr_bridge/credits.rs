//! Credit Synchronization with Local Ledger
//!
//! MVP implementation uses a local HashMap as the ledger.
//! Transfers are broadcast via gossip and applied optimistically.
//! Full consensus (OpenRaft) deferred to Phase 3+.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use univrs_enr::{
    core::{AccountId, Credits, CreditTransfer, NodeId, Timestamp},
    revival::calculate_entropy_tax,
};

use crate::enr_bridge::messages::{
    BalanceQueryMsg, BalanceResponseMsg, CreditTransferMsg, EnrMessage, CREDIT_TOPIC,
};

/// Initial credit grant for new nodes
pub const INITIAL_NODE_CREDITS: u64 = 1000;

/// Callback type for publishing to gossipsub
pub type PublishFn = Box<dyn Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync>;

/// Local credit ledger and synchronization manager
pub struct CreditSynchronizer {
    /// This node's ID
    local_node: NodeId,
    /// Local ledger: AccountId -> balance
    ledger: Arc<RwLock<HashMap<AccountId, Credits>>>,
    /// Processed transfer nonces (replay protection)
    processed_nonces: Arc<RwLock<HashMap<NodeId, u64>>>,
    /// Next nonce for outgoing transfers
    next_nonce: Arc<RwLock<u64>>,
    /// Callback to publish to gossipsub
    publish_fn: PublishFn,
}

impl CreditSynchronizer {
    /// Create a new credit synchronizer with initial balance
    pub fn new<F>(local_node: NodeId, publish_fn: F) -> Self
    where
        F: Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync + 'static,
    {
        let mut ledger = HashMap::new();
        // Initialize local node with starting credits
        let local_account = AccountId::node_account(local_node);
        ledger.insert(local_account, Credits::new(INITIAL_NODE_CREDITS));

        info!(
            node = %local_node,
            initial_balance = INITIAL_NODE_CREDITS,
            "Initialized credit ledger"
        );

        Self {
            local_node,
            ledger: Arc::new(RwLock::new(ledger)),
            processed_nonces: Arc::new(RwLock::new(HashMap::new())),
            next_nonce: Arc::new(RwLock::new(1)),
            publish_fn: Box::new(publish_fn),
        }
    }

    /// Get balance for an account
    pub async fn get_balance(&self, account: &AccountId) -> Credits {
        let ledger = self.ledger.read().await;
        ledger.get(account).copied().unwrap_or(Credits::ZERO)
    }

    /// Get local node's balance
    pub async fn local_balance(&self) -> Credits {
        let account = AccountId::node_account(self.local_node);
        self.get_balance(&account).await
    }

    /// Transfer credits to another node
    pub async fn transfer(
        &self,
        to: NodeId,
        amount: Credits,
    ) -> Result<CreditTransfer, TransferError> {
        if amount.is_zero() {
            return Err(TransferError::ZeroAmount);
        }

        if to == self.local_node {
            return Err(TransferError::SelfTransfer);
        }

        let from_account = AccountId::node_account(self.local_node);
        let to_account = AccountId::node_account(to);

        // Calculate entropy tax (2% per ENR spec)
        let entropy_cost = calculate_entropy_tax(amount);
        let total_cost = amount.saturating_add(entropy_cost);

        // Check and debit balance atomically
        let mut ledger = self.ledger.write().await;
        let from_balance = ledger.get(&from_account).copied().unwrap_or(Credits::ZERO);

        if from_balance.amount < total_cost.amount {
            return Err(TransferError::InsufficientCredits {
                available: from_balance,
                required: total_cost,
            });
        }

        // Debit sender
        ledger.insert(from_account.clone(), from_balance.saturating_sub(total_cost));

        // Credit receiver
        let to_balance = ledger.get(&to_account).copied().unwrap_or(Credits::ZERO);
        ledger.insert(to_account.clone(), to_balance.saturating_add(amount));

        drop(ledger);

        // Create transfer record
        let transfer = CreditTransfer::new(from_account, to_account, amount, entropy_cost);

        // Get nonce and broadcast
        let nonce = {
            let mut n = self.next_nonce.write().await;
            let current = *n;
            *n += 1;
            current
        };

        let msg = CreditTransferMsg {
            transfer: transfer.clone(),
            nonce,
            signature: vec![], // TODO: Sign with Ed25519
        };

        let envelope = EnrMessage::CreditTransfer(msg);
        let bytes = envelope.encode().map_err(TransferError::Encode)?;
        (self.publish_fn)(CREDIT_TOPIC.to_string(), bytes).map_err(TransferError::Publish)?;

        info!(
            to = %to,
            amount = amount.amount,
            tax = entropy_cost.amount,
            "Transferred credits"
        );

        Ok(transfer)
    }

    /// Handle incoming transfer from gossip
    pub async fn handle_transfer(&self, msg: CreditTransferMsg) -> Result<(), HandleTransferError> {
        let transfer = &msg.transfer;

        // Skip if this is our own transfer (already applied locally)
        if transfer.from.node == self.local_node {
            return Ok(());
        }

        // Check for replay
        {
            let mut nonces = self.processed_nonces.write().await;
            let last_nonce = nonces.get(&transfer.from.node).copied().unwrap_or(0);
            if msg.nonce <= last_nonce {
                warn!(
                    from = %transfer.from.node,
                    nonce = msg.nonce,
                    last = last_nonce,
                    "Rejecting replayed transfer"
                );
                return Err(HandleTransferError::ReplayedNonce);
            }
            nonces.insert(transfer.from.node, msg.nonce);
        }

        // TODO: Verify signature

        // Apply transfer optimistically
        // In MVP, we trust broadcasts. Consensus comes in Phase 3+.
        // Credit receiver if this transfer is TO us
        if transfer.to.node == self.local_node {
            let mut ledger = self.ledger.write().await;
            let to_balance = ledger.get(&transfer.to).copied().unwrap_or(Credits::ZERO);
            ledger.insert(transfer.to.clone(), to_balance.saturating_add(transfer.amount));

            debug!(
                from = %transfer.from.node,
                to = %transfer.to.node,
                amount = transfer.amount.amount,
                "Credited incoming transfer to local account"
            );
        } else {
            // Track transfer in ledger for network state (optimistic)
            let mut ledger = self.ledger.write().await;
            let total_cost = transfer.amount.saturating_add(transfer.entropy_cost);

            // Debit sender
            let from_balance = ledger.get(&transfer.from).copied().unwrap_or(Credits::ZERO);
            if from_balance.amount >= total_cost.amount {
                ledger.insert(transfer.from.clone(), from_balance.saturating_sub(total_cost));
            }

            // Credit receiver
            let to_balance = ledger.get(&transfer.to).copied().unwrap_or(Credits::ZERO);
            ledger.insert(transfer.to.clone(), to_balance.saturating_add(transfer.amount));

            debug!(
                from = %transfer.from.node,
                to = %transfer.to.node,
                amount = transfer.amount.amount,
                "Applied incoming transfer to ledger"
            );
        }

        Ok(())
    }

    /// Handle balance query from another node
    pub async fn handle_balance_query(
        &self,
        query: BalanceQueryMsg,
    ) -> Result<(), HandleQueryError> {
        // Only respond if we're the target
        if query.target != self.local_node {
            return Ok(());
        }

        let balance = self.local_balance().await;

        let response = BalanceResponseMsg {
            request_id: query.request_id,
            balance,
            as_of: Timestamp::now(),
        };

        let envelope = EnrMessage::BalanceResponse(response);
        let bytes = envelope.encode().map_err(HandleQueryError::Encode)?;
        (self.publish_fn)(CREDIT_TOPIC.to_string(), bytes).map_err(HandleQueryError::Publish)?;

        debug!(
            requester = %query.requester,
            balance = balance.amount,
            "Responded to balance query"
        );

        Ok(())
    }

    /// Ensure account exists with minimum balance (for new nodes joining)
    pub async fn ensure_account(&self, node: NodeId) {
        let account = AccountId::node_account(node);
        let mut ledger = self.ledger.write().await;
        ledger
            .entry(account)
            .or_insert(Credits::new(INITIAL_NODE_CREDITS));
    }

    /// Get all known account balances (for debugging/UI)
    pub async fn all_balances(&self) -> HashMap<AccountId, Credits> {
        let ledger = self.ledger.read().await;
        ledger.clone()
    }

    /// Get total credits in circulation (for invariant checking)
    pub async fn total_supply(&self) -> Credits {
        let ledger = self.ledger.read().await;
        ledger.values().fold(Credits::ZERO, |acc, c| acc.saturating_add(*c))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    #[error("Cannot transfer zero credits")]
    ZeroAmount,
    #[error("Cannot transfer to self")]
    SelfTransfer,
    #[error("Insufficient credits: have {available}, need {required}")]
    InsufficientCredits { available: Credits, required: Credits },
    #[error("Encoding error: {0}")]
    Encode(#[from] crate::enr_bridge::messages::EncodeError),
    #[error("Publish error: {0}")]
    Publish(String),
}

#[derive(Debug, thiserror::Error)]
pub enum HandleTransferError {
    #[error("Replayed nonce")]
    ReplayedNonce,
    #[error("Invalid signature")]
    InvalidSignature,
}

#[derive(Debug, thiserror::Error)]
pub enum HandleQueryError {
    #[error("Encoding error: {0}")]
    Encode(#[from] crate::enr_bridge::messages::EncodeError),
    #[error("Publish error: {0}")]
    Publish(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn mock_publish() -> (impl Fn(String, Vec<u8>) -> Result<(), String> + Clone, Arc<AtomicUsize>) {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        let f = move |_topic: String, _bytes: Vec<u8>| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        };
        (f, counter)
    }

    #[tokio::test]
    async fn test_initial_balance() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, _) = mock_publish();
        let sync = CreditSynchronizer::new(node, publish);

        let balance = sync.local_balance().await;
        assert_eq!(balance.amount, INITIAL_NODE_CREDITS);
    }

    #[tokio::test]
    async fn test_transfer_success() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, counter) = mock_publish();
        let sync = CreditSynchronizer::new(node1, publish);

        // Transfer 100 credits
        let transfer = sync.transfer(node2, Credits::new(100)).await.unwrap();

        // Should have broadcast
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Verify transfer details
        assert_eq!(transfer.amount.amount, 100);
        assert_eq!(transfer.entropy_cost.amount, 2); // 2% of 100

        // Balance should be 1000 - 100 - 2 = 898
        let balance = sync.local_balance().await;
        assert_eq!(balance.amount, 898);
    }

    #[tokio::test]
    async fn test_transfer_insufficient() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();
        let sync = CreditSynchronizer::new(node1, publish);

        // Try to transfer more than we have
        let result = sync.transfer(node2, Credits::new(2000)).await;
        assert!(matches!(result, Err(TransferError::InsufficientCredits { .. })));

        // Balance unchanged
        let balance = sync.local_balance().await;
        assert_eq!(balance.amount, INITIAL_NODE_CREDITS);
    }

    #[tokio::test]
    async fn test_transfer_zero() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();
        let sync = CreditSynchronizer::new(node1, publish);

        let result = sync.transfer(node2, Credits::ZERO).await;
        assert!(matches!(result, Err(TransferError::ZeroAmount)));
    }

    #[tokio::test]
    async fn test_transfer_self() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, _) = mock_publish();
        let sync = CreditSynchronizer::new(node, publish);

        let result = sync.transfer(node, Credits::new(100)).await;
        assert!(matches!(result, Err(TransferError::SelfTransfer)));
    }

    #[tokio::test]
    async fn test_handle_incoming_transfer() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();
        let sync = CreditSynchronizer::new(node1, publish);

        // Simulate incoming transfer from node2 to node1
        let transfer = CreditTransfer::new(
            AccountId::node_account(node2),
            AccountId::node_account(node1),
            Credits::new(50),
            Credits::new(1), // tax
        );

        let msg = CreditTransferMsg {
            transfer,
            nonce: 1,
            signature: vec![],
        };

        // Ensure node2 has balance first
        sync.ensure_account(node2).await;

        sync.handle_transfer(msg).await.unwrap();

        // Node1 should have received 50
        let balance = sync.local_balance().await;
        assert_eq!(balance.amount, INITIAL_NODE_CREDITS + 50);
    }

    #[tokio::test]
    async fn test_replay_protection() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();
        let sync = CreditSynchronizer::new(node1, publish);

        sync.ensure_account(node2).await;

        let transfer = CreditTransfer::new(
            AccountId::node_account(node2),
            AccountId::node_account(node1),
            Credits::new(50),
            Credits::new(1),
        );

        let msg = CreditTransferMsg {
            transfer: transfer.clone(),
            nonce: 1,
            signature: vec![],
        };

        // First should succeed
        sync.handle_transfer(msg.clone()).await.unwrap();

        // Replay should fail
        let result = sync.handle_transfer(msg).await;
        assert!(matches!(result, Err(HandleTransferError::ReplayedNonce)));
    }
}
