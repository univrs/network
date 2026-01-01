//! Septal Gate Manager - Circuit Breaker for P2P Network
//!
//! Manages distributed circuit breakers (septal gates) across the network.
//! When a node becomes unhealthy, its gate closes and Woronin bodies
//! block transactions to/from that node.
//!
//! ## State Machine
//!
//! ```text
//! Open ──[failures exceed threshold]──► Closed
//!   ▲                                      │
//!   │                                      │
//!   └──[recovery test passes]── HalfOpen ◄─┘
//!                                  │        [timeout]
//!                                  │
//!                                  └──[recovery test fails]──► Closed
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use mycelial_network::enr_bridge::SeptalGateManager;
//!
//! let manager = SeptalGateManager::new(local_node, |topic, bytes| {
//!     swarm.behaviour_mut().gossipsub.publish(topic, bytes)
//! });
//!
//! // Record a failure for a peer
//! manager.record_failure(peer_id, "Connection timeout").await;
//!
//! // Check if traffic is allowed
//! if manager.allows_traffic(&peer_id).await {
//!     // Proceed with transaction
//! }
//! ```

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};
use univrs_enr::{
    core::{NodeId, Timestamp},
    septal::{
        RecoveryResult, SeptalGate, SeptalGateConfig, SeptalGateState,
        SeptalGateTransition, WoroninManager, FAILURE_THRESHOLD,
    },
};

use super::messages::{
    EnrMessage, SeptalHealthProbe, SeptalHealthResponse, SeptalMessage, SeptalStateMsg,
    SEPTAL_TOPIC,
};

/// Publish function type for gossipsub
type PublishFn = Arc<dyn Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync>;

/// Distributed septal gate manager
///
/// Tracks gate states for all known peers and synchronizes
/// state changes via gossipsub.
pub struct SeptalGateManager {
    /// Local node identity
    local_node: NodeId,
    /// Gates for each peer node
    gates: Arc<RwLock<HashMap<NodeId, SeptalGate>>>,
    /// Woronin body manager for blocking isolated nodes
    woronin: Arc<RwLock<WoroninManager>>,
    /// Gate configuration
    config: Arc<RwLock<SeptalGateConfig>>,
    /// Recent state transitions for observability
    transitions: Arc<RwLock<Vec<SeptalGateTransition>>>,
    /// Gossipsub publish callback
    publish_fn: PublishFn,
}

impl SeptalGateManager {
    /// Create a new septal gate manager
    pub fn new<F>(local_node: NodeId, publish_fn: F) -> Self
    where
        F: Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync + 'static,
    {
        Self {
            local_node,
            gates: Arc::new(RwLock::new(HashMap::new())),
            woronin: Arc::new(RwLock::new(WoroninManager::new())),
            config: Arc::new(RwLock::new(SeptalGateConfig::default())),
            transitions: Arc::new(RwLock::new(Vec::new())),
            publish_fn: Arc::new(publish_fn),
        }
    }

    /// Record a failure for a peer node
    ///
    /// If failures exceed threshold, the gate closes and
    /// Woronin body is activated to block traffic.
    pub async fn record_failure(&self, peer: NodeId, reason: &str) -> Option<SeptalGateTransition> {
        let mut gates = self.gates.write();
        let gate = gates.entry(peer).or_insert_with(|| SeptalGate::new(peer));

        gate.record_failure();
        debug!(
            peer = %peer,
            failures = gate.failure_count,
            threshold = FAILURE_THRESHOLD,
            "Recorded failure for peer"
        );

        if gate.should_trip() && gate.state.is_open() {
            let transition = SeptalGateTransition {
                from_state: SeptalGateState::Open,
                to_state: SeptalGateState::Closed,
                reason: format!("{} (failures: {})", reason, gate.failure_count),
                timestamp: Timestamp::now(),
            };

            gate.trip();

            // Activate Woronin body
            {
                let mut woronin = self.woronin.write();
                woronin.activate(peer, &transition.reason);
            }

            // Record transition
            {
                let mut transitions = self.transitions.write();
                transitions.push(transition.clone());
                // Keep last 100 transitions
                if transitions.len() > 100 {
                    transitions.remove(0);
                }
            }

            info!(
                peer = %peer,
                reason = %transition.reason,
                "Gate closed - peer isolated"
            );

            // Broadcast state change
            self.broadcast_state_change(peer, &transition).await;

            return Some(transition);
        }

        None
    }

    /// Record a success for a peer node (resets failure count)
    pub async fn record_success(&self, peer: NodeId) {
        let mut gates = self.gates.write();
        if let Some(gate) = gates.get_mut(&peer) {
            gate.record_success();
        }
    }

    /// Check if traffic is allowed to/from a peer
    pub async fn allows_traffic(&self, peer: &NodeId) -> bool {
        let gates = self.gates.read();
        match gates.get(peer) {
            Some(gate) => gate.state.allows_traffic(),
            None => true, // Unknown peers are allowed by default
        }
    }

    /// Check if a peer is isolated
    pub async fn is_isolated(&self, peer: &NodeId) -> bool {
        let woronin = self.woronin.read();
        woronin.is_isolated(peer)
    }

    /// Check if a transaction should be blocked
    pub async fn should_block_transaction(&self, from: &NodeId, to: &NodeId) -> bool {
        let woronin = self.woronin.read();
        woronin.should_block(from, to)
    }

    /// Get the current state of a peer's gate
    pub async fn get_gate_state(&self, peer: &NodeId) -> SeptalGateState {
        let gates = self.gates.read();
        gates
            .get(peer)
            .map(|g| g.state)
            .unwrap_or(SeptalGateState::Open)
    }

    /// Get all isolated nodes
    pub async fn isolated_nodes(&self) -> Vec<NodeId> {
        let woronin = self.woronin.read();
        woronin.isolated_nodes()
    }

    /// Get gate statistics
    pub async fn stats(&self) -> SeptalStats {
        let gates = self.gates.read();
        let woronin = self.woronin.read();

        let mut open = 0;
        let mut half_open = 0;
        let mut closed = 0;

        for gate in gates.values() {
            match gate.state {
                SeptalGateState::Open => open += 1,
                SeptalGateState::HalfOpen => half_open += 1,
                SeptalGateState::Closed => closed += 1,
            }
        }

        SeptalStats {
            total_gates: gates.len(),
            open_gates: open,
            half_open_gates: half_open,
            closed_gates: closed,
            isolated_nodes: woronin.isolated_nodes().len(),
        }
    }

    /// Attempt recovery for isolated nodes
    ///
    /// Should be called periodically. Transitions closed gates
    /// to half-open after timeout, and tests recovery.
    pub async fn attempt_recoveries(&self) -> Vec<RecoveryResult> {
        let mut results = Vec::new();
        let mut gates = self.gates.write();
        let mut woronin = self.woronin.write();
        let config = self.config.read().clone();

        for (node_id, gate) in gates.iter_mut() {
            let result = self.try_recovery(gate, &mut woronin, &config).await;
            if result != RecoveryResult::NotNeeded && result != RecoveryResult::TooSoon {
                debug!(
                    peer = %node_id,
                    result = ?result,
                    "Recovery attempt result"
                );
                results.push(result);
            }
        }

        results
    }

    /// Try recovery for a single gate
    async fn try_recovery(
        &self,
        gate: &mut SeptalGate,
        woronin: &mut WoroninManager,
        _config: &SeptalGateConfig,
    ) -> RecoveryResult {
        match gate.state {
            SeptalGateState::Open => RecoveryResult::NotNeeded,
            SeptalGateState::Closed => {
                // Check if timeout elapsed
                if gate.attempt_half_open() {
                    let transition = SeptalGateTransition {
                        from_state: SeptalGateState::Closed,
                        to_state: SeptalGateState::HalfOpen,
                        reason: "Recovery timeout elapsed".to_string(),
                        timestamp: Timestamp::now(),
                    };

                    info!(
                        peer = %gate.node,
                        "Gate entering half-open state for recovery test"
                    );

                    // Store transition and broadcast
                    {
                        let mut transitions = self.transitions.write();
                        transitions.push(transition.clone());
                    }

                    // Broadcast asynchronously (fire and forget for now)
                    let node = gate.node;
                    let publish_fn = self.publish_fn.clone();
                    let msg = EnrMessage::Septal(SeptalMessage::StateChange(SeptalStateMsg {
                        node,
                        from_state: transition.from_state,
                        to_state: transition.to_state,
                        reason: transition.reason.clone(),
                        timestamp: transition.timestamp,
                    }));

                    if let Ok(bytes) = msg.encode() {
                        let _ = publish_fn(SEPTAL_TOPIC.to_string(), bytes);
                    }

                    RecoveryResult::EnteredHalfOpen
                } else {
                    RecoveryResult::StillClosed
                }
            }
            SeptalGateState::HalfOpen => {
                // For now, use a simple health check based on no recent failures
                // In production, this would ping the node or check metrics
                let healthy = gate.failure_count == 0;

                if healthy {
                    gate.recover();
                    woronin.deactivate(&gate.node);

                    let transition = SeptalGateTransition {
                        from_state: SeptalGateState::HalfOpen,
                        to_state: SeptalGateState::Open,
                        reason: "Recovery test passed".to_string(),
                        timestamp: Timestamp::now(),
                    };

                    info!(
                        peer = %gate.node,
                        "Gate recovered - peer no longer isolated"
                    );

                    {
                        let mut transitions = self.transitions.write();
                        transitions.push(transition.clone());
                    }

                    // Broadcast recovery
                    let node = gate.node;
                    let publish_fn = self.publish_fn.clone();
                    let msg = EnrMessage::Septal(SeptalMessage::StateChange(SeptalStateMsg {
                        node,
                        from_state: transition.from_state,
                        to_state: transition.to_state,
                        reason: transition.reason.clone(),
                        timestamp: transition.timestamp,
                    }));

                    if let Ok(bytes) = msg.encode() {
                        let _ = publish_fn(SEPTAL_TOPIC.to_string(), bytes);
                    }

                    RecoveryResult::Recovered
                } else {
                    gate.fail_recovery();

                    let transition = SeptalGateTransition {
                        from_state: SeptalGateState::HalfOpen,
                        to_state: SeptalGateState::Closed,
                        reason: "Recovery test failed".to_string(),
                        timestamp: Timestamp::now(),
                    };

                    warn!(
                        peer = %gate.node,
                        "Recovery test failed - peer remains isolated"
                    );

                    {
                        let mut transitions = self.transitions.write();
                        transitions.push(transition.clone());
                    }

                    RecoveryResult::RecoveryFailed
                }
            }
        }
    }

    /// Handle incoming septal message from gossip
    pub async fn handle_message(&self, msg: SeptalMessage) -> Result<(), SeptalError> {
        match msg {
            SeptalMessage::StateChange(state_msg) => {
                self.handle_state_change(state_msg).await
            }
            SeptalMessage::HealthProbe(probe) => {
                self.handle_health_probe(probe).await
            }
            SeptalMessage::HealthResponse(response) => {
                self.handle_health_response(response).await
            }
        }
    }

    /// Handle state change from another node
    async fn handle_state_change(&self, msg: SeptalStateMsg) -> Result<(), SeptalError> {
        let mut gates = self.gates.write();
        let gate = gates.entry(msg.node).or_insert_with(|| SeptalGate::new(msg.node));

        // Apply the state change
        gate.state = msg.to_state;

        // Update Woronin body
        let mut woronin = self.woronin.write();
        match msg.to_state {
            SeptalGateState::Closed => {
                if !woronin.is_isolated(&msg.node) {
                    woronin.activate(msg.node, &msg.reason);
                }
            }
            SeptalGateState::Open => {
                woronin.deactivate(&msg.node);
            }
            SeptalGateState::HalfOpen => {
                // Keep Woronin active during half-open
            }
        }

        debug!(
            peer = %msg.node,
            from = ?msg.from_state,
            to = ?msg.to_state,
            reason = %msg.reason,
            "Applied remote state change"
        );

        Ok(())
    }

    /// Handle health probe request
    async fn handle_health_probe(&self, probe: SeptalHealthProbe) -> Result<(), SeptalError> {
        // Respond with our health status
        let response = SeptalHealthResponse {
            request_id: probe.request_id,
            node: self.local_node,
            is_healthy: true, // We're healthy if we can respond
            failure_count: 0,
            timestamp: Timestamp::now(),
        };

        let msg = EnrMessage::Septal(SeptalMessage::HealthResponse(response));
        let bytes = msg.encode().map_err(|_| SeptalError::EncodeFailed)?;
        (self.publish_fn)(SEPTAL_TOPIC.to_string(), bytes)
            .map_err(SeptalError::PublishFailed)?;

        Ok(())
    }

    /// Handle health response
    async fn handle_health_response(&self, response: SeptalHealthResponse) -> Result<(), SeptalError> {
        if response.is_healthy {
            // Reset failure count for healthy peer
            let mut gates = self.gates.write();
            if let Some(gate) = gates.get_mut(&response.node) {
                gate.failure_count = response.failure_count;
            }
        }

        debug!(
            peer = %response.node,
            healthy = response.is_healthy,
            "Received health response"
        );

        Ok(())
    }

    /// Broadcast a state change to the network
    async fn broadcast_state_change(&self, node: NodeId, transition: &SeptalGateTransition) {
        let msg = EnrMessage::Septal(SeptalMessage::StateChange(SeptalStateMsg {
            node,
            from_state: transition.from_state,
            to_state: transition.to_state,
            reason: transition.reason.clone(),
            timestamp: transition.timestamp,
        }));

        match msg.encode() {
            Ok(bytes) => {
                if let Err(e) = (self.publish_fn)(SEPTAL_TOPIC.to_string(), bytes) {
                    warn!(error = %e, "Failed to broadcast state change");
                }
            }
            Err(e) => {
                warn!(error = ?e, "Failed to encode state change");
            }
        }
    }

    /// Send health probe to a peer
    pub async fn probe_health(&self, peer: NodeId) -> Result<(), SeptalError> {
        let probe = SeptalHealthProbe {
            request_id: rand::random(),
            target: peer,
            timestamp: Timestamp::now(),
        };

        let msg = EnrMessage::Septal(SeptalMessage::HealthProbe(probe));
        let bytes = msg.encode().map_err(|_| SeptalError::EncodeFailed)?;
        (self.publish_fn)(SEPTAL_TOPIC.to_string(), bytes)
            .map_err(SeptalError::PublishFailed)?;

        Ok(())
    }

    /// Get recent transitions for observability
    pub async fn recent_transitions(&self) -> Vec<SeptalGateTransition> {
        self.transitions.read().clone()
    }

    /// Update gate configuration
    pub async fn set_config(&self, config: SeptalGateConfig) {
        if config.is_valid() {
            *self.config.write() = config;
        }
    }

    /// Get current configuration
    pub async fn get_config(&self) -> SeptalGateConfig {
        self.config.read().clone()
    }
}

/// Septal gate statistics
#[derive(Debug, Clone, Default)]
pub struct SeptalStats {
    pub total_gates: usize,
    pub open_gates: usize,
    pub half_open_gates: usize,
    pub closed_gates: usize,
    pub isolated_nodes: usize,
}

/// Septal gate errors
#[derive(Debug, thiserror::Error)]
pub enum SeptalError {
    #[error("Failed to encode message")]
    EncodeFailed,
    #[error("Failed to publish message: {0}")]
    PublishFailed(String),
    #[error("Invalid configuration")]
    InvalidConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn mock_publish() -> (impl Fn(String, Vec<u8>) -> Result<(), String> + Clone, Arc<AtomicUsize>)
    {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        let f = move |_topic: String, _bytes: Vec<u8>| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        };
        (f, counter)
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, _) = mock_publish();
        let manager = SeptalGateManager::new(node, publish);

        let stats = manager.stats().await;
        assert_eq!(stats.total_gates, 0);
        assert_eq!(stats.isolated_nodes, 0);
    }

    #[tokio::test]
    async fn test_record_failure_under_threshold() {
        let node = NodeId::from_bytes([1u8; 32]);
        let peer = NodeId::from_bytes([2u8; 32]);
        let (publish, counter) = mock_publish();
        let manager = SeptalGateManager::new(node, publish);

        // Record failures under threshold
        for _ in 0..(FAILURE_THRESHOLD - 1) {
            let transition = manager.record_failure(peer, "test failure").await;
            assert!(transition.is_none());
        }

        // Gate should still be open
        assert!(manager.allows_traffic(&peer).await);
        assert!(!manager.is_isolated(&peer).await);
        assert_eq!(counter.load(Ordering::SeqCst), 0); // No broadcasts yet
    }

    #[tokio::test]
    async fn test_record_failure_trips_gate() {
        let node = NodeId::from_bytes([1u8; 32]);
        let peer = NodeId::from_bytes([2u8; 32]);
        let (publish, counter) = mock_publish();
        let manager = SeptalGateManager::new(node, publish);

        // Record failures to reach threshold
        for i in 0..FAILURE_THRESHOLD {
            let transition = manager.record_failure(peer, "test failure").await;
            if i == FAILURE_THRESHOLD - 1 {
                assert!(transition.is_some());
                let t = transition.unwrap();
                assert_eq!(t.from_state, SeptalGateState::Open);
                assert_eq!(t.to_state, SeptalGateState::Closed);
            }
        }

        // Gate should be closed
        assert!(!manager.allows_traffic(&peer).await);
        assert!(manager.is_isolated(&peer).await);
        assert_eq!(counter.load(Ordering::SeqCst), 1); // One broadcast

        let stats = manager.stats().await;
        assert_eq!(stats.closed_gates, 1);
        assert_eq!(stats.isolated_nodes, 1);
    }

    #[tokio::test]
    async fn test_record_success_resets_failures() {
        let node = NodeId::from_bytes([1u8; 32]);
        let peer = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();
        let manager = SeptalGateManager::new(node, publish);

        // Record some failures
        for _ in 0..3 {
            manager.record_failure(peer, "test").await;
        }

        // Record success
        manager.record_success(peer).await;

        // Should need full threshold again to trip
        for _ in 0..(FAILURE_THRESHOLD - 1) {
            let transition = manager.record_failure(peer, "test").await;
            assert!(transition.is_none());
        }
    }

    #[tokio::test]
    async fn test_should_block_transaction() {
        let node = NodeId::from_bytes([1u8; 32]);
        let peer1 = NodeId::from_bytes([2u8; 32]);
        let peer2 = NodeId::from_bytes([3u8; 32]);
        let (publish, _) = mock_publish();
        let manager = SeptalGateManager::new(node, publish);

        // Initially no blocking
        assert!(!manager.should_block_transaction(&peer1, &peer2).await);

        // Isolate peer1
        for _ in 0..FAILURE_THRESHOLD {
            manager.record_failure(peer1, "test").await;
        }

        // Transactions involving peer1 should be blocked
        assert!(manager.should_block_transaction(&peer1, &peer2).await);
        assert!(manager.should_block_transaction(&peer2, &peer1).await);

        // Transactions between non-isolated peers are fine
        assert!(!manager.should_block_transaction(&peer2, &node).await);
    }

    #[tokio::test]
    async fn test_handle_state_change() {
        let node = NodeId::from_bytes([1u8; 32]);
        let peer = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();
        let manager = SeptalGateManager::new(node, publish);

        // Receive state change from network
        let msg = SeptalStateMsg {
            node: peer,
            from_state: SeptalGateState::Open,
            to_state: SeptalGateState::Closed,
            reason: "Remote failure".to_string(),
            timestamp: Timestamp::now(),
        };

        manager
            .handle_message(SeptalMessage::StateChange(msg))
            .await
            .unwrap();

        // Should be isolated now
        assert!(manager.is_isolated(&peer).await);
        assert_eq!(
            manager.get_gate_state(&peer).await,
            SeptalGateState::Closed
        );
    }

    #[tokio::test]
    async fn test_stats() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, _) = mock_publish();
        let manager = SeptalGateManager::new(node, publish);

        // Create gates in different states
        let peer1 = NodeId::from_bytes([2u8; 32]);
        let peer2 = NodeId::from_bytes([3u8; 32]);

        // Record success for peer1 (creates open gate)
        manager.record_failure(peer1, "test").await;
        manager.record_success(peer1).await;

        // Trip gate for peer2
        for _ in 0..FAILURE_THRESHOLD {
            manager.record_failure(peer2, "test").await;
        }

        let stats = manager.stats().await;
        assert_eq!(stats.total_gates, 2);
        assert_eq!(stats.open_gates, 1);
        assert_eq!(stats.closed_gates, 1);
        assert_eq!(stats.isolated_nodes, 1);
    }

    #[tokio::test]
    async fn test_config_validation() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, _) = mock_publish();
        let manager = SeptalGateManager::new(node, publish);

        // Default config should be valid
        let config = manager.get_config().await;
        assert!(config.is_valid());

        // Invalid config (weights don't sum to 1)
        let invalid = SeptalGateConfig {
            timeout_weight: 0.5,
            credit_default_weight: 0.5,
            reputation_weight: 0.5,
            ..Default::default()
        };

        // Should not apply invalid config
        manager.set_config(invalid).await;
        let config = manager.get_config().await;
        assert!(config.is_valid()); // Still has valid config
    }
}
