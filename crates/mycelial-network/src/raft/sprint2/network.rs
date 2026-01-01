//! Gossipsub-based Raft network transport

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use openraft::{
    error::{InstallSnapshotError, RPCError, RaftError as OpenRaftError, RemoteError},
    network::{RPCOption, RaftNetwork, RaftNetworkFactory},
    raft::{
        AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest,
        InstallSnapshotResponse, VoteRequest, VoteResponse,
    },
    BasicNode,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use super::types::CreditTypeConfig;
use super::PublishFn;

/// Gossipsub topic for Raft protocol messages
pub const RAFT_TOPIC: &str = "/vudo/enr/raft/1.0.0";

/// Raft protocol message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftMessage {
    /// AppendEntries RPC
    AppendEntries(AppendEntriesRequest<CreditTypeConfig>),
    /// AppendEntries response
    AppendEntriesResponse(AppendEntriesResponse<u64>),
    /// Vote request
    Vote(VoteRequest<u64>),
    /// Vote response
    VoteResponse(VoteResponse<u64>),
    /// Install snapshot request
    InstallSnapshot(InstallSnapshotRequest<CreditTypeConfig>),
    /// Install snapshot response
    InstallSnapshotResponse(InstallSnapshotResponse<u64>),
}

impl RaftMessage {
    /// Encode message to bytes
    pub fn encode(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Decode message from bytes
    pub fn decode(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

/// Gossipsub-based Raft network transport
///
/// Uses the existing gossipsub infrastructure to send Raft messages.
/// In Phase 1, this is a simplified implementation that broadcasts to all nodes.
/// In Phase 2, we'll add targeted messaging.
pub struct GossipsubRaftNetwork {
    /// Callback to publish to gossipsub
    publish_fn: PublishFn,
    /// Pending responses (request_id -> response)
    pending: Arc<RwLock<std::collections::HashMap<u64, RaftMessage>>>,
    /// Next request ID
    next_request_id: Arc<RwLock<u64>>,
}

impl GossipsubRaftNetwork {
    /// Create a new gossipsub Raft network
    pub fn new(publish_fn: PublishFn) -> Self {
        Self {
            publish_fn,
            pending: Arc::new(RwLock::new(std::collections::HashMap::new())),
            next_request_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Handle incoming Raft message
    pub async fn handle_message(&self, bytes: &[u8]) -> Result<Option<RaftMessage>, String> {
        let msg = RaftMessage::decode(bytes).map_err(|e| e.to_string())?;

        // Store responses for pending requests
        match &msg {
            RaftMessage::AppendEntriesResponse(_)
            | RaftMessage::VoteResponse(_)
            | RaftMessage::InstallSnapshotResponse(_) => {
                // TODO: Route to pending request
                debug!(?msg, "Received Raft response");
                Ok(Some(msg))
            }
            _ => Ok(Some(msg)),
        }
    }

    /// Publish a Raft message
    async fn publish(&self, msg: RaftMessage) -> Result<(), String> {
        let bytes = msg.encode().map_err(|e| e.to_string())?;
        (self.publish_fn)(RAFT_TOPIC.to_string(), bytes)
    }
}

/// Network factory for creating connections to peers
pub struct GossipsubRaftNetworkFactory {
    network: Arc<GossipsubRaftNetwork>,
}

impl GossipsubRaftNetworkFactory {
    pub fn new(network: Arc<GossipsubRaftNetwork>) -> Self {
        Self { network }
    }
}

#[async_trait]
impl RaftNetworkFactory<CreditTypeConfig> for GossipsubRaftNetworkFactory {
    type Network = GossipsubRaftNetworkConnection;

    async fn new_client(&mut self, target: u64, _node: &BasicNode) -> Self::Network {
        GossipsubRaftNetworkConnection {
            target,
            network: self.network.clone(),
        }
    }
}

/// A connection to a single Raft peer
pub struct GossipsubRaftNetworkConnection {
    target: u64,
    network: Arc<GossipsubRaftNetwork>,
}

#[async_trait]
impl RaftNetwork<CreditTypeConfig> for GossipsubRaftNetworkConnection {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<CreditTypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<u64>, RPCError<u64, BasicNode, OpenRaftError<u64>>> {
        debug!(
            target = self.target,
            entries = rpc.entries.len(),
            "Sending AppendEntries"
        );

        self.network
            .publish(RaftMessage::AppendEntries(rpc))
            .await
            .map_err(|e| RPCError::Network(openraft::error::NetworkError::new(&e)))?;

        // TODO: Wait for response with timeout
        // For now, return a simulated success response
        // This will be improved in Sprint 2
        Ok(AppendEntriesResponse {
            vote: rpc.vote,
            success: true,
            conflict: None,
        })
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<u64>,
        _option: RPCOption,
    ) -> Result<VoteResponse<u64>, RPCError<u64, BasicNode, OpenRaftError<u64>>> {
        debug!(
            target = self.target,
            candidate = rpc.vote.leader_id.node_id,
            "Sending Vote"
        );

        self.network
            .publish(RaftMessage::Vote(rpc.clone()))
            .await
            .map_err(|e| RPCError::Network(openraft::error::NetworkError::new(&e)))?;

        // TODO: Wait for response with timeout
        // For now, grant vote (will be improved in Sprint 2)
        Ok(VoteResponse {
            vote: rpc.vote,
            vote_granted: true,
            last_log_id: None,
        })
    }

    async fn install_snapshot(
        &mut self,
        rpc: InstallSnapshotRequest<CreditTypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<u64>,
        RPCError<u64, BasicNode, OpenRaftError<u64, InstallSnapshotError>>,
    > {
        debug!(
            target = self.target,
            snapshot_id = %rpc.meta.snapshot_id,
            "Sending InstallSnapshot"
        );

        self.network
            .publish(RaftMessage::InstallSnapshot(rpc.clone()))
            .await
            .map_err(|e| RPCError::Network(openraft::error::NetworkError::new(&e)))?;

        // TODO: Wait for response with timeout
        Ok(InstallSnapshotResponse { vote: rpc.vote })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_message_roundtrip() {
        let msg = RaftMessage::VoteResponse(VoteResponse {
            vote: openraft::Vote::new(1, 42),
            vote_granted: true,
            last_log_id: None,
        });

        let bytes = msg.encode().unwrap();
        let decoded = RaftMessage::decode(&bytes).unwrap();

        match decoded {
            RaftMessage::VoteResponse(resp) => {
                assert!(resp.vote_granted);
                assert_eq!(resp.vote.leader_id().node_id, 42);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[tokio::test]
    async fn test_network_publish() {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        let publish_fn = Box::new(move |_topic: String, _bytes: Vec<u8>| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        let network = GossipsubRaftNetwork::new(publish_fn);

        network
            .publish(RaftMessage::VoteResponse(VoteResponse {
                vote: openraft::Vote::new(1, 42),
                vote_granted: true,
                last_log_id: None,
            }))
            .await
            .unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
