use chrono::{TimeZone, Utc};

use hockey_reminder_lambda_rust::daysmart::DaySmart;

fn load_sample() -> String {
    std::fs::read_to_string("tests/sample_response.json").expect("failed to read sample_response.json")
}

#[test]
fn formats_specific_game_with_locker_room() {
    // Arrange
    let json = load_sample();
    let ds = DaySmart::from_json(&json).expect("from_json failed");

    // Act: pick a fixed time before 2025-09-21 so that the next game is 2025-09-21 (event 312149)
    let now = Utc.with_ymd_and_hms(2025, 9, 20, 0, 0, 0).unwrap();
    let msg = ds.get_next_game_message(3, now).expect("expected a game within window");

    // Assert key elements from the 2025-09-21 game, including locker room
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

#[test]
fn daysmart_generates_benchapp_like_csv_next_4_months() {
    // Arrange
    let json = load_sample();
    let ds = DaySmart::from_json(&json).expect("from_json failed");

    // Act: choose a fixed date early in September, to include 9/21 and 9/28 games within ~4 months
    let now = Utc.with_ymd_and_hms(2025, 9, 1, 0, 0, 0).unwrap();
    let csv = ds.to_benchapp_csv(now);

    // Assert: header present
    assert!(csv.starts_with("Type,Game Type,Title (Optional),Away,Home,Date,Time,Duration,Location (Optional),Address (Optional),Notes (Optional)\n"));
    // Should contain at least one data row
    assert!(csv.lines().count() > 1, "csv was: {}", csv);
    // Contains team and location names derived from DaySmart
    assert!(csv.contains("Yacht Flippers"), "csv was: {}", csv);
    assert!(csv.contains("Seal Team Sticks") || csv.contains("Blackbirds"), "csv was: {}", csv);
    assert!(csv.contains("Starbucks Rink 1") || csv.contains("Olympic View Arena"), "csv was: {}", csv);
    // Notes should include jersey color and locker room when known
    assert!(csv.contains("Light Jerseys") || csv.contains("Dark Jerseys"), "expected jersey note in CSV. csv was: {}", csv);
    assert!(csv.contains("Locker Room:"), "expected locker room note in CSV when known. csv was: {}", csv);
}
