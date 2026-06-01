// src/panel/calendar.rs

use chrono::{Datelike, Local, Timelike};
use minijinja::context;

use crate::api::gcalendar::CalendarEvent;
use crate::model::calendar::{CalendarDayModel, CalendarEventModel};
use crate::render::{render_template_from_ctx, UiTheme};

// ── Constants ────────────────────────────────────────────────────────────────

/// Earliest hour shown in the grid (8 = 8 AM).
const START_HOUR: i32 = 8;
/// First hour NOT shown (19 = 7 PM, so the last visible slot is 6–7 PM).
const END_HOUR: i32 = 19;
/// Minimum displayed duration for short/zero-length events, in minutes.
const MIN_DISPLAY_MINUTES: i32 = 30;
/// Default number of day columns.
const DEFAULT_NUM_COLS: usize = 3;

// ── Public view model ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CalendarPanelViewModel {
    pub now_time: String,
    pub columns: Vec<CalendarDayModel>,
    pub num_cols: usize,
    /// First hour displayed in the grid (e.g. 8 for 8 AM).
    pub start_hour: i32,
    /// Exclusive end hour (e.g. 19 means the last tick is at 7 PM).
    pub end_hour: i32,
    /// Current time as minutes since midnight, for the now-indicator line.
    pub now_min: i32,
}

impl CalendarPanelViewModel {
    /// Build the weekly-grid view model.
    ///
    /// `events` must be sorted by start time and cover at least [start-of-today,
    /// start-of-today + num_cols days].
    pub fn from_events(events: &[CalendarEvent], num_cols: usize) -> Self {
        let now = Local::now();
        let today = now.date_naive();
        let now_min = now.hour() as i32 * 60 + now.minute() as i32;

        // Weekdays: start from today.  Weekends: jump to next Monday so
        // the visible columns cover the upcoming work week.
        let start_date = match today.weekday() {
            chrono::Weekday::Sat => today + chrono::Duration::days(2),
            chrono::Weekday::Sun => today + chrono::Duration::days(1),
            _ => today,
        };

        let columns: Vec<CalendarDayModel> = (0..num_cols)
            .map(|col_idx| {
                let col_date = start_date + chrono::Duration::days(col_idx as i64);
                let day_label = col_date.format("%a").to_string().to_uppercase();
                let date_num = col_date.format("%-d").to_string();
                let is_today = col_date == today;

                // Partition events for this calendar date.
                let day_events: Vec<&CalendarEvent> = events
                    .iter()
                    .filter(|e| e.start.date_naive() == col_date)
                    .collect();

                let all_day_events: Vec<String> = day_events
                    .iter()
                    .filter(|e| e.all_day)
                    .map(|e| e.summary.clone())
                    .collect();

                let timed_events: Vec<CalendarEventModel> = day_events
                    .iter()
                    .filter(|e| !e.all_day)
                    .map(|e| {
                        let start_min =
                            e.start.hour() as i32 * 60 + e.start.minute() as i32;
                        let end_min =
                            e.end.hour() as i32 * 60 + e.end.minute() as i32;
                        let duration_min = (end_min - start_min).max(MIN_DISPLAY_MINUTES);
                        CalendarEventModel {
                            summary: e.summary.clone(),
                            tentative: e.tentative.clone(),
                            time_label: e.time_label(),
                            location: e.location.clone(),
                            all_day: false,
                            start_min,
                            duration_min,
                            end_min,
                        }
                    })
                    .collect();

                CalendarDayModel {
                    day_label,
                    date_num,
                    is_today,
                    all_day_events,
                    events: timed_events,
                }
            })
            .collect();

        // Dynamic y-axis: shrink the visible time range to the span of actual
        // events so that hour rows are taller when events are clustered.
        let (start_hour, end_hour) = {
            let mut min_min = i32::MAX;
            let mut max_min = i32::MIN;
            for ev in columns.iter().flat_map(|col| col.events.iter()) {
                min_min = min_min.min(ev.start_min);
                max_min = max_min.max(ev.start_min + ev.duration_min);
            }
            if min_min == i32::MAX {
                // No timed events — use the default full-day range.
                (START_HOUR, END_HOUR)
            } else {
                // 30-minute padding on each side, then floor/ceil to the hour.
                let sh = ((min_min - 30).max(0) / 60).max(0);
                let eh = ((max_min + 89) / 60).min(24); // +89 = +30 min buffer, ceil
                (sh, eh)
            }
        };

        CalendarPanelViewModel {
            now_time: now.format("%-I:%M %p").to_string(),
            columns,
            num_cols,
            start_hour,
            end_hour,
            now_min,
        }
    }

    pub fn render_svg(&self, theme: UiTheme) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let template_name = match theme {
            UiTheme::Light => "calendar_panel.svg.j2",
            UiTheme::Dark => "calendar_panel_dark.svg.j2",
        };

        let svg = render_template_from_ctx(
            template_name,
            context! {
                now_time   => self.now_time,
                num_cols   => self.num_cols,
                start_hour => self.start_hour,
                end_hour   => self.end_hour,
                now_min    => self.now_min,
                columns => self.columns.iter().map(|col| context! {
                    day_label       => col.day_label.clone(),
                    date_num        => col.date_num.clone(),
                    is_today        => col.is_today,
                    all_day_events  => col.all_day_events.clone(),
                    events => col.events.iter().map(|e| context! {
                        summary      => e.summary.clone(),
                        tentative    => e.tentative,
                        time_label   => e.time_label.clone(),
                        start_min    => e.start_min,
                        duration_min => e.duration_min,
                    }).collect::<Vec<_>>(),
                }).collect::<Vec<_>>(),
            },
        )?;

        Ok(svg)
    }
}

/// Used by `main.rs` to get the appropriate `from` time for the weekly panel.
/// Mirrors the same weekend-snapping logic used in `from_events`, so that the
/// fetch window starts on the same Monday the columns start on.
pub fn today_start() -> chrono::DateTime<Local> {
    use chrono::{Datelike, TimeZone};
    let today = Local::now().date_naive();
    let start_date = match today.weekday() {
        chrono::Weekday::Sat => today + chrono::Duration::days(2),
        chrono::Weekday::Sun => today + chrono::Duration::days(1),
        _ => today,
    };
    start_date
        .and_hms_opt(0, 0, 0)
        .and_then(|dt| Local.from_local_datetime(&dt).single())
        .unwrap_or_else(Local::now)
}

/// Default number of columns if not specified in config — exported so
/// `main.rs` can use it without duplicating the constant.
pub const CALENDAR_DEFAULT_NUM_COLS: usize = DEFAULT_NUM_COLS;
