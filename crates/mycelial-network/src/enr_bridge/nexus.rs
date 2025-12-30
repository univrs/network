//! Distributed Nexus Election via Gossipsub
//!
//! Implements distributed consensus for nexus election across the P2P network.
//! Nodes broadcast candidacy, collect votes, and agree on a winner.
//!
//! Election phases:
//! 1. Announcement: Initiator broadcasts election start
//! 2. Candidacy: Eligible nodes submit candidacy
//! 3. Voting: All nodes vote for their preferred candidate
//! 4. Result: Winner is announced and confirmed

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use univrs_enr::{
    core::{NodeId, Timestamp},
    nexus::{
        calculate_election_score, is_nexus_eligible, NexusCandidate, NexusRole, NexusRoleType,
    },
};

use crate::enr_bridge::messages::{
    ElectionAnnouncement, ElectionMessage, ElectionResult, ElectionVote, EnrMessage,
    NexusCandidacy, ELECTION_TOPIC,
};

/// Election timeout in milliseconds
pub const ELECTION_TIMEOUT_MS: u64 = 30_000;

/// Candidacy phase duration in milliseconds
pub const CANDIDACY_PHASE_MS: u64 = 10_000;

/// Voting phase duration in milliseconds
pub const VOTING_PHASE_MS: u64 = 15_000;

/// Minimum votes required for valid election (as fraction of participants)
pub const MIN_VOTE_FRACTION: f64 = 0.5;

/// Callback type for publishing to gossipsub
pub type PublishFn = Box<dyn Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync>;

/// Election state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectionPhase {
    /// No election in progress
    Idle,
    /// Collecting candidacies
    Candidacy,
    /// Collecting votes
    Voting,
    /// Election complete, awaiting confirmation
    Confirming,
}

/// Active election state
#[derive(Debug, Clone)]
pub struct ActiveElection {
    /// Unique election identifier
    pub election_id: u64,
    /// Node that initiated the election
    pub initiator: NodeId,
    /// Region being elected
    pub region_id: String,
    /// Current phase
    pub phase: ElectionPhase,
    /// When the election started
    pub started_at: Timestamp,
    /// Candidates and their metrics
    pub candidates: HashMap<NodeId, NexusCandidate>,
    /// Votes received (voter -> candidate)
    pub votes: HashMap<NodeId, NodeId>,
    /// Known participants in this region
    pub participants: Vec<NodeId>,
}

impl ActiveElection {
    pub fn new(election_id: u64, initiator: NodeId, region_id: String) -> Self {
        Self {
            election_id,
            initiator,
            region_id,
            phase: ElectionPhase::Candidacy,
            started_at: Timestamp::now(),
            candidates: HashMap::new(),
            votes: HashMap::new(),
            participants: Vec::new(),
        }
    }

    /// Check if candidacy phase has expired
    pub fn candidacy_expired(&self) -> bool {
        let now = Timestamp::now();
        now.millis.saturating_sub(self.started_at.millis) > CANDIDACY_PHASE_MS
    }

    /// Check if voting phase has expired
    pub fn voting_expired(&self) -> bool {
        let now = Timestamp::now();
        now.millis.saturating_sub(self.started_at.millis) > CANDIDACY_PHASE_MS + VOTING_PHASE_MS
    }

    /// Check if entire election has timed out
    pub fn timed_out(&self) -> bool {
        let now = Timestamp::now();
        now.millis.saturating_sub(self.started_at.millis) > ELECTION_TIMEOUT_MS
    }

    /// Tally votes and determine winner
    pub fn tally_votes(&self) -> Option<NodeId> {
        if self.votes.is_empty() {
            return None;
        }

        // Count votes per candidate
        let mut vote_counts: HashMap<NodeId, u32> = HashMap::new();
        for candidate in self.votes.values() {
            *vote_counts.entry(*candidate).or_insert(0) += 1;
        }

        // Find candidate with most votes
        vote_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(node, _)| node)
    }

    /// Check if we have enough votes for a valid election
    pub fn has_quorum(&self) -> bool {
        if self.participants.is_empty() {
            return self.votes.len() >= 1;
        }
        let required = (self.participants.len() as f64 * MIN_VOTE_FRACTION).ceil() as usize;
        self.votes.len() >= required.max(1)
    }
}

/// Distributed election manager
pub struct DistributedElection {
    /// This node's ID
    local_node: NodeId,
    /// Current election state
    active_election: Arc<RwLock<Option<ActiveElection>>>,
    /// Current nexus for this node's region
    current_nexus: Arc<RwLock<Option<NodeId>>>,
    /// This node's role
    current_role: Arc<RwLock<NexusRole>>,
    /// Local node metrics for candidacy
    local_metrics: Arc<RwLock<LocalNodeMetrics>>,
    /// Next election ID
    next_election_id: Arc<RwLock<u64>>,
    /// Callback to publish to gossipsub
    publish_fn: PublishFn,
}

/// Local node metrics for election eligibility
#[derive(Debug, Clone, Default)]
pub struct LocalNodeMetrics {
    pub uptime: f64,
    pub bandwidth: u64,
    pub reputation: f64,
    pub connection_count: u32,
}

impl LocalNodeMetrics {
    pub fn is_eligible(&self) -> bool {
        is_nexus_eligible(self.uptime, self.bandwidth, self.reputation)
    }

    pub fn to_candidate(&self, node: NodeId) -> NexusCandidate {
        let mut candidate = NexusCandidate {
            node,
            uptime: self.uptime,
            bandwidth: self.bandwidth,
            reputation: self.reputation,
            current_leaf_count: self.connection_count,
            election_score: 0.0,
        };
        candidate.election_score = calculate_election_score(&candidate);
        candidate
    }
}

impl DistributedElection {
    /// Create a new distributed election manager
    pub fn new<F>(local_node: NodeId, publish_fn: F) -> Self
    where
        F: Fn(String, Vec<u8>) -> Result<(), String> + Send + Sync + 'static,
    {
        Self {
            local_node,
            active_election: Arc::new(RwLock::new(None)),
            current_nexus: Arc::new(RwLock::new(None)),
            current_role: Arc::new(RwLock::new(NexusRole::default())),
            local_metrics: Arc::new(RwLock::new(LocalNodeMetrics::default())),
            next_election_id: Arc::new(RwLock::new(1)),
            publish_fn: Box::new(publish_fn),
        }
    }

    /// Update local node metrics
    pub async fn update_metrics(&self, metrics: LocalNodeMetrics) {
        let mut m = self.local_metrics.write().await;
        *m = metrics;
    }

    /// Get current nexus
    pub async fn current_nexus(&self) -> Option<NodeId> {
        *self.current_nexus.read().await
    }

    /// Get current role
    pub async fn current_role(&self) -> NexusRole {
        self.current_role.read().await.clone()
    }

    /// Check if an election is in progress
    pub async fn election_in_progress(&self) -> bool {
        self.active_election.read().await.is_some()
    }

    /// Trigger a new nexus election
    pub async fn trigger_election(&self, region_id: String) -> Result<u64, ElectionError> {
        // Check if election already in progress
        {
            let election = self.active_election.read().await;
            if let Some(ref e) = *election {
                if !e.timed_out() {
                    return Err(ElectionError::ElectionInProgress);
                }
            }
        }

        // Generate new election ID
        let election_id = {
            let mut id = self.next_election_id.write().await;
            let current = *id;
            *id += 1;
            current
        };

        // Create new election
        let election = ActiveElection::new(election_id, self.local_node, region_id.clone());
        {
            let mut active = self.active_election.write().await;
            *active = Some(election);
        }

        // Broadcast announcement
        let announcement = ElectionAnnouncement {
            election_id,
            initiator: self.local_node,
            region_id,
            timestamp: Timestamp::now(),
        };

        let msg = EnrMessage::Election(ElectionMessage::Announcement(announcement));
        let bytes = msg.encode().map_err(ElectionError::Encode)?;
        (self.publish_fn)(ELECTION_TOPIC.to_string(), bytes).map_err(ElectionError::Publish)?;

        info!(
            election_id = election_id,
            initiator = %self.local_node,
            "Triggered nexus election"
        );

        // Submit our own candidacy if eligible
        self.maybe_submit_candidacy(election_id).await?;

        Ok(election_id)
    }

    /// Submit candidacy if eligible
    async fn maybe_submit_candidacy(&self, election_id: u64) -> Result<(), ElectionError> {
        let metrics = self.local_metrics.read().await.clone();

        if !metrics.is_eligible() {
            debug!(
                uptime = metrics.uptime,
                bandwidth = metrics.bandwidth,
                reputation = metrics.reputation,
                "Node not eligible for nexus candidacy"
            );
            return Ok(());
        }

        let candidate = metrics.to_candidate(self.local_node);

        // Add to local election state
        {
            let mut election = self.active_election.write().await;
            if let Some(ref mut e) = *election {
                if e.election_id == election_id {
                    e.candidates.insert(self.local_node, candidate.clone());
                }
            }
        }

        // Broadcast candidacy
        let candidacy = NexusCandidacy {
            election_id,
            candidate,
        };

        let msg = EnrMessage::Election(ElectionMessage::Candidacy(candidacy));
        let bytes = msg.encode().map_err(ElectionError::Encode)?;
        (self.publish_fn)(ELECTION_TOPIC.to_string(), bytes).map_err(ElectionError::Publish)?;

        info!(
            election_id = election_id,
            node = %self.local_node,
            "Submitted nexus candidacy"
        );

        Ok(())
    }

    /// Handle incoming election announcement
    pub async fn handle_announcement(
        &self,
        announcement: ElectionAnnouncement,
    ) -> Result<(), ElectionError> {
        // Check if we already have an election for this region
        {
            let election = self.active_election.read().await;
            if let Some(ref e) = *election {
                if !e.timed_out() && e.election_id >= announcement.election_id {
                    // We already know about a newer or same election
                    return Ok(());
                }
            }
        }

        // Start tracking this election
        let election =
            ActiveElection::new(announcement.election_id, announcement.initiator, announcement.region_id);

        {
            let mut active = self.active_election.write().await;
            *active = Some(election);
        }

        debug!(
            election_id = announcement.election_id,
            initiator = %announcement.initiator,
            "Received election announcement"
        );

        // Submit our candidacy if eligible
        self.maybe_submit_candidacy(announcement.election_id).await?;

        Ok(())
    }

    /// Handle incoming candidacy
    pub async fn handle_candidacy(&self, candidacy: NexusCandidacy) -> Result<(), ElectionError> {
        let mut election = self.active_election.write().await;

        if let Some(ref mut e) = *election {
            if e.election_id != candidacy.election_id {
                return Ok(()); // Different election
            }

            if e.phase != ElectionPhase::Candidacy && e.phase != ElectionPhase::Voting {
                return Ok(()); // Too late for candidacy
            }

            // Verify eligibility
            let candidate = &candidacy.candidate;
            if !is_nexus_eligible(candidate.uptime, candidate.bandwidth, candidate.reputation) {
                warn!(
                    node = %candidate.node,
                    uptime = candidate.uptime,
                    bandwidth = candidate.bandwidth,
                    reputation = candidate.reputation,
                    "Rejecting ineligible candidate"
                );
                return Err(ElectionError::IneligibleCandidate);
            }

            e.candidates.insert(candidate.node, candidate.clone());

            debug!(
                election_id = e.election_id,
                candidate = %candidate.node,
                score = candidate.election_score,
                "Received candidacy"
            );
        }

        Ok(())
    }

    /// Cast vote for a candidate
    pub async fn cast_vote(&self, election_id: u64) -> Result<(), ElectionError> {
        let best_candidate = {
            let election = self.active_election.read().await;
            match &*election {
                Some(e) if e.election_id == election_id => {
                    // Find candidate with highest score
                    e.candidates
                        .values()
                        .max_by(|a, b| {
                            a.election_score
                                .partial_cmp(&b.election_score)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|c| c.node)
                }
                _ => None,
            }
        };

        let candidate = best_candidate.ok_or(ElectionError::NoCandidates)?;

        // Record our vote locally
        {
            let mut election = self.active_election.write().await;
            if let Some(ref mut e) = *election {
                if e.election_id == election_id {
                    e.votes.insert(self.local_node, candidate);
                    e.phase = ElectionPhase::Voting;
                }
            }
        }

        // Broadcast vote
        let vote = ElectionVote {
            election_id,
            voter: self.local_node,
            candidate,
            timestamp: Timestamp::now(),
        };

        let msg = EnrMessage::Election(ElectionMessage::Vote(vote));
        let bytes = msg.encode().map_err(ElectionError::Encode)?;
        (self.publish_fn)(ELECTION_TOPIC.to_string(), bytes).map_err(ElectionError::Publish)?;

        debug!(
            election_id = election_id,
            voter = %self.local_node,
            candidate = %candidate,
            "Cast vote"
        );

        Ok(())
    }

    /// Handle incoming vote
    pub async fn handle_vote(&self, vote: ElectionVote) -> Result<(), ElectionError> {
        let mut election = self.active_election.write().await;

        if let Some(ref mut e) = *election {
            if e.election_id != vote.election_id {
                return Ok(()); // Different election
            }

            // Record the vote
            e.votes.insert(vote.voter, vote.candidate);

            // Track participant
            if !e.participants.contains(&vote.voter) {
                e.participants.push(vote.voter);
            }

            debug!(
                election_id = e.election_id,
                voter = %vote.voter,
                candidate = %vote.candidate,
                total_votes = e.votes.len(),
                "Received vote"
            );
        }

        Ok(())
    }

    /// Finalize election and announce result
    pub async fn finalize_election(&self) -> Result<Option<NodeId>, ElectionError> {
        let (election_id, winner, region_id) = {
            let election = self.active_election.read().await;
            match &*election {
                Some(e) => {
                    if !e.has_quorum() {
                        return Err(ElectionError::InsufficientVotes);
                    }
                    let winner = e.tally_votes();
                    (e.election_id, winner, e.region_id.clone())
                }
                None => return Ok(None),
            }
        };

        let winner = match winner {
            Some(w) => w,
            None => return Err(ElectionError::NoCandidates),
        };

        // Update local state
        {
            let mut nexus = self.current_nexus.write().await;
            *nexus = Some(winner);
        }

        // Update our role
        {
            let mut role = self.current_role.write().await;
            if winner == self.local_node {
                *role = NexusRole {
                    role_type: NexusRoleType::Nexus,
                    parent: None,
                    children: Vec::new(),
                };
            } else {
                *role = NexusRole::leaf(winner);
            }
        }

        // Clear election state
        {
            let mut election = self.active_election.write().await;
            if let Some(ref mut e) = *election {
                e.phase = ElectionPhase::Confirming;
            }
        }

        // Broadcast result
        let result = ElectionResult {
            election_id,
            winner,
            region_id,
            vote_count: {
                let election = self.active_election.read().await;
                election.as_ref().map(|e| e.votes.len() as u32).unwrap_or(0)
            },
            timestamp: Timestamp::now(),
        };

        let msg = EnrMessage::Election(ElectionMessage::Result(result));
        let bytes = msg.encode().map_err(ElectionError::Encode)?;
        (self.publish_fn)(ELECTION_TOPIC.to_string(), bytes).map_err(ElectionError::Publish)?;

        info!(
            election_id = election_id,
            winner = %winner,
            "Election finalized"
        );

        // Clear active election
        {
            let mut election = self.active_election.write().await;
            *election = None;
        }

        Ok(Some(winner))
    }

    /// Handle incoming election result
    pub async fn handle_result(&self, result: ElectionResult) -> Result<(), ElectionError> {
        // Verify this is for an election we know about
        {
            let election = self.active_election.read().await;
            if let Some(ref e) = *election {
                if e.election_id != result.election_id {
                    return Ok(()); // Different election
                }
            }
        }

        // Update our nexus
        {
            let mut nexus = self.current_nexus.write().await;
            *nexus = Some(result.winner);
        }

        // Update our role
        {
            let mut role = self.current_role.write().await;
            if result.winner == self.local_node {
                *role = NexusRole {
                    role_type: NexusRoleType::Nexus,
                    parent: None,
                    children: Vec::new(),
                };
            } else {
                *role = NexusRole::leaf(result.winner);
            }
        }

        // Clear election state
        {
            let mut election = self.active_election.write().await;
            *election = None;
        }

        info!(
            election_id = result.election_id,
            winner = %result.winner,
            votes = result.vote_count,
            "Accepted election result"
        );

        Ok(())
    }

    /// Handle any election message
    pub async fn handle_election_message(
        &self,
        message: ElectionMessage,
    ) -> Result<(), ElectionError> {
        match message {
            ElectionMessage::Announcement(ann) => self.handle_announcement(ann).await,
            ElectionMessage::Candidacy(cand) => self.handle_candidacy(cand).await,
            ElectionMessage::Vote(vote) => self.handle_vote(vote).await,
            ElectionMessage::Result(result) => self.handle_result(result).await,
        }
    }

    /// Check election timeouts and advance phases
    pub async fn check_election_progress(&self) -> Result<(), ElectionError> {
        let should_vote;
        let should_finalize;

        {
            let election = self.active_election.read().await;
            match &*election {
                Some(e) => {
                    should_vote =
                        e.phase == ElectionPhase::Candidacy && e.candidacy_expired() && !e.candidates.is_empty();
                    should_finalize = e.phase == ElectionPhase::Voting && e.voting_expired();
                }
                None => return Ok(()),
            }
        }

        if should_vote {
            let election_id = {
                let election = self.active_election.read().await;
                election.as_ref().map(|e| e.election_id)
            };
            if let Some(id) = election_id {
                // Check if we already voted
                let already_voted = {
                    let election = self.active_election.read().await;
                    election
                        .as_ref()
                        .map(|e| e.votes.contains_key(&self.local_node))
                        .unwrap_or(false)
                };
                if !already_voted {
                    self.cast_vote(id).await?;
                }
            }
        }

        if should_finalize {
            self.finalize_election().await?;
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ElectionError {
    #[error("Election already in progress")]
    ElectionInProgress,
    #[error("No candidates available")]
    NoCandidates,
    #[error("Insufficient votes for quorum")]
    InsufficientVotes,
    #[error("Candidate not eligible")]
    IneligibleCandidate,
    #[error("Encoding error: {0}")]
    Encode(#[from] crate::enr_bridge::messages::EncodeError),
    #[error("Publish error: {0}")]
    Publish(String),
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
    async fn test_trigger_election() {
        let node = NodeId::from_bytes([1u8; 32]);
        let (publish, counter) = mock_publish();
        let election = DistributedElection::new(node, publish);

        // Set eligible metrics
        election
            .update_metrics(LocalNodeMetrics {
                uptime: 0.99,
                bandwidth: 50_000_000,
                reputation: 0.9,
                connection_count: 25,
            })
            .await;

        let id = election.trigger_election("region-1".to_string()).await.unwrap();
        assert_eq!(id, 1);

        // Should have published announcement and candidacy
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        // Should be in candidacy phase
        let active = election.active_election.read().await;
        assert!(active.is_some());
        assert_eq!(active.as_ref().unwrap().phase, ElectionPhase::Candidacy);
    }

    #[tokio::test]
    async fn test_handle_candidacy() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();
        let election = DistributedElection::new(node1, publish);

        // Trigger election first
        election
            .update_metrics(LocalNodeMetrics {
                uptime: 0.99,
                bandwidth: 50_000_000,
                reputation: 0.9,
                connection_count: 25,
            })
            .await;
        election.trigger_election("region-1".to_string()).await.unwrap();

        // Handle candidacy from another node
        let candidacy = NexusCandidacy {
            election_id: 1,
            candidate: NexusCandidate {
                node: node2,
                uptime: 0.98,
                bandwidth: 40_000_000,
                reputation: 0.85,
                current_leaf_count: 20,
                election_score: 0.8,
            },
        };

        election.handle_candidacy(candidacy).await.unwrap();

        // Should have 2 candidates now
        let active = election.active_election.read().await;
        assert_eq!(active.as_ref().unwrap().candidates.len(), 2);
    }

    #[tokio::test]
    async fn test_vote_and_tally() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();
        let election = DistributedElection::new(node1, publish);

        // Trigger election
        election
            .update_metrics(LocalNodeMetrics {
                uptime: 0.99,
                bandwidth: 50_000_000,
                reputation: 0.9,
                connection_count: 25,
            })
            .await;
        election.trigger_election("region-1".to_string()).await.unwrap();

        // Add another candidate with higher score
        let candidacy = NexusCandidacy {
            election_id: 1,
            candidate: NexusCandidate {
                node: node2,
                uptime: 0.99,
                bandwidth: 80_000_000,
                reputation: 0.95,
                current_leaf_count: 27,
                election_score: 0.95,
            },
        };
        election.handle_candidacy(candidacy).await.unwrap();

        // Cast vote
        election.cast_vote(1).await.unwrap();

        // Vote should be for node2 (higher score)
        let active = election.active_election.read().await;
        let vote = active.as_ref().unwrap().votes.get(&node1);
        assert_eq!(vote, Some(&node2));
    }

    #[tokio::test]
    async fn test_finalize_election() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let node3 = NodeId::from_bytes([3u8; 32]);
        let (publish, _) = mock_publish();
        let election = DistributedElection::new(node1, publish);

        // Trigger election
        election
            .update_metrics(LocalNodeMetrics {
                uptime: 0.99,
                bandwidth: 50_000_000,
                reputation: 0.9,
                connection_count: 25,
            })
            .await;
        election.trigger_election("region-1".to_string()).await.unwrap();

        // Add candidates and votes manually
        {
            let mut active = election.active_election.write().await;
            let e = active.as_mut().unwrap();
            e.votes.insert(node1, node2);
            e.votes.insert(node2, node2);
            e.votes.insert(node3, node1);
            e.phase = ElectionPhase::Voting;
        }

        // Finalize
        let winner = election.finalize_election().await.unwrap();
        assert_eq!(winner, Some(node2)); // node2 has 2 votes

        // Current nexus should be updated
        assert_eq!(election.current_nexus().await, Some(node2));

        // Our role should be leaf pointing to node2
        let role = election.current_role().await;
        assert!(role.is_leaf());
        assert_eq!(role.parent, Some(node2));
    }

    #[tokio::test]
    async fn test_ineligible_candidate_rejected() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let (publish, _) = mock_publish();
        let election = DistributedElection::new(node1, publish);

        // Trigger election
        election
            .update_metrics(LocalNodeMetrics {
                uptime: 0.99,
                bandwidth: 50_000_000,
                reputation: 0.9,
                connection_count: 25,
            })
            .await;
        election.trigger_election("region-1".to_string()).await.unwrap();

        // Try to add ineligible candidate
        let candidacy = NexusCandidacy {
            election_id: 1,
            candidate: NexusCandidate {
                node: node2,
                uptime: 0.5, // Too low
                bandwidth: 1_000_000, // Too low
                reputation: 0.3, // Too low
                current_leaf_count: 5,
                election_score: 0.2,
            },
        };

        let result = election.handle_candidacy(candidacy).await;
        assert!(matches!(result, Err(ElectionError::IneligibleCandidate)));
    }

    #[test]
    fn test_active_election_tally() {
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);
        let node3 = NodeId::from_bytes([3u8; 32]);

        let mut election = ActiveElection::new(1, node1, "region".to_string());
        election.votes.insert(node1, node2);
        election.votes.insert(node2, node2);
        election.votes.insert(node3, node1);

        let winner = election.tally_votes();
        assert_eq!(winner, Some(node2));
    }

    #[test]
    fn test_local_metrics_eligibility() {
        let eligible = LocalNodeMetrics {
            uptime: 0.96,
            bandwidth: 15_000_000,
            reputation: 0.75,
            connection_count: 20,
        };
        assert!(eligible.is_eligible());

        let ineligible = LocalNodeMetrics {
            uptime: 0.80,
            bandwidth: 5_000_000,
            reputation: 0.5,
            connection_count: 5,
        };
        assert!(!ineligible.is_eligible());
    }
}
