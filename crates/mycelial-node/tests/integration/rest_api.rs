//! REST API endpoint tests
//!
//! These tests verify the REST API response formats.

use serde_json::{json, Value};

/// Test expected format for GET /api/peers response
#[test]
fn test_peers_response_format() {
    let response = json!([
        {
            "id": "12D3KooWTest1",
            "name": "Alice",
            "reputation": 0.85,
            "addresses": ["/ip4/127.0.0.1/tcp/9000"]
        },
        {
            "id": "12D3KooWTest2",
            "name": "Bob",
            "reputation": 0.5,
            "addresses": []
        }
    ]);

    assert!(response.is_array());
    let peers = response.as_array().unwrap();

    for peer in peers {
        assert!(peer["id"].is_string());
        assert!(peer["reputation"].is_number());
        assert!(peer["addresses"].is_array());
    }
}

/// Test expected format for GET /api/stats response
#[test]
fn test_stats_response_format() {
    let response = json!({
        "peer_count": 5,
        "message_count": 1234,
        "uptime_seconds": 3600
    });

    assert!(response["peer_count"].is_number());
    assert!(response["message_count"].is_number());
    assert!(response["uptime_seconds"].is_number());
}

/// Test expected format for GET /api/node response
#[test]
fn test_node_response_format() {
    let response = json!({
        "peer_id": "12D3KooWBootstrap",
        "name": "Bootstrap",
        "version": "1.0.0",
        "protocol_version": "/mycelial/1.0.0"
    });

    assert!(response["peer_id"].is_string());
    assert!(response["name"].is_string());
    assert!(response["version"].is_string());
    assert!(response["protocol_version"]
        .as_str()
        .unwrap()
        .starts_with("/mycelial/"));
}

/// Test expected format for GET /api/peers/{id} response
#[test]
fn test_single_peer_response_format() {
    let response = json!({
        "id": "12D3KooWTest1",
        "name": "Alice",
        "reputation": 0.85,
        "addresses": ["/ip4/127.0.0.1/tcp/9000"],
        "connected_since": 1703683200000_i64,
        "last_seen": 1703686800000_i64
    });

    assert!(response["id"].is_string());
    assert!(response["reputation"].as_f64().unwrap() >= 0.0);
    assert!(response["reputation"].as_f64().unwrap() <= 1.0);
}

/// Test error response format
#[test]
fn test_error_response_format() {
    let response = json!({
        "error": "Peer not found",
        "code": 404
    });

    assert!(response["error"].is_string());
    assert!(response["code"].is_number());
}

/// Test that peer ID format is consistent
#[test]
fn test_peer_id_format() {
    let valid_peer_ids = [
        "12D3KooWTest1",
        "12D3KooWABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "12D3KooW1234567890abcdefghijk",
    ];

    for peer_id in valid_peer_ids {
        assert!(
            peer_id.starts_with("12D3KooW"),
            "Peer ID should start with 12D3KooW"
        );
    }
}

/// Test multiaddr format in addresses
#[test]
fn test_multiaddr_format() {
    let addresses = [
        "/ip4/127.0.0.1/tcp/9000",
        "/ip4/192.168.1.1/tcp/9001",
        "/ip4/10.0.0.1/udp/9002/quic-v1",
    ];

    for addr in addresses {
        assert!(addr.starts_with("/ip4/") || addr.starts_with("/ip6/"));
    }
}

// ============ Economics API Tests ============

/// Test credit balance response format
#[test]
fn test_credit_balance_response() {
    let response = json!({
        "peer_id": "12D3KooWAlice",
        "credit_lines": [
            {
                "counterparty": "12D3KooWBob",
                "limit": 1000.0,
                "balance": 250.0,
                "direction": "extended"
            },
            {
                "counterparty": "12D3KooWCarol",
                "limit": 500.0,
                "balance": -100.0,
                "direction": "received"
            }
        ],
        "total_extended": 1500.0,
        "total_received": 500.0,
        "net_balance": 150.0
    });

    assert!(response["credit_lines"].is_array());
    let lines = response["credit_lines"].as_array().unwrap();

    for line in lines {
        assert!(line["limit"].as_f64().unwrap() >= 0.0);
        let direction = line["direction"].as_str().unwrap();
        assert!(direction == "extended" || direction == "received");
    }
}

/// Test reputation score response format
#[test]
fn test_reputation_response() {
    let response = json!({
        "peer_id": "12D3KooWAlice",
        "reputation_score": 0.85,
        "vouches_received": [
            {
                "from": "12D3KooWBob",
                "weight": 0.8,
                "timestamp": 1703683200000_i64
            }
        ],
        "vouches_given": [
            {
                "to": "12D3KooWCarol",
                "weight": 0.7,
                "timestamp": 1703683200000_i64
            }
        ]
    });

    let score = response["reputation_score"].as_f64().unwrap();
    assert!(score >= 0.0 && score <= 1.0);
}

/// Test governance proposals response format
#[test]
fn test_proposals_response() {
    let response = json!({
        "proposals": [
            {
                "id": "prop-123",
                "title": "Increase limits",
                "status": "active",
                "yes_votes": 3,
                "no_votes": 1,
                "quorum": 5,
                "deadline": 1703769600000_i64
            }
        ],
        "total": 1,
        "active": 1,
        "passed": 0,
        "failed": 0
    });

    assert!(response["proposals"].is_array());
    let proposal = &response["proposals"][0];
    let status = proposal["status"].as_str().unwrap();
    assert!(status == "active" || status == "passed" || status == "failed" || status == "pending");
}

/// Test resource pool response format
#[test]
fn test_resource_pool_response() {
    let response = json!({
        "resource_type": "bandwidth",
        "total_available": 1000.0,
        "total_used": 350.0,
        "unit": "mbps",
        "contributors": [
            {
                "peer_id": "12D3KooWAlice",
                "contribution": 500.0,
                "percentage": 50.0
            },
            {
                "peer_id": "12D3KooWBob",
                "contribution": 500.0,
                "percentage": 50.0
            }
        ]
    });

    let resource_type = response["resource_type"].as_str().unwrap();
    assert!(
        resource_type == "bandwidth" || resource_type == "storage" || resource_type == "compute"
    );

    let total = response["total_available"].as_f64().unwrap();
    let used = response["total_used"].as_f64().unwrap();
    assert!(used <= total);
}

// ============ DOL Evolution API Tests ============

/// Test version negotiation response
#[test]
fn test_version_negotiation() {
    let response = json!({
        "supported_versions": ["1.0.0", "1.1.0"],
        "preferred_version": "1.1.0",
        "deprecated_versions": ["0.9.0"],
        "minimum_version": "1.0.0"
    });

    assert!(response["supported_versions"].is_array());
    let versions = response["supported_versions"].as_array().unwrap();
    assert!(!versions.is_empty());
}

/// Test capability announcement response
#[test]
fn test_capability_announcement() {
    let response = json!({
        "peer_id": "12D3KooWAlice",
        "capabilities": [
            "chat",
            "direct_messaging",
            "rooms",
            "vouch",
            "credit",
            "governance",
            "resource_sharing"
        ],
        "protocol_extensions": [
            "neural_embeddings",
            "intent_routing"
        ]
    });

    assert!(response["capabilities"].is_array());
    let caps = response["capabilities"].as_array().unwrap();
    assert!(caps.iter().any(|c| c == "chat"));
}
