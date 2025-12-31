//! Per-peer economics state tracking
//!
//! Maintains in-memory state for:
//! - Credit lines between peers
//! - Active governance proposals
//! - Vouch relationships
//! - Resource contributions

use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

/// Credit line between two peers
#[derive(Debug, Clone, Serialize)]
pub struct CreditLine {
    pub id: String,
    pub creditor: String,
    pub debtor: String,
    pub limit: f64,
    pub balance: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Governance proposal
#[derive(Debug, Clone, Serialize)]
pub struct Proposal {
    pub id: String,
    pub proposer: String,
    pub title: String,
    pub description: String,
    pub proposal_type: String,
    pub status: ProposalStatus,
    pub yes_votes: f64,
    pub no_votes: f64,
    pub quorum: f64,
    pub deadline: i64,
    pub created_at: i64,
    pub votes: HashMap<String, Vote>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Expired,
    Executed,
}

impl std::fmt::Display for ProposalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalStatus::Active => write!(f, "active"),
            ProposalStatus::Passed => write!(f, "passed"),
            ProposalStatus::Rejected => write!(f, "rejected"),
            ProposalStatus::Expired => write!(f, "expired"),
            ProposalStatus::Executed => write!(f, "executed"),
        }
    }
}

/// Vote on a proposal
#[derive(Debug, Clone, Serialize)]
pub struct Vote {
    pub voter: String,
    pub vote_type: VoteType,
    pub weight: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum VoteType {
    Yes,
    No,
    Abstain,
}

/// Vouch relationship
#[derive(Debug, Clone, Serialize)]
pub struct Vouch {
    pub id: String,
    pub voucher: String,
    pub vouchee: String,
    pub weight: f64,
    pub accepted: bool,
    pub created_at: i64,
}

/// Resource contribution from a peer
#[derive(Debug, Clone, Serialize)]
pub struct ResourceContribution {
    pub peer_id: String,
    pub resource_type: String,
    pub amount: f64,
    pub unit: String,
    pub timestamp: i64,
}

/// Aggregated resource pool
#[derive(Debug, Clone, Default, Serialize)]
pub struct ResourcePool {
    pub total_bandwidth: f64,
    pub total_compute: f64,
    pub total_storage: f64,
    pub contributions: Vec<ResourceContribution>,
}

/// Per-peer economics state manager
pub struct EconomicsStateManager {
    /// Credit lines indexed by line ID
    credit_lines: RwLock<HashMap<String, CreditLine>>,
    /// Credit lines by peer pair (creditor-debtor -> line_id)
    credit_lines_by_peers: RwLock<HashMap<String, String>>,
    /// Active proposals indexed by proposal ID
    proposals: RwLock<HashMap<String, Proposal>>,
    /// Vouch relationships indexed by vouch ID
    vouches: RwLock<HashMap<String, Vouch>>,
    /// Vouches by peer pair (voucher-vouchee -> vouch_id)
    vouches_by_peers: RwLock<HashMap<String, String>>,
    /// Resource pool
    resource_pool: RwLock<ResourcePool>,
    /// Peer reputations (calculated from vouches)
    reputations: RwLock<HashMap<String, f64>>,
    /// Creation time for uptime tracking
    created_at: Instant,
}

impl EconomicsStateManager {
    pub fn new() -> Self {
        Self {
            credit_lines: RwLock::new(HashMap::new()),
            credit_lines_by_peers: RwLock::new(HashMap::new()),
            proposals: RwLock::new(HashMap::new()),
            vouches: RwLock::new(HashMap::new()),
            vouches_by_peers: RwLock::new(HashMap::new()),
            resource_pool: RwLock::new(ResourcePool::default()),
            reputations: RwLock::new(HashMap::new()),
            created_at: Instant::now(),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Credit Line Operations
    // ─────────────────────────────────────────────────────────────────────────────

    /// Add or update a credit line
    pub fn upsert_credit_line(&self, line: CreditLine) {
        let peer_key = format!("{}-{}", line.creditor, line.debtor);
        let id = line.id.clone();

        self.credit_lines_by_peers.write().insert(peer_key, id.clone());
        self.credit_lines.write().insert(id, line);
    }

    /// Get credit line by ID
    pub fn get_credit_line(&self, id: &str) -> Option<CreditLine> {
        self.credit_lines.read().get(id).cloned()
    }

    /// Get credit line between two peers
    pub fn get_credit_line_between(&self, creditor: &str, debtor: &str) -> Option<CreditLine> {
        let peer_key = format!("{}-{}", creditor, debtor);
        let id = self.credit_lines_by_peers.read().get(&peer_key).cloned()?;
        self.credit_lines.read().get(&id).cloned()
    }

    /// Update credit line balance after transfer
    pub fn update_credit_balance(&self, line_id: &str, new_balance: f64) {
        if let Some(line) = self.credit_lines.write().get_mut(line_id) {
            line.balance = new_balance;
            line.updated_at = chrono::Utc::now().timestamp_millis();
        }
    }

    /// Get all credit lines for a peer (as creditor or debtor)
    pub fn get_credit_lines_for_peer(&self, peer_id: &str) -> Vec<CreditLine> {
        self.credit_lines
            .read()
            .values()
            .filter(|l| l.creditor == peer_id || l.debtor == peer_id)
            .cloned()
            .collect()
    }

    /// Get all credit lines
    pub fn get_all_credit_lines(&self) -> Vec<CreditLine> {
        self.credit_lines.read().values().cloned().collect()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Proposal Operations
    // ─────────────────────────────────────────────────────────────────────────────

    /// Add a new proposal
    pub fn add_proposal(&self, proposal: Proposal) {
        self.proposals.write().insert(proposal.id.clone(), proposal);
    }

    /// Get proposal by ID
    pub fn get_proposal(&self, id: &str) -> Option<Proposal> {
        self.proposals.read().get(id).cloned()
    }

    /// Record a vote on a proposal
    pub fn record_vote(&self, proposal_id: &str, vote: Vote) {
        if let Some(proposal) = self.proposals.write().get_mut(proposal_id) {
            // Update vote counts
            match vote.vote_type {
                VoteType::Yes => proposal.yes_votes += vote.weight,
                VoteType::No => proposal.no_votes += vote.weight,
                VoteType::Abstain => {}
            }
            proposal.votes.insert(vote.voter.clone(), vote);

            // Check if proposal should be resolved
            self.check_proposal_resolution(proposal);
        }
    }

    /// Check if proposal has reached quorum and update status
    fn check_proposal_resolution(&self, proposal: &mut Proposal) {
        let total_votes = proposal.yes_votes + proposal.no_votes;
        let now = chrono::Utc::now().timestamp_millis();

        // Check expiry
        if now > proposal.deadline {
            if total_votes >= proposal.quorum {
                proposal.status = if proposal.yes_votes > proposal.no_votes {
                    ProposalStatus::Passed
                } else {
                    ProposalStatus::Rejected
                };
            } else {
                proposal.status = ProposalStatus::Expired;
            }
        }
    }

    /// Update proposal status
    pub fn update_proposal_status(&self, proposal_id: &str, status: ProposalStatus) {
        if let Some(proposal) = self.proposals.write().get_mut(proposal_id) {
            proposal.status = status;
        }
    }

    /// Get all active proposals
    pub fn get_active_proposals(&self) -> Vec<Proposal> {
        self.proposals
            .read()
            .values()
            .filter(|p| p.status == ProposalStatus::Active)
            .cloned()
            .collect()
    }

    /// Get all proposals
    pub fn get_all_proposals(&self) -> Vec<Proposal> {
        self.proposals.read().values().cloned().collect()
    }

    /// Check and expire old proposals
    pub fn expire_old_proposals(&self) {
        let now = chrono::Utc::now().timestamp_millis();
        let mut proposals = self.proposals.write();

        for proposal in proposals.values_mut() {
            if proposal.status == ProposalStatus::Active && now > proposal.deadline {
                let total_votes = proposal.yes_votes + proposal.no_votes;
                if total_votes >= proposal.quorum {
                    proposal.status = if proposal.yes_votes > proposal.no_votes {
                        ProposalStatus::Passed
                    } else {
                        ProposalStatus::Rejected
                    };
                } else {
                    proposal.status = ProposalStatus::Expired;
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Vouch Operations
    // ─────────────────────────────────────────────────────────────────────────────

    /// Add a vouch request
    pub fn add_vouch(&self, vouch: Vouch) {
        let peer_key = format!("{}-{}", vouch.voucher, vouch.vouchee);
        let id = vouch.id.clone();

        self.vouches_by_peers.write().insert(peer_key, id.clone());
        self.vouches.write().insert(id, vouch);
    }

    /// Get vouch by ID
    pub fn get_vouch(&self, id: &str) -> Option<Vouch> {
        self.vouches.read().get(id).cloned()
    }

    /// Accept or reject a vouch
    pub fn respond_to_vouch(&self, vouch_id: &str, accepted: bool) -> Option<Vouch> {
        let mut vouches = self.vouches.write();
        if let Some(vouch) = vouches.get_mut(vouch_id) {
            vouch.accepted = accepted;

            if accepted {
                // Update reputation
                self.update_reputation(&vouch.vouchee, vouch.weight);
            }

            return Some(vouch.clone());
        }
        None
    }

    /// Get vouches for a peer (as vouchee)
    pub fn get_vouches_for_peer(&self, peer_id: &str) -> Vec<Vouch> {
        self.vouches
            .read()
            .values()
            .filter(|v| v.vouchee == peer_id && v.accepted)
            .cloned()
            .collect()
    }

    /// Get vouches from a peer (as voucher)
    pub fn get_vouches_from_peer(&self, peer_id: &str) -> Vec<Vouch> {
        self.vouches
            .read()
            .values()
            .filter(|v| v.voucher == peer_id)
            .cloned()
            .collect()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Reputation Operations
    // ─────────────────────────────────────────────────────────────────────────────

    /// Update peer reputation based on vouch
    fn update_reputation(&self, peer_id: &str, vouch_weight: f64) {
        let mut reputations = self.reputations.write();
        let current = reputations.get(peer_id).copied().unwrap_or(0.5);

        // Simple reputation update: weighted average with new vouch
        // New rep = current * 0.9 + vouch_weight * 0.1
        let new_rep = (current * 0.9 + vouch_weight * 0.1).clamp(0.0, 1.0);
        reputations.insert(peer_id.to_string(), new_rep);
    }

    /// Get peer reputation
    pub fn get_reputation(&self, peer_id: &str) -> f64 {
        self.reputations.read().get(peer_id).copied().unwrap_or(0.5)
    }

    /// Calculate reputation from vouches
    pub fn calculate_reputation(&self, peer_id: &str) -> f64 {
        let vouches = self.get_vouches_for_peer(peer_id);
        if vouches.is_empty() {
            return 0.5; // Default reputation
        }

        let total_weight: f64 = vouches.iter().map(|v| v.weight).sum();
        let count = vouches.len() as f64;

        // Weighted average of vouches
        (total_weight / count).clamp(0.0, 1.0)
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Resource Operations
    // ─────────────────────────────────────────────────────────────────────────────

    /// Record a resource contribution
    pub fn record_resource_contribution(&self, contribution: ResourceContribution) {
        let mut pool = self.resource_pool.write();

        // Update totals based on resource type
        match contribution.resource_type.to_lowercase().as_str() {
            "bandwidth" => pool.total_bandwidth += contribution.amount,
            "compute" | "cpu" => pool.total_compute += contribution.amount,
            "storage" => pool.total_storage += contribution.amount,
            _ => {}
        }

        pool.contributions.push(contribution);
    }

    /// Get resource pool summary
    pub fn get_resource_pool(&self) -> ResourcePool {
        self.resource_pool.read().clone()
    }

    /// Get contributions by peer
    pub fn get_contributions_by_peer(&self, peer_id: &str) -> Vec<ResourceContribution> {
        self.resource_pool
            .read()
            .contributions
            .iter()
            .filter(|c| c.peer_id == peer_id)
            .cloned()
            .collect()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Statistics
    // ─────────────────────────────────────────────────────────────────────────────

    /// Get economics state summary
    pub fn get_summary(&self) -> EconomicsSummary {
        EconomicsSummary {
            credit_line_count: self.credit_lines.read().len(),
            active_proposal_count: self.get_active_proposals().len(),
            total_proposal_count: self.proposals.read().len(),
            vouch_count: self.vouches.read().values().filter(|v| v.accepted).count(),
            pending_vouch_count: self.vouches.read().values().filter(|v| !v.accepted).count(),
            contributor_count: self.resource_pool.read().contributions
                .iter()
                .map(|c| &c.peer_id)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            uptime_seconds: self.created_at.elapsed().as_secs(),
        }
    }
}

impl Default for EconomicsStateManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of economics state
#[derive(Debug, Clone, serde::Serialize)]
pub struct EconomicsSummary {
    pub credit_line_count: usize,
    pub active_proposal_count: usize,
    pub total_proposal_count: usize,
    pub vouch_count: usize,
    pub pending_vouch_count: usize,
    pub contributor_count: usize,
    pub uptime_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credit_line_operations() {
        let manager = EconomicsStateManager::new();

        let line = CreditLine {
            id: "line1".to_string(),
            creditor: "alice".to_string(),
            debtor: "bob".to_string(),
            limit: 100.0,
            balance: 0.0,
            created_at: 0,
            updated_at: 0,
        };

        manager.upsert_credit_line(line.clone());

        assert!(manager.get_credit_line("line1").is_some());
        assert!(manager.get_credit_line_between("alice", "bob").is_some());
        assert!(manager.get_credit_line_between("bob", "alice").is_none());

        manager.update_credit_balance("line1", 50.0);
        assert_eq!(manager.get_credit_line("line1").unwrap().balance, 50.0);
    }

    #[test]
    fn test_proposal_operations() {
        let manager = EconomicsStateManager::new();

        let proposal = Proposal {
            id: "prop1".to_string(),
            proposer: "alice".to_string(),
            title: "Test Proposal".to_string(),
            description: "A test".to_string(),
            proposal_type: "text".to_string(),
            status: ProposalStatus::Active,
            yes_votes: 0.0,
            no_votes: 0.0,
            quorum: 0.5,
            deadline: chrono::Utc::now().timestamp_millis() + 86400000,
            created_at: chrono::Utc::now().timestamp_millis(),
            votes: HashMap::new(),
        };

        manager.add_proposal(proposal);
        assert_eq!(manager.get_active_proposals().len(), 1);

        let vote = Vote {
            voter: "bob".to_string(),
            vote_type: VoteType::Yes,
            weight: 1.0,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        manager.record_vote("prop1", vote);
        assert_eq!(manager.get_proposal("prop1").unwrap().yes_votes, 1.0);
    }

    #[test]
    fn test_vouch_operations() {
        let manager = EconomicsStateManager::new();

        let vouch = Vouch {
            id: "vouch1".to_string(),
            voucher: "alice".to_string(),
            vouchee: "bob".to_string(),
            weight: 0.8,
            accepted: false,
            created_at: 0,
        };

        manager.add_vouch(vouch);
        assert!(manager.get_vouch("vouch1").is_some());

        manager.respond_to_vouch("vouch1", true);
        assert!(manager.get_vouch("vouch1").unwrap().accepted);

        // Reputation should be updated
        let rep = manager.get_reputation("bob");
        assert!(rep > 0.5); // Should increase from default
    }

    #[test]
    fn test_resource_contributions() {
        let manager = EconomicsStateManager::new();

        let contrib = ResourceContribution {
            peer_id: "alice".to_string(),
            resource_type: "bandwidth".to_string(),
            amount: 100.0,
            unit: "mbps".to_string(),
            timestamp: 0,
        };

        manager.record_resource_contribution(contrib);

        let pool = manager.get_resource_pool();
        assert_eq!(pool.total_bandwidth, 100.0);
        assert_eq!(pool.contributions.len(), 1);
    }

    #[test]
    fn test_summary() {
        let manager = EconomicsStateManager::new();

        let summary = manager.get_summary();
        assert_eq!(summary.credit_line_count, 0);
        assert_eq!(summary.active_proposal_count, 0);
    }
}
