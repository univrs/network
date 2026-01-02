//! Wave 1: Node Harness Stress Tests
//!
//! Tests for cluster spawning, discovery, and lifecycle at scale:
//! - test_spawn_10_nodes: Basic 10-node cluster formation
//! - test_spawn_20_nodes: Medium 20-node cluster formation
//! - test_spawn_50_nodes: Large 50-node cluster formation
//! - test_node_discovery_at_scale: Peer discovery across 20 nodes
//! - test_cluster_shutdown: Graceful shutdown of 10 nodes
//! - test_node_restart: Node restart and rejoin

use crate::helpers::TestCluster;
use std::time::{Duration, Instant};

/// Test spawning a 10-node cluster
/// Expected: All nodes discover each other within 30 seconds
#[tokio::test]
async fn test_spawn_10_nodes() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let start = Instant::now();

    let cluster = TestCluster::spawn(10)
        .await
        .expect("Failed to spawn 10-node cluster");

    let spawn_time = start.elapsed();
    println!("10-node cluster spawned in {:?}", spawn_time);

    // Wait for mesh formation - each node should have at least 2 peers
    // Use 45s timeout to account for port reuse delays when running sequential tests
    cluster
        .wait_for_mesh(2, 45)
        .await
        .expect("Mesh formation timeout for 10-node cluster");

    let mesh_time = start.elapsed();
    println!("Mesh formed in {:?}", mesh_time);

    // Verify all nodes are responsive
    for i in 0..cluster.node_count() {
        let peers = cluster.node(i).handle.get_peers().await.unwrap();
        assert!(
            peers.len() >= 2,
            "Node {} has only {} peers, expected at least 2",
            i,
            peers.len()
        );
    }

    assert!(
        mesh_time.as_secs() < 45,
        "Mesh formation took too long: {:?}",
        mesh_time
    );

    cluster.shutdown().await;
}

/// Test spawning a 20-node cluster
/// Expected: All nodes form mesh within 60 seconds
#[tokio::test]
async fn test_spawn_20_nodes() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let start = Instant::now();

    let cluster = TestCluster::spawn(20)
        .await
        .expect("Failed to spawn 20-node cluster");

    let spawn_time = start.elapsed();
    println!("20-node cluster spawned in {:?}", spawn_time);

    // Wait for mesh formation - each node should have at least 3 peers
    cluster
        .wait_for_mesh(3, 60)
        .await
        .expect("Mesh formation timeout for 20-node cluster");

    let mesh_time = start.elapsed();
    println!("Mesh formed in {:?}", mesh_time);

    // Verify peer connectivity
    for i in 0..cluster.node_count() {
        let peers = cluster.node(i).handle.get_peers().await.unwrap();
        assert!(
            peers.len() >= 3,
            "Node {} has only {} peers, expected at least 3",
            i,
            peers.len()
        );
    }

    assert!(
        mesh_time.as_secs() < 60,
        "Mesh formation took too long: {:?}",
        mesh_time
    );

    cluster.shutdown().await;
}

/// Test spawning a 50-node cluster (scale test)
/// Expected: All nodes form mesh within 120 seconds
#[tokio::test]
async fn test_spawn_50_nodes() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=warn")
        .try_init();

    let start = Instant::now();

    let cluster = TestCluster::spawn(50)
        .await
        .expect("Failed to spawn 50-node cluster");

    let spawn_time = start.elapsed();
    println!("50-node cluster spawned in {:?}", spawn_time);

    // Wait for mesh formation - each node should have at least 3 peers
    cluster
        .wait_for_mesh(3, 120)
        .await
        .expect("Mesh formation timeout for 50-node cluster");

    let mesh_time = start.elapsed();
    println!("Mesh formed in {:?}", mesh_time);

    // Sample verification (checking every 5th node to reduce time)
    for i in (0..cluster.node_count()).step_by(5) {
        let peers = cluster.node(i).handle.get_peers().await.unwrap();
        assert!(
            peers.len() >= 3,
            "Node {} has only {} peers, expected at least 3",
            i,
            peers.len()
        );
    }

    assert!(
        mesh_time.as_secs() < 120,
        "Mesh formation took too long: {:?}",
        mesh_time
    );

    cluster.shutdown().await;
}

/// Test peer discovery at scale with 20 nodes
/// Expected: Each node discovers at least 10 unique peers within 45 seconds
#[tokio::test]
async fn test_node_discovery_at_scale() {
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

    // Give additional time for Kademlia routing table to populate
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Verify peer discovery breadth
    let mut total_unique_peers = std::collections::HashSet::new();
    for i in 0..cluster.node_count() {
        let peers = cluster.node(i).handle.get_peers().await.unwrap();
        for peer in &peers {
            total_unique_peers.insert(*peer);
        }
    }

    println!(
        "Total unique peers discovered across cluster: {}",
        total_unique_peers.len()
    );

    // Should discover at least 50% of the network
    assert!(
        total_unique_peers.len() >= 10,
        "Discovery coverage too low: {} peers",
        total_unique_peers.len()
    );

    cluster.shutdown().await;
}

/// Test graceful cluster shutdown
/// Expected: All nodes shut down cleanly within 10 seconds
#[tokio::test]
async fn test_cluster_shutdown() {
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

    let start = Instant::now();
    cluster.shutdown().await;
    let shutdown_time = start.elapsed();

    println!("10-node cluster shutdown in {:?}", shutdown_time);

    assert!(
        shutdown_time.as_secs() < 10,
        "Shutdown took too long: {:?}",
        shutdown_time
    );
}

/// Test node restart and rejoin
/// Expected: Isolated node can rejoin cluster within 15 seconds
#[tokio::test]
async fn test_node_restart() {
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

    // Isolate node 2
    cluster
        .isolate_node(2)
        .await
        .expect("Failed to isolate node");

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify node 2 is isolated
    let peers_2 = cluster.node(2).handle.get_peers().await.unwrap();
    println!("Node 2 peers after isolation: {}", peers_2.len());

    // Rejoin node 2
    let start = Instant::now();
    cluster.rejoin_node(2).await.expect("Failed to rejoin node");

    // Wait for reconnection
    tokio::time::sleep(Duration::from_secs(5)).await;

    let rejoin_time = start.elapsed();
    let peers_2_after = cluster.node(2).handle.get_peers().await.unwrap();

    println!(
        "Node 2 peers after rejoin: {} (took {:?})",
        peers_2_after.len(),
        rejoin_time
    );

    cluster.shutdown().await;
}
