//! Domain models for EventLedger
//!
//! These types represent the core entities in the system:
//! - Streams: Named event logs with configurable partitions
//! - Events: Individual records in the log
//! - Subscriptions: Consumer configurations with offset tracking
//! - Compacted State: Latest value per key

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stream metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stream {
    /// Unique stream identifier
    pub stream_id: String,
    /// Number of partitions for parallel processing
    pub partition_count: u32,
    /// Retention period in hours for hot storage
    pub retention_hours: u32,
    /// When the stream was created
    pub created_at: DateTime<Utc>,
}

impl Stream {
    pub fn new(stream_id: String, partition_count: u32, retention_hours: u32) -> Self {
        Self {
            stream_id,
            partition_count,
            retention_hours,
            created_at: Utc::now(),
        }
    }
}

/// Request to create a new stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStreamRequest {
    /// Unique stream identifier (alphanumeric, hyphens, underscores)
    pub stream_id: String,
    /// Number of partitions (default: 3)
    #[serde(default = "default_partition_count")]
    pub partition_count: u32,
    /// Retention period in hours (default: 168 = 7 days)
    #[serde(default = "default_retention_hours")]
    pub retention_hours: u32,
}

fn default_partition_count() -> u32 {
    3
}

fn default_retention_hours() -> u32 {
    168 // 7 days
}

/// An event in the log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Stream this event belongs to
    pub stream_id: String,
    /// Partition number (0-based)
    pub partition: u32,
    /// Monotonically increasing sequence number within partition
    pub sequence: u64,
    /// Key for compaction (e.g., entity ID)
    pub key: String,
    /// Event type (e.g., "order.created")
    pub event_type: String,
    /// Event payload (JSON)
    pub data: serde_json::Value,
    /// When the event was published
    pub timestamp: DateTime<Utc>,
}

/// Request to publish event(s)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishRequest {
    /// Events to publish
    pub events: Vec<PublishEvent>,
}

/// Single event to publish
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishEvent {
    /// Key for partitioning and compaction
    pub key: String,
    /// Event type
    #[serde(rename = "type")]
    pub event_type: String,
    /// Event payload
    pub data: serde_json::Value,
}

/// Response after publishing events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResponse {
    /// Published event references
    pub events: Vec<PublishedEvent>,
}

/// Reference to a published event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedEvent {
    pub stream_id: String,
    pub partition: u32,
    pub sequence: u64,
    pub key: String,
    pub timestamp: DateTime<Utc>,
}

/// Subscription configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// Stream being subscribed to
    pub stream_id: String,
    /// Unique subscription identifier
    pub subscription_id: String,
    /// When the subscription was created
    pub created_at: DateTime<Utc>,
}

impl Subscription {
    pub fn new(stream_id: String, subscription_id: String) -> Self {
        Self {
            stream_id,
            subscription_id,
            created_at: Utc::now(),
        }
    }
}

/// Request to create a subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionRequest {
    /// Unique subscription identifier
    pub subscription_id: String,
    /// Where to start consuming from
    #[serde(default)]
    pub start_from: StartFrom,
}

/// Starting position for a new subscription
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartFrom {
    /// Start from the earliest available event
    Earliest,
    /// Start from new events only (default)
    #[default]
    Latest,
    /// Start from compacted state (latest per key)
    Compacted,
}

/// Consumer offset for a subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerOffset {
    pub stream_id: String,
    pub subscription_id: String,
    pub partition: u32,
    pub offset: u64,
    pub committed_at: DateTime<Utc>,
}

/// Request to poll for events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollRequest {
    /// Maximum number of events to return
    #[serde(default = "default_batch_size")]
    pub limit: u32,
}

fn default_batch_size() -> u32 {
    100
}

/// Response from polling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollResponse {
    /// Events retrieved
    pub events: Vec<Event>,
    /// Opaque cursor for committing
    pub cursor: String,
    /// Number of events remaining (approximate)
    pub remaining: u64,
}

/// Cursor state (encoded in the cursor string)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorState {
    /// Offsets per partition at time of poll
    pub offsets: Vec<PartitionOffset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionOffset {
    pub partition: u32,
    pub offset: u64,
}

/// Request to commit offset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRequest {
    /// Cursor from poll response
    pub cursor: String,
}

/// Response after committing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResponse {
    /// Whether the commit succeeded
    pub success: bool,
}

/// Compacted state (latest per key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactedEvent {
    pub stream_id: String,
    pub key: String,
    pub event_type: String,
    pub data: serde_json::Value,
    /// Original sequence number
    pub sequence: u64,
    pub partition: u32,
    pub timestamp: DateTime<Utc>,
}

/// API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_creation() {
        let stream = Stream::new("orders".into(), 3, 168);
        assert_eq!(stream.stream_id, "orders");
        assert_eq!(stream.partition_count, 3);
        assert_eq!(stream.retention_hours, 168);
    }

    #[test]
    fn test_create_stream_request_defaults() {
        let json = r#"{"stream_id": "orders"}"#;
        let req: CreateStreamRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.partition_count, 3);
        assert_eq!(req.retention_hours, 168);
    }

    #[test]
    fn test_start_from_serialization() {
        assert_eq!(
            serde_json::to_string(&StartFrom::Earliest).unwrap(),
            r#""earliest""#
        );
        assert_eq!(
            serde_json::to_string(&StartFrom::Compacted).unwrap(),
            r#""compacted""#
        );
    }

    #[test]
    fn test_publish_event_type_rename() {
        let json = r#"{"key": "order-123", "type": "order.created", "data": {}}"#;
        let event: PublishEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "order.created");
    }

    #[test]
    fn test_error_response() {
        let err = ErrorResponse::new("not_found", "Stream not found");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("not_found"));
        assert!(json.contains("Stream not found"));
        assert!(!json.contains("details"));
    }
}
