//! ENR Bridge - Connects P2P Gossip Layer to ENR Economics
//!
//! This module bridges the mycelial-network gossipsub layer with the
//! univrs-enr economic primitives:
//!
//! - **Gradient Broadcasting**: Propagate resource availability via gossip
//! - **Credit Synchronization**: Transfer credits between nodes
//!
//! ## MVP Scope (Phase 0)
//!
//! - Local ledger with optimistic updates
//! - Gossipsub broadcast for transfers
//! - No consensus (deferred to Phase 3)
//!
//! ## Future Additions (Phase 3+)
//!
//! - Distributed nexus election
//! - OpenRaft consensus for ledger
//! - Septal gates (circuit breakers)
//!
//! ## Example
//!
//! ```rust,ignore
//! use mycelial_network::enr_bridge::{EnrBridge, GRADIENT_TOPIC, CREDIT_TOPIC};
//! use univrs_enr::{Credits, NodeId, ResourceGradient};
//!
//! // Create bridge with gossipsub publish callback
//! let bridge = EnrBridge::new(local_node_id, |topic, bytes| {
//!     swarm.behaviour_mut().gossipsub.publish(topic.into(), bytes)
//!         .map_err(|e| e.to_string())
//! });
//!
//! // Broadcast resource availability
//! bridge.broadcast_gradient(ResourceGradient {
//!     cpu_available: 0.75,
//!     memory_available: 0.60,
//!     ..Default::default()
//! }).await?;
//!
//! // Transfer credits
//! bridge.transfer_credits(peer_id, Credits::new(100)).await?;
//!
//! // Handle incoming message
//! bridge.handle_message(&gossip_message.data).await?;
//! ```

pub mod credits;
pub mod gradient;
pub mod messages;

pub use credits::{CreditSynchronizer, TransferError, INITIAL_NODE_CREDITS};
pub use gradient::{GradientBroadcaster, BroadcastError, MAX_GRADIENT_AGE_MS};
pub use messages::{EnrMessage, CREDIT_TOPIC, GRADIENT_TOPIC};

use tracing::{debug, error};
use univrs_enr::{
    core::{Credits, NodeId},
    nexus::ResourceGradient,
};

/// Unified ENR Bridge coordinator
///
/// Ties together gradient broadcasting and credit synchronization,
/// routing incoming messages to the appropriate handler.
pub struct EnrBridge {
    /// Gradient state and broadcasting
    pub gradient: GradientBroadcaster,
    /// Credit ledger and transfers
    pub credits: CreditSynchronizer,
}

impl EnrBridge {
    /// Create a new ENR bridge
    ///
    /// # Arguments
    ///
    /// * `local_node` - This node's identity
    /// * `publish_fn` - Callback to publish messages to gossipsub
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let bridge = EnrBridge::new(node_id, |topic, bytes| {
    ///     // Publish to libp2p gossipsub
    ///     swarm.behaviour_mut().gossipsub.publish(topic, bytes)
    /// });
    /// ```
    pub fn new<F>(local_node: NodeId, publish_fn: F) -> Self
    where
        F: Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync + Clone + 'static,
    {
        Self {
            gradient: GradientBroadcaster::new(local_node, publish_fn.clone()),
            credits: CreditSynchronizer::new(local_node, publish_fn),
        }
    }

    /// Handle incoming ENR message from gossip
    ///
    /// Routes message to appropriate handler based on type.
    /// Returns error only for malformed messages; application-level
    /// errors are logged but don't propagate.
    pub async fn handle_message(&self, bytes: &[u8]) -> Result<(), HandleError> {
        let msg = EnrMessage::decode(bytes).map_err(HandleError::Decode)?;

        match msg {
            EnrMessage::GradientUpdate(update) => {
                if let Err(e) = self.gradient.handle_gradient(update).await {
                    debug!("Gradient update rejected: {}", e);
                }
            }
            EnrMessage::CreditTransfer(transfer) => {
                if let Err(e) = self.credits.handle_transfer(transfer).await {
                    debug!("Credit transfer rejected: {}", e);
                }
            }
            EnrMessage::BalanceQuery(query) => {
                if let Err(e) = self.credits.handle_balance_query(query).await {
                    error!("Failed to respond to balance query: {}", e);
                }
            }
            EnrMessage::BalanceResponse(response) => {
                // TODO: Store for pending queries
                debug!(
                    request_id = response.request_id,
                    balance = response.balance.amount,
                    "Received balance response"
                );
            }
        }

        Ok(())
    }

    /// Broadcast local resource gradient to the network
    pub async fn broadcast_gradient(&self, gradient: ResourceGradient) -> Result<(), BroadcastError> {
        self.gradient.broadcast_update(gradient).await
    }

    /// Transfer credits to another node
    pub async fn transfer_credits(&self, to: NodeId, amount: Credits) -> Result<(), TransferError> {
        self.credits.transfer(to, amount).await?;
        Ok(())
    }

    /// Get local credit balance
    pub async fn local_balance(&self) -> Credits {
        self.credits.local_balance().await
    }

    /// Get aggregated network gradient view
    pub async fn network_gradient(&self) -> ResourceGradient {
        self.gradient.get_network_gradient().await
    }

    /// Get number of active nodes with fresh gradients
    pub async fn active_node_count(&self) -> usize {
        self.gradient.active_node_count().await
    }

    /// Perform maintenance (prune stale data)
    pub async fn maintenance(&self) {
        let pruned = self.gradient.prune_stale().await;
        if pruned > 0 {
            debug!(count = pruned, "Pruned stale gradients");
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HandleError {
    #[error("Failed to decode message: {0}")]
    Decode(#[from] messages::DecodeError),
}

/// Helper to get gossipsub topics for subscription
pub fn enr_topics() -> Vec<&'static str> {
    vec![GRADIENT_TOPIC, CREDIT_TOPIC]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

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
    async fn test_bridge_creation() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, _) = mock_publish();
        let bridge = EnrBridge::new(node, publish);

        // Should have initial credits
        let balance = bridge.local_balance().await;
        assert_eq!(balance.amount, INITIAL_NODE_CREDITS);
    }

    #[tokio::test]
    async fn test_gradient_broadcast_and_handle() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, counter) = mock_publish();
        let bridge1 = EnrBridge::new(node1, publish.clone());
        let bridge2 = EnrBridge::new(node2, publish);

        // Node1 broadcasts gradient
        let gradient = ResourceGradient {
            cpu_available: 0.42,
            memory_available: 0.73,
            ..Default::default()
        };
        bridge1.broadcast_gradient(gradient).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Simulate bridge2 receiving the message
        let msg = EnrMessage::GradientUpdate(messages::GradientUpdate {
            source: node1,
            gradient,
            timestamp: univrs_enr::Timestamp::now(),
            signature: vec![],
        });
        let bytes = msg.encode().unwrap();
        bridge2.handle_message(&bytes).await.unwrap();

        // Bridge2 should now see the gradient
        let net = bridge2.network_gradient().await;
        assert!((net.cpu_available - 0.42).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_credit_transfer_roundtrip() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, counter) = mock_publish();
        let bridge1 = EnrBridge::new(node1, publish.clone());
        let bridge2 = EnrBridge::new(node2, publish);

        // Transfer from node1 to node2
        bridge1.transfer_credits(node2, Credits::new(100)).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Node1 balance: 1000 - 100 - 2 (tax) = 898
        assert_eq!(bridge1.local_balance().await.amount, 898);

        // Simulate bridge2 receiving the transfer
        let transfer = univrs_enr::CreditTransfer::new(
            univrs_enr::AccountId::node_account(node1),
            univrs_enr::AccountId::node_account(node2),
            Credits::new(100),
            Credits::new(2),
        );
        let msg = EnrMessage::CreditTransfer(messages::CreditTransferMsg {
            transfer,
            nonce: 1,
            signature: vec![],
        });
        let bytes = msg.encode().unwrap();
        bridge2.handle_message(&bytes).await.unwrap();

        // Node2 balance: 1000 + 100 = 1100
        assert_eq!(bridge2.local_balance().await.amount, 1100);
    }

    #[tokio::test]
    async fn test_malformed_message() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, _) = mock_publish();
        let bridge = EnrBridge::new(node, publish);

        // Random bytes should fail to decode
        let result = bridge.handle_message(&[0xFF, 0xFF, 0xFF]).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_enr_topics() {
        let topics = enr_topics();
        assert!(topics.contains(&GRADIENT_TOPIC));
        assert!(topics.contains(&CREDIT_TOPIC));
    }
}
