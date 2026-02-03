//! EventLedger Publish Lambda
//!
//! Handles POST /streams/{stream_id}/events

use aws_config::BehaviorVersion;
use eventledger_core::{DynamoClient, Error, ErrorResponse, PublishEvent, PublishRequest, PublishResponse};
use lambda_http::{run, service_fn, Body, Error as LambdaError, Request, RequestExt, Response};
use tracing::{error, info};

async fn handler(event: Request) -> Result<Response<Body>, LambdaError> {
    // Extract stream_id from path
    let path_params = event.path_parameters();
    let stream_id = path_params
        .first("stream_id")
        .ok_or_else(|| "Missing stream_id")?
        .to_string();

    info!(stream_id = %stream_id, "Processing publish request");

    // Parse request body
    let body = event.body();
    let body_str = std::str::from_utf8(body).map_err(|_| "Invalid UTF-8 in body")?;

    // Support both single event and batch
    let events: Vec<PublishEvent> = if body_str.trim().starts_with('[') {
        serde_json::from_str(body_str)?
    } else if body_str.contains("\"events\"") {
        let req: PublishRequest = serde_json::from_str(body_str)?;
        req.events
    } else {
        // Single event
        vec![serde_json::from_str(body_str)?]
    };

    if events.is_empty() {
        return Ok(Response::builder()
            .status(400)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&ErrorResponse::new(
                "validation_error",
                "No events provided",
            ))?))?);
    }

    // Initialize AWS clients
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let dynamo_client = aws_sdk_dynamodb::Client::new(&config);
    let client = DynamoClient::new(dynamo_client);

    // Publish events
    match client.publish_events(&stream_id, &events).await {
        Ok(published) => {
            let response = PublishResponse { events: published };
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response)?))?)
        }
        Err(e) => {
            error!(error = %e, "Failed to publish events");
            let status = e.status_code();
            let body = ErrorResponse::new(e.code(), e.to_string());
            Ok(Response::builder()
                .status(status)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body)?))?)
        }
    }
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
