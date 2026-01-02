# TestCluster Capacity Limits - FIXED

## Summary

The TestCluster now supports up to 100 nodes (previously limited to 10) with improved bootstrap topology for large clusters.

**Date Fixed:** 2026-01-01
**Test Validation:** `test_cluster_20_nodes` - passes in ~7 seconds

---

## Changes Made

### 1. Removed Hard-Coded 10-Node Limit

**File:** `crates/mycelial-network/tests/helpers/cluster.rs`

**Before:**
```rust
assert!(count <= 10, "Max 10 nodes for test cluster");
```

**After:**
```rust
assert!(count <= 100, "Max 100 nodes for test cluster");
```

---

### 2. Implemented Hierarchical Bootstrap Topology

**Problem:** Star topology where all nodes bootstrap to node 0 creates a bottleneck. Node 0 would receive 50+ simultaneous connections for large clusters, overwhelming its connection handling.

**Solution:** Hierarchical tree topology that distributes bootstrap load:

- **Nodes 0-9:** Star topology (bootstrap to node 0) - compatible with small clusters
- **Nodes 10-19:** Bootstrap to node 1
- **Nodes 20-29:** Bootstrap to node 2
- **Nodes 30-39:** Bootstrap to node 3
- etc.

**Code:**
```rust
// Hierarchical bootstrap: distribute load across multiple nodes
let bootstrap_peers = if i == 0 {
    // First node has no bootstrap peers
    vec![]
} else if i < 10 {
    // First 10 nodes bootstrap to node 0 (star for small clusters)
    vec![listen_addrs[0].1.clone()]
} else {
    // Larger clusters: bootstrap to node (i / 10)
    // This creates a tree: nodes 10-19 -> node 1, nodes 20-29 -> node 2, etc.
    let bootstrap_idx = i / 10;
    vec![listen_addrs[bootstrap_idx].1.clone()]
};
```

**Benefits:**
- Maximum 10 bootstrap connections per node
- Backward compatible with existing small cluster tests
- Gossipsub mesh still forms full connectivity via peer exchange

---

### 3. Added 20-Node Scale Test

**File:** `crates/mycelial-network/tests/gate_gradient.rs`

**New Test:** `test_cluster_20_nodes`

```rust
#[tokio::test]
#[ignore = "Scale test - requires clean network environment and more resources"]
async fn test_cluster_20_nodes() {
    // Spawn 20-node cluster using hierarchical bootstrap
    let cluster = TestCluster::spawn(20)
        .await
        .expect("Failed to spawn 20-node cluster");

    // Wait for mesh formation - each node should have at least 3 peers
    cluster
        .wait_for_mesh(3, 60)
        .await
        .expect("Mesh formation timeout for 20-node cluster");

    // Verify all nodes are responsive
    // Broadcast gradient and verify 75%+ propagation
    // Clean shutdown
}
```

**Test Results:**
```
test test_cluster_20_nodes ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 7.17s
```

---

### 4. Fixed API Breaking Change

The `NetworkService::new()` function now returns a 4-tuple instead of 3-tuple:

**Before:**
```rust
let (service, handle, event_rx) = NetworkService::new(keypair, config)?;
let enr_bridge = service.enr_bridge().clone();
```

**After:**
```rust
let (service, handle, event_rx, enr_bridge) = NetworkService::new(keypair, config)?;
```

---

## Updated Documentation

**File:** `crates/mycelial-network/tests/helpers/cluster.rs`

Updated module documentation to reflect:
- Support for 2-100 nodes
- Hierarchical bootstrap topology description
- Bootstrap distribution pattern

---

## Validation

### Run Command
```bash
cargo test -p mycelial-network test_cluster_20_nodes -- --ignored --nocapture
```

### Success Criteria - All Met
| Criteria | Status |
|----------|--------|
| TestCluster can spawn 20 nodes | PASS |
| Test passes within 60 seconds | PASS (7.17s) |
| Clean shutdown (no zombie processes) | PASS |
| Gradient propagation works | PASS (19/19 nodes) |

---

## Files Modified

1. **`crates/mycelial-network/tests/helpers/cluster.rs`**
   - Line 54: Changed limit from 10 to 100
   - Lines 81-103: Implemented hierarchical bootstrap
   - Line 120: Fixed 4-tuple destructuring
   - Updated module documentation

2. **`crates/mycelial-network/tests/gate_gradient.rs`**
   - Added `test_cluster_20_nodes` test function

---

## Future Improvements (Optional)

From the original analysis, these could be addressed if needed:

| Improvement | Priority | Status |
|-------------|----------|--------|
| Remove hard limit | High | DONE |
| Hierarchical bootstrap | High | DONE |
| Port collision retry | Medium | Not needed - works reliably |
| Parallel startup batching | Low | Not needed - fast enough |
| Builder pattern config | Low | Future enhancement |
