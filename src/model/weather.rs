//! Weather domain types.
//!
//! Defines the weather-specific enums and logic used by both the API layer
//! (to convert Open-Meteo codes into domain types) and the panel layer
//! (to select icons and display labels).
//!
//! ## Map from Open-Meteo WMO weather codes
//!
//! | Code range              | Condition       |
//! |-------------------------|-----------------|
//! | 0                       | Clear           |
//! | 1–2                     | Partly cloudy   |
//! | 3                       | Overcast        |
//! | 45                      | Fog             |
//! | 48                      | Dense fog       |
//! | 51, 53, 61, 80          | Light rain      |
//! | 55, 63, 65, 81, 82     | Rain            |
//! | 56, 57, 66, 67         | Freezing rain   |
//! | 71, 85                  | Light snow      |
//! | 73, 75, 77              | Snow            |
//! | 86                      | Blizzard        |
//! | 95                      | Thunderstorm    |
//! | 96, 99                  | Hail            |

use include_dir::{Dir, include_dir};

static ICONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/icons");

/// Whether the display is in day-time or night-time colour mode.
///
/// Affects icon selection: clear sky shows a sun during day and a moon at night.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DayPhase {
    Day,
    Night,
}

/// A weather condition mapped from WMO weather codes.
///
/// This is the canonical weather-state enum used throughout the application.
/// API responses are mapped to this, panel view-models consume it for display.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeatherCondition {
    Clear,
    PartlyCloudy,
    Cloudy,
    Rain,
    LightRain,
    LightSnow,
    Blizzard,
    Snow,
    FreezingRain,
    Hail,
    Thunderstorm,
    LightFog,
    HeavyFog,
}

/// Metric-card icon type (not weather-code-based, always the same icon).
#[derive(Debug, Clone)]
pub enum WeatherIcon {
    UVIndex,
    Rain,
    Wind,
    Pressure,
    Humidity,
    Visibility,
}

impl DayPhase {
    /// Determine day/night based purely on the hour in an ISO time string.
    ///
    /// Simple heuristic: 6 AM to 6 PM is day, everything else is night.
    /// Falls back to [`DayPhase::Day`] when the time cannot be parsed.
    pub fn from_time_iso(time_iso: &str) -> Self {
        let hour = time_iso
            .split_once('T')
            .and_then(|(_, hhmm)| hhmm.get(0..2))
            .and_then(|hh| hh.parse::<u8>().ok())
            .unwrap_or(12);
        if (6..18).contains(&hour) {
            DayPhase::Day
        } else {
            DayPhase::Night
        }
    }

    /// Determine day/night using actual sunrise/sunset ISO times.
    ///
    /// More accurate than [`from_time_iso`] because it accounts for seasonal
    /// and latitudinal variation. ISO string comparison works because the
    /// format is lexicographically ordered (YYYY-MM-DDTHH:MM).
    pub fn from_time_iso_with_sun(time_iso: &str, sunrise: &str, sunset: &str) -> Self {
        if time_iso >= sunrise && time_iso < sunset {
            DayPhase::Day
        } else {
            DayPhase::Night
        }
    }
}

impl WeatherIcon {
    /// Filename of the SVG icon for this metric-card type.
    pub fn filename(&self) -> &'static str {
        match self {
            WeatherIcon::UVIndex => "uv-index.svg",
            WeatherIcon::Rain => "raindrops.svg",
            WeatherIcon::Humidity => "raindrop.svg",
            WeatherIcon::Wind => "wind.svg",
            WeatherIcon::Pressure => "pressure.svg",
            WeatherIcon::Visibility => "visibility.svg",
        }
    }
}

impl WeatherCondition {
    /// Map a WMO weather code to the domain [`WeatherCondition`].
    ///
    /// Unknown codes are mapped to [`WeatherCondition::Cloudy`] as a safe
    /// fallback. See the [WMO specification](https://www.nodc.noaa.gov/archive/arc0021/0002199/1.1/data/0-data/HTML/WMO-CODE/WMO4677.HTM)
    /// for the full code table.
    pub fn from_weather_code(code: u16) -> Self {
        match code {
            0 => WeatherCondition::Clear,
            1 | 2 => WeatherCondition::PartlyCloudy,
            3 => WeatherCondition::Cloudy,
            45 => WeatherCondition::LightFog,
            48 => WeatherCondition::HeavyFog,
            51 | 53 | 61 | 80 => WeatherCondition::LightRain,
            55 | 63 | 65 | 81 | 82 => WeatherCondition::Rain,
            56 | 57 | 66 | 67 => WeatherCondition::FreezingRain,
            71 | 85 => WeatherCondition::LightSnow,
            73 | 75 | 77 => WeatherCondition::Snow,
            86 => WeatherCondition::Blizzard,
            95 => WeatherCondition::Thunderstorm,
            96 | 99 => WeatherCondition::Hail,
            _ => WeatherCondition::Cloudy,
        }
    }

    /// Return the SVG filename for this condition, given day or night phase.
    ///
    /// Falls back to `unknown.svg` if the icon file isn't found in `icons/`
    /// at compile time (embed safety check).
    pub fn icon(&self, day_phase: &DayPhase) -> &'static str {
        let icon = match (self, day_phase) {
            (WeatherCondition::Clear, DayPhase::Day) => "sun.svg",
            (WeatherCondition::Clear, DayPhase::Night) => "moon-clear.svg",
            (WeatherCondition::PartlyCloudy, DayPhase::Day) => "partly-cloudy.svg",
            (WeatherCondition::PartlyCloudy, DayPhase::Night) => "partly-cloudy-night.svg",
            (WeatherCondition::Cloudy, DayPhase::Day) => "cloudy.svg",
            (WeatherCondition::Cloudy, DayPhase::Night) => "cloudy-night.svg",
            (WeatherCondition::Rain, _) => "rain.svg",
            (WeatherCondition::LightRain, _) => "light-rain.svg",
            (WeatherCondition::LightSnow, _) => "snow.svg",
            (WeatherCondition::Blizzard, _) => "snow.svg",
            (WeatherCondition::Snow, _) => "snow.svg",
            (WeatherCondition::FreezingRain, _) => "rain.svg",
            (WeatherCondition::Hail, _) => "rain.svg",
            (WeatherCondition::Thunderstorm, _) => "thunder.svg",
            (WeatherCondition::LightFog, _) => "fog.svg",
            (WeatherCondition::HeavyFog, _) => "fog.svg",
        };

        if ICONS_DIR.get_file(icon).is_some() {
            icon
        } else {
            "unknown.svg"
        }
    }

    /// A human-readable summary of this condition (e.g. "Partly cloudy").
    pub fn summary(&self) -> &'static str {
        match self {
            WeatherCondition::Clear => "Clear",
            WeatherCondition::PartlyCloudy => "Partly cloudy",
            WeatherCondition::Cloudy => "Overcast",
            WeatherCondition::Rain => "Rain",
            WeatherCondition::LightRain => "Light rain",
            WeatherCondition::LightSnow => "Light snow",
            WeatherCondition::Blizzard => "Blizzard",
            WeatherCondition::Snow => "Snow",
            WeatherCondition::FreezingRain => "Freezing rain",
            WeatherCondition::Hail => "Hail",
            WeatherCondition::Thunderstorm => "Thunderstorm",
            WeatherCondition::LightFog => "Fog",
            WeatherCondition::HeavyFog => "Dense fog",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── DayPhase ──────────────────────────────────────────────────────

    #[test]
    fn day_phase_midday() {
        assert_eq!(DayPhase::from_time_iso("2026-06-05T14:00"), DayPhase::Day);
    }

    #[test]
    fn day_phase_midnight() {
        assert_eq!(DayPhase::from_time_iso("2026-06-05T02:00"), DayPhase::Night);
    }

    #[test]
    fn day_phase_boundary_sunrise() {
        assert_eq!(DayPhase::from_time_iso("2026-06-05T06:00"), DayPhase::Day);
    }

    #[test]
    fn day_phase_boundary_sunset() {
        assert_eq!(DayPhase::from_time_iso("2026-06-05T17:59"), DayPhase::Day);
        assert_eq!(DayPhase::from_time_iso("2026-06-05T18:00"), DayPhase::Night);
    }

    #[test]
    fn day_phase_with_sun() {
        let sr = "2026-06-05T05:30";
        let ss = "2026-06-05T20:45";
        assert_eq!(
            DayPhase::from_time_iso_with_sun("2026-06-05T12:00", sr, ss),
            DayPhase::Day
        );
        assert_eq!(
            DayPhase::from_time_iso_with_sun("2026-06-05T22:00", sr, ss),
            DayPhase::Night
        );
        assert_eq!(
            DayPhase::from_time_iso_with_sun("2026-06-05T05:29", sr, ss),
            DayPhase::Night
        );
    }

    #[test]
    fn day_phase_fallback_default() {
        // Malformed ISO string — should fall back to Day (hour defaults to 12).
        assert_eq!(DayPhase::from_time_iso("not-a-date"), DayPhase::Day);
    }

    // ── WeatherCondition::from_weather_code ───────────────────────────

    #[test]
    fn weather_code_clear() {
        assert_eq!(
            WeatherCondition::from_weather_code(0),
            WeatherCondition::Clear
        );
    }

    #[test]
    fn weather_code_partly_cloudy() {
        assert_eq!(
            WeatherCondition::from_weather_code(1),
            WeatherCondition::PartlyCloudy
        );
        assert_eq!(
            WeatherCondition::from_weather_code(2),
            WeatherCondition::PartlyCloudy
        );
    }

    #[test]
    fn weather_code_rain_variants() {
        assert_eq!(
            WeatherCondition::from_weather_code(61),
            WeatherCondition::LightRain
        );
        assert_eq!(
            WeatherCondition::from_weather_code(65),
            WeatherCondition::Rain
        );
    }

    #[test]
    fn weather_code_thunder() {
        assert_eq!(
            WeatherCondition::from_weather_code(95),
            WeatherCondition::Thunderstorm
        );
    }

    #[test]
    fn weather_code_unknown_falls_to_cloudy() {
        assert_eq!(
            WeatherCondition::from_weather_code(999),
            WeatherCondition::Cloudy
        );
    }

    // ── WeatherCondition::icon ────────────────────────────────────────

    #[test]
    fn icon_clear_day_is_sun() {
        assert_eq!(WeatherCondition::Clear.icon(&DayPhase::Day), "sun.svg");
    }

    #[test]
    fn icon_clear_night_is_moon() {
        assert_eq!(
            WeatherCondition::Clear.icon(&DayPhase::Night),
            "moon-clear.svg"
        );
    }

    #[test]
    fn icon_rain_is_same_day_or_night() {
        assert_eq!(WeatherCondition::Rain.icon(&DayPhase::Day), "rain.svg");
        assert_eq!(WeatherCondition::Rain.icon(&DayPhase::Night), "rain.svg");
    }

    #[test]
    fn icon_fallback_unknown() {
        let icon = WeatherCondition::Clear.icon(&DayPhase::Day);
        assert!(
            ICONS_DIR.get_file(icon).is_some(),
            "icon file {icon} should exist"
        );
    }

    // ── WeatherCondition::summary ─────────────────────────────────────

    #[test]
    fn summary_clear() {
        assert_eq!(WeatherCondition::Clear.summary(), "Clear");
    }

    #[test]
    fn summary_partly_cloudy() {
        assert_eq!(WeatherCondition::PartlyCloudy.summary(), "Partly cloudy");
    }

    // ── WeatherIcon::filename ─────────────────────────────────────────

    #[test]
    fn weather_icon_filenames() {
        assert_eq!(WeatherIcon::UVIndex.filename(), "uv-index.svg");
        assert_eq!(WeatherIcon::Rain.filename(), "raindrops.svg");
        assert_eq!(WeatherIcon::Wind.filename(), "wind.svg");
        assert_eq!(WeatherIcon::Pressure.filename(), "pressure.svg");
        assert_eq!(WeatherIcon::Humidity.filename(), "raindrop.svg");
        assert_eq!(WeatherIcon::Visibility.filename(), "visibility.svg");
    }
}
