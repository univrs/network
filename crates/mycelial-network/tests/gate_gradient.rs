//! Phase 0 Gate Test: Gradient Propagation
//!
//! Tests that resource gradients propagate to all nodes in the cluster
//! within the expected time frame (<15 seconds).

mod helpers;

use std::time::Duration;
use tokio::time::timeout;
use univrs_enr::nexus::ResourceGradient;

use helpers::TestCluster;

/// Test that gradient broadcasts propagate to all nodes in the cluster
///
/// Setup:
/// - Spawn 3 nodes
/// - Wait for mesh formation
/// - Node 0 broadcasts a gradient with distinct values
/// - Verify all other nodes receive and store the gradient within 15s
///
/// Note: This test requires a clean network environment (no Docker bridges)
/// Run with: cargo test --test gate_gradient -- --ignored
#[tokio::test]
// Note: Run with --test-threads=1 to avoid port conflicts
async fn test_gradient_propagates_to_all_nodes() {
    // Initialize tracing for debugging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=debug,gate_gradient=debug")
        .try_init();

    // Spawn a 3-node cluster
    let cluster = TestCluster::spawn(3)
        .await
        .expect("Failed to spawn cluster");

    // Wait for mesh formation (each node should see at least 1 peer)
    // Note: Bootstrap creates a star topology where nodes 1,2 connect to node 0
    // so min_peers=1 is sufficient for gossipsub to function
    cluster
        .wait_for_mesh(1, 10)
        .await
        .expect("Mesh formation timeout");

    // Node 0 broadcasts a distinctive gradient
    let test_gradient = ResourceGradient {
        cpu_available: 0.42,
        memory_available: 0.73,
        gpu_available: 0.0,
        storage_available: 0.85,
        bandwidth_available: 0.91,
        credit_balance: 1000.0,
    };

    cluster
        .node(0)
        .enr_bridge
        .broadcast_gradient(test_gradient)
        .await
        .expect("Failed to broadcast gradient");

    // Wait for propagation (max 15 seconds)
    let propagation_result = timeout(Duration::from_secs(15), async {
        loop {
            let mut all_received = true;

            // Check nodes 1 and 2 have received the gradient
            for i in 1..cluster.node_count() {
                let net_gradient = cluster.node(i).enr_bridge.network_gradient().await;

                // Check if our distinctive values are present
                if (net_gradient.cpu_available - 0.42).abs() > 0.01
                    || (net_gradient.memory_available - 0.73).abs() > 0.01
                {
                    all_received = false;
                    break;
                }
            }

            if all_received {
                return true;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    assert!(
        propagation_result.is_ok(),
        "Gradient did not propagate to all nodes within 15 seconds"
    );

    // Verify the gradient values on each receiving node
    for i in 1..cluster.node_count() {
        let net_gradient = cluster.node(i).enr_bridge.network_gradient().await;

        assert!(
            (net_gradient.cpu_available - 0.42).abs() < 0.01,
            "Node {} has incorrect cpu_available: {}",
            i,
            net_gradient.cpu_available
        );
        assert!(
            (net_gradient.memory_available - 0.73).abs() < 0.01,
            "Node {} has incorrect memory_available: {}",
            i,
            net_gradient.memory_available
        );
    }

    // Verify active node count
    let active_count = cluster.node(1).enr_bridge.active_node_count().await;
    assert!(
        active_count >= 1,
        "Expected at least 1 active node, got {}",
        active_count
    );

    cluster.shutdown().await;
}

/// Test cluster formation with 20 nodes (scale test)
///
/// This test validates that the hierarchical bootstrap topology works for
/// larger clusters without overwhelming node 0.
#[tokio::test]
// Note: Run with --test-threads=1 to avoid port conflicts. Scale test with 20 nodes.
async fn test_cluster_20_nodes() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    // Spawn 20-node cluster using hierarchical bootstrap
    let cluster = TestCluster::spawn(20)
        .await
        .expect("Failed to spawn 20-node cluster");

    // Wait for mesh formation - each node should have at least 3 peers
    // With hierarchical bootstrap, gossipsub should form a mesh within 60 seconds
    cluster
        .wait_for_mesh(3, 60)
        .await
        .expect("Mesh formation timeout for 20-node cluster");

    // Verify all nodes are responsive by checking peer counts
    for i in 0..cluster.node_count() {
        let peers = cluster.node(i).handle.get_peers().await.unwrap();
        assert!(
            peers.len() >= 3,
            "Node {} has only {} peers, expected at least 3",
            i,
            peers.len()
        );
    }

    // Broadcast a gradient and verify propagation across the larger mesh
    let test_gradient = ResourceGradient {
        cpu_available: 0.99,
        memory_available: 0.88,
        ..Default::default()
    };

    cluster
        .node(10) // Broadcast from middle of cluster
        .enr_bridge
        .broadcast_gradient(test_gradient)
        .await
        .expect("Failed to broadcast gradient");

    // Wait for propagation to at least 15 nodes (75%)
    let result = timeout(Duration::from_secs(30), async {
        loop {
            let mut received_count = 0;

            for i in 0..cluster.node_count() {
                if i == 10 {
                    continue; // Skip sender
                }
                let grad = cluster.node(i).enr_bridge.network_gradient().await;
                if (grad.cpu_available - 0.99).abs() < 0.01 {
                    received_count += 1;
                }
            }

            if received_count >= 15 {
                return received_count;
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await;

    assert!(
        result.is_ok(),
        "Gradient did not propagate to 75%+ of 20 nodes within 30 seconds"
    );

    println!("Gradient propagated to {} of 19 nodes", result.unwrap());

    cluster.shutdown().await;
}

/// Test gradient propagation with 5 nodes (larger cluster)
#[tokio::test]
// Note: Run with --test-threads=1 to avoid port conflicts
async fn test_gradient_propagates_5_nodes() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("mycelial_network=info")
        .try_init();

    let cluster = TestCluster::spawn(5)
        .await
        .expect("Failed to spawn cluster");

    // Wait for mesh (min 1 peer for star bootstrap topology)
    cluster
        .wait_for_mesh(1, 15)
        .await
        .expect("Mesh formation timeout");

    // Broadcast from node 2 (middle of cluster)
    let test_gradient = ResourceGradient {
        cpu_available: 0.55,
        memory_available: 0.66,
        ..Default::default()
    };

    cluster
        .node(2)
        .enr_bridge
        .broadcast_gradient(test_gradient)
        .await
        .expect("Failed to broadcast");

    // Wait for propagation
    let result = timeout(Duration::from_secs(15), async {
        loop {
            let mut received_count = 0;

            for i in 0..cluster.node_count() {
                if i == 2 {
                    continue; // Skip sender
                }
                let grad = cluster.node(i).enr_bridge.network_gradient().await;
                if (grad.cpu_available - 0.55).abs() < 0.01 {
                    received_count += 1;
                }
            }

            if received_count >= 4 {
                // All 4 other nodes received it
                return true;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    assert!(
        result.is_ok(),
        "Gradient did not propagate to all 5 nodes within 15 seconds"
    );

    cluster.shutdown().await;
}
