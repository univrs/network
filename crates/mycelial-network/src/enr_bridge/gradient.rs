//! Gradient Broadcasting via Gossipsub
//!
//! Broadcasts local resource availability and aggregates
//! gradients from other nodes in the network.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use univrs_enr::{
    core::{NodeId, Timestamp},
    nexus::ResourceGradient,
};

use crate::enr_bridge::messages::{EnrMessage, GradientUpdate, GRADIENT_TOPIC};

/// Maximum age of gradient before considered stale (15 seconds)
pub const MAX_GRADIENT_AGE_MS: u64 = 15_000;

/// Maximum clock drift tolerance (5 seconds into future)
pub const MAX_FUTURE_TOLERANCE_MS: u64 = 5_000;

/// Callback type for publishing to gossipsub
pub type PublishFn = Box<dyn Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync>;

/// Manages gradient state and broadcasting
pub struct GradientBroadcaster {
    /// This node's ID
    local_node: NodeId,
    /// Received gradients from other nodes
    gradients: Arc<RwLock<HashMap<NodeId, GradientUpdate>>>,
    /// Callback to publish to gossipsub
    publish_fn: PublishFn,
}

impl GradientBroadcaster {
    /// Create a new gradient broadcaster
    pub fn new<F>(local_node: NodeId, publish_fn: F) -> Self
    where
        F: Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync + 'static,
    {
        Self {
            local_node,
            gradients: Arc::new(RwLock::new(HashMap::new())),
            publish_fn: Box::new(publish_fn),
        }
    }

    /// Broadcast local gradient to network
    pub async fn broadcast_update(&self, gradient: ResourceGradient) -> Result<(), BroadcastError> {
        // Validate gradient
        if !gradient.is_valid() {
            return Err(BroadcastError::InvalidGradient);
        }

        let update = GradientUpdate {
            source: self.local_node,
            gradient,
            timestamp: Timestamp::now(),
            signature: vec![], // TODO: Sign with Ed25519
        };

        let msg = EnrMessage::GradientUpdate(update);
        let bytes = msg.encode().map_err(BroadcastError::Encode)?;

        (self.publish_fn)(GRADIENT_TOPIC.to_string(), bytes).map_err(BroadcastError::Publish)?;

        debug!(
            cpu = %gradient.cpu_available,
            memory = %gradient.memory_available,
            "Broadcast gradient update"
        );

        Ok(())
    }

    /// Handle incoming gradient from gossip
    pub async fn handle_gradient(&self, update: GradientUpdate) -> Result<(), HandleError> {
        let now = Timestamp::now();

        // Reject gradients from the future (with tolerance for clock drift)
        if update.timestamp.millis > now.millis + MAX_FUTURE_TOLERANCE_MS {
            warn!(
                source = %update.source,
                timestamp = update.timestamp.millis,
                "Rejecting gradient with future timestamp"
            );
            return Err(HandleError::FutureTimestamp);
        }

        // Reject very old gradients
        if now.millis.saturating_sub(update.timestamp.millis) > MAX_GRADIENT_AGE_MS * 2 {
            return Err(HandleError::TooOld);
        }

        // TODO: Verify signature
        // if !verify_signature(&update) {
        //     return Err(HandleError::InvalidSignature);
        // }

        let mut gradients = self.gradients.write().await;

        // Only update if newer than existing
        if let Some(existing) = gradients.get(&update.source) {
            if existing.timestamp.millis >= update.timestamp.millis {
                debug!(source = %update.source, "Ignoring older gradient update");
                return Ok(()); // Ignore older update
            }
        }

        debug!(
            source = %update.source,
            cpu = %update.gradient.cpu_available,
            "Received gradient update"
        );

        gradients.insert(update.source, update);
        Ok(())
    }

    /// Get aggregated view of network gradients
    pub async fn get_network_gradient(&self) -> ResourceGradient {
        let gradients = self.gradients.read().await;
        let now = Timestamp::now();

        // Filter stale gradients and collect fresh ones
        let fresh: Vec<&GradientUpdate> = gradients
            .values()
            .filter(|g| now.millis.saturating_sub(g.timestamp.millis) < MAX_GRADIENT_AGE_MS)
            .collect();

        if fresh.is_empty() {
            return ResourceGradient::zero();
        }

        // Simple average aggregation
        // TODO: Weight by reputation or stake
        let count = fresh.len() as f64;
        ResourceGradient {
            cpu_available: fresh.iter().map(|g| g.gradient.cpu_available).sum::<f64>() / count,
            memory_available: fresh
                .iter()
                .map(|g| g.gradient.memory_available)
                .sum::<f64>()
                / count,
            gpu_available: fresh.iter().map(|g| g.gradient.gpu_available).sum::<f64>() / count,
            storage_available: fresh
                .iter()
                .map(|g| g.gradient.storage_available)
                .sum::<f64>()
                / count,
            bandwidth_available: fresh
                .iter()
                .map(|g| g.gradient.bandwidth_available)
                .sum::<f64>()
                / count,
            credit_balance: fresh.iter().map(|g| g.gradient.credit_balance).sum::<f64>() / count,
        }
    }

    /// Get gradient for a specific node
    pub async fn get_node_gradient(&self, node: &NodeId) -> Option<ResourceGradient> {
        let gradients = self.gradients.read().await;
        let now = Timestamp::now();

        gradients.get(node).and_then(|g| {
            if now.millis.saturating_sub(g.timestamp.millis) < MAX_GRADIENT_AGE_MS {
                Some(g.gradient)
            } else {
                None
            }
        })
    }

    /// Get count of nodes with fresh gradients
    pub async fn active_node_count(&self) -> usize {
        let gradients = self.gradients.read().await;
        let now = Timestamp::now();

        gradients
            .values()
            .filter(|g| now.millis.saturating_sub(g.timestamp.millis) < MAX_GRADIENT_AGE_MS)
            .count()
    }

    /// Prune stale gradients to free memory
    pub async fn prune_stale(&self) -> usize {
        let mut gradients = self.gradients.write().await;
        let now = Timestamp::now();
        let before_count = gradients.len();

        gradients
            .retain(|_, g| now.millis.saturating_sub(g.timestamp.millis) < MAX_GRADIENT_AGE_MS * 2);

        before_count - gradients.len()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    #[error("Invalid gradient values")]
    InvalidGradient,
    #[error("Encoding error: {0}")]
    Encode(#[from] crate::enr_bridge::messages::EncodeError),
    #[error("Publish error: {0}")]
    Publish(String),
}

#[derive(Debug, thiserror::Error)]
pub enum HandleError {
    #[error("Gradient timestamp is in the future")]
    FutureTimestamp,
    #[error("Gradient is too old")]
    TooOld,
    #[error("Invalid signature")]
    InvalidSignature,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
    async fn test_broadcast_gradient() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, counter) = mock_publish();
        let broadcaster = GradientBroadcaster::new(node, publish);

        let gradient = ResourceGradient {
            cpu_available: 0.5,
            memory_available: 0.6,
            gpu_available: 0.0,
            storage_available: 0.8,
            bandwidth_available: 0.9,
            credit_balance: 1000.0,
        };

        broadcaster.broadcast_update(gradient).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_handle_gradient() {
        let local = NodeId::from_bytes([1u8; 32]);
        let remote = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();
        let broadcaster = GradientBroadcaster::new(local, publish);

        let update = GradientUpdate {
            source: remote,
            gradient: ResourceGradient {
                cpu_available: 0.42,
                memory_available: 0.73,
                ..Default::default()
            },
            timestamp: Timestamp::now(),
            signature: vec![],
        };

        broadcaster.handle_gradient(update).await.unwrap();

        let net_gradient = broadcaster.get_network_gradient().await;
        assert!((net_gradient.cpu_available - 0.42).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_reject_future_timestamp() {
        let local = NodeId::from_bytes([1u8; 32]);
        let remote = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();
        let broadcaster = GradientBroadcaster::new(local, publish);

        let update = GradientUpdate {
            source: remote,
            gradient: ResourceGradient::default(),
            timestamp: Timestamp::new(Timestamp::now().millis + 60_000), // 1 minute in future
            signature: vec![],
        };

        let result = broadcaster.handle_gradient(update).await;
        assert!(matches!(result, Err(HandleError::FutureTimestamp)));
    }

    #[tokio::test]
    async fn test_aggregation() {
        let local = NodeId::from_bytes([0u8; 32]);
        let (publish, _) = mock_publish();
        let broadcaster = GradientBroadcaster::new(local, publish);

        // Add gradients from 2 nodes
        for i in 1..=2u8 {
            let update = GradientUpdate {
                source: NodeId::from_bytes([i; 32]),
                gradient: ResourceGradient {
                    cpu_available: i as f64 * 0.3,
                    ..Default::default()
                },
                timestamp: Timestamp::now(),
                signature: vec![],
            };
            broadcaster.handle_gradient(update).await.unwrap();
        }

        let net = broadcaster.get_network_gradient().await;
        // Average of 0.3 and 0.6 = 0.45
        assert!((net.cpu_available - 0.45).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_only_keeps_newer() {
        let local = NodeId::from_bytes([1u8; 32]);
        let remote = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();
        let broadcaster = GradientBroadcaster::new(local, publish);

        let now = Timestamp::now();

        // First update (recent timestamp)
        let update1 = GradientUpdate {
            source: remote,
            gradient: ResourceGradient {
                cpu_available: 0.5,
                ..Default::default()
            },
            timestamp: Timestamp::new(now.millis - 1000), // 1 second ago
            signature: vec![],
        };
        broadcaster.handle_gradient(update1).await.unwrap();

        // Older update should be ignored
        let update2 = GradientUpdate {
            source: remote,
            gradient: ResourceGradient {
                cpu_available: 0.1,
                ..Default::default()
            },
            timestamp: Timestamp::new(now.millis - 2000), // 2 seconds ago (older)
            signature: vec![],
        };
        // This should succeed but the older timestamp should be ignored
        broadcaster.handle_gradient(update2).await.unwrap();

        let grad = broadcaster.get_node_gradient(&remote).await;
        // Should still have 0.5, not 0.1 (newer value preserved)
        assert!(grad.is_some());
        assert!((grad.unwrap().cpu_available - 0.5).abs() < 0.001);
    }
}
