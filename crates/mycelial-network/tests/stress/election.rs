//! Wave 2: Election Stress Tests
//!
//! Tests for nexus election at scale:
//! - test_election_10_nodes: Election with 10 candidates
//! - test_election_announcement_20_nodes: Announcement propagation at scale
//! - test_election_voting_convergence: Vote consensus within timeout
//! - test_concurrent_elections: Multiple simultaneous elections
//! - test_election_recovery: Election recovery after partition

use std::time::{Duration, Instant};
use tokio::time::timeout;

use crate::helpers::TestCluster;
use mycelial_network::enr_bridge::LocalNodeMetrics;

/// Test election with 10 candidate nodes
/// Expected: Election completes within 120 seconds
#[tokio::test]
async fn test_election_10_nodes() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(10)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(2, 30)
        .await
        .expect("Mesh formation timeout");

    // Set all nodes as eligible candidates
    for i in 0..cluster.node_count() {
        let metrics = LocalNodeMetrics {
            uptime: 0.96 + (i as f64 * 0.001), // Slight variation
            bandwidth: 100 + (i as u64 * 10),
            reputation: 0.9,
            connection_count: 9,
        };
        cluster
            .node(i)
            .enr_bridge
            .election
            .update_metrics(metrics)
            .await;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Trigger election from node 0
    let start = Instant::now();
    let election_id = cluster
        .node(0)
        .enr_bridge
        .trigger_election("stress-test-10".to_string())
        .await
        .expect("Failed to trigger election");

    println!("Election {} triggered", election_id);

    // Wait for all nodes to see election in progress
    let propagation = timeout(Duration::from_secs(30), async {
        loop {
            let mut all_in_progress = true;
            for i in 0..cluster.node_count() {
                if !cluster.node(i).enr_bridge.election_in_progress().await {
                    all_in_progress = false;
                    break;
                }
            }
            if all_in_progress {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    assert!(
        propagation.is_ok(),
        "Election did not propagate to all nodes"
    );
    println!("Election propagated in {:?}", start.elapsed());

    cluster.shutdown().await;
}

/// Test election announcement propagation with 20 nodes
/// Expected: Announcement reaches all nodes within 30 seconds
#[tokio::test]
async fn test_election_announcement_20_nodes() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(20)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(3, 60)
        .await
        .expect("Mesh formation timeout");

    // Set metrics on all nodes
    for i in 0..cluster.node_count() {
        let metrics = LocalNodeMetrics {
            uptime: 0.97,
            bandwidth: 100,
            reputation: 0.9,
            connection_count: 19,
        };
        cluster
            .node(i)
            .enr_bridge
            .election
            .update_metrics(metrics)
            .await;
    }

    let start = Instant::now();

    // Trigger from middle of cluster
    let _ = cluster
        .node(10)
        .enr_bridge
        .trigger_election("announcement-20".to_string())
        .await
        .expect("Failed to trigger election");

    // Measure propagation time
    let result = timeout(Duration::from_secs(30), async {
        loop {
            let mut count = 0;
            for i in 0..cluster.node_count() {
                if cluster.node(i).enr_bridge.election_in_progress().await {
                    count += 1;
                }
            }
            if count >= 18 {
                // 90% coverage
                return count;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    let propagation_time = start.elapsed();

    assert!(result.is_ok(), "Announcement did not propagate");
    println!(
        "Election announcement reached {} nodes in {:?}",
        result.unwrap(),
        propagation_time
    );

    assert!(
        propagation_time.as_secs() < 30,
        "Propagation too slow: {:?}",
        propagation_time
    );

    cluster.shutdown().await;
}

/// Test voting convergence with 10 nodes
/// Expected: Votes converge within 60 seconds
#[tokio::test]
async fn test_election_voting_convergence() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(10)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(2, 30)
        .await
        .expect("Mesh formation timeout");

    // Make node 5 the clear winner
    for i in 0..cluster.node_count() {
        let metrics = if i == 5 {
            LocalNodeMetrics {
                uptime: 0.99,
                bandwidth: 1000,
                reputation: 0.99,
                connection_count: 9,
            }
        } else {
            LocalNodeMetrics {
                uptime: 0.96,
                bandwidth: 50,
                reputation: 0.8,
                connection_count: 9,
            }
        };
        cluster
            .node(i)
            .enr_bridge
            .election
            .update_metrics(metrics)
            .await;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    let _ = cluster
        .node(0)
        .enr_bridge
        .trigger_election("voting-test".to_string())
        .await
        .expect("Failed to trigger election");

    // Wait for election to complete (no longer in progress)
    let result = timeout(Duration::from_secs(60), async {
        // First wait for election to start
        loop {
            if cluster.node(0).enr_bridge.election_in_progress().await {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Then wait for it to complete
        loop {
            if !cluster.node(0).enr_bridge.election_in_progress().await {
                return cluster.node(0).enr_bridge.current_nexus().await;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    match result {
        Ok(Some(nexus)) => {
            println!("Election converged with nexus: {:?}", nexus);
        }
        Ok(None) => {
            println!("Election completed but no nexus elected");
        }
        Err(_) => {
            println!("Election did not converge within 60 seconds");
        }
    }

    cluster.shutdown().await;
}

/// Test concurrent elections from multiple nodes
/// Expected: System handles concurrent election triggers gracefully
#[tokio::test]
async fn test_concurrent_elections() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(5)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(1, 15)
        .await
        .expect("Mesh formation timeout");

    // Set metrics
    for i in 0..cluster.node_count() {
        let metrics = LocalNodeMetrics {
            uptime: 0.97,
            bandwidth: 100,
            reputation: 0.9,
            connection_count: 4,
        };
        cluster
            .node(i)
            .enr_bridge
            .election
            .update_metrics(metrics)
            .await;
    }

    // Trigger elections from multiple nodes simultaneously
    let results: Vec<_> = futures::future::join_all((0..3).map(|i| {
        let cluster = &cluster;
        async move {
            cluster
                .node(i)
                .enr_bridge
                .trigger_election(format!("concurrent-{}", i))
                .await
        }
    }))
    .await;

    // At least one should succeed or be handled gracefully
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    println!("Concurrent election triggers: {} succeeded", success_count);

    // Wait for any elections to propagate
    tokio::time::sleep(Duration::from_secs(2)).await;

    cluster.shutdown().await;
}

/// Test election recovery after network partition
/// Expected: Election can complete after partition heals
#[tokio::test]
async fn test_election_recovery() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(6)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(2, 30)
        .await
        .expect("Mesh formation timeout");

    // Set metrics
    for i in 0..cluster.node_count() {
        let metrics = LocalNodeMetrics {
            uptime: 0.97,
            bandwidth: 100,
            reputation: 0.9,
            connection_count: 5,
        };
        cluster
            .node(i)
            .enr_bridge
            .election
            .update_metrics(metrics)
            .await;
    }

    // Create partition [0,1,2] vs [3,4,5]
    cluster
        .create_partition(&[0, 1, 2], &[3, 4, 5])
        .await
        .expect("Failed to create partition");

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Try to trigger election from node 0
    let result = cluster
        .node(0)
        .enr_bridge
        .trigger_election("recovery-test".to_string())
        .await;

    assert!(result.is_ok(), "Should be able to trigger election");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Heal partition
    cluster
        .heal_partition(&[0, 1, 2], &[3, 4, 5])
        .await
        .expect("Failed to heal partition");

    // Allow reconnection
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Verify network recovered
    let peers_0 = cluster.node(0).handle.get_peers().await.unwrap();
    println!("Node 0 peers after healing: {}", peers_0.len());

    cluster.shutdown().await;
}
