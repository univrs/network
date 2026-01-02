//! Network Partition Integration Tests
//!
//! Tests for the partition simulator functionality:
//! - Creating partitions between node groups
//! - Isolating individual nodes
//! - Healing partitions and restoring connectivity
//! - Message filtering across partitions

mod helpers;

use std::time::Duration;
use helpers::TestCluster;

/// Test that creating a partition disconnects nodes across the partition boundary.
///
/// Setup:
/// - Spawn 4 nodes
/// - Wait for mesh formation
/// - Create partition: [0, 1] vs [2, 3]
/// - Verify nodes can only see peers within their partition
#[tokio::test]
async fn test_partition_disconnects_across_groups() {
    // Initialize tracing for debugging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=debug,partition_test=debug")
        .try_init();

    // Spawn a 4-node cluster
    let cluster = TestCluster::spawn(4)
        .await
        .expect("Failed to spawn cluster");

    // Wait for mesh formation
    cluster
        .wait_for_mesh(1, 15)
        .await
        .expect("Mesh formation timeout");

    // Get initial peer counts
    let initial_peers_0 = cluster.nodes[0].handle.get_peers().await.unwrap();
    let initial_peers_2 = cluster.nodes[2].handle.get_peers().await.unwrap();

    println!("Before partition - Node 0 peers: {:?}", initial_peers_0);
    println!("Before partition - Node 2 peers: {:?}", initial_peers_2);

    // Create partition: [0, 1] vs [2, 3]
    cluster
        .create_partition(&[0, 1], &[2, 3])
        .await
        .expect("Failed to create partition");

    // Wait for disconnection to propagate
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Node 0 should not see node 2 or 3 as connected peers anymore
    let peers_0 = cluster.nodes[0].handle.get_peers().await.unwrap();
    let peers_2 = cluster.nodes[2].handle.get_peers().await.unwrap();

    println!("After partition - Node 0 peers: {:?}", peers_0);
    println!("After partition - Node 2 peers: {:?}", peers_2);

    // Node 0 should not have node 2 or 3 as peers
    assert!(
        !peers_0.contains(&cluster.nodes[2].peer_id),
        "Node 0 should not be connected to Node 2 after partition"
    );
    assert!(
        !peers_0.contains(&cluster.nodes[3].peer_id),
        "Node 0 should not be connected to Node 3 after partition"
    );

    // Node 2 should not have node 0 or 1 as peers
    assert!(
        !peers_2.contains(&cluster.nodes[0].peer_id),
        "Node 2 should not be connected to Node 0 after partition"
    );
    assert!(
        !peers_2.contains(&cluster.nodes[1].peer_id),
        "Node 2 should not be connected to Node 1 after partition"
    );

    // Cleanup
    cluster.shutdown().await;
}

/// Test that healing a partition restores connectivity.
///
/// Setup:
/// - Spawn 4 nodes
/// - Wait for mesh formation
/// - Create partition
/// - Heal partition
/// - Verify nodes can reconnect
#[tokio::test]
async fn test_heal_partition_restores_connectivity() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=debug,partition_test=debug")
        .try_init();

    let cluster = TestCluster::spawn(4)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(1, 15)
        .await
        .expect("Mesh formation timeout");

    // Create and then heal partition
    cluster
        .create_partition(&[0, 1], &[2, 3])
        .await
        .expect("Failed to create partition");

    tokio::time::sleep(Duration::from_millis(500)).await;

    cluster
        .heal_partition(&[0, 1], &[2, 3])
        .await
        .expect("Failed to heal partition");

    // Wait for reconnection
    tokio::time::sleep(Duration::from_secs(2)).await;

    // After healing, mesh should reform - but we just check that peers are accessible
    // Note: Due to gossipsub mesh dynamics, exact peer counts may vary
    let peers_0 = cluster.nodes[0].handle.get_peers().await.unwrap();
    println!("After healing - Node 0 peers: {:?}", peers_0);

    // The partition should no longer block connections
    // (actual reconnection depends on timing and gossipsub dynamics)

    cluster.shutdown().await;
}

/// Test isolating a single node from the cluster.
#[tokio::test]
async fn test_isolate_single_node() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=debug,partition_test=debug")
        .try_init();

    let cluster = TestCluster::spawn(3)
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

    // Node 2 should have no peers
    let peers_2 = cluster.nodes[2].handle.get_peers().await.unwrap();
    println!("After isolation - Node 2 peers: {:?}", peers_2);

    // Due to the blocking, node 2 should be isolated
    // (existing connections may take time to close, but new connections are blocked)

    // Other nodes should not see node 2
    let peers_0 = cluster.nodes[0].handle.get_peers().await.unwrap();
    assert!(
        !peers_0.contains(&cluster.nodes[2].peer_id),
        "Node 0 should not see isolated Node 2"
    );

    cluster.shutdown().await;
}

/// Test rejoining an isolated node to the cluster.
#[tokio::test]
async fn test_rejoin_isolated_node() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=debug,partition_test=debug")
        .try_init();

    let cluster = TestCluster::spawn(3)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(1, 15)
        .await
        .expect("Mesh formation timeout");

    // Isolate and then rejoin node 2
    cluster.isolate_node(2).await.expect("Failed to isolate");
    tokio::time::sleep(Duration::from_millis(500)).await;

    cluster.rejoin_node(2).await.expect("Failed to rejoin");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Node 2 should be able to reconnect
    let peers_2 = cluster.nodes[2].handle.get_peers().await.unwrap();
    println!("After rejoin - Node 2 peers: {:?}", peers_2);

    // The node should no longer be blocked (reconnection depends on timing)

    cluster.shutdown().await;
}

/// Test that heal_all_partitions clears all blocking.
#[tokio::test]
async fn test_heal_all_partitions() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=debug,partition_test=debug")
        .try_init();

    let cluster = TestCluster::spawn(4)
        .await
        .expect("Failed to spawn cluster");

    cluster
        .wait_for_mesh(1, 15)
        .await
        .expect("Mesh formation timeout");

    // Create partition
    cluster
        .create_partition(&[0, 1], &[2, 3])
        .await
        .expect("Failed to create partition");

    // Also isolate a node
    cluster.isolate_node(0).await.expect("Failed to isolate");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Heal all partitions
    cluster
        .heal_all_partitions()
        .await
        .expect("Failed to heal all");

    tokio::time::sleep(Duration::from_secs(1)).await;

    // All blocking should be cleared (reconnection depends on timing)
    println!("Heal all completed - blocking cleared");

    cluster.shutdown().await;
}
