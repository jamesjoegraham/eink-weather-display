//! Panel view-models: convert domain models into template-ready context.
//!
//! Each panel module provides a view-model struct with a `render_svg` method
//! that passes data to a Minijinja SVG template. Panels are selected at
//! runtime via `--panel <name>` in `main.rs`.

mod calendar;
mod weather;

pub use calendar::{CALENDAR_DEFAULT_NUM_COLS, CalendarPanelViewModel, today_start};
pub use weather::WeatherPanelViewModel;
