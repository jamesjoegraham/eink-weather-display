// src/api/weather.rs
// Weather API module extracted from weather.rs

use crate::config::{Config, MockConfig, WeatherConfig};
use crate::model::weather::{DayPhase, WeatherCondition};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::{error::Error, time::Duration};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForecastSource {
    Live,
    Demo,
    Mock,
}

impl std::str::FromStr for ForecastSource {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "demo" => Ok(ForecastSource::Demo),
            "mock" => Ok(ForecastSource::Mock),
            _ => Ok(ForecastSource::Live),
        }
    }
}

impl Default for ForecastSource {
    fn default() -> Self {
        ForecastSource::Live
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MockPreset {
    pub name: &'static str,
    pub weather_code: u16,
    pub precipitation_probability: u8,
    pub base_temp_c: f32,
}

const MOCK_PRESETS: [MockPreset; 11] = [
    MockPreset {
        name: "clear",
        weather_code: 0,
        precipitation_probability: 0,
        base_temp_c: 20.0,
    },
    MockPreset {
        name: "partly-cloudy",
        weather_code: 2,
        precipitation_probability: 5,
        base_temp_c: 18.0,
    },
    MockPreset {
        name: "cloudy",
        weather_code: 3,
        precipitation_probability: 10,
        base_temp_c: 16.0,
    },
    MockPreset {
        name: "fog",
        weather_code: 45,
        precipitation_probability: 15,
        base_temp_c: 9.0,
    },
    MockPreset {
        name: "drizzle",
        weather_code: 53,
        precipitation_probability: 45,
        base_temp_c: 13.0,
    },
    MockPreset {
        name: "rain",
        weather_code: 63,
        precipitation_probability: 70,
        base_temp_c: 11.0,
    },
    MockPreset {
        name: "showers",
        weather_code: 81,
        precipitation_probability: 65,
        base_temp_c: 12.0,
    },
    MockPreset {
        name: "snow",
        weather_code: 73,
        precipitation_probability: 60,
        base_temp_c: -2.0,
    },
    MockPreset {
        name: "snow-showers",
        weather_code: 85,
        precipitation_probability: 55,
        base_temp_c: -1.0,
    },
    MockPreset {
        name: "thunder",
        weather_code: 95,
        precipitation_probability: 85,
        base_temp_c: 14.0,
    },
    MockPreset {
        name: "hail-thunder",
        weather_code: 99,
        precipitation_probability: 95,
        base_temp_c: 8.0,
    },
];

#[derive(Debug, Clone)]
pub struct HourForecast {
    pub time_iso: String,
    pub hour_label: String,
    pub temperature_c: f32,
    pub wind_speed_kmh: f32,
    pub precipitation_probability: u8,
    pub humidity_percent: u8,
    pub pressure_hpa: f32,
    pub visibility_km: f32,
    pub uv_index: f32,
    pub weather_code: u16,
    pub day_phase: DayPhase,
    pub condition: WeatherCondition,
}

#[derive(Debug, Clone)]
pub struct ForecastStrip {
    pub hours: Vec<HourForecast>,
    pub sunrise: Option<String>,
    pub sunset: Option<String>,
}

pub fn load_forecast_by_source(
    source: ForecastSource,
    config: &Config,
) -> Result<ForecastStrip, Box<dyn Error>> {
    match source {
        ForecastSource::Live => {
            if let Some(ref weather_cfg) = config.weather {
                load_forecast_from_config(weather_cfg)
            } else {
                Err("Missing weather config section".into())
            }
        }
        ForecastSource::Demo => Ok(demo_forecast()),
        ForecastSource::Mock => {
            if let Some(ref mock_cfg) = config.mock {
                load_mock_forecast_from_config(mock_cfg)
            } else {
                Err("Missing mock config section".into())
            }
        }
    }
}

pub fn mock_preset_names() -> &'static [&'static str] {
    &[
        "clear",
        "partly-cloudy",
        "cloudy",
        "fog",
        "drizzle",
        "rain",
        "showers",
        "snow",
        "snow-showers",
        "thunder",
        "hail-thunder",
    ]
}

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    hourly: OpenMeteoHourly,
    daily: Option<OpenMeteoDaily>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoDaily {
    sunrise: Vec<String>,
    sunset: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoHourly {
    time: Vec<String>,
    temperature_2m: Vec<f32>,
    wind_speed_10m: Vec<f32>,
    precipitation_probability: Vec<u8>,
    relative_humidity_2m: Vec<u8>,
    surface_pressure: Vec<f32>,
    visibility: Vec<f32>,
    uv_index: Vec<f32>,
    weather_code: Vec<u16>,
}

pub fn load_forecast_from_config(cfg: &WeatherConfig) -> Result<ForecastStrip, Box<dyn Error>> {
    let latitude = cfg.lat.unwrap_or(44.655215);
    let longitude = cfg.lon.unwrap_or(-63.590768);
    let timezone = cfg.tz.clone().unwrap_or_else(|| "auto".to_owned());
    let hours = cfg.hours.unwrap_or(12);
    fetch_open_meteo_forecast(latitude as f32, longitude as f32, &timezone, hours)
}

pub fn fetch_open_meteo_forecast(
    latitude: f32,
    longitude: f32,
    timezone: &str,
    hours: usize,
) -> Result<ForecastStrip, Box<dyn Error>> {
    let tz = timezone.replace('/', "%2F");
    let forecast_hours = hours.clamp(2, 48);
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={latitude}&longitude={longitude}&hourly=temperature_2m,wind_speed_10m,precipitation_probability,relative_humidity_2m,surface_pressure,visibility,uv_index,weather_code&daily=sunrise,sunset&timezone={tz}&forecast_hours={forecast_hours}"
    );

    println!("Sending request to {}", url);

    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
    let response = client
        .get(url)
        .header("User-Agent", "eink-writer/0.1")
        .send()?
        .error_for_status()?;

    let payload: OpenMeteoResponse = response.json()?;

    let available = payload
        .hourly
        .time
        .len()
        .min(payload.hourly.temperature_2m.len())
        .min(payload.hourly.wind_speed_10m.len())
        .min(payload.hourly.precipitation_probability.len())
        .min(payload.hourly.relative_humidity_2m.len())
        .min(payload.hourly.surface_pressure.len())
        .min(payload.hourly.visibility.len())
        .min(payload.hourly.uv_index.len())
        .min(payload.hourly.weather_code.len());

    let count = hours.min(available);
    let mut entries = Vec::with_capacity(count);

    // Pick the first sunrise/sunset event that falls inside the forecast window.
    // This keeps overnight strips (e.g., 22:00 -> 10:00) aligned with the next day's sunrise.
    let (sunrise, sunset) = if let Some(daily) = &payload.daily {
        let first_hour = payload.hourly.time.first().map(String::as_str);
        let last_hour = payload
            .hourly
            .time
            .get(count.saturating_sub(1))
            .map(String::as_str);

        let sunrise = first_event_in_window(&daily.sunrise, first_hour, last_hour)
            .or_else(|| daily.sunrise.first().cloned());
        let sunset = first_event_in_window(&daily.sunset, first_hour, last_hour)
            .or_else(|| daily.sunset.first().cloned());

        (sunrise, sunset)
    } else {
        (None, None)
    };

    for index in 0..count {
        let weather_code = payload.hourly.weather_code[index];
        let time_iso = payload.hourly.time[index].clone();
        let day_phase = match (&sunrise, &sunset) {
            (Some(sr), Some(ss)) => DayPhase::from_time_iso_with_sun(&time_iso, sr, ss),
            _ => DayPhase::from_time_iso(&time_iso),
        };
        let condition = WeatherCondition::from_weather_code(weather_code);
        entries.push(HourForecast {
            time_iso,
            hour_label: hour_label(&payload.hourly.time[index]),
            temperature_c: payload.hourly.temperature_2m[index],
            wind_speed_kmh: payload.hourly.wind_speed_10m[index],
            precipitation_probability: payload.hourly.precipitation_probability[index],
            humidity_percent: payload.hourly.relative_humidity_2m[index],
            pressure_hpa: payload.hourly.surface_pressure[index],
            visibility_km: payload.hourly.visibility[index] / 1000.0,
            uv_index: payload.hourly.uv_index[index],
            weather_code,
            day_phase,
            condition,
        });
    }

    Ok(ForecastStrip {
        hours: entries,
        sunrise,
        sunset,
    })
}

pub fn demo_forecast() -> ForecastStrip {
    let demo_codes = [0, 1, 3, 61, 63, 95, 2];
    let mut hours = Vec::with_capacity(demo_codes.len());

    for (offset, code) in demo_codes.into_iter().enumerate() {
        let hour = 8 + offset as i32;
        let label = format!("{:02}:00", hour);
        let time_iso = format!("2026-05-30T{}", label);
        let day_phase = DayPhase::from_time_iso(&time_iso);
        let condition = WeatherCondition::from_weather_code(code);
        let precipitation_probability = (offset as u8) * 10;
        hours.push(HourForecast {
            time_iso,
            hour_label: label,
            temperature_c: 15.0 + offset as f32,
            wind_speed_kmh: 8.0 + (offset as f32 * 1.5),
            precipitation_probability,
            humidity_percent: (45_u8 + precipitation_probability / 2).min(95),
            pressure_hpa: 1018.0 - (offset as f32 * 1.3),
            visibility_km: (16.0 - (offset as f32 * 1.2)).max(3.0),
            uv_index: demo_uv_index(hour, code),
            weather_code: code,
            day_phase,
            condition,
        });
    }

    ForecastStrip {
        hours,
        sunrise: None,
        sunset: None,
    }
}

pub fn load_mock_forecast_from_config(cfg: &MockConfig) -> Result<ForecastStrip, Box<dyn Error>> {
    let preset_name = cfg.preset.clone().unwrap_or_else(|| "thunder".to_owned());
    let preset = lookup_mock_preset(&preset_name).ok_or_else(|| {
        let names = mock_preset_names().join(", ");
        format!("unknown mock preset='{preset_name}'. Supported values: {names}")
    })?;

    let hours = cfg.hours.unwrap_or(12);
    let is_night = cfg.night.unwrap_or(false);

    let start_hour = if is_night { 21 } else { 9 };
    let mut entries = Vec::with_capacity(hours);

    for offset in 0..hours {
        let hour = (start_hour + offset as i32) % 24;
        let label = format!("{:02}:00", hour);
        let temp = preset.base_temp_c + mock_temp_offset(offset, is_night);
        let precip = mock_precip_offset(preset.precipitation_probability, offset);
        let weather_code = if offset == 0 {
            preset.weather_code
        } else {
            next_weather_code_for_hour(preset.weather_code, offset)
        };
        let time_iso = format!("2026-05-30T{}", label);
        let day_phase = DayPhase::from_time_iso(&time_iso);
        let condition = WeatherCondition::from_weather_code(weather_code);

        entries.push(HourForecast {
            time_iso,
            hour_label: label,
            temperature_c: temp,
            wind_speed_kmh: mock_wind_offset(offset, is_night),
            precipitation_probability: precip,
            humidity_percent: mock_humidity(weather_code, precip),
            pressure_hpa: mock_pressure_hpa(weather_code, offset),
            visibility_km: mock_visibility_km(weather_code, precip),
            uv_index: mock_uv_index(weather_code, hour),
            weather_code,
            day_phase,
            condition,
        });
    }

    use chrono::{Duration as ChronoDuration, Local};
    let today = Local::now().date_naive();
    let sunrise_date = if is_night {
        today + ChronoDuration::days(1)
    } else {
        today
    };
    let sunrise = Some(format!("{}T05:30", sunrise_date.format("%Y-%m-%d")));
    let sunset = Some(format!("{}T21:00", today.format("%Y-%m-%d")));

    Ok(ForecastStrip {
        hours: entries,
        sunrise,
        sunset,
    })
}

fn lookup_mock_preset(name: &str) -> Option<MockPreset> {
    MOCK_PRESETS
        .iter()
        .copied()
        .find(|preset| preset.name.eq_ignore_ascii_case(name))
}

fn mock_temp_offset(offset: usize, is_night: bool) -> f32 {
    let pattern = if is_night {
        [0.0, -1.2, -2.0, -1.4, -0.8, -1.0, -0.5, -0.2]
    } else {
        [0.0, 0.8, 1.6, 1.0, 0.4, -0.3, -0.8, -1.1]
    };

    pattern[offset % pattern.len()]
}

fn mock_wind_offset(offset: usize, is_night: bool) -> f32 {
    let cycle = [7.0, 9.5, 11.0, 13.0, 10.0, 8.5, 12.0, 9.0];
    let base = cycle[offset % cycle.len()];
    if is_night {
        (base - 1.5_f32).max(1.0_f32)
    } else {
        base
    }
}

fn mock_precip_offset(base: u8, offset: usize) -> u8 {
    let deltas = [0i16, 8, -6, 10, -4, 6, -8, 4];
    let value = (base as i16) + deltas[offset % deltas.len()];
    value.clamp(0, 100) as u8
}

fn demo_uv_index(hour: i32, weather_code: u16) -> f32 {
    if !(6..19).contains(&hour) {
        return 0.0;
    }

    let cloud_factor = match weather_code {
        0 => 1.0,
        1 | 2 => 0.75,
        3 | 45 | 48 => 0.45,
        51 | 53 | 55 | 56 | 57 | 61 | 63 | 65 | 66 | 67 | 71 | 73 | 75 | 77 | 80 | 81 | 82 | 85
        | 86 => 0.35,
        95 | 96 | 99 => 0.25,
        _ => 0.5,
    };

    let hour_from_noon = ((hour - 12).abs()) as f32;
    let solar_strength = (1.0 - (hour_from_noon / 6.0)).max(0.0);
    (8.0 * solar_strength * cloud_factor).max(0.0)
}

fn mock_humidity(weather_code: u16, precipitation_probability: u8) -> u8 {
    let weather_boost = match weather_code {
        45 | 48 => 30,
        95 | 96 | 99 => 25,
        61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 | 51 | 53 | 55 | 56 | 57 | 71 | 73 | 75 | 77 | 85
        | 86 => 20,
        3 => 10,
        _ => 0,
    };

    let base = 40_i16 + (precipitation_probability as i16 / 2) + weather_boost;
    base.clamp(25, 99) as u8
}

fn mock_pressure_hpa(weather_code: u16, offset: usize) -> f32 {
    let baseline = match weather_code {
        95 | 96 | 99 => 998.0,
        61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 => 1005.0,
        45 | 48 => 1008.0,
        71 | 73 | 75 | 77 | 85 | 86 => 1002.0,
        _ => 1015.0,
    };
    let wobble = [0.0, -0.8, -1.2, -0.4, 0.3, 0.8, 0.2, -0.5];
    baseline + wobble[offset % wobble.len()]
}

fn mock_visibility_km(weather_code: u16, precipitation_probability: u8) -> f32 {
    let base = match weather_code {
        45 | 48 => 2.0,
        71 | 73 | 75 | 77 | 85 | 86 => 5.0,
        61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 | 51 | 53 | 55 | 56 | 57 => 7.0,
        95 | 96 | 99 => 6.0,
        3 => 12.0,
        _ => 18.0,
    };
    let precip_penalty = (precipitation_probability as f32 / 100.0) * 4.0;
    (base - precip_penalty).clamp(1.0, 20.0)
}

fn mock_uv_index(weather_code: u16, hour: i32) -> f32 {
    demo_uv_index(hour, weather_code)
}

fn next_weather_code_for_hour(base: u16, offset: usize) -> u16 {
    if offset == 0 {
        return base;
    }

    match base {
        95 | 96 | 99 => {
            if offset == 1 {
                81
            } else if offset == 2 {
                63
            } else {
                3
            }
        }
        71 | 73 | 75 | 77 | 85 | 86 => {
            if offset == 1 {
                85
            } else if offset == 2 {
                45
            } else {
                3
            }
        }
        45 | 48 => {
            if offset == 1 {
                48
            } else if offset == 2 {
                3
            } else {
                2
            }
        }
        61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 | 51 | 53 | 55 | 56 | 57 => {
            if offset == 1 {
                81
            } else if offset == 2 {
                63
            } else {
                2
            }
        }
        _ => {
            if offset == 1 {
                2
            } else if offset == 2 {
                3
            } else {
                61
            }
        }
    }
}

fn hour_label(time_iso: &str) -> String {
    if let Some((_, hhmm)) = time_iso.split_once('T') {
        if hhmm.len() >= 5 {
            return hhmm[0..5].to_owned();
        }
    }
    time_iso.chars().take(5).collect()
}

fn first_event_in_window(
    events: &[String],
    start: Option<&str>,
    end: Option<&str>,
) -> Option<String> {
    match (start, end) {
        (Some(start), Some(end)) => events
            .iter()
            .find(|event| event.as_str() >= start && event.as_str() <= end)
            .cloned(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── mock_preset_names ─────────────────────────────────────────────

    #[test]
    fn mock_preset_list_has_all_entries() {
        let names = mock_preset_names();
        assert_eq!(names.len(), 11);
        assert!(names.contains(&"clear"));
        assert!(names.contains(&"thunder"));
        assert!(names.contains(&"hail-thunder"));
    }

    // ── lookup_mock_preset ────────────────────────────────────────────

    #[test]
    fn lookup_known_preset() {
        let preset = lookup_mock_preset("thunder").unwrap();
        assert_eq!(preset.weather_code, 95);
        assert_eq!(preset.precipitation_probability, 85);
        assert!((preset.base_temp_c - 14.0).abs() < 0.01);
    }

    #[test]
    fn lookup_unknown_preset_returns_none() {
        assert!(lookup_mock_preset("nonexistent").is_none());
    }

    // ── ForecastSource ────────────────────────────────────────────────

    #[test]
    fn forecast_source_live_by_default() {
        assert_eq!("".parse::<ForecastSource>().unwrap(), ForecastSource::Live);
        assert_eq!(
            "unknown".parse::<ForecastSource>().unwrap(),
            ForecastSource::Live
        );
    }

    #[test]
    fn forecast_source_from_str() {
        assert_eq!(
            "live".parse::<ForecastSource>().unwrap(),
            ForecastSource::Live
        );
        assert_eq!(
            "demo".parse::<ForecastSource>().unwrap(),
            ForecastSource::Demo
        );
        assert_eq!(
            "mock".parse::<ForecastSource>().unwrap(),
            ForecastSource::Mock
        );
        assert_eq!(
            "LIVE".parse::<ForecastSource>().unwrap(),
            ForecastSource::Live
        );
        assert_eq!(
            "DEMO".parse::<ForecastSource>().unwrap(),
            ForecastSource::Demo
        );
        assert_eq!(
            "MOCK".parse::<ForecastSource>().unwrap(),
            ForecastSource::Mock
        );
    }

    // ── hour_label ────────────────────────────────────────────────────

    #[test]
    fn hour_label_extracts_hhmm() {
        assert_eq!(hour_label("2026-06-05T14:30"), "14:30");
    }

    #[test]
    fn hour_label_no_t_returns_first_5_chars() {
        assert_eq!(hour_label("hello world"), "hello");
    }

    // ── demo_forecast ─────────────────────────────────────────────────

    #[test]
    fn demo_forecast_has_hours() {
        let f = demo_forecast();
        assert!(!f.hours.is_empty());
        assert_eq!(f.hours.len(), 7); // 7 demo codes
        // First hour should be at 08:00
        assert!(f.hours[0].hour_label.starts_with("08"));
    }

    #[test]
    fn demo_forecast_sunrise_sunset_none() {
        let f = demo_forecast();
        assert!(f.sunrise.is_none());
        assert!(f.sunset.is_none());
    }

    // ── demo_uv_index ─────────────────────────────────────────────────

    #[test]
    fn demo_uv_night_is_zero() {
        assert_eq!(demo_uv_index(3, 0), 0.0); // 3 AM = night
    }

    #[test]
    fn demo_uv_peak_at_noon() {
        let uv = demo_uv_index(12, 0); // noon, clear
        assert!((uv - 8.0).abs() < 0.01);
    }

    #[test]
    fn demo_uv_cloudy_reduced() {
        let uv_clear = demo_uv_index(12, 0);
        let uv_cloudy = demo_uv_index(12, 3);
        assert!(uv_cloudy < uv_clear);
        assert!((uv_cloudy - 8.0 * 0.45).abs() < 0.01);
    }

    // ── mock_temp_offset ──────────────────────────────────────────────

    #[test]
    fn mock_temp_offset_daytime() {
        let t = mock_temp_offset(0, false);
        assert!((t - 0.0).abs() < 0.01);
    }

    #[test]
    fn mock_temp_offset_night_colder() {
        let day_temp = mock_temp_offset(1, false);
        let night_temp = mock_temp_offset(1, true);
        assert!(night_temp < day_temp);
    }

    // ── mock_wind_offset ──────────────────────────────────────────────

    #[test]
    fn mock_wind_positive() {
        let w = mock_wind_offset(0, false);
        assert!(w > 0.0);
    }

    #[test]
    fn mock_wind_night_lower() {
        assert!(mock_wind_offset(0, true) < mock_wind_offset(0, false));
    }

    // ── mock_precip_offset ────────────────────────────────────────────

    #[test]
    fn mock_precip_clamped() {
        let p = mock_precip_offset(95, 0);
        assert!(p <= 100);
    }

    // ── next_weather_code_for_hour ────────────────────────────────────

    #[test]
    fn next_code_offset_zero_is_base() {
        assert_eq!(next_weather_code_for_hour(95, 0), 95);
        assert_eq!(next_weather_code_for_hour(0, 0), 0);
    }

    #[test]
    fn next_code_thunder_transitions() {
        assert_eq!(next_weather_code_for_hour(95, 1), 81); // showers
        assert_eq!(next_weather_code_for_hour(95, 2), 63); // rain
        assert_eq!(next_weather_code_for_hour(95, 3), 3); // overcast
    }

    #[test]
    fn next_code_clear_transitions() {
        assert_eq!(next_weather_code_for_hour(0, 1), 2); // partly cloudy
        assert_eq!(next_weather_code_for_hour(0, 2), 3); // cloudy
        assert_eq!(next_weather_code_for_hour(0, 3), 61); // rain
    }

    // ── first_event_in_window ─────────────────────────────────────────

    #[test]
    fn first_event_in_window_none_when_no_start() {
        assert!(first_event_in_window(&["a".to_string()], None, None).is_none());
    }

    #[test]
    fn first_event_in_window_finds_matching() {
        let events = vec![
            "2026-06-05T05:30".to_string(),
            "2026-06-05T20:00".to_string(),
        ];
        let result =
            first_event_in_window(&events, Some("2026-06-05T00:00"), Some("2026-06-06T00:00"));
        assert_eq!(result.as_deref(), Some("2026-06-05T05:30"));
    }

    #[test]
    fn first_event_outside_window_returns_none() {
        let events = vec!["2026-06-06T05:30".to_string()];
        let result =
            first_event_in_window(&events, Some("2026-06-05T00:00"), Some("2026-06-05T23:59"));
        assert!(result.is_none());
    }

    // ── MOCK_PRESETS table integrity ──────────────────────────────────

    #[test]
    fn all_mock_presets_are_lookupable() {
        for preset in &MOCK_PRESETS {
            let found = lookup_mock_preset(preset.name);
            assert!(
                found.is_some(),
                "preset '{}' should be lookupable by name",
                preset.name
            );
        }
    }

    #[test]
    fn mock_preset_rain_has_correct_code() {
        let p = lookup_mock_preset("rain").unwrap();
        assert_eq!(p.weather_code, 63);
    }

    // ── demo_forecast data shape ──────────────────────────────────────

    #[test]
    fn demo_forecast_hours_have_required_fields() {
        let f = demo_forecast();
        for hour in &f.hours {
            assert!(!hour.time_iso.is_empty(), "time_iso must not be empty");
            assert!(!hour.hour_label.is_empty(), "hour_label must not be empty");
            assert!(
                hour.humidity_percent >= 25 && hour.humidity_percent <= 99,
                "humidity out of range: {}",
                hour.humidity_percent
            );
        }
    }

    // ── mock_forecast helper sanity ───────────────────────────────────

    #[test]
    fn mock_humidity_thunderstorm_is_high() {
        let h = mock_humidity(95, 80);
        assert!(h > 70);
    }

    #[test]
    fn mock_humidity_clear_is_low() {
        let h = mock_humidity(0, 0);
        assert!(h < 50);
    }

    #[test]
    fn mock_visibility_clear_is_high() {
        let v = mock_visibility_km(0, 0);
        // Clear skies: base is 18.0, precip penalty is 0
        assert!((v - 18.0).abs() < 0.01);
    }

    #[test]
    fn mock_visibility_fog_is_low() {
        let v = mock_visibility_km(45, 60);
        assert!(v < 10.0);
    }

    #[test]
    fn mock_pressure_thunder_is_low() {
        let p = mock_pressure_hpa(95, 0);
        assert!((p - 998.0).abs() < 1.0);
    }
}
