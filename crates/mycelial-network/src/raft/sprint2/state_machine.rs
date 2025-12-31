//! Credit ledger as a Raft state machine

use std::collections::HashMap;
use std::io::Cursor;

use async_trait::async_trait;
use openraft::{
    Entry, EntryPayload, LogId, RaftSnapshotBuilder, RaftStateMachine, Snapshot, SnapshotMeta,
    StorageError, StoredMembership,
};
use tracing::{debug, info};
use univrs_enr::{
    core::{AccountId, Credits},
    revival::calculate_entropy_tax,
};

use super::types::{CreditCommand, CreditResponse, CreditSnapshot, CreditTypeConfig};
use crate::enr_bridge::credits::{TransferError, INITIAL_NODE_CREDITS};

/// The credit ledger as a Raft state machine
pub struct CreditStateMachine {
    /// Account balances: AccountId -> Credits
    balances: HashMap<AccountId, Credits>,
    /// Revival pool balance (accumulated entropy taxes)
    revival_pool: Credits,
    /// Last applied log entry
    last_applied_log: Option<LogId<u64>>,
    /// Current membership
    last_membership: StoredMembership<CreditTypeConfig>,
}

impl CreditStateMachine {
    /// Create a new empty state machine
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            revival_pool: Credits::ZERO,
            last_applied_log: None,
            last_membership: StoredMembership::default(),
        }
    }

    /// Get balance for an account
    pub fn get_balance(&self, account: &AccountId) -> Credits {
        self.balances.get(account).copied().unwrap_or(Credits::ZERO)
    }

    /// Get all balances
    pub fn all_balances(&self) -> HashMap<AccountId, Credits> {
        self.balances.clone()
    }

    /// Get total supply (sum of all balances)
    pub fn total_supply(&self) -> Credits {
        self.balances
            .values()
            .fold(Credits::ZERO, |acc, c| acc.saturating_add(*c))
    }

    /// Get revival pool balance
    pub fn revival_pool(&self) -> Credits {
        self.revival_pool
    }

    /// Apply a credit command and return the response
    fn apply_command(&mut self, command: &CreditCommand) -> CreditResponse {
        match command {
            CreditCommand::Transfer(transfer) => {
                let result = self.apply_transfer(transfer);
                CreditResponse::Transfer(result)
            }
            CreditCommand::GrantCredits { node, amount } => {
                let account = AccountId::node_account(*node);
                let current = self.balances.get(&account).copied().unwrap_or(Credits::ZERO);
                self.balances.insert(account, current.saturating_add(*amount));
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
    fn apply_transfer(
        &mut self,
        transfer: &univrs_enr::core::CreditTransfer,
    ) -> Result<(), TransferError> {
        let from_balance = self.get_balance(&transfer.from);
        let total_cost = transfer.amount.saturating_add(transfer.entropy_cost);

        // Check sufficient balance
        if from_balance.amount < total_cost.amount {
            return Err(TransferError::InsufficientCredits {
                available: from_balance,
                required: total_cost,
            });
        }

        // Debit sender
        self.balances
            .insert(transfer.from.clone(), from_balance.saturating_sub(total_cost));

        // Credit receiver
        let to_balance = self.get_balance(&transfer.to);
        self.balances
            .insert(transfer.to.clone(), to_balance.saturating_add(transfer.amount));

        // Add tax to revival pool
        self.revival_pool = self.revival_pool.saturating_add(transfer.entropy_cost);

        debug!(
            from = %transfer.from,
            to = %transfer.to,
            amount = transfer.amount.amount,
            tax = transfer.entropy_cost.amount,
            "Applied transfer"
        );

        Ok(())
    }

    /// Create a snapshot of current state
    fn snapshot(&self) -> CreditSnapshot {
        CreditSnapshot {
            balances: self.balances.clone(),
            revival_pool: self.revival_pool,
            last_applied: self.last_applied_log.as_ref().map(|l| l.index),
        }
    }

    /// Restore state from a snapshot
    fn restore(&mut self, snapshot: CreditSnapshot) {
        self.balances = snapshot.balances;
        self.revival_pool = snapshot.revival_pool;
        // Note: last_applied_log is set by the caller
    }
}

impl Default for CreditStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RaftStateMachine<CreditTypeConfig> for CreditStateMachine {
    type SnapshotBuilder = Self;

    async fn applied_state(
        &mut self,
    ) -> Result<(Option<LogId<u64>>, StoredMembership<CreditTypeConfig>), StorageError<u64>> {
        Ok((self.last_applied_log, self.last_membership.clone()))
    }

    async fn apply<I>(&mut self, entries: I) -> Result<Vec<CreditResponse>, StorageError<u64>>
    where
        I: IntoIterator<Item = Entry<CreditTypeConfig>> + Send,
    {
        let mut responses = Vec::new();

        for entry in entries {
            self.last_applied_log = Some(entry.log_id);

            match entry.payload {
                EntryPayload::Normal(command) => {
                    let response = self.apply_command(&command);
                    responses.push(response);
                }
                EntryPayload::Membership(membership) => {
                    self.last_membership = StoredMembership::new(Some(entry.log_id), membership);
                    responses.push(CreditResponse::Noop);
                }
                EntryPayload::Blank => {
                    responses.push(CreditResponse::Noop);
                }
            }
        }

        Ok(responses)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        // Clone self for snapshot building
        Self {
            balances: self.balances.clone(),
            revival_pool: self.revival_pool,
            last_applied_log: self.last_applied_log,
            last_membership: self.last_membership.clone(),
        }
    }

    async fn begin_receiving_snapshot(&mut self) -> Result<Box<Cursor<Vec<u8>>>, StorageError<u64>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<CreditTypeConfig>,
        snapshot: Box<Cursor<Vec<u8>>>,
    ) -> Result<(), StorageError<u64>> {
        let data = snapshot.into_inner();
        let credit_snapshot: CreditSnapshot = bincode::deserialize(&data)
            .map_err(|e| StorageError::IO {
                source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
            })?;

        self.restore(credit_snapshot);
        self.last_applied_log = meta.last_log_id;
        self.last_membership = meta.last_membership.clone();

        info!(
            last_log_id = ?meta.last_log_id,
            accounts = self.balances.len(),
            "Installed snapshot"
        );

        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<Snapshot<CreditTypeConfig>>, StorageError<u64>> {
        let snapshot = self.snapshot();
        let data = bincode::serialize(&snapshot).map_err(|e| StorageError::IO {
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })?;

        let meta = SnapshotMeta {
            last_log_id: self.last_applied_log,
            last_membership: self.last_membership.clone(),
            snapshot_id: format!(
                "{}-{}",
                self.last_applied_log.map(|l| l.index).unwrap_or(0),
                chrono::Utc::now().timestamp_millis()
            ),
        };

        Ok(Some(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(data)),
        }))
    }
}

#[async_trait]
impl RaftSnapshotBuilder<CreditTypeConfig> for CreditStateMachine {
    async fn build_snapshot(&mut self) -> Result<Snapshot<CreditTypeConfig>, StorageError<u64>> {
        let snapshot = self.snapshot();
        let data = bincode::serialize(&snapshot).map_err(|e| StorageError::IO {
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })?;

        let meta = SnapshotMeta {
            last_log_id: self.last_applied_log,
            last_membership: self.last_membership.clone(),
            snapshot_id: format!(
                "snapshot-{}-{}",
                self.last_applied_log.map(|l| l.index).unwrap_or(0),
                chrono::Utc::now().timestamp_millis()
            ),
        };

        Ok(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(data)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use univrs_enr::core::{CreditTransfer, NodeId};

    #[test]
    fn test_state_machine_grant() {
        let mut sm = CreditStateMachine::new();
        let node = NodeId::from_bytes([1u8; 32]);

        let response = sm.apply_command(&CreditCommand::GrantCredits {
            node,
            amount: Credits::new(1000),
        });

        assert!(matches!(response, CreditResponse::Grant));
        assert_eq!(
            sm.get_balance(&AccountId::node_account(node)).amount,
            1000
        );
    }

    #[test]
    fn test_state_machine_transfer() {
        let mut sm = CreditStateMachine::new();
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);

        // Grant initial credits
        sm.apply_command(&CreditCommand::GrantCredits {
            node: node1,
            amount: Credits::new(1000),
        });

        // Create transfer
        let transfer = CreditTransfer::new(
            AccountId::node_account(node1),
            AccountId::node_account(node2),
            Credits::new(100),
            calculate_entropy_tax(Credits::new(100)),
        );

        let response = sm.apply_command(&CreditCommand::Transfer(transfer));
        assert!(matches!(response, CreditResponse::Transfer(Ok(()))));

        // Check balances
        assert_eq!(
            sm.get_balance(&AccountId::node_account(node1)).amount,
            898 // 1000 - 100 - 2 tax
        );
        assert_eq!(
            sm.get_balance(&AccountId::node_account(node2)).amount,
            100
        );
        assert_eq!(sm.revival_pool.amount, 2);
    }

    #[test]
    fn test_state_machine_insufficient() {
        let mut sm = CreditStateMachine::new();
        let node1 = NodeId::from_bytes([1u8; 32]);
        let node2 = NodeId::from_bytes([2u8; 32]);

        // Grant only 50 credits
        sm.apply_command(&CreditCommand::GrantCredits {
            node: node1,
            amount: Credits::new(50),
        });

        // Try to transfer 100
        let transfer = CreditTransfer::new(
            AccountId::node_account(node1),
            AccountId::node_account(node2),
            Credits::new(100),
            calculate_entropy_tax(Credits::new(100)),
        );

        let response = sm.apply_command(&CreditCommand::Transfer(transfer));
        assert!(matches!(
            response,
            CreditResponse::Transfer(Err(TransferError::InsufficientCredits { .. }))
        ));
    }

    #[test]
    fn test_snapshot_roundtrip() {
        let mut sm = CreditStateMachine::new();
        let node = NodeId::from_bytes([1u8; 32]);

        sm.apply_command(&CreditCommand::GrantCredits {
            node,
            amount: Credits::new(1000),
        });

        // Create snapshot
        let snapshot = sm.snapshot();
        let data = bincode::serialize(&snapshot).unwrap();

        // Create new state machine and restore
        let mut sm2 = CreditStateMachine::new();
        let restored: CreditSnapshot = bincode::deserialize(&data).unwrap();
        sm2.restore(restored);

        // Verify state matches
        assert_eq!(
            sm2.get_balance(&AccountId::node_account(node)).amount,
            1000
        );
    }
}
