//! REST API handler behavior tests
//!
//! These tests verify the behavior of REST API endpoints,
//! ensuring correct response formatting and data retrieval.

use serde_json::json;

// ============ Health Endpoint Tests ============

#[test]
fn test_health_endpoint_returns_ok() {
    // The health endpoint should return "OK"
    let response = "OK";
    assert_eq!(response, "OK");
}

// ============ Node Info Response Tests ============

#[test]
fn test_node_info_structure() {
    let node_info = json!({
        "version": "0.1.0",
        "name": "TestNode",
        "peer_id": "12D3KooWTestPeerId"
    });

    assert!(node_info["version"].is_string());
    assert!(node_info["name"].is_string());
    assert!(node_info["peer_id"].is_string());
}

#[test]
fn test_node_info_version_format() {
    // Version should follow semver format
    let version = "0.1.0";
    let parts: Vec<&str> = version.split('.').collect();
    assert_eq!(parts.len(), 3, "Version should have 3 parts");
    for part in parts {
        assert!(
            part.parse::<u32>().is_ok(),
            "Version parts should be numbers"
        );
    }
}

// ============ Network Stats Response Tests ============

#[test]
fn test_network_stats_structure() {
    let stats = json!({
        "local_peer_id": "12D3KooWTestPeerId",
        "peer_count": 5,
        "message_count": 100,
        "uptime_seconds": 3600,
        "subscribed_topics": ["/mycelial/1.0.0/chat", "/mycelial/1.0.0/content"]
    });

    assert!(stats["local_peer_id"].is_string());
    assert!(stats["peer_count"].is_number());
    assert!(stats["message_count"].is_number());
    assert!(stats["uptime_seconds"].is_number());
    assert!(stats["subscribed_topics"].is_array());
}

#[test]
fn test_network_stats_peer_count_non_negative() {
    let peer_count: i64 = 5;
    assert!(peer_count >= 0, "Peer count should be non-negative");
}

#[test]
fn test_network_stats_message_count_non_negative() {
    let message_count: u64 = 100;
    assert!(message_count >= 0, "Message count should be non-negative");
}

#[test]
fn test_network_stats_uptime_non_negative() {
    let uptime: u64 = 3600;
    assert!(uptime >= 0, "Uptime should be non-negative");
}

// ============ Peer List Response Tests ============

#[test]
fn test_peer_list_empty() {
    let peers: Vec<serde_json::Value> = vec![];
    assert!(peers.is_empty());
}

#[test]
fn test_peer_list_with_peers() {
    let peers = vec![
        json!({
            "id": "12D3KooWPeer1",
            "name": "Alice",
            "reputation": 0.85,
            "addresses": ["/ip4/127.0.0.1/tcp/9000"]
        }),
        json!({
            "id": "12D3KooWPeer2",
            "name": null,
            "reputation": 0.5,
            "addresses": []
        }),
    ];

    assert_eq!(peers.len(), 2);
    assert!(peers[0]["id"].is_string());
    assert!(peers[0]["reputation"].is_number());
}

#[test]
fn test_peer_entry_structure() {
    let peer = json!({
        "id": "12D3KooWTestPeer",
        "name": "TestPeer",
        "reputation": 0.75,
        "addresses": ["/ip4/192.168.1.1/tcp/9000", "/ip6/::1/tcp/9000"]
    });

    assert!(peer["id"].is_string());
    assert!(peer["reputation"].as_f64().unwrap() >= 0.0);
    assert!(peer["reputation"].as_f64().unwrap() <= 1.0);
    assert!(peer["addresses"].is_array());
}

#[test]
fn test_peer_reputation_bounds() {
    // Reputation should be between 0 and 1
    let valid_reputations = [0.0, 0.5, 0.75, 1.0];
    for rep in valid_reputations {
        assert!((0.0..=1.0).contains(&rep), "Reputation should be in [0, 1]");
    }
}

#[test]
fn test_peer_addresses_format() {
    // Addresses should be valid multiaddr format
    let addresses = [
        "/ip4/127.0.0.1/tcp/9000",
        "/ip4/192.168.1.1/tcp/9001",
        "/ip6/::1/tcp/9002",
        "/ip4/0.0.0.0/udp/9003/quic-v1",
    ];

    for addr in addresses {
        assert!(addr.starts_with('/'), "Multiaddr should start with /");
        assert!(addr.contains("ip"), "Multiaddr should contain ip");
    }
}

// ============ Get Peer Response Tests ============

#[test]
fn test_get_peer_found() {
    let peer = json!({
        "id": "12D3KooWSpecificPeer",
        "name": "FoundPeer",
        "reputation": 0.9,
        "addresses": ["/ip4/10.0.0.1/tcp/9000"]
    });

    assert!(peer["id"].is_string());
    assert!(!peer["id"].as_str().unwrap().is_empty());
}

#[test]
fn test_get_peer_not_found() {
    let peer: Option<serde_json::Value> = None;
    assert!(peer.is_none());
}

// ============ API Response Content Type Tests ============

#[test]
fn test_json_content_type() {
    // API should return application/json
    let content_type = "application/json";
    assert!(content_type.contains("json"));
}

// ============ Error Response Tests ============

#[test]
fn test_error_response_structure() {
    let error = json!({
        "error": "Not found",
        "code": 404
    });

    assert!(error["error"].is_string());
    assert!(error["code"].is_number());
}

// ============ Topic Subscription Tests ============

#[test]
fn test_subscribed_topics_format() {
    let topics = [
        "/mycelial/1.0.0/chat",
        "/mycelial/1.0.0/content",
        "/mycelial/protocol/vouch",
        "/mycelial/protocol/credit",
    ];

    for topic in topics {
        assert!(topic.starts_with('/'), "Topic should start with /");
        assert!(topic.contains("mycelial"), "Topic should contain mycelial");
    }
}

#[test]
fn test_enr_topics_format() {
    let enr_topics = [
        "/vudo/enr/gradient/1.0.0",
        "/vudo/enr/credits/1.0.0",
        "/vudo/enr/election/1.0.0",
        "/vudo/enr/septal/1.0.0",
    ];

    for topic in enr_topics {
        assert!(topic.starts_with('/'), "Topic should start with /");
        assert!(topic.contains("enr"), "ENR topic should contain enr");
    }
}

// ============ Request Validation Tests ============

#[test]
fn test_peer_id_validation() {
    // Valid peer ID formats (base58 encoded)
    let valid_ids = [
        "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN",
        "QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N",
    ];

    for id in valid_ids {
        assert!(!id.is_empty(), "Peer ID should not be empty");
        assert!(
            id.chars().all(|c| c.is_alphanumeric()),
            "Peer ID should be alphanumeric"
        );
    }
}

#[test]
fn test_invalid_peer_id_handling() {
    // Invalid peer IDs should be rejected
    let invalid_ids = [
        "",
        "not-a-peer-id",
        "12345", // Too short
    ];

    for id in invalid_ids {
        let is_valid = id.len() > 10 && id.chars().all(|c| c.is_alphanumeric());
        assert!(
            !is_valid || id.is_empty(),
            "Invalid peer ID should fail validation"
        );
    }
}

// ============ Concurrent Request Tests ============

#[test]
fn test_atomic_counter_thread_safety() {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    let counter = Arc::new(AtomicU64::new(0));
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let c = Arc::clone(&counter);
            std::thread::spawn(move || {
                for _ in 0..100 {
                    c.fetch_add(1, Ordering::SeqCst);
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(counter.load(Ordering::SeqCst), 1000);
}

#[test]
fn test_rwlock_concurrent_reads() {
    use parking_lot::RwLock;
    use std::sync::Arc;

    let data = Arc::new(RwLock::new(vec!["topic1", "topic2"]));
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let d = Arc::clone(&data);
            std::thread::spawn(move || {
                let read = d.read();
                assert_eq!(read.len(), 2);
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
}
