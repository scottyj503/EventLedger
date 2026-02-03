//! EventLedger Core Library
//!
//! Shared functionality for EventLedger Lambda functions including:
//! - Domain models
//! - DynamoDB operations
//! - Partitioning logic
//! - Error types

pub mod models;
pub mod dynamo;
pub mod partitioner;
pub mod errors;

pub use models::*;
pub use dynamo::DynamoClient;
pub use partitioner::Partitioner;
pub use errors::{Error, Result};
