use hockey_reminder_lambda_rust::benchapp_csv::BenchAppCsv;
use chrono::NaiveDate;

#[test]
fn generates_expected_benchapp_csv_from_ics() {
    let ics = include_str!("sample.ics");
    let generator = BenchAppCsv::from_ics(ics);
    // cutoff before the sample event (event is at 2025-09-28 15:15)
    let cutoff = NaiveDate::from_ymd_opt(2025, 9, 28).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let csv = generator.to_csv(cutoff).expect("csv generation");
    let expected = include_str!("schedule.csv");
    // Normalize newlines to avoid Windows vs Unix differences
    let norm = |s: &str| s.replace("\r\n", "\n");
    assert_eq!(norm(&csv).trim_end(), norm(expected).trim_end());
}

#[test]
fn builds_discord_message_with_latest_date() {
    let ics = include_str!("sample.ics");
    let generator = BenchAppCsv::from_ics(ics);
    // cutoff before the sample event
    let cutoff = NaiveDate::from_ymd_opt(2025, 9, 28).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let msg = generator.discord_message(cutoff).expect("message generation");
    assert!(msg.contains("Games scheduled until 2025-09-28"), "message was: {}", msg);
}
