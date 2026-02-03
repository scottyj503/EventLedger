//! Partitioner for mapping event keys to partitions
//!
//! Uses consistent hashing to ensure the same key always goes to the same partition.
//! This is critical for maintaining order per key.

use sha2::{Digest, Sha256};

/// Partitioner maps keys to partition numbers
pub struct Partitioner {
    partition_count: u32,
}

impl Partitioner {
    /// Create a new partitioner with the given partition count
    pub fn new(partition_count: u32) -> Self {
        assert!(partition_count > 0, "partition_count must be > 0");
        Self { partition_count }
    }

    /// Map a key to a partition number (0-based)
    ///
    /// Uses SHA-256 hash for consistent distribution.
    /// The same key will always map to the same partition.
    pub fn partition(&self, key: &str) -> u32 {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let hash = hasher.finalize();

        // Use first 4 bytes of hash as u32
        let hash_value = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]);

        hash_value % self.partition_count
    }

    /// Get the partition count
    pub fn partition_count(&self) -> u32 {
        self.partition_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_consistent_partitioning() {
        let partitioner = Partitioner::new(3);

        // Same key should always return same partition
        let key = "order-123";
        let partition = partitioner.partition(key);

        for _ in 0..100 {
            assert_eq!(partitioner.partition(key), partition);
        }
    }

    #[test]
    fn test_partition_range() {
        let partitioner = Partitioner::new(5);

        // All partitions should be within range
        let keys = ["a", "b", "c", "d", "e", "order-1", "order-2", "user-abc"];
        for key in keys {
            let partition = partitioner.partition(key);
            assert!(partition < 5, "Partition {} out of range for key {}", partition, key);
        }
    }

    #[test]
    fn test_distribution() {
        let partitioner = Partitioner::new(4);
        let mut counts: HashMap<u32, u32> = HashMap::new();

        // Generate many keys and check distribution
        for i in 0..10000 {
            let key = format!("key-{}", i);
            let partition = partitioner.partition(&key);
            *counts.entry(partition).or_insert(0) += 1;
        }

        // Each partition should have roughly 25% (allow 20-30%)
        for partition in 0..4 {
            let count = counts.get(&partition).unwrap_or(&0);
            let percentage = (*count as f64 / 10000.0) * 100.0;
            assert!(
                percentage > 20.0 && percentage < 30.0,
                "Partition {} has {}% which is outside expected range",
                partition,
                percentage
            );
        }
    }

    #[test]
    fn test_different_keys_can_map_to_same_partition() {
        let partitioner = Partitioner::new(2);

        // With only 2 partitions, many keys will collide
        let mut found_collision = false;
        let base_partition = partitioner.partition("key-0");

        for i in 1..100 {
            let key = format!("key-{}", i);
            if partitioner.partition(&key) == base_partition {
                found_collision = true;
                break;
            }
        }

        assert!(found_collision, "Expected to find partition collision");
    }

    #[test]
    #[should_panic(expected = "partition_count must be > 0")]
    fn test_zero_partitions_panics() {
        Partitioner::new(0);
    }
}
