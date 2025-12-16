-- Initial schema for mycelial-state SQLite database
-- Version: 001

-- Peers table: stores information about known peers in the network
CREATE TABLE IF NOT EXISTS peers (
    peer_id TEXT PRIMARY KEY,
    public_key BLOB NOT NULL,
    display_name TEXT,
    addresses_json TEXT NOT NULL DEFAULT '[]',
    location_json TEXT,
    reputation_score REAL NOT NULL DEFAULT 0.5,
    successful_interactions INTEGER NOT NULL DEFAULT 0,
    failed_interactions INTEGER NOT NULL DEFAULT 0,
    reputation_history_json TEXT NOT NULL DEFAULT '[]',
    first_seen INTEGER NOT NULL,
    last_seen INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Messages table: stores network messages for history and replay
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    message_type TEXT NOT NULL,
    sender_peer_id TEXT NOT NULL,
    recipient_peer_id TEXT,
    payload BLOB NOT NULL,
    signature BLOB,
    timestamp INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (sender_peer_id) REFERENCES peers(peer_id)
);

-- Credit relationships table: stores mutual credit relationships between peers
CREATE TABLE IF NOT EXISTS credit_relationships (
    id TEXT PRIMARY KEY,
    creditor_peer_id TEXT NOT NULL,
    debtor_peer_id TEXT NOT NULL,
    credit_limit REAL NOT NULL DEFAULT 0.0,
    balance REAL NOT NULL DEFAULT 0.0,
    active INTEGER NOT NULL DEFAULT 1,
    established INTEGER NOT NULL,
    last_transaction INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (creditor_peer_id) REFERENCES peers(peer_id),
    FOREIGN KEY (debtor_peer_id) REFERENCES peers(peer_id),
    UNIQUE(creditor_peer_id, debtor_peer_id)
);

-- Credit transactions table: stores history of credit transactions
CREATE TABLE IF NOT EXISTS credit_transactions (
    id TEXT PRIMARY KEY,
    relationship_id TEXT NOT NULL,
    amount REAL NOT NULL,
    balance_after REAL NOT NULL,
    description TEXT,
    timestamp INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (relationship_id) REFERENCES credit_relationships(id)
);

-- Key-value store for generic state sync
CREATE TABLE IF NOT EXISTS state_sync (
    key TEXT PRIMARY KEY,
    value BLOB NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender_peer_id);
CREATE INDEX IF NOT EXISTS idx_messages_recipient ON messages(recipient_peer_id);
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
CREATE INDEX IF NOT EXISTS idx_messages_type ON messages(message_type);

CREATE INDEX IF NOT EXISTS idx_credit_creditor ON credit_relationships(creditor_peer_id);
CREATE INDEX IF NOT EXISTS idx_credit_debtor ON credit_relationships(debtor_peer_id);
CREATE INDEX IF NOT EXISTS idx_credit_active ON credit_relationships(active);

CREATE INDEX IF NOT EXISTS idx_peers_last_seen ON peers(last_seen);
CREATE INDEX IF NOT EXISTS idx_peers_reputation ON peers(reputation_score);

CREATE INDEX IF NOT EXISTS idx_transactions_relationship ON credit_transactions(relationship_id);
CREATE INDEX IF NOT EXISTS idx_transactions_timestamp ON credit_transactions(timestamp);
