//! EventLedger API Client for testing

use reqwest::{Client, Response, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;

/// API client for EventLedger
pub struct EventLedgerClient {
    client: Client,
    base_url: String,
}

// Request/Response types

#[derive(Debug, Clone, Serialize)]
pub struct CreateStreamRequest {
    pub stream_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_hours: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Stream {
    pub stream_id: String,
    pub partition_count: u32,
    pub retention_hours: u32,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListStreamsResponse {
    pub streams: Vec<Stream>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublishEvent {
    pub key: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublishRequest {
    pub events: Vec<PublishEvent>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishedEvent {
    pub stream_id: String,
    pub partition: u32,
    pub sequence: u64,
    pub key: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishResponse {
    pub events: Vec<PublishedEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateSubscriptionRequest {
    pub subscription_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_from: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Subscription {
    pub stream_id: String,
    pub subscription_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Event {
    pub stream_id: String,
    pub partition: u32,
    pub sequence: u64,
    pub key: String,
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PollResponse {
    pub events: Vec<Event>,
    pub cursor: String,
    pub remaining: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitRequest {
    pub cursor: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteResponse {
    pub success: bool,
}

/// Result type for API responses
pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    /// HTTP error with status code and body
    Http { status: StatusCode, body: String },
    /// Network or serialization error
    Request(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Http { status, body } => write!(f, "HTTP {}: {}", status, body),
            ApiError::Request(msg) => write!(f, "Request error: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

impl EventLedgerClient {
    /// Create a new client with the given base URL
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Create a client from environment variable
    pub fn from_env() -> Self {
        let base_url = std::env::var("EVENTLEDGER_API_URL")
            .expect("EVENTLEDGER_API_URL environment variable not set");
        Self::new(&base_url)
    }

    // =========================================================================
    // Stream Operations
    // =========================================================================

    /// Create a new stream
    pub async fn create_stream(&self, req: &CreateStreamRequest) -> ApiResult<Stream> {
        self.post("/streams", req).await
    }

    /// List all streams
    pub async fn list_streams(&self) -> ApiResult<ListStreamsResponse> {
        self.get("/streams").await
    }

    /// Get a stream by ID
    pub async fn get_stream(&self, stream_id: &str) -> ApiResult<Stream> {
        self.get(&format!("/streams/{}", stream_id)).await
    }

    /// Delete a stream
    pub async fn delete_stream(&self, stream_id: &str) -> ApiResult<DeleteResponse> {
        self.delete(&format!("/streams/{}", stream_id)).await
    }

    // =========================================================================
    // Event Operations
    // =========================================================================

    /// Publish a single event
    pub async fn publish_event(
        &self,
        stream_id: &str,
        event: PublishEvent,
    ) -> ApiResult<PublishResponse> {
        self.post(&format!("/streams/{}/events", stream_id), &event)
            .await
    }

    /// Publish multiple events
    pub async fn publish_events(
        &self,
        stream_id: &str,
        events: Vec<PublishEvent>,
    ) -> ApiResult<PublishResponse> {
        let req = PublishRequest { events };
        self.post(&format!("/streams/{}/events", stream_id), &req)
            .await
    }

    // =========================================================================
    // Subscription Operations
    // =========================================================================

    /// Create a subscription
    pub async fn create_subscription(
        &self,
        stream_id: &str,
        req: &CreateSubscriptionRequest,
    ) -> ApiResult<Subscription> {
        self.post(&format!("/streams/{}/subscriptions", stream_id), req)
            .await
    }

    /// Poll for events
    pub async fn poll(
        &self,
        stream_id: &str,
        subscription_id: &str,
        limit: Option<u32>,
    ) -> ApiResult<PollResponse> {
        let path = match limit {
            Some(l) => format!(
                "/streams/{}/subscriptions/{}/poll?limit={}",
                stream_id, subscription_id, l
            ),
            None => format!(
                "/streams/{}/subscriptions/{}/poll",
                stream_id, subscription_id
            ),
        };
        self.get(&path).await
    }

    /// Commit offset
    pub async fn commit(
        &self,
        stream_id: &str,
        subscription_id: &str,
        cursor: &str,
    ) -> ApiResult<CommitResponse> {
        let req = CommitRequest {
            cursor: cursor.to_string(),
        };
        self.post(
            &format!(
                "/streams/{}/subscriptions/{}/commit",
                stream_id, subscription_id
            ),
            &req,
        )
        .await
    }

    // =========================================================================
    // HTTP Helpers
    // =========================================================================

    async fn get<T: DeserializeOwned>(&self, path: &str) -> ApiResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ApiError::Request(e.to_string()))?;

        self.handle_response(response).await
    }

    async fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> ApiResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| ApiError::Request(e.to_string()))?;

        self.handle_response(response).await
    }

    async fn delete<T: DeserializeOwned>(&self, path: &str) -> ApiResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| ApiError::Request(e.to_string()))?;

        self.handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(&self, response: Response) -> ApiResult<T> {
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ApiError::Request(e.to_string()))?;

        if status.is_success() {
            serde_json::from_str(&body).map_err(|e| ApiError::Request(e.to_string()))
        } else {
            Err(ApiError::Http { status, body })
        }
    }
}
