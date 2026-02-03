//! EventLedger Integration Tests
//!
//! These tests run against either:
//! - A deployed API (set EVENTLEDGER_API_URL environment variable)
//! - Local DynamoDB (for unit-style integration tests)
//!
//! Run with: cargo test --package eventledger-integration-tests

pub mod client;
pub mod fixtures;

pub use client::EventLedgerClient;
pub use fixtures::*;
