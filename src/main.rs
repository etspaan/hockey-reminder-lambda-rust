mod model;
mod daysmart;
mod discord;

use std::env;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::{error, info, instrument};
use crate::daysmart::DaySmart;
use crate::discord::Discord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Test,
    Production,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub mode: Mode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub message: String,
}

#[instrument(skip(event))]
async fn handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    // Config
    let hook_url = env::var("DISCORD_HOOK_URL").expect("DISCORD_HOOK_URL must be set");
    let test_hook_url = env::var("TEST_DISCORD_HOOK_URL").expect("TEST_DISCORD_HOOK_URL must be set for tests payloads");
    let team_id = env::var("TEAM_ID").expect("TEAM_ID env var is required");

    let day_smart = match DaySmart::for_team(&team_id) {
        Ok(ds) => ds,
        Err(e) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e).into());
        }
    };

    // Select destination based on request mode
    let payload = event.payload; // Derived from the Lambda event
    let message_destination = match payload.mode {
        Mode::Test => test_hook_url,
        Mode::Production => hook_url,
    };

    // Build the next game message within 5 days
    match day_smart.get_next_game_message(5, chrono::Utc::now()) {
        Some(message) => {
            info!(message = %message, "Prepared message");
            // Post to destination via Discord client
            let discord = Discord::new(message_destination);
            if let Err(e) = discord.post(&message) {
                error!(error = %e, "Failed to post message to Discord");
            }
            Ok(Response { message })
        }
        None => {
            use chrono::Utc;
            let msg = format!("No games in the next 5 days from {}.", Utc::now());
            info!("{}", msg);
            Ok(Response { message: msg })
        }
    }
}


#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize structured logging with tracing
    let _ = tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        .with_current_span(false)
        .with_target(false)
        .with_ansi(false)
        .without_time()
        .try_init();

    lambda_runtime::run(service_fn(handler)).await
}
