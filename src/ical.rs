use chrono::{Datelike, NaiveDateTime};
use icalendar::Component;

/// Minimal BenchAppCsv type for future CSV/ICS ingestion from KHL
pub struct Ical {
    pub calendar: Option<icalendar::Calendar>,
}

impl Ical {
    /// Construct from the provided KHL schedule URL.
    /// Performs a GET to the URL and attempts to parse the ICS into a Calendar; errors are logged.
    pub fn from_url(url: &str) -> Self {
        let mut calendar: Option<icalendar::Calendar> = None;

        // Attempt to fetch and print the body; report any errors, but keep constructor infallible.
        match ureq::get(url).call() {
            Ok(resp) => {
                // Note: ureq returns Ok(Response) even for HTTP error codes; warn if not 2xx.
                let status = resp.status();
                let code = status.as_u16();
                let mut body_reader = resp.into_body();
                match body_reader.read_to_string() {
                    Ok(body) => {
                        if code < 200 || code >= 300 {
                            eprintln!("BenchAppCsv GET non-success status: {}. Body: {}", code, body);
                        } else {
                            // Try to parse ICS into an icalendar::Calendar
                            match icalendar::parser::read_calendar(&body) {
                                Ok(parsed) => {
                                    let cal: icalendar::Calendar = parsed.into();
                                    calendar = Some(cal);
                                }
                                Err(e) => {
                                    eprintln!("BenchAppCsv ICS parse error: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("BenchAppCsv read body error: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("BenchAppCsv GET error: {}", e);
            }
        }

        Self { calendar }
    }

    /// Build from a raw ICS string (no network).
    #[allow(dead_code)]
    pub fn from_ics(ics: &str) -> Self {
        let calendar = match icalendar::parser::read_calendar(ics) {
            Ok(parsed) => Some(parsed.into()),
            Err(e) => {
                eprintln!("BenchAppCsv ICS parse error (from_ics): {}", e);
                None
            }
        };
        Self { calendar }
    }

    /// Generate a BenchApp import CSV representing all VEVENT entries in the ICS that start AFTER the provided cutoff datetime.
    /// Columns: Type,Game Type,Title (Optional),Away,Home,Date,Time,Duration,Location (Optional),Address (Optional),Notes (Optional)
    pub fn to_bench_app_csv(&self, cutoff: NaiveDateTime) -> Result<String, String> {
        let cal = self.calendar.as_ref().ok_or_else(|| "No ICS available".to_string())?;

        let mut out = String::new();
        out.push_str("Type,Game Type,Title (Optional),Away,Home,Date,Time,Duration,Location (Optional),Address (Optional),Notes (Optional)\n");

        for comp in &cal.components {
            if let icalendar::CalendarComponent::Event(e) = comp {
                // Extract properties directly from the event without serializing the calendar
                let summary = e.property_value("SUMMARY").unwrap_or("").to_string();
                let (home, away) = split_home_away(&summary);

                let dtstart_s = e.property_value("DTSTART").unwrap_or("").to_string();
                let dtend_s = e.property_value("DTEND").map(|s| s.to_string());

                let start = parse_dt(&dtstart_s).ok_or_else(|| format!("Invalid DTSTART: {}", dtstart_s))?;
                // Only include events strictly after the cutoff
                if !(start > cutoff) {
                    continue;
                }
                let end = dtend_s.and_then(|s| parse_dt(&s)).unwrap_or_else(|| start + chrono::Duration::minutes(60));

                let date_str = format!("{}/{}/{}", start.day(), start.month(), start.year());
                let time_str = start.format("%I:%M %p").to_string();

                let dur = end - start;
                let mins = dur.num_minutes().max(0);
                let duration_str = format!("{}:{:02}", mins / 60, mins % 60);

                let location_full = e.property_value("LOCATION").unwrap_or("").to_string();
                let (location_name, address) = split_location_address(&location_full);

                let notes = e.property_value("DESCRIPTION").unwrap_or("").to_string();

                let row = vec![
                    "GAME".to_string(),
                    "REGULAR".to_string(),
                    String::new(), // Title (optional)
                    away,
                    home,
                    date_str,
                    time_str,
                    duration_str,
                    location_name,
                    address,
                    notes,
                ]
                    .into_iter()
                    .map(|s| format!("\"{}\"", escape_quotes(&s)))
                    .collect::<Vec<String>>()
                    .join(",");

                out.push_str(&row);
                out.push('\n');
            }
        }

        Ok(out)
    }

    /// Build a concise Discord message indicating the latest scheduled game date
    /// among events strictly after the provided cutoff. Falls back to a generic
    /// message when none are found.
    pub fn discord_message(&self, cutoff: NaiveDateTime) -> Result<String, String> {
        let cal = self.calendar.as_ref().ok_or_else(|| "No ICS available".to_string())?;
        let mut latest: Option<NaiveDateTime> = None;
        for comp in &cal.components {
            if let icalendar::CalendarComponent::Event(e) = comp {
                let dtstart_s = e.property_value("DTSTART").unwrap_or("").to_string();
                if let Some(start) = parse_dt(&dtstart_s) {
                    if start > cutoff {
                        latest = Some(match latest { Some(cur) => cur.max(start), None => start });
                    }
                }
            }
        }
        if let Some(dt) = latest {
            Ok(format!("BenchApp import schedule attached. Games scheduled until {}.", dt.date()))
        } else {
            Ok("BenchApp import schedule attached. No upcoming games found.".to_string())
        }
    }
}

fn parse_dt(s: &str) -> Option<NaiveDateTime> {
    if s.is_empty() { return None; }
    // Strip trailing Z if present (treat as local/naive for CSV)
    let s2 = if s.ends_with('Z') { &s[..s.len()-1] } else { s };
    for pat in ["%Y%m%dT%H%M%S", "%Y%m%dT%H%M"].iter() {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s2, pat) { return Some(dt); }
    }
    // All-day dates (no time)
    if let Ok(date) = chrono::NaiveDate::parse_from_str(s2, "%Y%m%d") {
        return Some(date.and_hms_opt(0, 0, 0)?);
    }
    None
}

fn split_home_away(summary: &str) -> (String, String) {
    // Some summaries include a non-team prefix like "🏒Kraken Hockey League Game - ".
    // If there is a " - " and the trailing part looks like a matchup, drop the prefix.
    let trimmed = if let Some(idx) = summary.rfind(" - ") {
        let candidate = &summary[idx + 3..];
        if candidate.contains(" @ ") || candidate.contains(" vs ") {
            candidate
        } else {
            summary
        }
    } else {
        summary
    };

    if let Some((home, away)) = trimmed.split_once(" vs ") {
        (home.trim().to_string(), away.trim().to_string())
    } else if let Some((away, home)) = trimmed.split_once(" @ ") { // Away @ Home
        (home.trim().to_string(), away.trim().to_string())
    } else {
        (String::new(), String::new())
    }
}

fn split_location_address(location: &str) -> (String, String) {
    if let Some((name, addr)) = location.split_once('\n') {
        (name.trim().to_string(), addr.trim().to_string())
    } else if let Some((name, addr)) = location.split_once("\\n") {
        (name.trim().to_string(), addr.trim().to_string())
    } else {
        (location.trim().to_string(), String::new())
    }
}

fn escape_quotes(s: &str) -> String { s.replace('"', "\"") }

