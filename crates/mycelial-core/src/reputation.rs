//! Reputation scoring and trust management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Reputation score for a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reputation {
    /// Current reputation score (0.0 to 1.0)
    pub score: f64,
    /// Number of successful interactions
    pub successful_interactions: u64,
    /// Number of failed interactions
    pub failed_interactions: u64,
    /// When reputation was last updated
    pub last_updated: DateTime<Utc>,
    /// Historical scores for trend analysis
    pub history: Vec<ReputationSnapshot>,
}

/// A snapshot of reputation at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationSnapshot {
    pub score: f64,
    pub timestamp: DateTime<Utc>,
}

impl Default for Reputation {
    fn default() -> Self {
        Self {
            score: 0.5, // Start neutral
            successful_interactions: 0,
            failed_interactions: 0,
            last_updated: Utc::now(),
            history: Vec::new(),
        }
    }
}

impl Reputation {
    /// Create a new reputation with a starting score
    pub fn new(initial_score: f64) -> Self {
        Self {
            score: initial_score.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Update reputation based on interaction outcome
    /// Uses exponential moving average: R(T) = α·R(T-1) + β·C(T)
    pub fn update(&mut self, success: bool, alpha: f64, beta: f64) {
        // Save snapshot before update
        self.history.push(ReputationSnapshot {
            score: self.score,
            timestamp: self.last_updated,
        });

        // Trim history to last 100 entries
        if self.history.len() > 100 {
            self.history.remove(0);
        }

        let contribution = if success {
            self.successful_interactions += 1;
            1.0
        } else {
            self.failed_interactions += 1;
            0.0
        };

        self.score = (alpha * self.score + beta * contribution).clamp(0.0, 1.0);
        self.last_updated = Utc::now();
    }

    /// Check if peer is trusted (above threshold)
    pub fn is_trusted(&self, threshold: f64) -> bool {
        self.score >= threshold
    }

    /// Calculate trend (positive = improving, negative = declining)
    pub fn trend(&self) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }

        let recent: Vec<_> = self.history.iter().rev().take(10).collect();
        if recent.len() < 2 {
            return 0.0;
        }

        let sum: f64 = recent.iter().map(|s| s.score).sum();
        let avg = sum / recent.len() as f64;

        self.score - avg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_update() {
        let mut rep = Reputation::default();
        assert_eq!(rep.score, 0.5);

        // Successful interaction
        rep.update(true, 0.4, 0.6);
        assert!(rep.score > 0.5);

        // Failed interaction
        rep.update(false, 0.4, 0.6);
        assert!(rep.score < rep.history.last().unwrap().score);
    }
}
