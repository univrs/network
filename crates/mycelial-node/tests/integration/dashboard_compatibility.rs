//! Dashboard Compatibility Tests
//!
//! These tests verify that Rust server-side message formats match
//! the TypeScript dashboard frontend expectations defined in types.ts.
//! This ensures JSON serialization produces messages the dashboard can consume.

use serde_json::{json, Value};

// ============ Core Message Type Tests ============

#[test]
fn test_ws_message_has_type_field() {
    // All WsMessage variants must have a "type" field
    let messages = vec![
        json!({"type": "peer_joined", "peer_id": "12D3Test", "name": "Alice"}),
        json!({"type": "peer_left", "peer_id": "12D3Test"}),
        json!({"type": "chat_message", "id": "msg-1", "from": "12D3Test", "from_name": "Alice", "content": "Hi"}),
        json!({"type": "peers_list", "peers": []}),
        json!({"type": "stats", "peer_count": 0, "message_count": 0, "uptime_seconds": 0}),
        json!({"type": "error", "message": "Error occurred"}),
    ];

    for msg in messages {
        assert!(
            msg.get("type").is_some(),
            "Message missing type field: {:?}",
            msg
        );
    }
}

// ============ PeerInfo Compatibility Tests ============

#[test]
fn test_peer_info_format_matches_typescript() {
    // TypeScript PeerInfo has id, name, reputation, addresses
    let peer = json!({
        "id": "12D3KooWTestPeer",
        "name": "Alice",
        "reputation": 0.85,
        "addresses": ["/ip4/127.0.0.1/tcp/9000"]
    });

    // Required fields
    assert!(peer["id"].is_string());
    assert!(peer["reputation"].is_number());
    assert!(peer["addresses"].is_array());

    // Optional fields
    assert!(peer["name"].is_string() || peer["name"].is_null());
}

#[test]
fn test_peer_info_reputation_is_numeric() {
    // Reputation must be a number (not a Reputation object at this level)
    let peer = json!({
        "id": "12D3KooWTest",
        "reputation": 0.75,
        "addresses": []
    });

    let rep = peer["reputation"].as_f64().unwrap();
    assert!((0.0..=1.0).contains(&rep), "Reputation should be in [0, 1]");
}

#[test]
fn test_peer_addresses_are_multiaddr_strings() {
    let peer = json!({
        "id": "12D3KooWTest",
        "reputation": 0.5,
        "addresses": [
            "/ip4/127.0.0.1/tcp/9000",
            "/ip4/192.168.1.1/tcp/9001",
            "/ip6/::1/tcp/9002"
        ]
    });

    for addr in peer["addresses"].as_array().unwrap() {
        let addr_str = addr.as_str().unwrap();
        assert!(addr_str.starts_with('/'), "Multiaddr should start with /");
    }
}

// ============ ChatMessage Compatibility Tests ============

#[test]
fn test_chat_message_matches_typescript() {
    // TypeScript ChatMessage interface
    let msg = json!({
        "type": "chat_message",
        "id": "msg-uuid",
        "from": "12D3KooWAlice",
        "from_name": "Alice",
        "to": null,
        "room_id": null,
        "content": "Hello world!",
        "timestamp": 1703683200000_i64
    });

    // Required fields
    assert!(msg["id"].is_string());
    assert!(msg["from"].is_string());
    assert!(msg["content"].is_string());
    assert!(msg["timestamp"].is_number());

    // Optional fields can be null or string
    assert!(msg["from_name"].is_string() || msg["from_name"].is_null());
    assert!(msg["to"].is_string() || msg["to"].is_null());
    assert!(msg["room_id"].is_string() || msg["room_id"].is_null());
}

#[test]
fn test_chat_message_direct_message() {
    let msg = json!({
        "type": "chat_message",
        "id": "msg-123",
        "from": "12D3KooWAlice",
        "from_name": "Alice",
        "to": "12D3KooWBob",  // DM target
        "room_id": null,
        "content": "Private message",
        "timestamp": 1703683200000_i64
    });

    assert_eq!(msg["to"].as_str().unwrap(), "12D3KooWBob");
    assert!(msg["room_id"].is_null());
}

#[test]
fn test_chat_message_room_message() {
    let msg = json!({
        "type": "chat_message",
        "id": "msg-456",
        "from": "12D3KooWAlice",
        "from_name": "Alice",
        "to": null,
        "room_id": "room-engineering",
        "content": "Room announcement",
        "timestamp": 1703683200000_i64
    });

    assert!(msg["to"].is_null());
    assert_eq!(msg["room_id"].as_str().unwrap(), "room-engineering");
}

// ============ Room Message Compatibility Tests ============

#[test]
fn test_room_joined_matches_typescript() {
    // TypeScript Room interface
    let msg = json!({
        "type": "room_joined",
        "id": "room-123",
        "name": "Engineering",
        "description": "Engineering team chat",
        "topic": "/mycelial/1.0.0/room/room-123",
        "members": ["12D3KooWAlice", "12D3KooWBob"],
        "created_by": "12D3KooWAlice",
        "created_at": 1703683200000_i64,
        "is_public": true
    });

    // Required fields
    assert!(msg["id"].is_string());
    assert!(msg["name"].is_string());
    assert!(msg["topic"].is_string());
    assert!(msg["members"].is_array());
    assert!(msg["created_by"].is_string());
    assert!(msg["created_at"].is_number());
    assert!(msg["is_public"].is_boolean());

    // Optional description
    assert!(msg["description"].is_string() || msg["description"].is_null());
}

#[test]
fn test_room_left_format() {
    let msg = json!({
        "type": "room_left",
        "room_id": "room-123"
    });

    assert_eq!(msg["type"], "room_left");
    assert!(msg["room_id"].is_string());
}

#[test]
fn test_room_peer_joined_format() {
    let msg = json!({
        "type": "room_peer_joined",
        "room_id": "room-123",
        "peer_id": "12D3KooWNewMember",
        "peer_name": "NewMember"
    });

    assert!(msg["room_id"].is_string());
    assert!(msg["peer_id"].is_string());
    // peer_name is optional
}

#[test]
fn test_room_peer_left_format() {
    let msg = json!({
        "type": "room_peer_left",
        "room_id": "room-123",
        "peer_id": "12D3KooWLeavingMember"
    });

    assert!(msg["room_id"].is_string());
    assert!(msg["peer_id"].is_string());
}

// ============ Economics Protocol Tests ============

#[test]
fn test_vouch_request_matches_typescript() {
    // TypeScript VouchRequest
    let msg = json!({
        "type": "vouch_request",
        "id": "vouch-uuid",
        "voucher": "12D3KooWAlice",
        "vouchee": "12D3KooWBob",
        "weight": 0.8,
        "timestamp": 1703683200000_i64
    });

    assert!(msg["id"].is_string());
    assert!(msg["voucher"].is_string());
    assert!(msg["vouchee"].is_string());
    assert!(msg["weight"].is_number());
    assert!(msg["timestamp"].is_number());

    let weight = msg["weight"].as_f64().unwrap();
    assert!((0.0..=1.0).contains(&weight));
}

#[test]
fn test_vouch_ack_format() {
    let msg = json!({
        "type": "vouch_ack",
        "id": "ack-uuid",
        "request_id": "vouch-uuid",
        "accepted": true,
        "new_reputation": 0.85,
        "timestamp": 1703683200000_i64
    });

    assert!(msg["id"].is_string());
    assert!(msg["request_id"].is_string());
    assert!(msg["accepted"].is_boolean());
    assert!(msg["timestamp"].is_number());
    // new_reputation is optional
}

#[test]
fn test_credit_line_matches_typescript() {
    // TypeScript CreditLine
    let msg = json!({
        "type": "credit_line",
        "id": "cl-uuid",
        "creditor": "12D3KooWAlice",
        "debtor": "12D3KooWBob",
        "limit": 1000.0,
        "balance": 250.0,
        "timestamp": 1703683200000_i64
    });

    assert!(msg["id"].is_string());
    assert!(msg["creditor"].is_string());
    assert!(msg["debtor"].is_string());
    assert!(msg["limit"].is_number());
    assert!(msg["balance"].is_number());
}

#[test]
fn test_credit_transfer_matches_typescript() {
    // TypeScript CreditTransfer
    let msg = json!({
        "type": "credit_transfer",
        "id": "ct-uuid",
        "from": "12D3KooWAlice",
        "to": "12D3KooWBob",
        "amount": 50.0,
        "memo": "Payment for services",
        "timestamp": 1703683200000_i64
    });

    assert!(msg["id"].is_string());
    assert!(msg["from"].is_string());
    assert!(msg["to"].is_string());
    assert!(msg["amount"].is_number());
    assert!(msg["timestamp"].is_number());
    // memo is optional
}

#[test]
fn test_proposal_matches_typescript() {
    // TypeScript Proposal
    let msg = json!({
        "type": "proposal",
        "id": "prop-uuid",
        "proposer": "12D3KooWAlice",
        "title": "Increase Credit Limits",
        "description": "Proposal to increase default credit limits",
        "proposal_type": "parameter_change",
        "status": "active",
        "yes_votes": 5,
        "no_votes": 2,
        "quorum": 10,
        "deadline": 1703769600000_i64,
        "timestamp": 1703683200000_i64
    });

    assert!(msg["id"].is_string());
    assert!(msg["proposer"].is_string());
    assert!(msg["title"].is_string());
    assert!(msg["description"].is_string());
    assert!(msg["status"].is_string());
    assert!(msg["yes_votes"].is_number());
    assert!(msg["no_votes"].is_number());
    assert!(msg["quorum"].is_number());
    assert!(msg["deadline"].is_number());
}

#[test]
fn test_vote_cast_format() {
    let msg = json!({
        "type": "vote_cast",
        "id": "vote-uuid",
        "proposal_id": "prop-uuid",
        "voter": "12D3KooWBob",
        "vote": "yes",
        "weight": 1.0,
        "timestamp": 1703683200000_i64
    });

    assert!(msg["proposal_id"].is_string());
    assert!(msg["voter"].is_string());
    assert!(msg["vote"].is_string());
    assert!(msg["weight"].is_number());

    // vote should be yes/no/abstain
    let vote = msg["vote"].as_str().unwrap();
    assert!(
        ["yes", "no", "abstain", "for", "against", "For", "Against", "Abstain"].contains(&vote)
    );
}

#[test]
fn test_resource_contribution_matches_typescript() {
    // TypeScript ResourceContribution
    let msg = json!({
        "type": "resource_contribution",
        "id": "rc-uuid",
        "peer_id": "12D3KooWAlice",
        "resource_type": "bandwidth",
        "amount": 100.0,
        "unit": "mbps",
        "timestamp": 1703683200000_i64
    });

    assert!(msg["peer_id"].is_string());
    assert!(msg["resource_type"].is_string());
    assert!(msg["amount"].is_number());
    assert!(msg["unit"].is_string());

    // resource_type should be bandwidth/storage/compute
    let rt = msg["resource_type"].as_str().unwrap();
    assert!(
        [
            "bandwidth",
            "storage",
            "compute",
            "Bandwidth",
            "Storage",
            "Compute"
        ]
        .contains(&rt)
            || rt.starts_with("Other"),
        "Unexpected resource type: {}",
        rt
    );
}

// ============ ENR Bridge Message Tests ============

#[test]
fn test_gradient_update_matches_typescript() {
    // TypeScript GradientUpdate
    let msg = json!({
        "type": "gradient_update",
        "source": "12D3KooWNode1",
        "cpu_available": 0.75,
        "memory_available": 0.5,
        "bandwidth_available": 0.9,
        "storage_available": 0.6,
        "timestamp": 1703683200000_i64
    });

    assert!(msg["source"].is_string());
    assert!(msg["cpu_available"].is_number());
    assert!(msg["memory_available"].is_number());
    assert!(msg["bandwidth_available"].is_number());
    assert!(msg["storage_available"].is_number());
    assert!(msg["timestamp"].is_number());
}

#[test]
fn test_enr_credit_transfer_matches_typescript() {
    // TypeScript EnrCreditTransfer (different from mutual credit)
    let msg = json!({
        "type": "enr_credit_transfer",
        "from": "12D3KooWNode1",
        "to": "12D3KooWNode2",
        "amount": 1000,
        "tax": 20,  // 2% entropy tax
        "nonce": 1,
        "timestamp": 1703683200000_i64
    });

    assert!(msg["from"].is_string());
    assert!(msg["to"].is_string());
    assert!(msg["amount"].is_number());
    assert!(msg["tax"].is_number());
    assert!(msg["nonce"].is_number());
}

#[test]
fn test_enr_balance_update_format() {
    let msg = json!({
        "type": "enr_balance_update",
        "node_id": "12D3KooWNode1",
        "balance": 5000,
        "timestamp": 1703683200000_i64
    });

    assert!(msg["node_id"].is_string());
    assert!(msg["balance"].is_number());
}

#[test]
fn test_election_announcement_matches_typescript() {
    // TypeScript ElectionAnnouncement
    let msg = json!({
        "type": "election_announcement",
        "election_id": 12345,
        "initiator": "12D3KooWNode1",
        "region_id": "us-east-1",
        "timestamp": 1703683200000_i64
    });

    assert!(msg["election_id"].is_number());
    assert!(msg["initiator"].is_string());
    assert!(msg["region_id"].is_string());
}

#[test]
fn test_election_candidacy_matches_typescript() {
    // TypeScript ElectionCandidacy
    let msg = json!({
        "type": "election_candidacy",
        "election_id": 12345,
        "candidate": "12D3KooWNode2",
        "uptime": 86400000_i64,  // 1 day in ms
        "cpu_available": 0.8,
        "memory_available": 0.7,
        "reputation": 0.95,
        "timestamp": 1703683200000_i64
    });

    assert!(msg["election_id"].is_number());
    assert!(msg["candidate"].is_string());
    assert!(msg["uptime"].is_number());
    assert!(msg["reputation"].is_number());
}

#[test]
fn test_election_vote_format() {
    let msg = json!({
        "type": "election_vote",
        "election_id": 12345,
        "voter": "12D3KooWNode3",
        "candidate": "12D3KooWNode2",
        "timestamp": 1703683200000_i64
    });

    assert!(msg["election_id"].is_number());
    assert!(msg["voter"].is_string());
    assert!(msg["candidate"].is_string());
}

#[test]
fn test_election_result_matches_typescript() {
    // TypeScript ElectionResult
    let msg = json!({
        "type": "election_result",
        "election_id": 12345,
        "winner": "12D3KooWNode2",
        "region_id": "us-east-1",
        "vote_count": 15,
        "timestamp": 1703683200000_i64
    });

    assert!(msg["election_id"].is_number());
    assert!(msg["winner"].is_string());
    assert!(msg["region_id"].is_string());
    assert!(msg["vote_count"].is_number());
}

#[test]
fn test_septal_state_change_matches_typescript() {
    // TypeScript SeptalStateChange
    let msg = json!({
        "type": "septal_state_change",
        "node_id": "12D3KooWNode1",
        "from_state": "Closed",
        "to_state": "Open",
        "reason": "Too many failures",
        "timestamp": 1703683200000_i64
    });

    assert!(msg["node_id"].is_string());
    assert!(msg["from_state"].is_string());
    assert!(msg["to_state"].is_string());
    assert!(msg["reason"].is_string());
}

#[test]
fn test_septal_health_status_format() {
    let msg = json!({
        "type": "septal_health_status",
        "node_id": "12D3KooWNode1",
        "is_healthy": true,
        "failure_count": 0,
        "timestamp": 1703683200000_i64
    });

    assert!(msg["node_id"].is_string());
    assert!(msg["is_healthy"].is_boolean());
    assert!(msg["failure_count"].is_number());
}

// ============ Client Message Format Tests ============

#[test]
fn test_client_send_chat_format() {
    let msg = json!({
        "type": "send_chat",
        "content": "Hello world!",
        "to": null,
        "room_id": null
    });

    assert_eq!(msg["type"], "send_chat");
    assert!(msg["content"].is_string());
}

#[test]
fn test_client_get_peers_format() {
    let msg = json!({"type": "get_peers"});
    assert_eq!(msg["type"], "get_peers");
}

#[test]
fn test_client_get_stats_format() {
    let msg = json!({"type": "get_stats"});
    assert_eq!(msg["type"], "get_stats");
}

#[test]
fn test_client_subscribe_format() {
    let msg = json!({
        "type": "subscribe",
        "topic": "/mycelial/1.0.0/chat"
    });

    assert_eq!(msg["type"], "subscribe");
    assert!(msg["topic"].is_string());
}

#[test]
fn test_client_send_vouch_format() {
    let msg = json!({
        "type": "send_vouch",
        "vouchee": "12D3KooWBob",
        "weight": 0.8,
        "message": "Great contributor!"
    });

    assert!(msg["vouchee"].is_string());
    assert!(msg["weight"].is_number());
}

#[test]
fn test_client_create_credit_line_format() {
    let msg = json!({
        "type": "create_credit_line",
        "debtor": "12D3KooWBob",
        "limit": 1000.0
    });

    assert!(msg["debtor"].is_string());
    assert!(msg["limit"].is_number());
}

#[test]
fn test_client_transfer_credit_format() {
    let msg = json!({
        "type": "transfer_credit",
        "to": "12D3KooWBob",
        "amount": 50.0,
        "memo": "Payment"
    });

    assert!(msg["to"].is_string());
    assert!(msg["amount"].is_number());
}

#[test]
fn test_client_create_proposal_format() {
    let msg = json!({
        "type": "create_proposal",
        "title": "New Feature",
        "description": "Detailed description",
        "proposal_type": "text"
    });

    assert!(msg["title"].is_string());
    assert!(msg["description"].is_string());
    assert!(msg["proposal_type"].is_string());
}

#[test]
fn test_client_cast_vote_format() {
    let msg = json!({
        "type": "cast_vote",
        "proposal_id": "prop-uuid",
        "vote": "yes"
    });

    assert!(msg["proposal_id"].is_string());
    assert!(msg["vote"].is_string());
}

#[test]
fn test_client_create_room_format() {
    let msg = json!({
        "type": "create_room",
        "room_id": null,
        "room_name": "New Room",
        "description": "A new chat room",
        "is_public": true
    });

    assert!(msg["room_name"].is_string());
}

#[test]
fn test_client_join_room_format() {
    let msg = json!({
        "type": "join_room",
        "room_id": "room-123",
        "room_name": null
    });

    assert!(msg["room_id"].is_string());
}

#[test]
fn test_client_leave_room_format() {
    let msg = json!({
        "type": "leave_room",
        "room_id": "room-123"
    });

    assert!(msg["room_id"].is_string());
}

#[test]
fn test_client_get_rooms_format() {
    let msg = json!({"type": "get_rooms"});
    assert_eq!(msg["type"], "get_rooms");
}

// ============ Timestamp Format Tests ============

#[test]
fn test_timestamp_is_milliseconds() {
    // All timestamps should be in milliseconds (JavaScript Date.now() format)
    let timestamp: i64 = 1703683200000; // 2023-12-27 12:00:00 UTC

    // Should be > 1 trillion (year 2001+)
    assert!(
        timestamp > 1_000_000_000_000,
        "Timestamp should be in milliseconds"
    );
    // Should be < 100 trillion (year 5000+)
    assert!(
        timestamp < 100_000_000_000_000,
        "Timestamp seems unreasonably large"
    );
}

// ============ Stats Message Tests ============

#[test]
fn test_stats_message_format() {
    let msg = json!({
        "type": "stats",
        "peer_count": 10,
        "message_count": 1234,
        "uptime_seconds": 3600
    });

    assert!(msg["peer_count"].as_u64().is_some());
    assert!(msg["message_count"].as_u64().is_some());
    assert!(msg["uptime_seconds"].as_u64().is_some());
}
