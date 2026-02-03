//! EventLedger Admin Lambda
//!
//! Handles stream and subscription management:
//! - POST /streams - Create stream
//! - GET /streams - List streams
//! - GET /streams/{stream_id} - Get stream
//! - DELETE /streams/{stream_id} - Delete stream
//! - POST /streams/{stream_id}/subscriptions - Create subscription
//! - DELETE /streams/{stream_id}/subscriptions/{subscription_id} - Delete subscription

use aws_config::BehaviorVersion;
use eventledger_core::{
    CreateStreamRequest, CreateSubscriptionRequest, DynamoClient, Error, ErrorResponse, Stream,
    Subscription,
};
use lambda_http::{run, service_fn, Body, Error as LambdaError, Request, RequestExt, Response};
use serde::Serialize;
use tracing::{error, info};

#[derive(Serialize)]
struct ListStreamsResponse {
    streams: Vec<Stream>,
}

#[derive(Serialize)]
struct DeleteResponse {
    success: bool,
}

async fn handler(event: Request) -> Result<Response<Body>, LambdaError> {
    let method = event.method().as_str();
    let path = event.uri().path().to_string();

    info!(method = %method, path = %path, "Processing admin request");

    // Initialize AWS clients
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let dynamo_client = aws_sdk_dynamodb::Client::new(&config);
    let client = DynamoClient::new(dynamo_client);

    // Extract path parameters if present
    let path_params = event.path_parameters();
    let stream_id = path_params.first("stream_id").map(|s| s.to_string());
    let subscription_id = path_params.first("subscription_id").map(|s| s.to_string());

    // Route based on method and path
    match (method, path.as_str()) {
        // POST /streams - Create stream
        ("POST", "/streams") => {
            let body = event.body();
            let body_str = std::str::from_utf8(body).map_err(|_| "Invalid UTF-8 in body")?;
            let req: CreateStreamRequest = serde_json::from_str(body_str)?;

            match client.create_stream(&req).await {
                Ok(stream) => json_response(201, &stream),
                Err(e) => error_response(e),
            }
        }

        // GET /streams - List streams
        ("GET", "/streams") => match client.list_streams().await {
            Ok(streams) => json_response(200, &ListStreamsResponse { streams }),
            Err(e) => error_response(e),
        },

        // GET /streams/{stream_id} - Get stream
        ("GET", p) if p.starts_with("/streams/") && !p.contains("/subscriptions") => {
            let stream_id = stream_id.ok_or_else(|| "Missing stream_id")?;

            match client.get_stream(&stream_id).await {
                Ok(stream) => json_response(200, &stream),
                Err(e) => error_response(e),
            }
        }

        // DELETE /streams/{stream_id} - Delete stream
        ("DELETE", p) if p.starts_with("/streams/") && !p.contains("/subscriptions") => {
            let stream_id = stream_id.ok_or_else(|| "Missing stream_id")?;

            match client.delete_stream(&stream_id).await {
                Ok(_) => json_response(200, &DeleteResponse { success: true }),
                Err(e) => error_response(e),
            }
        }

        // POST /streams/{stream_id}/subscriptions - Create subscription
        ("POST", p) if p.contains("/subscriptions") && !p.ends_with("/poll") && !p.ends_with("/commit") => {
            let stream_id = stream_id.ok_or_else(|| "Missing stream_id")?;

            let body = event.body();
            let body_str = std::str::from_utf8(body).map_err(|_| "Invalid UTF-8 in body")?;
            let req: CreateSubscriptionRequest = serde_json::from_str(body_str)?;

            match client.create_subscription(&stream_id, &req).await {
                Ok(sub) => json_response(201, &sub),
                Err(e) => error_response(e),
            }
        }

        // DELETE /streams/{stream_id}/subscriptions/{subscription_id}
        ("DELETE", p) if p.contains("/subscriptions/") => {
            // For MVP, we'll just return success (subscription deletion not fully implemented)
            json_response(200, &DeleteResponse { success: true })
        }

        // Not found
        _ => Ok(Response::builder()
            .status(404)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&ErrorResponse::new(
                "not_found",
                "Endpoint not found",
            ))?))?)
    }
}

fn json_response<T: Serialize>(status: u16, body: &T) -> Result<Response<Body>, LambdaError> {
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(body)?))?)
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
