//! Mutual credit and economic relationships

use crate::peer::PeerId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A credit relationship between two peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditRelationship {
    /// The peer who extends credit
    pub creditor: PeerId,
    /// The peer who receives credit
    pub debtor: PeerId,
    /// Maximum credit limit
    pub credit_limit: f64,
    /// Current balance (positive = creditor is owed, negative = debtor is owed)
    pub balance: f64,
    /// When the relationship was established
    pub established: DateTime<Utc>,
    /// When the balance was last updated
    pub last_transaction: DateTime<Utc>,
    /// Whether the relationship is active
    pub active: bool,
}

impl CreditRelationship {
    /// Create a new credit relationship
    pub fn new(creditor: PeerId, debtor: PeerId, credit_limit: f64) -> Self {
        let now = Utc::now();
        Self {
            creditor,
            debtor,
            credit_limit,
            balance: 0.0,
            established: now,
            last_transaction: now,
            active: true,
        }
    }

    /// Available credit for the debtor
    pub fn available_credit(&self) -> f64 {
        if !self.active {
            return 0.0;
        }
        (self.credit_limit - self.balance).max(0.0)
    }

    /// Transfer credit (positive amount = creditor gives to debtor)
    pub fn transfer(&mut self, amount: f64) -> Result<(), CreditError> {
        if !self.active {
            return Err(CreditError::InactiveRelationship);
        }

        let new_balance = self.balance + amount;

        if new_balance > self.credit_limit {
            return Err(CreditError::ExceedsLimit {
                requested: amount,
                available: self.available_credit(),
            });
        }

        self.balance = new_balance;
        self.last_transaction = Utc::now();
        Ok(())
    }
}

/// Errors related to credit operations
#[derive(Debug, thiserror::Error)]
pub enum CreditError {
    #[error("Credit relationship is inactive")]
    InactiveRelationship,

    #[error("Transfer exceeds credit limit: requested {requested}, available {available}")]
    ExceedsLimit { requested: f64, available: f64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credit_transfer() {
        let creditor = PeerId("creditor".to_string());
        let debtor = PeerId("debtor".to_string());
        let mut rel = CreditRelationship::new(creditor, debtor, 100.0);

        assert_eq!(rel.available_credit(), 100.0);

        rel.transfer(50.0).unwrap();
        assert_eq!(rel.balance, 50.0);
        assert_eq!(rel.available_credit(), 50.0);

        // Should fail - exceeds limit
        assert!(rel.transfer(60.0).is_err());
    }
}
