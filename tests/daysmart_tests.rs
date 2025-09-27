use chrono::{NaiveDateTime, TimeZone, Utc};

use hockey_reminder_lambda_rust::model;
use hockey_reminder_lambda_rust::model::game::GameInfo;
use hockey_reminder_lambda_rust::daysmart::DaySmart;

fn load_sample() -> String {
    std::fs::read_to_string("tests/sample_response.json").expect("failed to read sample_response.json")
}

fn parse_dt(dt_str: &str) -> chrono::DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(dt_str)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%dT%H:%M:%S")
                .map(|naive| Utc.from_utc_datetime(&naive))
        })
        .expect("failed to parse date from sample")
}

#[test]
fn formats_specific_game_with_locker_room() {
    // Arrange
    let json = load_sample();
    let ds = DaySmart::from_json(&json).expect("from_json failed");

    // Also parse raw to locate specific event
    let doc: model::team::TeamDocument = serde_json::from_str(&json).expect("bad sample json");
    let mut target_game: Option<GameInfo> = None;

    for item in &doc.included {
        if let model::team::Included::Event { id, attributes, .. } = item {
            if id == "312149" {
                // This is a game on 2025-09-21 with a known locker room within ±8 hours
                let dt_str = attributes.start_gmt.as_ref().or(attributes.start.as_ref()).expect("missing time");
                let dt = parse_dt(dt_str);
                // Pre-compute locker room for the home team for this test case
                target_game = Some(GameInfo { dt, h_id: attributes.hteam_id, v_id: attributes.vteam_id, res_id: attributes.resource_id, home_locker_room: Some("LR11".to_string()), away_locker_room: None });
                break;
            }
        }
    }

    let game = target_game.expect("expected event 312149 in sample");

    // Act
    let msg = ds.format_game_message(&game);

    // Assert key elements
    assert!(msg.contains("Kraken Hockey League Game"));
    assert!(msg.contains("Starbucks Rink 1"), "message was: {}", msg);
    assert!(msg.contains("Yacht Flippers"), "message was: {}", msg);
    assert!(msg.contains("Seal Team Sticks"), "message was: {}", msg);
    assert!(msg.contains("Locker Room: LR11"), "message was: {}", msg);
    assert!(msg.contains(":shirt: Light Jerseys"), "message was: {}", msg);
}

#[test]
fn gets_next_upcoming_game_message() {
    // Arrange
    let json = load_sample();
    let ds = DaySmart::from_json(&json).expect("from_json failed");

    // Act: within the next 7 days from a fixed date (2025-09-27) for deterministic testing
    let now = Utc.with_ymd_and_hms(2025, 9, 27, 0, 0, 0).unwrap();
    let msg_opt = ds.get_next_game_message(7, now);

    let msg = msg_opt.expect("expected an upcoming game within 7 days in sample");

    // Assert: the 2025-09-28 game at Olympic View Arena vs Blackbirds
    assert!(msg.contains("Olympic View Arena"), "message was: {}", msg);
    assert!(msg.contains("Blackbirds"), "message was: {}", msg);
    assert!(msg.contains("Yacht Flippers"), "message was: {}", msg);
    // Our team is away in this game
    assert!(msg.contains(":shirt: Dark Jerseys"), "message was: {}", msg);
    // No locker room in sample for this day
    assert!(!msg.contains("Locker Room:"), "message was: {}", msg);
}
