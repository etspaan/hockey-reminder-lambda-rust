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

    /// Post a message with a single file attachment to a Discord webhook using multipart/form-data.
    /// See: https://discord.com/developers/docs/resources/webhook#execute-webhook
    /// The filename is what will appear in Discord; bytes are the file content.
    pub fn post_with_attachment(&self, content: &str, filename: &str, bytes: &[u8]) -> Result<(), String> {
        // Build payload_json for Discord attachments metadata
        let payload_json = serde_json::json!({
            "content": content,
            "attachments": [ { "id": 0, "filename": filename } ]
        }).to_string();

        // Build a simple multipart/form-data body manually to avoid extra crate features
        let boundary = format!("---------------------------{:x}{:x}", rand_seed(), rand_seed());
        let mut body: Vec<u8> = Vec::new();
        let crlf = b"\r\n";

        // Part 1: payload_json
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"payload_json\"\r\n");
        body.extend_from_slice(b"Content-Type: application/json\r\n\r\n");
        body.extend_from_slice(payload_json.as_bytes());
        body.extend_from_slice(crlf);

        // Part 2: file as files[0]
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(format!("Content-Disposition: form-data; name=\"files[0]\"; filename=\"{}\"\r\n", escape_header_value(filename)).as_bytes());
        body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
        body.extend_from_slice(bytes);
        body.extend_from_slice(crlf);

        // Close boundary
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let content_type = format!("multipart/form-data; boundary={}", boundary);

        let req = ureq::post(&self.hook_url).content_type(&content_type);
        match req.send(&body) {
            Ok(resp) => {
                info!(status = resp.status().as_u16(), "Posted message with attachment to Discord webhook");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "Failed to post attachment to Discord webhook");
                Err(format!("Failed to post attachment to Discord webhook: {}", e))
            }
        }
    }
}

// Tiny helper to make a boundary that's unlikely to collide; not cryptographically strong.
fn rand_seed() -> u64 {
    // Use a simple time-based seed; if std::time errors, fall back to a constant.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xABCDEF01)
}

// Escape double quotes in header values if any
fn escape_header_value(s: &str) -> String { s.replace('"', "'") }
