//! EventLedger Compactor Lambda
//!
//! Triggered by DynamoDB Streams to maintain compacted state.
//! For each new event, updates the compacted table with the latest value per key.

use aws_config::BehaviorVersion;
use aws_lambda_events::event::dynamodb::{Event, EventRecord};
use serde_dynamo::AttributeValue;
use chrono::Utc;
use eventledger_core::{CompactedEvent, DynamoClient};
use lambda_runtime::{run, service_fn, Error as LambdaError, LambdaEvent};
use tracing::{error, info, warn};

/// Extract string value from AttributeValue
fn get_string(av: &AttributeValue) -> Option<&str> {
    match av {
        AttributeValue::S(s) => Some(s.as_str()),
        _ => None,
    }
}

/// Extract number as string from AttributeValue
fn get_number_str(av: &AttributeValue) -> Option<&str> {
    match av {
        AttributeValue::N(n) => Some(n.as_str()),
        _ => None,
    }
}

/// Process a single DynamoDB Stream record
async fn process_record(client: &DynamoClient, record: &EventRecord) -> Result<(), String> {
    // Only process INSERT and MODIFY events
    let event_name = record.event_name.as_str();
    if event_name != "INSERT" && event_name != "MODIFY" {
        return Ok(());
    }

    // Get the new image (the event that was written)
    let new_image = &record.change.new_image;

    if new_image.is_empty() {
        warn!("Empty new image in record");
        return Ok(());
    }

    // Check if this is an event record (has SEQ# in SK)
    let sk = new_image
        .get("SK")
        .and_then(get_string)
        .unwrap_or("");

    if !sk.starts_with("SEQ#") {
        // Not an event record, skip
        return Ok(());
    }

    // Parse the event from the DynamoDB record
    let stream_id: String = new_image
        .get("stream_id")
        .and_then(get_string)
        .map(|s| s.to_string())
        .ok_or("Missing stream_id")?;

    let key: String = new_image
        .get("key")
        .and_then(get_string)
        .map(|s| s.to_string())
        .ok_or("Missing key")?;

    let event_type: String = new_image
        .get("event_type")
        .and_then(get_string)
        .map(|s| s.to_string())
        .ok_or("Missing event_type")?;

    let sequence: u64 = new_image
        .get("sequence")
        .and_then(get_number_str)
        .and_then(|n| n.parse().ok())
        .ok_or("Missing or invalid sequence")?;

    let partition: u32 = new_image
        .get("partition")
        .and_then(get_number_str)
        .and_then(|n| n.parse().ok())
        .ok_or("Missing or invalid partition")?;

    let data: serde_json::Value = new_image
        .get("data")
        .and_then(|v| {
            match v {
                AttributeValue::S(s) => serde_json::from_str(s).ok(),
                AttributeValue::M(_) => Some(serde_json::json!({})),
                _ => None,
            }
        })
        .unwrap_or(serde_json::Value::Null);

    let timestamp = new_image
        .get("timestamp")
        .and_then(get_string)
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);

    // Check if we should update compacted state
    // Only update if this is a newer sequence than what we have
    if let Ok(Some(existing)) = client.get_compacted(&stream_id, &key).await {
        if existing.sequence >= sequence {
            // Existing compacted state is newer, skip
            return Ok(());
        }
    }

    // Create compacted event
    let compacted = CompactedEvent {
        stream_id: stream_id.clone(),
        key: key.clone(),
        event_type,
        data,
        sequence,
        partition,
        timestamp,
    };

    // Store compacted state
    client
        .put_compacted(&compacted)
        .await
        .map_err(|e| format!("Failed to put compacted: {}", e))?;

    info!(
        stream_id = %stream_id,
        key = %key,
        sequence = sequence,
        "Updated compacted state"
    );

    Ok(())
}

async fn handler(event: LambdaEvent<Event>) -> Result<(), LambdaError> {
    let (payload, _context) = event.into_parts();

    info!(record_count = payload.records.len(), "Processing DynamoDB Stream batch");

    // Initialize AWS clients
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let dynamo_client = aws_sdk_dynamodb::Client::new(&config);
    let client = DynamoClient::new(dynamo_client);

    // Process each record
    for record in &payload.records {
        if let Err(e) = process_record(&client, record).await {
            error!(error = %e, "Failed to process record");
            // Continue processing other records
        }
    }

    Ok(())
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
