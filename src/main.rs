mod model;
mod daysmart;
mod discord;
mod benchapp_csv;

use lambda_runtime::{service_fn, Error, LambdaEvent};
use tracing::{error, info, instrument};
use crate::daysmart::DaySmart;
use crate::discord::Discord;
use crate::benchapp_csv::BenchAppCsv;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Test,
    Production,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Workflow {
    Benchapp,
    Daysmart,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub mode: Mode,
    pub discord_hook_url: String,
    pub test_discord_hook_url: String,
    pub ical_url: String,
    pub team_id: String,
    #[serde(default)]
    pub workflows: Vec<Workflow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub message: String,
}

#[instrument(skip(event))]
async fn handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    // Extract config from the request payload instead of environment variables
    let payload = event.payload; // Derived from the Lambda event

    // Select destination based on request mode
    let message_destination = match payload.mode {
        Mode::Test => payload.test_discord_hook_url,
        Mode::Production => payload.discord_hook_url,
    };
    let discord = Discord::new(message_destination);

    // Decide workflows: default to Daysmart if none specified for backward compatibility
    let workflows = if payload.workflows.is_empty() {
        vec![Workflow::Daysmart]
    } else {
        payload.workflows.clone()
    };

    let mut summaries: Vec<String> = Vec::new();

    for wf in workflows {
        match wf {
            Workflow::Daysmart => {
                let day_smart = match DaySmart::for_team(&payload.team_id) {
                    Ok(ds) => ds,
                    Err(e) => {
                        let msg = format!("DaySmart init error: {}", e);
                        error!(error = %msg, "DaySmart init failed");
                        summaries.push(msg);
                        continue;
                    }
                };
                match day_smart.get_next_game_message(5, chrono::Utc::now()) {
                    Some(message) => {
                        info!(message = %message, "Prepared DaySmart message");
                        if let Err(e) = discord.post(&message) {
                            error!(error = %e, "Failed to post DaySmart message to Discord");
                            summaries.push(format!("DaySmart post failed: {}", e));
                        } else {
                            summaries.push("DaySmart message posted".to_string());
                        }
                    }
                    None => {
                        use chrono::Utc;
                        let msg = format!("No games in the next 5 days from {}.", Utc::now());
                        info!("{}", msg);
                        if let Err(e) = discord.post(&msg) {
                            error!(error = %e, "Failed to post DaySmart 'no games' message");
                            summaries.push(format!("DaySmart no-game post failed: {}", e));
                        } else {
                            summaries.push("DaySmart: no upcoming games".to_string());
                        }
                    }
                }
            }
            Workflow::Benchapp => {
                // Generate BenchApp CSV from the provided iCal URL and post as an attachment
                let generator = BenchAppCsv::from_url(&payload.ical_url);
                let cutoff = chrono::Utc::now().naive_utc();
                match generator.to_csv(cutoff) {
                    Ok(csv) => {
                        let filename = "benchapp_schedule.csv";
                        let content = generator.discord_message(cutoff).unwrap_or_else(|_| "BenchApp import schedule attached.".to_string());
                        if let Err(e) = discord.post_with_attachment(&content, filename, csv.as_bytes()) {
                            error!(error = %e, "Failed to post BenchApp CSV to Discord");
                            summaries.push(format!("BenchApp post failed: {}", e));
                        } else {
                            summaries.push("BenchApp CSV posted".to_string());
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to generate BenchApp CSV");
                        summaries.push(format!("BenchApp CSV generation failed: {}", e));
                    }
                }
            }
        }
    }

    let summary = if summaries.is_empty() {
        "No workflows executed".to_string()
    } else {
        summaries.join("; ")
    };

    Ok(Response { message: summary })
}


#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize structured logging with tracing
    let _ = tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        // Emit a closing event for each span, which includes its total duration
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .with_current_span(false)
        .with_target(false)
        .with_ansi(false)
        .try_init();

    lambda_runtime::run(service_fn(handler)).await
}
