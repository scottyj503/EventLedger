//! EventLedger Poll Lambda
//!
//! Handles:
//! - GET /streams/{stream_id}/subscriptions/{subscription_id}/poll
//! - POST /streams/{stream_id}/subscriptions/{subscription_id}/commit

use aws_config::BehaviorVersion;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use eventledger_core::{
    CommitRequest, CommitResponse, CursorState, DynamoClient, Error, ErrorResponse, Event,
    PartitionOffset, PollResponse,
};
use lambda_http::{run, service_fn, Body, Error as LambdaError, Request, RequestExt, Response};
use tracing::{error, info};

async fn handler(event: Request) -> Result<Response<Body>, LambdaError> {
    let method = event.method().as_str();
    let path = event.uri().path().to_string();

    // Extract path parameters
    let path_params = event.path_parameters();
    let stream_id = path_params
        .first("stream_id")
        .ok_or_else(|| "Missing stream_id")?
        .to_string();
    let subscription_id = path_params
        .first("subscription_id")
        .ok_or_else(|| "Missing subscription_id")?
        .to_string();

    // Initialize AWS clients
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let dynamo_client = aws_sdk_dynamodb::Client::new(&config);
    let client = DynamoClient::new(dynamo_client);

    // Route based on method and path
    if method == "GET" && path.ends_with("/poll") {
        handle_poll(&client, &stream_id, &subscription_id, &event).await
    } else if method == "POST" && path.ends_with("/commit") {
        handle_commit(&client, &stream_id, &subscription_id, &event).await
    } else {
        Ok(Response::builder()
            .status(404)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&ErrorResponse::new(
                "not_found",
                "Endpoint not found",
            ))?))?
        )
    }
}

async fn handle_poll(
    client: &DynamoClient,
    stream_id: &str,
    subscription_id: &str,
    event: &Request,
) -> Result<Response<Body>, LambdaError> {
    info!(stream_id = %stream_id, subscription_id = %subscription_id, "Processing poll request");

    // Parse limit from query string
    let query_params = event.query_string_parameters();
    let limit: u32 = query_params
        .first("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    // Verify subscription exists and get stream info
    let stream = match client.get_stream(stream_id).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(error_response(e)?);
        }
    };

    if let Err(e) = client.get_subscription(stream_id, subscription_id).await {
        return Ok(error_response(e)?);
    }

    // Collect events from all partitions
    let mut all_events: Vec<Event> = Vec::new();
    let mut offsets: Vec<PartitionOffset> = Vec::new();
    let total_remaining: u64 = 0;

    let per_partition_limit = (limit / stream.partition_count).max(1);

    for partition in 0..stream.partition_count {
        let offset = client
            .get_offset(stream_id, subscription_id, partition)
            .await
            .unwrap_or(0);

        let events = client
            .read_events(stream_id, partition, offset, per_partition_limit)
            .await
            .unwrap_or_default();

        if let Some(last) = events.last() {
            offsets.push(PartitionOffset {
                partition,
                offset: last.sequence,
            });
        } else {
            offsets.push(PartitionOffset { partition, offset });
        }

        all_events.extend(events);
    }

    // Sort by timestamp for consistent ordering across partitions
    all_events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    // Truncate to limit
    all_events.truncate(limit as usize);

    // Encode cursor
    let cursor_state = CursorState { offsets };
    let cursor_json = serde_json::to_string(&cursor_state)?;
    let cursor = URL_SAFE_NO_PAD.encode(cursor_json.as_bytes());

    let response = PollResponse {
        events: all_events,
        cursor,
        remaining: total_remaining,
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&response)?))?)
}

async fn handle_commit(
    client: &DynamoClient,
    stream_id: &str,
    subscription_id: &str,
    event: &Request,
) -> Result<Response<Body>, LambdaError> {
    info!(stream_id = %stream_id, subscription_id = %subscription_id, "Processing commit request");

    // Parse request body
    let body = event.body();
    let body_str = std::str::from_utf8(body).map_err(|_| "Invalid UTF-8 in body")?;
    let req: CommitRequest = serde_json::from_str(body_str)?;

    // Decode cursor
    let cursor_bytes = URL_SAFE_NO_PAD
        .decode(&req.cursor)
        .map_err(|_| Error::InvalidCursor("Invalid base64".to_string()))?;
    let cursor_json = std::str::from_utf8(&cursor_bytes)
        .map_err(|_| Error::InvalidCursor("Invalid UTF-8".to_string()))?;
    let cursor_state: CursorState = serde_json::from_str(cursor_json)
        .map_err(|_| Error::InvalidCursor("Invalid JSON".to_string()))?;

    // Commit offsets
    match client
        .commit_offsets(stream_id, subscription_id, &cursor_state.offsets)
        .await
    {
        Ok(_) => {
            let response = CommitResponse { success: true };
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response)?))?)
        }
        Err(e) => Ok(error_response(e)?),
    }
}

fn error_response(e: Error) -> Result<Response<Body>, LambdaError> {
    error!(error = %e, "Request failed");
    let status = e.status_code();
    let body = ErrorResponse::new(e.code(), e.to_string());
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body)?))?)
}

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(handler)).await
}
