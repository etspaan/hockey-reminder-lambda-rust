use std::collections::HashMap;

use tracing::{error, info};

use crate::model;
use crate::model::game::GameInfo;

/// Simple wrapper for the Daysmart API base URL used by this application.
///
/// This struct intentionally contains only the URL string as requested.
#[derive(Debug)]
pub struct DaySmart {
    document: Option<model::team::TeamDocument>,
    team_names: HashMap<String, String>,
    resource_names: HashMap<i64, String>,
}

impl DaySmart {
    /// Construct a Daysmart instance for a specific team id and populate it with fetched data.
    pub fn for_team(team_id: &str) -> Result<Self, String> {
        let daysmart_url = format!("https://apps.daysmartrecreation.com/dash/jsonapi/api/v1/teams/{}?cache[save]=false&include=events.eventType%2Cevents.homeTeam%2Cevents.visitingTeam%2Cevents.resource.facility%2Cevents.resourceArea%2Cevents.comments%2Cleague.playoffEvents.eventType%2Cleague.playoffEvents.homeTeam%2Cleague.playoffEvents.visitingTeam%2Cleague.playoffEvents.resource.facility%2Cleague.playoffEvents.resourceArea%2Cleague.playoffEvents.comments%2Cleague.programType%2Cproduct.locations%2CprogramType%2Cseason%2CskillLevel%2CageRange%2Csport&company=kraken", team_id);
        match ureq::get(&daysmart_url).call() {
            Ok(response) => {
                let mut body_reader = response.into_body();
                match body_reader.read_to_string() {
                    Ok(body) => match Self::deserialize_team_document(&body) {
                        Ok(doc) => {
                            let team_name = doc.data.attributes.name.clone();
                            let total_included = doc.included.len();
                            let event_count = doc
                                .included
                                .iter()
                                .filter(|i| matches!(i, model::team::Included::Event { .. }))
                                .count();
                            let (team_names, resource_names) = Self::build_name_maps(&doc);
                            info!(team_name = %team_name, total_included, event_count, "Constructed DaySmart with TeamDocument");
                            Ok(DaySmart { document: Some(doc), team_names, resource_names })
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to deserialize into TeamDocument during construction");
                            Err(format!("Failed to deserialize into TeamDocument: {}", e))
                        }
                    },
                    Err(e) => {
                        error!(error = %e, "Failed to read response body during construction");
                        Err(format!("Failed to read response body: {}", e))
                    }
                }
            }
            Err(e) => {
                error!(error = %e, url = %daysmart_url, "Request failed during construction");
                Err(format!("Request failed: {}", e))
            }
        }
    }

    /// Construct a Daysmart instance from a raw JSON response body (no network).
    pub fn from_json(body: &str) -> Result<Self, String> {
        match Self::deserialize_team_document(body) {
            Ok(doc) => {
                let (team_names, resource_names) = Self::build_name_maps(&doc);
                Ok(DaySmart { document: Some(doc), team_names, resource_names })
            }
            Err(e) => Err(format!("Failed to deserialize into TeamDocument: {}", e)),
        }
    }

    /// Build lookup maps for team and resource names from a TeamDocument.
    fn build_name_maps(doc: &model::team::TeamDocument) -> (HashMap<String, String>, HashMap<i64, String>) {
        let mut team_names: HashMap<String, String> = HashMap::new();
        let mut resource_names: HashMap<i64, String> = HashMap::new();

        // Insert our own team name from root data
        team_names.insert(doc.data.id.clone(), doc.data.attributes.name.clone());

        for item in &doc.included {
            match item {
                model::team::Included::TeamIncluded { id, attributes, .. } => {
                    team_names.insert(id.clone(), attributes.name.clone());
                }
                model::team::Included::Resource { id, attributes, .. } => {
                    if let Ok(rid) = id.parse::<i64>() {
                        if let Some(name) = attributes.name.clone() {
                            resource_names.insert(rid, name);
                        }
                    }
                }
                _ => {}
            }
        }

        (team_names, resource_names)
    }

    /// Deserialize the Daysmart team document from a JSON string.
    fn deserialize_team_document(body: &str) -> Result<model::team::TeamDocument, serde_json::Error> {
        serde_json::from_str::<model::team::TeamDocument>(body)
    }

    /// Format a Discord-friendly game message using stored document and name maps.
    pub fn format_game_message(&self, game: &GameInfo) -> String {
        // Pull team info from stored document when available
        let (team_id_str, team_name) = if let Some(doc) = self.document.as_ref() {
            (doc.data.id.clone(), doc.data.attributes.name.clone())
        } else {
            (String::new(), String::from("Team"))
        };

        // Resolve names
        let h_name = game
            .h_id
            .and_then(|id| self.team_names.get(&id.to_string()).cloned())
            .unwrap_or_else(|| "Home".to_string());
        let v_name = game
            .v_id
            .and_then(|id| self.team_names.get(&id.to_string()).cloned())
            .unwrap_or_else(|| "Visitor".to_string());

        let resource_name = game
            .res_id
            .and_then(|rid| self.resource_names.get(&rid).cloned())
            .unwrap_or_else(|| "Unknown Arena".to_string());

        // Home vs away determines jersey color
        let our_team_id_i64 = team_id_str.parse::<i64>().ok();
        let is_home = match (our_team_id_i64, game.h_id) {
            (Some(our), Some(h)) => our == h,
            _ => false,
        };

        // Localize to Pacific time
        use chrono_tz::America::Los_Angeles;
        let local_dt = game.dt.with_timezone(&Los_Angeles);
        let date_str = local_dt.format("%a %b %e, %Y").to_string();
        let time_str = local_dt.format("%-I:%M %p").to_string();
        let jersey_color = if is_home { "Light" } else { "Dark" };

        // Try to find locker room info for this game
        let locker_line = if let Some(lr) = self.find_locker_room_for_game(game) {
            format!("\nLocker Room: {}", lr)
        } else {
            String::new()
        };

        let _ = team_name; // reserved for potential future use in the header

        format!(
            ":hockey: Kraken Hockey League Game :goal:\n{}\n{} at {}\n{} vs {}{}\n:shirt: {} Jerseys",
            date_str, time_str, resource_name, h_name, v_name, locker_line, jersey_color
        )
    }

    /// Try to find the locker room resource name for our team associated with this game.
    /// Strategy: look for a locker room event (event_type_id == "L") for our team (as home or visiting)
    /// with a start time within ±8 hours of the game's start, and return the associated resource name.
    fn find_locker_room_for_game(&self, game: &GameInfo) -> Option<String> {
        use chrono::{Duration, NaiveDateTime, TimeZone, Utc};

        let doc = self.document.as_ref()?;
        let our_team_id = doc.data.id.parse::<i64>().ok()?;

        let mut best: Option<(i64, i64)> = None; // (abs_diff_seconds, resource_id)

        for item in &doc.included {
            if let model::team::Included::Event { attributes, .. } = item {
                if attributes.event_type_id.as_deref() != Some("L") {
                    continue;
                }
                // Must be our team in either slot
                let is_ours = attributes.hteam_id == Some(our_team_id) || attributes.vteam_id == Some(our_team_id);
                if !is_ours { continue; }

                // Must have a resource id and a parsable time
                let Some(res_id) = attributes.resource_id else { continue; };
                let date_str_opt = attributes.start_gmt.as_ref().or(attributes.start.as_ref());
                let Some(dt_str) = date_str_opt else { continue; };

                let parsed_dt_utc = chrono::DateTime::parse_from_rfc3339(dt_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .or_else(|_| {
                        NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%dT%H:%M:%S").map(|naive| Utc.from_utc_datetime(&naive))
                    });

                let Ok(dt) = parsed_dt_utc else { continue; };

                let diff = (dt - game.dt).num_seconds().abs();
                // Consider only events within ±8 hours
                if diff <= Duration::hours(8).num_seconds() {
                    match best {
                        Some((best_diff, _)) if diff >= best_diff => {}
                        _ => best = Some((diff, res_id)),
                    }
                }
            }
        }

        if let Some((_, rid)) = best {
            return self.resource_names.get(&rid).cloned();
        }
        None
    }

    /// Find upcoming games within the next `days_ahead` days using the stored document.
    fn find_upcoming_games(&self, days_ahead: i64) -> Vec<GameInfo> {
        use chrono::{NaiveDateTime, TimeZone, Utc, Duration};

        let Some(doc) = self.document.as_ref() else {
            return Vec::new();
        };

        let now_utc = Utc::now();
        let window_end = now_utc + Duration::days(days_ahead);

        let mut games: Vec<GameInfo> = Vec::new();

        for item in &doc.included {
            if let model::team::Included::Event { attributes, .. } = item {
                if attributes.event_type_id.as_deref() != Some("g") {
                    continue;
                }
                let date_str_opt = attributes.start_gmt.as_ref().or(attributes.start.as_ref());
                if let Some(dt_str) = date_str_opt {
                    let parsed_dt_utc = chrono::DateTime::parse_from_rfc3339(dt_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .or_else(|_| {
                            NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%dT%H:%M:%S").map(|naive| Utc.from_utc_datetime(&naive))
                        });

                    if let Ok(dt) = parsed_dt_utc {
                        if dt >= now_utc && dt <= window_end {
                            games.push(GameInfo {
                                dt,
                                h_id: attributes.hteam_id,
                                v_id: attributes.vteam_id,
                                res_id: attributes.resource_id,
                            });
                        }
                    }
                }
            }
        }

        games
    }

    /// Determine the next game within `days_ahead` and return a formatted message if one exists.
    /// Returns Some(String) with the formatted message when a game is found, or None if not.
    pub fn get_next_game_message(&self, days_ahead: i64) -> Option<String> {
        let mut games = self.find_upcoming_games(days_ahead);
        if games.is_empty() {
            return None;
        }
        games.sort_by_key(|g| g.dt);
        let next_game = games[0];
        Some(self.format_game_message(&next_game))
    }
}
