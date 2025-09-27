use tracing::{error, info};

/// Simple Discord webhook client encapsulating the hook URL.
#[derive(Debug, Clone)]
pub struct Discord {
    hook_url: String,
}

impl Discord {
    /// Create a new Discord client with the provided webhook URL.
    pub fn new(hook_url: String) -> Self {
        Self { hook_url }
    }

    /// Post a simple text message to the webhook URL.
    /// Returns Ok(()) on success, or Err(String) with a description on failure.
    pub fn post(&self, content: &str) -> Result<(), String> {
        let payload = serde_json::json!({ "content": content });
        match ureq::post(&self.hook_url).send_json(payload) {
            Ok(resp) => {
                info!(status = resp.status().as_u16(), "Posted message to Discord webhook");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "Failed to post to Discord webhook");
                Err(format!("Failed to post to Discord webhook: {}", e))
            }
        }
    }
}
