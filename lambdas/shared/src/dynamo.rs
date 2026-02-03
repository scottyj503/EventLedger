//! DynamoDB operations for EventLedger
//!
//! Single-table design with the following key patterns:
//!
//! | PK                          | SK                    | Purpose              |
//! |-----------------------------|-----------------------|----------------------|
//! | STREAM#{id}                 | META                  | Stream metadata      |
//! | STREAM#{id}                 | SUB#{sub_id}          | Subscription config  |
//! | STREAM#{id}#P{n}            | SEQ#{seq:020}         | Event in partition   |
//! | STREAM#{id}#SUB#{sub_id}    | OFFSET#P{n}           | Consumer offset      |
//! | STREAM#{id}#COMPACT         | KEY#{key}             | Compacted state      |
//! | STREAM#{id}#P{n}            | COUNTER               | Sequence counter     |

use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use chrono::Utc;
use serde_dynamo::{from_item, to_item};
use std::collections::HashMap;

use crate::errors::{Error, Result};
use crate::models::*;
use crate::partitioner::Partitioner;

/// DynamoDB table name (from environment)
const TABLE_NAME_ENV: &str = "EVENTLEDGER_TABLE";
const DEFAULT_TABLE_NAME: &str = "eventledger";

/// DynamoDB client for EventLedger operations
pub struct DynamoClient {
    client: Client,
    table_name: String,
}

impl DynamoClient {
    /// Create a new DynamoDB client
    pub fn new(client: Client) -> Self {
        let table_name = std::env::var(TABLE_NAME_ENV).unwrap_or_else(|_| DEFAULT_TABLE_NAME.to_string());
        Self { client, table_name }
    }

    /// Create with explicit table name (for testing)
    pub fn with_table_name(client: Client, table_name: String) -> Self {
        Self { client, table_name }
    }

    // =========================================================================
    // Stream Operations
    // =========================================================================

    /// Create a new stream
    pub async fn create_stream(&self, req: &CreateStreamRequest) -> Result<Stream> {
        let stream = Stream::new(
            req.stream_id.clone(),
            req.partition_count,
            req.retention_hours,
        );

        let mut item: HashMap<String, AttributeValue> = to_item(&stream).map_err(|e| Error::DynamoSerialization(e.to_string()))?;
        item.insert("PK".to_string(), AttributeValue::S(format!("STREAM#{}", stream.stream_id)));
        item.insert("SK".to_string(), AttributeValue::S("META".to_string()));

        // Use condition to prevent overwriting existing stream
        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_not_exists(PK)")
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("ConditionalCheckFailed") {
                    Error::StreamAlreadyExists(req.stream_id.clone())
                } else {
                    Error::Database(e.to_string())
                }
            })?;

        // Initialize sequence counters for each partition
        for partition in 0..req.partition_count {
            self.init_partition_counter(&req.stream_id, partition).await?;
        }

        Ok(stream)
    }

    /// Initialize sequence counter for a partition
    async fn init_partition_counter(&self, stream_id: &str, partition: u32) -> Result<()> {
        let mut item = HashMap::new();
        item.insert("PK".to_string(), AttributeValue::S(format!("STREAM#{}#P{}", stream_id, partition)));
        item.insert("SK".to_string(), AttributeValue::S("COUNTER".to_string()));
        item.insert("sequence".to_string(), AttributeValue::N("0".to_string()));

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Get a stream by ID
    pub async fn get_stream(&self, stream_id: &str) -> Result<Stream> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(format!("STREAM#{}", stream_id)))
            .key("SK", AttributeValue::S("META".to_string()))
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        match result.item {
            Some(item) => from_item(item).map_err(|e| Error::DynamoSerialization(e.to_string())),
            None => Err(Error::StreamNotFound(stream_id.to_string())),
        }
    }

    /// List all streams
    pub async fn list_streams(&self) -> Result<Vec<Stream>> {
        // Use Scan with filter since we can't use begins_with on partition key in Query
        let result = self
            .client
            .scan()
            .table_name(&self.table_name)
            .filter_expression("begins_with(PK, :prefix) AND SK = :meta")
            .expression_attribute_values(":prefix", AttributeValue::S("STREAM#".to_string()))
            .expression_attribute_values(":meta", AttributeValue::S("META".to_string()))
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let streams: Vec<Stream> = result
            .items
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| from_item(item).ok())
            .collect();

        Ok(streams)
    }

    /// Delete a stream and all associated data
    pub async fn delete_stream(&self, stream_id: &str) -> Result<()> {
        // First verify stream exists
        let stream = self.get_stream(stream_id).await?;

        // Delete stream metadata
        self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(format!("STREAM#{}", stream_id)))
            .key("SK", AttributeValue::S("META".to_string()))
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        // Delete partition counters
        for partition in 0..stream.partition_count {
            self.client
                .delete_item()
                .table_name(&self.table_name)
                .key("PK", AttributeValue::S(format!("STREAM#{}#P{}", stream_id, partition)))
                .key("SK", AttributeValue::S("COUNTER".to_string()))
                .send()
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
        }

        // Note: In production, you'd want to delete events, subscriptions, etc.
        // This could be done via a background job or TTL

        Ok(())
    }

    // =========================================================================
    // Event Operations
    // =========================================================================

    /// Publish events to a stream
    pub async fn publish_events(
        &self,
        stream_id: &str,
        events: &[PublishEvent],
    ) -> Result<Vec<PublishedEvent>> {
        let stream = self.get_stream(stream_id).await?;
        let partitioner = Partitioner::new(stream.partition_count);
        let now = Utc::now();

        let mut published = Vec::with_capacity(events.len());

        for event in events {
            let partition = partitioner.partition(&event.key);
            let sequence = self.increment_sequence(stream_id, partition).await?;

            let stored_event = Event {
                stream_id: stream_id.to_string(),
                partition,
                sequence,
                key: event.key.clone(),
                event_type: event.event_type.clone(),
                data: event.data.clone(),
                timestamp: now,
            };

            // Store the event
            let mut item: HashMap<String, AttributeValue> = to_item(&stored_event).map_err(|e| Error::DynamoSerialization(e.to_string()))?;
            item.insert(
                "PK".to_string(),
                AttributeValue::S(format!("STREAM#{}#P{}", stream_id, partition)),
            );
            item.insert(
                "SK".to_string(),
                AttributeValue::S(format!("SEQ#{:020}", sequence)),
            );

            self.client
                .put_item()
                .table_name(&self.table_name)
                .set_item(Some(item))
                .send()
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

            published.push(PublishedEvent {
                stream_id: stream_id.to_string(),
                partition,
                sequence,
                key: event.key.clone(),
                timestamp: now,
            });
        }

        Ok(published)
    }

    /// Increment and return the next sequence number for a partition
    async fn increment_sequence(&self, stream_id: &str, partition: u32) -> Result<u64> {
        let result = self
            .client
            .update_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(format!("STREAM#{}#P{}", stream_id, partition)))
            .key("SK", AttributeValue::S("COUNTER".to_string()))
            .update_expression("SET #seq = #seq + :inc")
            .expression_attribute_names("#seq", "sequence")
            .expression_attribute_values(":inc", AttributeValue::N("1".to_string()))
            .return_values(aws_sdk_dynamodb::types::ReturnValue::UpdatedNew)
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let attrs = result.attributes.ok_or_else(|| Error::Internal("No attributes returned".to_string()))?;
        let seq_attr = attrs.get("sequence").ok_or_else(|| Error::Internal("No sequence attribute".to_string()))?;

        match seq_attr {
            AttributeValue::N(n) => n.parse::<u64>().map_err(|e| Error::Internal(e.to_string())),
            _ => Err(Error::Internal("Invalid sequence type".to_string())),
        }
    }

    /// Read events from a partition starting at an offset
    pub async fn read_events(
        &self,
        stream_id: &str,
        partition: u32,
        from_offset: u64,
        limit: u32,
    ) -> Result<Vec<Event>> {
        let result = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :pk AND SK > :sk")
            .expression_attribute_values(
                ":pk",
                AttributeValue::S(format!("STREAM#{}#P{}", stream_id, partition)),
            )
            .expression_attribute_values(
                ":sk",
                AttributeValue::S(format!("SEQ#{:020}", from_offset)),
            )
            .limit(limit as i32)
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let events: Vec<Event> = result
            .items
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| from_item(item).ok())
            .collect();

        Ok(events)
    }

    // =========================================================================
    // Subscription Operations
    // =========================================================================

    /// Create a subscription
    pub async fn create_subscription(
        &self,
        stream_id: &str,
        req: &CreateSubscriptionRequest,
    ) -> Result<Subscription> {
        // Verify stream exists
        let stream = self.get_stream(stream_id).await?;

        let subscription = Subscription::new(stream_id.to_string(), req.subscription_id.clone());

        let mut item: HashMap<String, AttributeValue> = to_item(&subscription).map_err(|e| Error::DynamoSerialization(e.to_string()))?;
        item.insert("PK".to_string(), AttributeValue::S(format!("STREAM#{}", stream_id)));
        item.insert("SK".to_string(), AttributeValue::S(format!("SUB#{}", req.subscription_id)));

        // Use condition to prevent overwriting
        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_not_exists(PK)")
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("ConditionalCheckFailed") {
                    Error::SubscriptionAlreadyExists(req.subscription_id.clone())
                } else {
                    Error::Database(e.to_string())
                }
            })?;

        // Initialize offsets based on start_from
        let initial_offset = match req.start_from {
            StartFrom::Earliest => 0,
            StartFrom::Latest => self.get_latest_offset(stream_id, 0).await.unwrap_or(0),
            StartFrom::Compacted => 0, // Will read from compacted first
        };

        for partition in 0..stream.partition_count {
            let offset = if matches!(req.start_from, StartFrom::Latest) {
                self.get_latest_offset(stream_id, partition).await.unwrap_or(0)
            } else {
                initial_offset
            };
            self.set_offset(stream_id, &req.subscription_id, partition, offset).await?;
        }

        Ok(subscription)
    }

    /// Get the latest sequence number for a partition
    async fn get_latest_offset(&self, stream_id: &str, partition: u32) -> Result<u64> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(format!("STREAM#{}#P{}", stream_id, partition)))
            .key("SK", AttributeValue::S("COUNTER".to_string()))
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        match result.item {
            Some(item) => {
                let seq = item.get("sequence").ok_or_else(|| Error::Internal("No sequence".to_string()))?;
                match seq {
                    AttributeValue::N(n) => n.parse::<u64>().map_err(|e| Error::Internal(e.to_string())),
                    _ => Err(Error::Internal("Invalid sequence type".to_string())),
                }
            }
            None => Ok(0),
        }
    }

    /// Set consumer offset for a partition
    async fn set_offset(
        &self,
        stream_id: &str,
        subscription_id: &str,
        partition: u32,
        offset: u64,
    ) -> Result<()> {
        let mut item = HashMap::new();
        item.insert(
            "PK".to_string(),
            AttributeValue::S(format!("STREAM#{}#SUB#{}", stream_id, subscription_id)),
        );
        item.insert(
            "SK".to_string(),
            AttributeValue::S(format!("OFFSET#P{}", partition)),
        );
        item.insert("offset".to_string(), AttributeValue::N(offset.to_string()));
        item.insert(
            "committed_at".to_string(),
            AttributeValue::S(Utc::now().to_rfc3339()),
        );

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Get consumer offset for a partition
    pub async fn get_offset(
        &self,
        stream_id: &str,
        subscription_id: &str,
        partition: u32,
    ) -> Result<u64> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key(
                "PK",
                AttributeValue::S(format!("STREAM#{}#SUB#{}", stream_id, subscription_id)),
            )
            .key("SK", AttributeValue::S(format!("OFFSET#P{}", partition)))
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        match result.item {
            Some(item) => {
                let offset = item.get("offset").ok_or_else(|| Error::Internal("No offset".to_string()))?;
                match offset {
                    AttributeValue::N(n) => n.parse::<u64>().map_err(|e| Error::Internal(e.to_string())),
                    _ => Err(Error::Internal("Invalid offset type".to_string())),
                }
            }
            None => Err(Error::SubscriptionNotFound(subscription_id.to_string())),
        }
    }

    /// Commit offsets from cursor
    pub async fn commit_offsets(
        &self,
        stream_id: &str,
        subscription_id: &str,
        offsets: &[PartitionOffset],
    ) -> Result<()> {
        for po in offsets {
            self.set_offset(stream_id, subscription_id, po.partition, po.offset).await?;
        }
        Ok(())
    }

    /// Get subscription
    pub async fn get_subscription(&self, stream_id: &str, subscription_id: &str) -> Result<Subscription> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(format!("STREAM#{}", stream_id)))
            .key("SK", AttributeValue::S(format!("SUB#{}", subscription_id)))
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        match result.item {
            Some(item) => from_item(item).map_err(|e| Error::DynamoSerialization(e.to_string())),
            None => Err(Error::SubscriptionNotFound(subscription_id.to_string())),
        }
    }

    // =========================================================================
    // Compaction Operations
    // =========================================================================

    /// Store compacted state for a key
    pub async fn put_compacted(&self, event: &CompactedEvent) -> Result<()> {
        let mut item: HashMap<String, AttributeValue> = to_item(event).map_err(|e| Error::DynamoSerialization(e.to_string()))?;
        item.insert(
            "PK".to_string(),
            AttributeValue::S(format!("STREAM#{}#COMPACT", event.stream_id)),
        );
        item.insert(
            "SK".to_string(),
            AttributeValue::S(format!("KEY#{}", event.key)),
        );

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Get compacted state for a key
    pub async fn get_compacted(&self, stream_id: &str, key: &str) -> Result<Option<CompactedEvent>> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(format!("STREAM#{}#COMPACT", stream_id)))
            .key("SK", AttributeValue::S(format!("KEY#{}", key)))
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        match result.item {
            Some(item) => Ok(Some(from_item(item).map_err(|e| Error::DynamoSerialization(e.to_string()))?)),
            None => Ok(None),
        }
    }

    /// List all compacted events for a stream
    pub async fn list_compacted(&self, stream_id: &str) -> Result<Vec<CompactedEvent>> {
        let result = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :pk AND begins_with(SK, :prefix)")
            .expression_attribute_values(
                ":pk",
                AttributeValue::S(format!("STREAM#{}#COMPACT", stream_id)),
            )
            .expression_attribute_values(":prefix", AttributeValue::S("KEY#".to_string()))
            .send()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let events: Vec<CompactedEvent> = result
            .items
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| from_item(item).ok())
            .collect();

        Ok(events)
    }
}
