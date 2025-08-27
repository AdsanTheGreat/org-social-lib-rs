use chrono::{DateTime, FixedOffset};

pub fn parse_timestamp(s: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt);
    }
    // Example: 2025-08-20T15:23:45+0200
    let custom_format = "%Y-%m-%dT%H:%M:%S%z";
    DateTime::parse_from_str(s, custom_format)
}

/// Get the current local time, with timezone, in RFC 3339 format 
pub fn get_current_timestamp() -> String {
    let now: DateTime<FixedOffset> = chrono::Local::now().into();
    now.to_rfc3339()
}