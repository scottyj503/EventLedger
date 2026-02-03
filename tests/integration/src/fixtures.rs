//! Test fixtures and utilities

use uuid::Uuid;

/// Generate a unique stream ID for testing
pub fn unique_stream_id() -> String {
    format!("test-stream-{}", Uuid::new_v4().to_string()[..8].to_string())
}

/// Generate a unique subscription ID for testing
pub fn unique_subscription_id() -> String {
    format!("test-sub-{}", Uuid::new_v4().to_string()[..8].to_string())
}

/// Generate a unique event key for testing
pub fn unique_key() -> String {
    format!("key-{}", Uuid::new_v4().to_string()[..8].to_string())
}

/// Check if API URL is configured
pub fn api_url_configured() -> bool {
    std::env::var("EVENTLEDGER_API_URL").is_ok()
}

/// Skip test if API URL is not configured
#[macro_export]
macro_rules! skip_if_no_api {
    () => {
        if !$crate::fixtures::api_url_configured() {
            eprintln!("Skipping test: EVENTLEDGER_API_URL not set");
            return;
        }
    };
}
