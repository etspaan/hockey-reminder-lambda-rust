use lambda_runtime::{Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};

use crate::benchapp_csv::BenchAppCsv;
use crate::daysmart::DaySmart;
use crate::discord::Discord;

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
pub async fn handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
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

    let mut handles: Vec<tokio::task::JoinHandle<String>> = Vec::new();

    for wf in workflows {
        let discord = discord.clone();
        match wf {
            Workflow::Daysmart => {
                // Clone because spawn_blocking's 'move' closure requires 'static owned data
                // and we cannot borrow from `payload` across await/join points. Each task
                // must own its inputs.
                let team_id = payload.team_id.clone();
                let handle = tokio::task::spawn_blocking(move || {
                    let day_smart = match DaySmart::for_team(&team_id) {
                        Ok(ds) => ds,
                        Err(e) => {
                            let msg = format!("DaySmart init error: {}", e);
                            error!(error = %msg, "DaySmart init failed");
                            return msg;
                        }
                    };
                    match day_smart.get_next_game_message(5, chrono::Utc::now()) {
                        Some(message) => {
                            info!(message = %message, "Prepared DaySmart message");
                            if let Err(e) = discord.post(&message) {
                                error!(error = %e, "Failed to post DaySmart message to Discord");
                                format!("DaySmart post failed: {}", e)
                            } else {
                                "DaySmart message posted".to_string()
                            }
                        }
                        None => {
                            use chrono::Utc;
                            let msg = format!("No games in the next 5 days from {}. Skipping Discord post.", Utc::now());
                            info!("{}", msg);
                            // Skip sending a Discord message when there are no upcoming games
                            "DaySmart: no upcoming games (skipped)".to_string()
                        }
                    }
                });
                handles.push(handle);
            }
            Workflow::Benchapp => {
                // Clone for the same reason: the spawned blocking task needs to own a 'static
                // String. Borrowing `&payload.ical_url` would not live long enough.
                let ical_url = payload.ical_url.clone();
                let handle = tokio::task::spawn_blocking(move || {
                    // Generate BenchApp CSV from the provided iCal URL and post as an attachment
                    let generator = BenchAppCsv::from_url(&ical_url);
                    let cutoff = chrono::Utc::now().naive_utc();
                    match generator.to_csv(cutoff) {
                        Ok(csv) => {
                            // If the CSV contains only the header (no data rows), skip posting to Discord
                            let has_rows = csv.lines().skip(1).any(|l| !l.trim().is_empty());
                            if !has_rows {
                                info!("No upcoming BenchApp events after cutoff; skipping Discord post");
                                return "BenchApp: no upcoming games (skipped)".to_string();
                            }

                            let filename = "benchapp_schedule.csv";
                            let content = generator
                                .discord_message(cutoff)
                                .unwrap_or_else(|_| "BenchApp import schedule attached.".to_string());
                            if let Err(e) = discord.post_with_attachment(&content, filename, csv.as_bytes()) {
                                error!(error = %e, "Failed to post BenchApp CSV to Discord");
                                format!("BenchApp post failed: {}", e)
                            } else {
                                "BenchApp CSV posted".to_string()
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to generate BenchApp CSV");
                            format!("BenchApp CSV generation failed: {}", e)
                        }
                    }
                });
                handles.push(handle);
            }
        }
    }

    let mut summaries: Vec<String> = Vec::new();
    for h in handles {
        match h.await {
            Ok(summary) => summaries.push(summary),
            Err(e) => summaries.push(format!("Workflow task join error: {}", e)),
        }
    }

    let summary = if summaries.is_empty() {
        "No workflows executed".to_string()
    } else {
        summaries.join("; ")
    };

    Ok(Response { message: summary })
}


