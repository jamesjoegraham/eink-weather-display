// src/model/calendar.rs
//
// Domain types for calendar data, shared between the API layer and panel layer.

/// A single calendar event with display-ready string fields and timing data
/// needed for the weekly grid view.
#[derive(Debug, Clone)]
pub struct CalendarEventModel {
    pub summary: String,
    /// Human-readable start time, e.g. "9:00 AM".
    pub time_label: String,
    pub location: Option<String>,
    pub tentative: bool,
    pub all_day: bool,
    /// Minutes since midnight for the event start.
    pub start_min: i32,
    /// Duration in minutes, clamped to a display minimum.
    pub duration_min: i32,
    /// Actual end time in minutes since midnight (unclamped, for active-event detection).
    pub end_min: i32,
}

/// A single day column for the weekly grid view.
#[derive(Debug, Clone)]
pub struct CalendarDayModel {
    /// Short uppercase day name, e.g. "SUN".
    pub day_label: String,
    /// Day-of-month number string, e.g. "31".
    pub date_num: String,
    pub is_today: bool,
    /// All-day event summaries for this day.
    pub all_day_events: Vec<String>,
    /// Timed events for this day.
    pub events: Vec<CalendarEventModel>,
}
