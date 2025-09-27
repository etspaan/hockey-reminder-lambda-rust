mod model;
mod daysmart;
mod discord;

use std::env;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::{error, info, instrument};
use crate::daysmart::DaySmart;
use crate::discord::Discord;
    
    
    
#[instrument(skip(event))]
async fn handler(event: LambdaEvent<String>) -> Result<String, Error> {
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

    // Select destination (simple placeholder based on payload)
    let payload = event.payload; // in future this can be derived from the Lambda event
    let message_destination = if payload.contains("tests") {
        test_hook_url
    } else {
        hook_url
    };


    // Build the next game message within 5 days
    match day_smart.get_next_game_message(5, chrono::Utc::now()) {
        Some(message) => {
            info!(message = %message, "Prepared message");
            // Post to destination via Discord client
            let discord = Discord::new(message_destination.clone());
            if let Err(e) = discord.post(&message) {
                error!(error = %e, "Failed to post message to Discord");
            }
            Ok(message)
        }
        None => {
            use chrono::Utc;
            let msg = format!("No games in the next 5 days from {}.", Utc::now());
            info!("{}", msg);
            Ok(msg)
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
