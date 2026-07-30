use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct CalendarEvent {
    pub uid: String,
    pub title: String,
    pub start: DateTime<Utc>,
    pub exdates: Vec<DateTime<Utc>>,
    pub rrule: Option<String>,
}

pub fn parse_datetime(value: &str, tzid: Option<&str>) -> Result<DateTime<Utc>, String> {
    if value.ends_with('Z') {
        return chrono::DateTime::parse_from_str(value, "%Y%m%dT%H%M%SZ")
            .map(|x| x.with_timezone(&Utc))
            .map_err(|e| e.to_string());
    }

    let naive = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
        .map_err(|e| e.to_string())?;

    if let Some(zone) = tzid {
        let tz = Tz::from_str(zone).map_err(|_| format!("unknown TZID {zone}"))?;
        tz.from_local_datetime(&naive)
            .single()
            .ok_or("ambiguous/nonexistent local time".into())
            .map(|x| x.with_timezone(&Utc))
    } else {
        Ok(Utc.from_utc_datetime(&naive))
    }
}

pub fn weekly_occurrences(
    event: &CalendarEvent,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Vec<DateTime<Utc>> {
    let mut out = vec![];
    let mut x = event.start;
    while x <= to {
        if x >= from && !event.exdates.contains(&x) {
            out.push(x);
        }
        x += chrono::Duration::weeks(1);
    }
    out
}
