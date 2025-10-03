use hockey_reminder_lambda_rust::benchapp_csv::BenchAppCsv;
use chrono::{NaiveDate, NaiveDateTime};

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


#[test]
fn to_csv_emits_header_and_filters_by_cutoff() {
    let ics = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nSUMMARY:Home vs Away\nDTSTART:20250102T030000Z\nDTEND:20250102T040000Z\nLOCATION:Rink X\\nAddr\nEND:VEVENT\nEND:VCALENDAR\n";
    let generator = BenchAppCsv::from_ics(ics);
    let cutoff_after = NaiveDateTime::parse_from_str("2025-01-03 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let cutoff_before = NaiveDateTime::parse_from_str("2025-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    let csv_after = generator.to_csv(cutoff_after).expect("csv generation");
    assert!(csv_after.lines().count() == 1, "should contain only header when after cutoff: {}", csv_after);

    let csv_before = generator.to_csv(cutoff_before).expect("csv generation");
    assert!(csv_before.lines().count() == 2, "should contain header + one row: {}", csv_before);
}

#[test]
fn discord_message_reports_latest_or_none() {
    let ics = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nSUMMARY:Home vs Away\nDTSTART:20250102T030000Z\nEND:VEVENT\nBEGIN:VEVENT\nSUMMARY:Another vs Team\nDTSTART:20250105T030000Z\nEND:VEVENT\nEND:VCALENDAR\n";
    let generator = BenchAppCsv::from_ics(ics);
    let cutoff = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let msg = generator.discord_message(cutoff).unwrap();
    assert!(msg.contains("2025-01-05"), "msg was: {}", msg);

    // Cutoff after all -> no upcoming
    let cutoff2 = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let msg2 = generator.discord_message(cutoff2).unwrap();
    assert!(msg2.contains("No upcoming games"), "msg was: {}", msg2);
}
