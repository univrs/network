//! Wave 2: Election Stress Tests
//!
//! Run: cargo test --test stress_election -- --test-threads=1

mod helpers;

#[path = "stress/election.rs"]
mod election;
