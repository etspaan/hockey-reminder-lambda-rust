use std::collections::HashMap;

use tracing::{error, info, instrument, info_span};

use crate::model;
use crate::model::game::{GameInfo, GameCore};

/// Simple wrapper for the DaySmart API base URL used by this application.
#[derive(Debug)]
pub struct DaySmart {
    // Store our team's id directly to avoid borrowing from the document
    our_team_id: Option<i64>,
    team_names: HashMap<i64, String>,
    resource_names: HashMap<i64, String>,
    // Map of game event id -> (home_locker_res_id, away_locker_res_id)
    locker_map: HashMap<i64, (Option<i64>, Option<i64>)>,
    // Map of game event id -> core game data (parsed time and ids)
    game_map: HashMap<i64, GameCore>,
}

impl DaySmart {
    /// Construct a Daysmart instance for a specific team id and populate it with fetched data.
    #[instrument(level = "info", skip(team_id))]
    pub fn for_team(team_id: &str) -> Result<Self, String> {
        let daysmart_url = format!("https://apps.daysmartrecreation.com/dash/jsonapi/api/v1/teams/{}?cache[save]=false&include=events.eventType%2Cevents.homeTeam%2Cevents.visitingTeam%2Cevents.resource.facility%2Cevents.resourceArea%2Cevents.comments%2Cleague.playoffEvents.eventType%2Cleague.playoffEvents.homeTeam%2Cleague.playoffEvents.visitingTeam%2Cleague.playoffEvents.resource.facility%2Cleague.playoffEvents.resourceArea%2Cleague.playoffEvents.comments%2Cleague.programType%2Cproduct.locations%2CprogramType%2Cseason%2CskillLevel%2CageRange%2Csport&company=kraken", team_id);
        let response_result = {
            let _span = info_span!("daysmart_fetch", url = %daysmart_url).entered();
            ureq::get(&daysmart_url).call()
        };
        match response_result {
            Ok(response) => {
                let mut body_reader = response.into_body();
                match body_reader.read_to_string() {
                    Ok(body) => match Self::deserialize_team_document(&body) {
                        Ok(doc) => {
                            let total_included = doc.included.len();
                            let event_count = doc
                                .included
                                .iter()
                                .filter(|i| matches!(i, model::team::Included::Event { .. }))
                                .count();
                            let our_team_id = doc.data.id.parse::<i64>().ok();
                            let (team_names, resource_names, locker_map, game_map) = Self::build_maps(doc);
                            let team_name_str: &str = our_team_id
                                .and_then(|tid| team_names.get(&tid).map(|s| s.as_str()))
                                .unwrap_or("Unknown Team");
                            info!(team_name = %team_name_str, total_included, event_count, "Constructed DaySmart with TeamDocument");
                            Ok(DaySmart { our_team_id, team_names, resource_names, locker_map, game_map })
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

    /// Construct a DaySmart instance from a raw JSON response body (no network).
    #[allow(dead_code)]
    pub fn from_json(body: &str) -> Result<Self, String> {
        match Self::deserialize_team_document(body) {
            Ok(doc) => {
                let our_team_id = doc.data.id.parse::<i64>().ok();
                let (team_names, resource_names, locker_map, game_map) = Self::build_maps(doc);
                Ok(DaySmart { our_team_id, team_names, resource_names, locker_map, game_map })
            }
            Err(e) => Err(format!("Failed to deserialize into TeamDocument: {}", e)),
        }
    }

    /// Build lookup maps in a single pass: team names, resource names, locker room assignments, and game core data.
    fn build_maps(doc: model::team::TeamDocument) -> (
        HashMap<i64, String>,                // team_names
        HashMap<i64, String>,                // resource_names
        HashMap<i64, (Option<i64>, Option<i64>)>, // locker_map: game_id -> (home_res_id, away_res_id)
        HashMap<i64, GameCore>,              // game_map: game_id -> core
    ) {
        let mut team_names: HashMap<i64, String> = HashMap::new();
        let mut resource_names: HashMap<i64, String> = HashMap::new();
        let mut locker_map: HashMap<i64, (Option<i64>, Option<i64>)> = HashMap::new();
        let mut game_map: HashMap<i64, GameCore> = HashMap::new();

        // Insert our own team name from root data (move, no clone)
        if let Ok(tid) = doc.data.id.parse::<i64>() {
            team_names.insert(tid, doc.data.attributes.name);
        }

        for item in doc.included.into_iter() {
            match item {
                model::team::Included::TeamIncluded { id, attributes, .. } => {
                    if let Ok(tid) = id.parse::<i64>() {
                        team_names.insert(tid, attributes.name);
                    }
                }
                model::team::Included::Resource { id, attributes, .. } => {
                    if let Ok(rid) = id.parse::<i64>() {
                        if let Some(name) = attributes.name {
                            resource_names.insert(rid, name);
                        }
                    }
                }
                model::team::Included::Event { id, attributes, .. } => {
                    // Build locker map from locker room events (type L)
                    let is_locker = attributes
                        .event_type_id
                        .as_deref()
                        .map(|s| s.eq_ignore_ascii_case("L"))
                        .unwrap_or(false);
                    if is_locker {
                        if let (Some(game_id), Some(res_id)) = (attributes.parent_event_id, attributes.resource_id) {
                            let is_home = attributes
                                .locker_room_type
                                .as_deref()
                                .map(|s| s.eq_ignore_ascii_case("h"))
                                .unwrap_or(false);

                            let entry = locker_map.entry(game_id).or_insert((None, None));
                            if is_home {
                                entry.0 = Some(res_id);
                            } else {
                                entry.1 = Some(res_id);
                            }
                        }
                    }

                    // Also build game map from game events (type G)
                    let is_game = attributes
                        .event_type_id
                        .as_deref()
                        .map(|s| s.eq_ignore_ascii_case("g"))
                        .unwrap_or(false);
                    if is_game {
                        let date_str_opt = attributes.start_gmt.as_deref().or(attributes.start.as_deref());
                        if let Some(dt_str) = date_str_opt {
                            let parsed_dt_utc = chrono::DateTime::parse_from_rfc3339(dt_str)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .or_else(|_| {
                                    chrono::NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%dT%H:%M:%S")
                                        .map(|naive| chrono::TimeZone::from_utc_datetime(&chrono::Utc, &naive))
                                });
                            if let (Ok(dt), Ok(gid)) = (parsed_dt_utc, id.parse::<i64>()) {
                                game_map.insert(gid, GameCore { dt, h_id: attributes.hteam_id, v_id: attributes.vteam_id, res_id: attributes.resource_id });
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        (team_names, resource_names, locker_map, game_map)
    }

    /// Deserialize the Daysmart team document from a JSON string.
    #[instrument(level = "info", skip(body), fields(bytes = body.len()))]
    fn deserialize_team_document(body: &str) -> Result<model::team::TeamDocument, serde_json::Error> {
        serde_json::from_str::<model::team::TeamDocument>(body)
    }

    /// Format a Discord-friendly game message using stored document and name maps.
    fn format_game_message(&self, game: &GameInfo) -> String {
        // Use stored team id (extracted at construction time)
        let our_team_id_i64 = self.our_team_id;

        // Resolve names (borrow to avoid allocations)
        let h_name: &str = game
            .h_id
            .and_then(|id| self.team_names.get(&id).map(|s| s.as_str()))
            .unwrap_or("Home");
        let v_name: &str = game
            .v_id
            .and_then(|id| self.team_names.get(&id).map(|s| s.as_str()))
            .unwrap_or("Visitor");

        let resource_name: &str = game
            .res_id
            .and_then(|rid| self.resource_names.get(&rid).map(|s| s.as_str()))
            .unwrap_or("Unknown Arena");

        // Home vs away determines jersey color
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

        // Use only the pre-computed locker room for our team; no fallback search here.
        let our_locker_room_name: Option<&str> = {
            let rid_opt = if is_home { game.home_locker_res_id } else { game.away_locker_res_id };
            rid_opt.and_then(|rid| self.resource_names.get(&rid).map(|s| s.as_str()))
        };
        let locker_line = if let Some(lr) = our_locker_room_name {
            let mut s = String::with_capacity(12 + lr.len());
            s.push_str("\nLocker Room: ");
            s.push_str(lr);
            s
        } else {
            String::new()
        };

        format!(
            ":hockey: Kraken Hockey League Game :goal:\n{}\n{} at {}\n{} vs {}{}\n:shirt: {} Jerseys",
            date_str, time_str, resource_name, h_name, v_name, locker_line, jersey_color
        )
    }


    /// Find upcoming games within the next `days_ahead` days using the stored document.
    /// Accepts a specific current time `now_utc` to make this function easier to test.
    fn find_upcoming_games(&self, days_ahead: i64, now_utc: chrono::DateTime<chrono::Utc>) -> Vec<GameInfo> {
        use chrono::Duration;

        let window_end = now_utc + Duration::days(days_ahead);
        let mut games: Vec<GameInfo> = Vec::new();

        for (gid, core) in &self.game_map {
            let dt = core.dt;
            if dt < now_utc || dt > window_end {
                continue;
            }

            let (home_lr_id, away_lr_id) = if let Some((home_rid_opt, away_rid_opt)) = self.locker_map.get(gid) {
                (*home_rid_opt, *away_rid_opt)
            } else {
                (None, None)
            };

            games.push(GameInfo {
                dt,
                h_id: core.h_id,
                v_id: core.v_id,
                res_id: core.res_id,
                home_locker_res_id: home_lr_id,
                away_locker_res_id: away_lr_id,
            });
        }

        games
    }

    /// Determine the next game within `days_ahead` and return a formatted message if one exists.
    /// Returns Some(String) with the formatted message when a game is found, or None if not.
    /// Accepts a specific current time `now_utc` to make this function easier to test.
    pub fn get_next_game_message(&self, days_ahead: i64, now_utc: chrono::DateTime<chrono::Utc>) -> Option<String> {
        let mut games = self.find_upcoming_games(days_ahead, now_utc);
        if games.is_empty() {
            return None;
        }
        games.sort_by_key(|g| g.dt);
        Some(self.format_game_message(&games[0]))
    }
}
