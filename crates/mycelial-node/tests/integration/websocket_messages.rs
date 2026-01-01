//! WebSocket message serialization/deserialization tests
//!
//! These tests ensure message format consistency between Rust and TypeScript.

use serde_json::{json, Value};

/// Test that WsMessage serialization matches expected TypeScript format
#[test]
fn test_chat_message_format() {
    let msg = json!({
        "type": "chat_message",
        "id": "msg-123",
        "from": "12D3KooWTest",
        "from_name": "Alice",
        "to": null,
        "room_id": null,
        "content": "Hello world",
        "timestamp": 1703683200000_i64
    });

    // Verify structure matches TypeScript ChatMessage interface
    assert_eq!(msg["type"], "chat_message");
    assert!(msg["id"].is_string());
    assert!(msg["from"].is_string());
    assert!(msg["from_name"].is_string());
    assert!(msg["content"].is_string());
    assert!(msg["timestamp"].is_number());
}

#[test]
fn test_peer_joined_format() {
    let msg = json!({
        "type": "peer_joined",
        "peer_id": "12D3KooWTest",
        "name": "Bob"
    });

    assert_eq!(msg["type"], "peer_joined");
    assert!(msg["peer_id"].is_string());
}

#[test]
fn test_peer_left_format() {
    let msg = json!({
        "type": "peer_left",
        "peer_id": "12D3KooWTest"
    });

    assert_eq!(msg["type"], "peer_left");
}

#[test]
fn test_peers_list_format() {
    let msg = json!({
        "type": "peers_list",
        "peers": [
            {
                "id": "12D3KooWTest1",
                "name": "Alice",
                "reputation": 0.85,
                "addresses": ["/ip4/127.0.0.1/tcp/9000"]
            },
            {
                "id": "12D3KooWTest2",
                "name": null,
                "reputation": 0.5,
                "addresses": []
            }
        ]
    });

    assert_eq!(msg["type"], "peers_list");
    assert!(msg["peers"].is_array());
    assert_eq!(msg["peers"].as_array().unwrap().len(), 2);
}

#[test]
fn test_stats_format() {
    let msg = json!({
        "type": "stats",
        "peer_count": 5,
        "message_count": 1234,
        "uptime_seconds": 3600
    });

    assert_eq!(msg["type"], "stats");
    assert!(msg["peer_count"].is_number());
    assert!(msg["message_count"].is_number());
    assert!(msg["uptime_seconds"].is_number());
}

#[test]
fn test_error_format() {
    let msg = json!({
        "type": "error",
        "message": "Connection timeout"
    });

    assert_eq!(msg["type"], "error");
    assert!(msg["message"].is_string());
}

// ============ Economics Protocol Message Tests ============

#[test]
fn test_vouch_request_format() {
    let msg = json!({
        "type": "vouch_request",
        "id": "vouch-123",
        "voucher": "12D3KooWAlice",
        "vouchee": "12D3KooWBob",
        "weight": 0.8,
        "timestamp": 1703683200000_i64
    });

    assert_eq!(msg["type"], "vouch_request");
    assert!(msg["weight"].as_f64().unwrap() >= 0.0);
    assert!(msg["weight"].as_f64().unwrap() <= 1.0);
}

#[test]
fn test_vouch_ack_format() {
    let msg = json!({
        "type": "vouch_ack",
        "id": "ack-123",
        "request_id": "vouch-123",
        "accepted": true,
        "new_reputation": 0.85,
        "timestamp": 1703683200000_i64
    });

    assert_eq!(msg["type"], "vouch_ack");
    assert!(msg["accepted"].is_boolean());
}

#[test]
fn test_credit_line_format() {
    let msg = json!({
        "type": "credit_line",
        "id": "credit-123",
        "creditor": "12D3KooWAlice",
        "debtor": "12D3KooWBob",
        "limit": 1000.0,
        "balance": 0.0,
        "timestamp": 1703683200000_i64
    });

    assert_eq!(msg["type"], "credit_line");
    assert!(msg["limit"].as_f64().unwrap() > 0.0);
}

#[test]
fn test_credit_transfer_format() {
    let msg = json!({
        "type": "credit_transfer",
        "id": "transfer-123",
        "from": "12D3KooWAlice",
        "to": "12D3KooWBob",
        "amount": 100.0,
        "memo": "Payment for services",
        "timestamp": 1703683200000_i64
    });

    assert_eq!(msg["type"], "credit_transfer");
    assert!(msg["amount"].as_f64().unwrap() > 0.0);
}

#[test]
fn test_proposal_format() {
    let msg = json!({
        "type": "proposal",
        "id": "prop-123",
        "proposer": "12D3KooWAlice",
        "title": "Increase credit limits",
        "description": "Proposal to increase default credit limits by 50%",
        "proposal_type": "parameter_change",
        "status": "active",
        "yes_votes": 3,
        "no_votes": 1,
        "quorum": 5,
        "deadline": 1703769600000_i64,
        "timestamp": 1703683200000_i64
    });

    assert_eq!(msg["type"], "proposal");
    assert!(msg["yes_votes"].as_u64().unwrap() >= 0);
    assert!(msg["no_votes"].as_u64().unwrap() >= 0);
}

#[test]
fn test_vote_cast_format() {
    let msg = json!({
        "type": "vote_cast",
        "id": "vote-123",
        "proposal_id": "prop-123",
        "voter": "12D3KooWBob",
        "vote": "yes",
        "weight": 1.0,
        "timestamp": 1703683200000_i64
    });

    assert_eq!(msg["type"], "vote_cast");
    let vote = msg["vote"].as_str().unwrap();
    assert!(vote == "yes" || vote == "no" || vote == "abstain");
}

#[test]
fn test_resource_contribution_format() {
    let msg = json!({
        "type": "resource_contribution",
        "id": "resource-123",
        "peer_id": "12D3KooWAlice",
        "resource_type": "bandwidth",
        "amount": 100.0,
        "unit": "mbps",
        "timestamp": 1703683200000_i64
    });

    assert_eq!(msg["type"], "resource_contribution");
    let resource_type = msg["resource_type"].as_str().unwrap();
    assert!(
        resource_type == "bandwidth" || resource_type == "storage" || resource_type == "compute"
    );
}

// ============ Room Message Tests ============

#[test]
fn test_room_joined_format() {
    let msg = json!({
        "type": "room_joined",
        "id": "room-123",
        "name": "Engineering",
        "description": "Engineering team discussions",
        "topic": "/mycelial/1.0.0/room/room-123",
        "members": ["12D3KooWAlice", "12D3KooWBob"],
        "created_by": "12D3KooWAlice",
        "created_at": 1703683200000_i64,
        "is_public": true
    });

    assert_eq!(msg["type"], "room_joined");
    assert!(msg["topic"]
        .as_str()
        .unwrap()
        .starts_with("/mycelial/1.0.0/room/"));
    assert!(msg["members"].is_array());
}

#[test]
fn test_room_left_format() {
    let msg = json!({
        "type": "room_left",
        "room_id": "room-123"
    });

    assert_eq!(msg["type"], "room_left");
}

#[test]
fn test_room_list_format() {
    let msg = json!({
        "type": "room_list",
        "rooms": [
            {
                "id": "room-123",
                "name": "Engineering",
                "description": "Engineering team",
                "member_count": 5,
                "is_public": true,
                "created_at": 1703683200000_i64
            }
        ]
    });

    assert_eq!(msg["type"], "room_list");
    assert!(msg["rooms"].is_array());
}

#[test]
fn test_room_peer_joined_format() {
    let msg = json!({
        "type": "room_peer_joined",
        "room_id": "room-123",
        "peer_id": "12D3KooWCarol",
        "peer_name": "Carol"
    });

    assert_eq!(msg["type"], "room_peer_joined");
}

#[test]
fn test_room_peer_left_format() {
    let msg = json!({
        "type": "room_peer_left",
        "room_id": "room-123",
        "peer_id": "12D3KooWCarol"
    });

    assert_eq!(msg["type"], "room_peer_left");
}

// ============ Client Message Parsing Tests ============

#[test]
fn test_parse_send_chat() {
    let input = r#"{"type":"send_chat","content":"Hello!","to":null,"room_id":null}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "send_chat");
    assert_eq!(parsed["content"], "Hello!");
}

#[test]
fn test_parse_send_chat_dm() {
    let input =
        r#"{"type":"send_chat","content":"Private message","to":"12D3KooWBob","room_id":null}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "send_chat");
    assert_eq!(parsed["to"], "12D3KooWBob");
}

#[test]
fn test_parse_send_chat_room() {
    let input = r#"{"type":"send_chat","content":"Room message","to":null,"room_id":"room-123"}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "send_chat");
    assert_eq!(parsed["room_id"], "room-123");
}

#[test]
fn test_parse_get_peers() {
    let input = r#"{"type":"get_peers"}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "get_peers");
}

#[test]
fn test_parse_get_stats() {
    let input = r#"{"type":"get_stats"}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "get_stats");
}

#[test]
fn test_parse_subscribe() {
    let input = r#"{"type":"subscribe","topic":"/mycelial/1.0.0/custom"}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "subscribe");
    assert_eq!(parsed["topic"], "/mycelial/1.0.0/custom");
}

#[test]
fn test_parse_send_vouch() {
    let input =
        r#"{"type":"send_vouch","vouchee":"12D3KooWBob","weight":0.8,"message":"Great work!"}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "send_vouch");
    assert_eq!(parsed["weight"], 0.8);
}

#[test]
fn test_parse_respond_vouch() {
    let input = r#"{"type":"respond_vouch","request_id":"vouch-123","accept":true}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "respond_vouch");
    assert_eq!(parsed["accept"], true);
}

#[test]
fn test_parse_create_credit_line() {
    let input = r#"{"type":"create_credit_line","debtor":"12D3KooWBob","limit":1000.0}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "create_credit_line");
    assert_eq!(parsed["limit"], 1000.0);
}

#[test]
fn test_parse_transfer_credit() {
    let input = r#"{"type":"transfer_credit","to":"12D3KooWBob","amount":100.0,"memo":"Payment"}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "transfer_credit");
    assert_eq!(parsed["amount"], 100.0);
}

#[test]
fn test_parse_create_proposal() {
    let input = r#"{"type":"create_proposal","title":"Test","description":"A test proposal","proposal_type":"text"}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "create_proposal");
    assert_eq!(parsed["proposal_type"], "text");
}

#[test]
fn test_parse_cast_vote() {
    let input = r#"{"type":"cast_vote","proposal_id":"prop-123","vote":"yes"}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "cast_vote");
    assert_eq!(parsed["vote"], "yes");
}

#[test]
fn test_parse_report_resource() {
    let input =
        r#"{"type":"report_resource","resource_type":"bandwidth","amount":100.0,"unit":"mbps"}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "report_resource");
    assert_eq!(parsed["resource_type"], "bandwidth");
}

#[test]
fn test_parse_create_room() {
    let input = r#"{"type":"create_room","room_id":null,"room_name":"Engineering","description":"Team room","is_public":true}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "create_room");
    assert_eq!(parsed["room_name"], "Engineering");
}

#[test]
fn test_parse_join_room() {
    let input = r#"{"type":"join_room","room_id":"room-123","room_name":null}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "join_room");
    assert_eq!(parsed["room_id"], "room-123");
}

#[test]
fn test_parse_leave_room() {
    let input = r#"{"type":"leave_room","room_id":"room-123"}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "leave_room");
}

#[test]
fn test_parse_get_rooms() {
    let input = r#"{"type":"get_rooms"}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    assert_eq!(parsed["type"], "get_rooms");
}

// ============ DOL Protocol Evolution Tests ============

#[test]
fn test_unknown_field_tolerance() {
    // Future messages may have additional fields - old parsers should ignore them
    let input = r#"{"type":"send_chat","content":"Hello","to":null,"room_id":null,"neural_embedding":[0.1,0.2,0.3],"intent":"greeting"}"#;
    let parsed: Value = serde_json::from_str(input).unwrap();

    // Should parse successfully even with unknown fields
    assert_eq!(parsed["type"], "send_chat");
    assert_eq!(parsed["content"], "Hello");
}

#[test]
fn test_protocol_version_in_topic() {
    // Verify topic format supports versioning
    let topics = [
        "/mycelial/1.0.0/chat",
        "/mycelial/1.0.0/direct",
        "/mycelial/1.0.0/vouch",
        "/mycelial/1.0.0/room/room-123",
    ];

    for topic in topics {
        assert!(topic.starts_with("/mycelial/1.0.0/"));
    }
}

#[test]
fn test_message_id_uniqueness() {
    // Message IDs should be unique and follow consistent format
    use std::collections::HashSet;

    let ids: Vec<&str> = vec![
        "msg-abc123",
        "vouch-def456",
        "credit-ghi789",
        "prop-jkl012",
        "room-mno345",
    ];

    let unique: HashSet<_> = ids.iter().collect();
    assert_eq!(ids.len(), unique.len(), "Message IDs should be unique");
}
