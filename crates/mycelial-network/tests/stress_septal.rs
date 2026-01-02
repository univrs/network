//! Wave 5: Septal (Cross-Shard) Stress Tests
//!
//! Run: cargo test --test stress_septal -- --test-threads=1

mod helpers;

#[path = "stress/septal.rs"]
mod septal;
