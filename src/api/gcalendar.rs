// src/api/gcalendar.rs
//
// Google Calendar integration via the private ICS feed.
//
// To get your ICS URL:
//   Google Calendar → Settings → [your calendar] → "Integrate calendar"
//   → copy "Secret address in iCal format"
//
// No OAuth or API key required; the secret URL acts as the auth token.

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use icalendar::{Calendar, CalendarComponent, CalendarDateTime, Component, DatePerhapsTime, EventLike, EventStatus};
use reqwest::blocking::Client;
use std::{error::Error, time::Duration as StdDuration};

use crate::config::CalendarConfig;

// ── Public model ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub summary: String,
    pub tentative: bool,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub location: Option<String>,
    pub description: Option<String>,
    /// True when the event has no specific time (all-day).
    pub all_day: bool,
}

// ── Entry point ─────────────────────────────────────────────────────────────

/// Fetch calendar events from the configured ICS feed.
///
/// `from` is the inclusive start of the window — pass `Local::now()` for an
/// agenda view or the start of today (midnight local) for a weekly grid.
/// Returns events ordered by start time, filtered to `[from, from + days_ahead]`.
pub fn fetch_events(config: &CalendarConfig, from: DateTime<Local>) -> Result<Vec<CalendarEvent>, Box<dyn Error>> {
    let days_ahead = config.days_ahead.unwrap_or(7);
    let max_events = config.max_events.unwrap_or(50);

    let ics_text = fetch_ics(&config.ics_url)?;
    let calendar: Calendar = ics_text
        .parse()
        .map_err(|_| "Failed to parse ICS calendar data")?;

    let cutoff = from + Duration::days(days_ahead);

    let mut events: Vec<CalendarEvent> = calendar
        .components
        .iter()
        .flat_map(|component| -> Vec<CalendarEvent> {
            let CalendarComponent::Event(event) = component else {
                return vec![];
            };
            // Recurring events: expand RRULE into individual occurrences.
            if let Some(rrule_val) = event.property_value("RRULE") {
                let rrule = parse_rrule(rrule_val);
                return expand_recurring_event(event, &rrule, from, cutoff);
            }
            // Non-recurring: single occurrence with window filter.
            parse_event(event, from, cutoff).into_iter().collect()
        })
        .collect();

    events.sort_by_key(|e| e.start);
    // Drop duplicates that arise when a recurring event has a RECURRENCE-ID
    // override — the base RRULE expansion and the override VEVENT both appear.
    // Compare at minute granularity: a RECURRENCE-ID override and its RRULE
    // expansion can produce timestamps that differ by sub-second amounts.
    events.dedup_by(|a, b| {
        a.start.date_naive() == b.start.date_naive()
            && a.start.hour() == b.start.hour()
            && a.start.minute() == b.start.minute()
            && a.summary == b.summary
    });
    events.truncate(max_events);
    Ok(events)
}

// ── HTTP fetch ───────────────────────────────────────────────────────────────

fn fetch_ics(url: &str) -> Result<String, Box<dyn Error>> {
    let client = Client::builder()
        .timeout(StdDuration::from_secs(15))
        .user_agent("eink-weather/1.0")
        .build()?;

    let response = client.get(url).send()?.error_for_status()?;
    Ok(response.text()?)
}

// ── ICS parsing ──────────────────────────────────────────────────────────────

fn parse_event(
    event: &icalendar::Event,
    from: DateTime<Local>,
    cutoff: DateTime<Local>,
) -> Option<CalendarEvent> {
    let summary = event.get_summary()?.to_string();

    let start_dpt = event.get_start()?;
    let (start, all_day) = dpt_to_local(start_dpt)?;

    // For all-day events, consider the event in-window if it starts before
    // the cutoff and ends after (or on) today, so multi-day events show up.
    let end_dpt = event.get_end();
    let end = end_dpt
        .and_then(|dpt| dpt_to_local(dpt))
        .map(|(dt, _)| dt)
        .unwrap_or_else(|| {
            if all_day {
                start + Duration::days(1)
            } else {
                start + Duration::hours(1)
            }
        });

    // Filter: keep events that overlap the [from, cutoff] window.
    if end <= from || start > cutoff {
        return None;
    }

    Some(CalendarEvent {
        summary,
        start,
        tentative: event.get_status() == Some(EventStatus::Tentative),
        end,
        location: event.get_location().map(|s: &str| s.to_string()),
        description: event.get_description().map(|s: &str| s.to_string()),
        all_day,
    })
}

/// Convert an icalendar `DatePerhapsTime` to a local `DateTime` plus an
/// `all_day` flag.  Returns `None` if the value cannot be represented.
fn dpt_to_local(dpt: DatePerhapsTime) -> Option<(DateTime<Local>, bool)> {
    match dpt {
        // All-day: treat midnight local time as the boundary.
        DatePerhapsTime::Date(date) => {
            let naive = naive_midnight(date)?;
            Some((local_from_naive(naive)?, true))
        }

        DatePerhapsTime::DateTime(cdt) => {
            let dt = cdt_to_local(cdt)?;
            Some((dt, false))
        }
    }
}

fn cdt_to_local(cdt: CalendarDateTime) -> Option<DateTime<Local>> {
    match cdt {
        // No timezone info — assume the system local timezone.
        CalendarDateTime::Floating(naive) => local_from_naive(naive),

        // Explicit UTC.
        CalendarDateTime::Utc(utc_dt) => Some(utc_dt.with_timezone(&Local)),

        // Named timezone (e.g., "America/Halifax").  Try chrono-tz; fall back
        // to treating the naive datetime as UTC on parse failure.
        CalendarDateTime::WithTimezone { date_time, tzid } => {
            if let Ok(tz) = tzid.parse::<chrono_tz::Tz>() {
                tz.from_local_datetime(&date_time)
                    .single()
                    .map(|dt| dt.with_timezone(&Local))
            } else {
                // Unknown timezone — treat as UTC (conservative fallback).
                let utc = Utc.from_utc_datetime(&date_time);
                Some(utc.with_timezone(&Local))
            }
        }
    }
}

fn naive_midnight(date: NaiveDate) -> Option<NaiveDateTime> {
    date.and_hms_opt(0, 0, 0)
}

fn local_from_naive(naive: NaiveDateTime) -> Option<DateTime<Local>> {
    Local.from_local_datetime(&naive).single()
}

// ── RRULE expansion ──────────────────────────────────────────────────────────

enum RRuleFreq { Daily, Weekly, Other }

struct RRuleInfo {
    freq: RRuleFreq,
    /// Days of week for WEEKLY (None = same weekday as DTSTART).
    byday: Option<Vec<chrono::Weekday>>,
    /// Recurrence interval (default 1).
    interval: i64,
    /// Optional hard end.
    until: Option<DateTime<Local>>,
    /// Max total occurrences counted from DTSTART.
    count: Option<u64>,
}

fn parse_rrule(s: &str) -> RRuleInfo {
    let mut freq = RRuleFreq::Other;
    let mut byday: Option<Vec<chrono::Weekday>> = None;
    let mut interval = 1i64;
    let mut until: Option<DateTime<Local>> = None;
    let mut count: Option<u64> = None;

    for part in s.split(';') {
        let mut kv = part.splitn(2, '=');
        let key = kv.next().unwrap_or("").trim();
        let val = kv.next().unwrap_or("").trim();
        match key {
            "FREQ" => freq = match val { "DAILY" => RRuleFreq::Daily, "WEEKLY" => RRuleFreq::Weekly, _ => RRuleFreq::Other },
            "BYDAY" => {
                let days: Vec<_> = val.split(',').filter_map(|d| parse_byday(d.trim())).collect();
                if !days.is_empty() { byday = Some(days); }
            }
            "INTERVAL" => interval = val.parse().unwrap_or(1),
            "UNTIL" => until = parse_until(val),
            "COUNT" => count = val.parse().ok(),
            _ => {}
        }
    }
    RRuleInfo { freq, byday, interval, until, count }
}

fn parse_byday(s: &str) -> Option<chrono::Weekday> {
    // Strip leading position prefix (e.g. "1MO" → "MO", "-1FR" → "FR").
    let s = s.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == '+');
    match s { "MO" => Some(chrono::Weekday::Mon), "TU" => Some(chrono::Weekday::Tue),
               "WE" => Some(chrono::Weekday::Wed), "TH" => Some(chrono::Weekday::Thu),
               "FR" => Some(chrono::Weekday::Fri), "SA" => Some(chrono::Weekday::Sat),
               "SU" => Some(chrono::Weekday::Sun), _ => None }
}

fn parse_until(s: &str) -> Option<DateTime<Local>> {
    // "20260601T170000Z" → UTC; "20260601" → midnight local.
    if let Ok(dt) = NaiveDateTime::parse_from_str(s.trim_end_matches('Z'), "%Y%m%dT%H%M%S") {
        return Some(Utc.from_utc_datetime(&dt).with_timezone(&Local));
    }
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y%m%d") {
        return local_from_naive(naive_midnight(date)?);
    }
    None
}

/// True if `date` is a valid occurrence of the rule rooted at `base`.
fn date_matches_rrule(rrule: &RRuleInfo, base: NaiveDate, date: NaiveDate) -> bool {
    let days = (date - base).num_days();
    if days < 0 { return false; }
    match rrule.freq {
        RRuleFreq::Daily => days % rrule.interval == 0,
        RRuleFreq::Weekly => {
            // Week offset from base's Monday.
            let base_mon = base - Duration::days(base.weekday().num_days_from_monday() as i64);
            let curr_mon = date - Duration::days(date.weekday().num_days_from_monday() as i64);
            let weeks = (curr_mon - base_mon).num_days() / 7;
            if weeks < 0 || weeks % rrule.interval != 0 { return false; }
            match &rrule.byday {
                Some(days) => days.contains(&date.weekday()),
                None => date.weekday() == base.weekday(),
            }
        }
        RRuleFreq::Other => false,
    }
}

/// Count occurrences in [base, before) for COUNT pre-computation.
fn occurrences_before(rrule: &RRuleInfo, base: NaiveDate, before: NaiveDate) -> u64 {
    let mut n = 0u64;
    let mut d = base;
    while d < before { if date_matches_rrule(rrule, base, d) { n += 1; } d += Duration::days(1); }
    n
}

fn expand_recurring_event(
    event: &icalendar::Event,
    rrule: &RRuleInfo,
    from: DateTime<Local>,
    cutoff: DateTime<Local>,
) -> Vec<CalendarEvent> {
    if matches!(rrule.freq, RRuleFreq::Other) { return vec![]; }

    let summary = match event.get_summary() { Some(s) => s.to_string(), None => return vec![] };
    let (base_start, all_day) = match event.get_start().and_then(dpt_to_local) {
        Some(v) => v, None => return vec![]
    };
    let base_end = event.get_end().and_then(dpt_to_local).map(|(dt, _)| dt)
        .unwrap_or_else(|| if all_day { base_start + Duration::days(1) } else { base_start + Duration::hours(1) });
    let duration = base_end - base_start;
    let location    = event.get_location().map(|s: &str| s.to_string());
    let description = event.get_description().map(|s: &str| s.to_string());

    let base_date    = base_start.date_naive();
    let window_start = from.date_naive();
    let window_end   = cutoff.date_naive();

    let mut occ_idx = if rrule.count.is_some() {
        occurrences_before(rrule, base_date, window_start)
    } else { 0 };

    let mut results = vec![];
    let mut current = base_date.max(window_start);

    while current <= window_end {
        if date_matches_rrule(rrule, base_date, current) {
            // COUNT gate.
            if let Some(max) = rrule.count {
                if occ_idx >= max { break; }
                occ_idx += 1;
            }
            // Rebuild occurrence start at same time-of-day in local tz.
            let Some(occ_start) = current
                .and_hms_opt(base_start.hour(), base_start.minute(), base_start.second())
                .and_then(|n| Local.from_local_datetime(&n).single())
            else { current += Duration::days(1); continue; };

            // UNTIL gate.
            if let Some(until) = &rrule.until { if occ_start > *until { break; } }

            let occ_end = occ_start + duration;
            if occ_end > from && occ_start < cutoff {
                results.push(CalendarEvent {
                    summary: summary.clone(), start: occ_start, end: occ_end,
                    tentative: event.get_status() == Some(EventStatus::Tentative),
                    location: location.clone(), description: description.clone(), all_day,
                });
            }
        }
        current += Duration::days(1);
    }
    results
}

// ── Helpers for callers ───────────────────────────────────────────────────────

impl CalendarEvent {
    /// Human-friendly time label, e.g. "2:30 PM" or "All day".
    pub fn time_label(&self) -> String {
        if self.all_day {
            "All day".to_string()
        } else {
            self.start.format("%-I:%M %p").to_string()
        }
    }

    /// Date label, e.g. "Mon Jun 2".
    pub fn date_label(&self) -> String {
        self.start.format("%a %b %-d").to_string()
    }

    /// True if the event starts today (local time).
    pub fn is_today(&self) -> bool {
        self.start.date_naive() == Local::now().date_naive()
    }
}
