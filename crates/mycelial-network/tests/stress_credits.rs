//! Wave 3: Credit System Stress Tests
//!
//! Run: cargo test --test stress_credits -- --test-threads=1

mod helpers;

#[path = "stress/credits.rs"]
mod credits;
