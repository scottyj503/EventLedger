//! Error types for EventLedger

use thiserror::Error;

/// Result type alias using EventLedger Error
pub type Result<T> = std::result::Result<T, Error>;

/// EventLedger error types
#[derive(Error, Debug)]
pub enum Error {
    /// Stream not found
    #[error("Stream not found: {0}")]
    StreamNotFound(String),

    /// Stream already exists
    #[error("Stream already exists: {0}")]
    StreamAlreadyExists(String),

    /// Subscription not found
    #[error("Subscription not found: {0}")]
    SubscriptionNotFound(String),

    /// Subscription already exists
    #[error("Subscription already exists: {0}")]
    SubscriptionAlreadyExists(String),

    /// Invalid stream ID format
    #[error("Invalid stream ID: {0}")]
    InvalidStreamId(String),

    /// Invalid subscription ID format
    #[error("Invalid subscription ID: {0}")]
    InvalidSubscriptionId(String),

    /// Invalid cursor
    #[error("Invalid cursor: {0}")]
    InvalidCursor(String),

    /// Invalid event key
    #[error("Invalid event key: {0}")]
    InvalidEventKey(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// DynamoDB error
    #[error("Database error: {0}")]
    Database(String),

    /// JSON Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// DynamoDB serialization error
    #[error("DynamoDB serialization error: {0}")]
    DynamoSerialization(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Returns the error code for API responses
    pub fn code(&self) -> &'static str {
        match self {
            Error::StreamNotFound(_) => "stream_not_found",
            Error::StreamAlreadyExists(_) => "stream_already_exists",
            Error::SubscriptionNotFound(_) => "subscription_not_found",
            Error::SubscriptionAlreadyExists(_) => "subscription_already_exists",
            Error::InvalidStreamId(_) => "invalid_stream_id",
            Error::InvalidSubscriptionId(_) => "invalid_subscription_id",
            Error::InvalidCursor(_) => "invalid_cursor",
            Error::InvalidEventKey(_) => "invalid_event_key",
            Error::Validation(_) => "validation_error",
            Error::Database(_) => "database_error",
            Error::Serialization(_) => "serialization_error",
            Error::DynamoSerialization(_) => "serialization_error",
            Error::Internal(_) => "internal_error",
        }
    }

    /// Returns the HTTP status code for this error
    pub fn status_code(&self) -> u16 {
        match self {
            Error::StreamNotFound(_) => 404,
            Error::StreamAlreadyExists(_) => 409,
            Error::SubscriptionNotFound(_) => 404,
            Error::SubscriptionAlreadyExists(_) => 409,
            Error::InvalidStreamId(_) => 400,
            Error::InvalidSubscriptionId(_) => 400,
            Error::InvalidCursor(_) => 400,
            Error::InvalidEventKey(_) => 400,
            Error::Validation(_) => 400,
            Error::Database(_) => 500,
            Error::Serialization(_) => 400,
            Error::DynamoSerialization(_) => 500,
            Error::Internal(_) => 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = Error::StreamNotFound("orders".into());
        assert_eq!(err.code(), "stream_not_found");
        assert_eq!(err.status_code(), 404);
    }

    #[test]
    fn test_error_display() {
        let err = Error::StreamNotFound("orders".into());
        assert_eq!(err.to_string(), "Stream not found: orders");
    }

    #[test]
    fn test_validation_error() {
        let err = Error::Validation("stream_id is required".into());
        assert_eq!(err.code(), "validation_error");
        assert_eq!(err.status_code(), 400);
    }
}
