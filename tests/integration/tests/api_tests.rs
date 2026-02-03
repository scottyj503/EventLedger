//! Integration tests for EventLedger API
//!
//! Run with: EVENTLEDGER_API_URL=https://your-api.execute-api.us-west-2.amazonaws.com cargo test
//!
//! These tests require a deployed EventLedger instance.

use eventledger_integration_tests::{
    client::{
        ApiError, CreateStreamRequest, CreateSubscriptionRequest, EventLedgerClient, PublishEvent,
    },
    fixtures::{unique_key, unique_stream_id, unique_subscription_id},
    skip_if_no_api,
};
use pretty_assertions::assert_eq;
use serde_json::json;

/// Helper to get client or skip test
fn get_client() -> Option<EventLedgerClient> {
    match std::env::var("EVENTLEDGER_API_URL") {
        Ok(url) => Some(EventLedgerClient::new(&url)),
        Err(_) => {
            eprintln!("Skipping: EVENTLEDGER_API_URL not set");
            None
        }
    }
}

// ============================================================================
// Stream Tests
// ============================================================================

#[tokio::test]
async fn test_create_stream() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();

    // Create stream
    let stream = client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: Some(3),
            retention_hours: Some(24),
        })
        .await
        .expect("Failed to create stream");

    assert_eq!(stream.stream_id, stream_id);
    assert_eq!(stream.partition_count, 3);
    assert_eq!(stream.retention_hours, 24);

    // Cleanup
    let _ = client.delete_stream(&stream_id).await;
}

#[tokio::test]
async fn test_create_stream_defaults() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();

    // Create stream with defaults
    let stream = client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: None,
            retention_hours: None,
        })
        .await
        .expect("Failed to create stream");

    assert_eq!(stream.stream_id, stream_id);
    assert_eq!(stream.partition_count, 3); // default
    assert_eq!(stream.retention_hours, 168); // default (7 days)

    // Cleanup
    let _ = client.delete_stream(&stream_id).await;
}

#[tokio::test]
async fn test_create_duplicate_stream_fails() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();

    // Create stream
    client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: None,
            retention_hours: None,
        })
        .await
        .expect("Failed to create stream");

    // Try to create duplicate
    let result = client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: None,
            retention_hours: None,
        })
        .await;

    assert!(result.is_err());
    if let Err(ApiError::Http { status, body }) = result {
        assert_eq!(status.as_u16(), 409);
        assert!(body.contains("already_exists"));
    }

    // Cleanup
    let _ = client.delete_stream(&stream_id).await;
}

#[tokio::test]
async fn test_get_stream() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();

    // Create stream
    client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: Some(5),
            retention_hours: None,
        })
        .await
        .expect("Failed to create stream");

    // Get stream
    let stream = client
        .get_stream(&stream_id)
        .await
        .expect("Failed to get stream");

    assert_eq!(stream.stream_id, stream_id);
    assert_eq!(stream.partition_count, 5);

    // Cleanup
    let _ = client.delete_stream(&stream_id).await;
}

#[tokio::test]
async fn test_get_nonexistent_stream_fails() {
    let Some(client) = get_client() else { return };

    let result = client.get_stream("nonexistent-stream-12345").await;

    assert!(result.is_err());
    if let Err(ApiError::Http { status, .. }) = result {
        assert_eq!(status.as_u16(), 404);
    }
}

#[tokio::test]
async fn test_list_streams() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();

    // Create stream
    client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: None,
            retention_hours: None,
        })
        .await
        .expect("Failed to create stream");

    // List streams
    let response = client.list_streams().await.expect("Failed to list streams");

    // Should contain our stream
    assert!(response.streams.iter().any(|s| s.stream_id == stream_id));

    // Cleanup
    let _ = client.delete_stream(&stream_id).await;
}

#[tokio::test]
async fn test_delete_stream() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();

    // Create stream
    client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: None,
            retention_hours: None,
        })
        .await
        .expect("Failed to create stream");

    // Delete stream
    let response = client
        .delete_stream(&stream_id)
        .await
        .expect("Failed to delete stream");

    assert!(response.success);

    // Verify it's gone
    let result = client.get_stream(&stream_id).await;
    assert!(result.is_err());
}

// ============================================================================
// Event Tests
// ============================================================================

#[tokio::test]
async fn test_publish_single_event() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();
    let key = unique_key();

    // Create stream
    client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: Some(3),
            retention_hours: None,
        })
        .await
        .expect("Failed to create stream");

    // Publish event
    let event = PublishEvent {
        key: key.clone(),
        event_type: "order.created".to_string(),
        data: json!({
            "order_id": "123",
            "customer": "acme",
            "total": 99.99
        }),
    };

    let response = client
        .publish_event(&stream_id, event)
        .await
        .expect("Failed to publish event");

    assert_eq!(response.events.len(), 1);
    assert_eq!(response.events[0].stream_id, stream_id);
    assert_eq!(response.events[0].key, key);
    assert!(response.events[0].sequence > 0);

    // Cleanup
    let _ = client.delete_stream(&stream_id).await;
}

#[tokio::test]
async fn test_publish_batch_events() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();

    // Create stream
    client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: Some(3),
            retention_hours: None,
        })
        .await
        .expect("Failed to create stream");

    // Publish batch
    let events = vec![
        PublishEvent {
            key: unique_key(),
            event_type: "order.created".to_string(),
            data: json!({"order_id": "1"}),
        },
        PublishEvent {
            key: unique_key(),
            event_type: "order.created".to_string(),
            data: json!({"order_id": "2"}),
        },
        PublishEvent {
            key: unique_key(),
            event_type: "order.created".to_string(),
            data: json!({"order_id": "3"}),
        },
    ];

    let response = client
        .publish_events(&stream_id, events)
        .await
        .expect("Failed to publish events");

    assert_eq!(response.events.len(), 3);

    // Cleanup
    let _ = client.delete_stream(&stream_id).await;
}

#[tokio::test]
async fn test_publish_to_nonexistent_stream_fails() {
    let Some(client) = get_client() else { return };

    let event = PublishEvent {
        key: unique_key(),
        event_type: "test.event".to_string(),
        data: json!({}),
    };

    let result = client
        .publish_event("nonexistent-stream-12345", event)
        .await;

    assert!(result.is_err());
    if let Err(ApiError::Http { status, .. }) = result {
        assert_eq!(status.as_u16(), 404);
    }
}

// ============================================================================
// Subscription Tests
// ============================================================================

#[tokio::test]
async fn test_create_subscription() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();
    let subscription_id = unique_subscription_id();

    // Create stream
    client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: Some(3),
            retention_hours: None,
        })
        .await
        .expect("Failed to create stream");

    // Create subscription
    let subscription = client
        .create_subscription(
            &stream_id,
            &CreateSubscriptionRequest {
                subscription_id: subscription_id.clone(),
                start_from: Some("earliest".to_string()),
            },
        )
        .await
        .expect("Failed to create subscription");

    assert_eq!(subscription.stream_id, stream_id);
    assert_eq!(subscription.subscription_id, subscription_id);

    // Cleanup
    let _ = client.delete_stream(&stream_id).await;
}

// ============================================================================
// Poll and Commit Tests
// ============================================================================

#[tokio::test]
async fn test_poll_empty_stream() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();
    let subscription_id = unique_subscription_id();

    // Create stream
    client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: Some(3),
            retention_hours: None,
        })
        .await
        .expect("Failed to create stream");

    // Create subscription
    client
        .create_subscription(
            &stream_id,
            &CreateSubscriptionRequest {
                subscription_id: subscription_id.clone(),
                start_from: Some("earliest".to_string()),
            },
        )
        .await
        .expect("Failed to create subscription");

    // Poll
    let response = client
        .poll(&stream_id, &subscription_id, Some(10))
        .await
        .expect("Failed to poll");

    assert!(response.events.is_empty());
    assert!(!response.cursor.is_empty());

    // Cleanup
    let _ = client.delete_stream(&stream_id).await;
}

#[tokio::test]
async fn test_full_publish_poll_commit_cycle() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();
    let subscription_id = unique_subscription_id();
    let key = unique_key();

    // Create stream
    client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: Some(1), // Single partition for ordered test
            retention_hours: None,
        })
        .await
        .expect("Failed to create stream");

    // Create subscription starting from earliest
    client
        .create_subscription(
            &stream_id,
            &CreateSubscriptionRequest {
                subscription_id: subscription_id.clone(),
                start_from: Some("earliest".to_string()),
            },
        )
        .await
        .expect("Failed to create subscription");

    // Publish events
    for i in 1..=5 {
        client
            .publish_event(
                &stream_id,
                PublishEvent {
                    key: key.clone(),
                    event_type: "counter.incremented".to_string(),
                    data: json!({ "value": i }),
                },
            )
            .await
            .expect("Failed to publish event");
    }

    // Poll for events
    let poll_response = client
        .poll(&stream_id, &subscription_id, Some(10))
        .await
        .expect("Failed to poll");

    assert_eq!(poll_response.events.len(), 5);

    // Verify event order and content
    for (i, event) in poll_response.events.iter().enumerate() {
        assert_eq!(event.key, key);
        assert_eq!(event.event_type, "counter.incremented");
        let value = event.data.get("value").unwrap().as_i64().unwrap();
        assert_eq!(value, (i + 1) as i64);
    }

    // Commit
    let commit_response = client
        .commit(&stream_id, &subscription_id, &poll_response.cursor)
        .await
        .expect("Failed to commit");

    assert!(commit_response.success);

    // Poll again - should get no new events
    let poll_response2 = client
        .poll(&stream_id, &subscription_id, Some(10))
        .await
        .expect("Failed to poll again");

    assert!(poll_response2.events.is_empty());

    // Cleanup
    let _ = client.delete_stream(&stream_id).await;
}

#[tokio::test]
async fn test_same_key_goes_to_same_partition() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();
    let key = unique_key();

    // Create stream with multiple partitions
    client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: Some(10),
            retention_hours: None,
        })
        .await
        .expect("Failed to create stream");

    // Publish multiple events with same key
    let mut partitions = Vec::new();
    for i in 1..=10 {
        let response = client
            .publish_event(
                &stream_id,
                PublishEvent {
                    key: key.clone(),
                    event_type: "test.event".to_string(),
                    data: json!({ "seq": i }),
                },
            )
            .await
            .expect("Failed to publish event");

        partitions.push(response.events[0].partition);
    }

    // All events should be in the same partition
    let first_partition = partitions[0];
    for p in &partitions {
        assert_eq!(*p, first_partition, "Events with same key should go to same partition");
    }

    // Cleanup
    let _ = client.delete_stream(&stream_id).await;
}

// ============================================================================
// Compaction Tests (requires waiting for compactor)
// ============================================================================

#[tokio::test]
#[ignore] // Run manually: cargo test test_compaction -- --ignored
async fn test_compaction_updates_latest_value() {
    let Some(client) = get_client() else { return };

    let stream_id = unique_stream_id();
    let key = unique_key();

    // Create stream
    client
        .create_stream(&CreateStreamRequest {
            stream_id: stream_id.clone(),
            partition_count: Some(1),
            retention_hours: None,
        })
        .await
        .expect("Failed to create stream");

    // Publish multiple updates for same key
    for status in ["created", "processing", "shipped", "delivered"] {
        client
            .publish_event(
                &stream_id,
                PublishEvent {
                    key: key.clone(),
                    event_type: format!("order.{}", status),
                    data: json!({ "status": status }),
                },
            )
            .await
            .expect("Failed to publish event");
    }

    // Wait for compactor (in real test, check compacted endpoint)
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // TODO: Add endpoint to get compacted state and verify
    // The compacted state should show only the last event (delivered)

    // Cleanup
    let _ = client.delete_stream(&stream_id).await;
}
