use reqwest::blocking::Client;
use serde::Deserialize;
use std::{env, error::Error, time::Duration};
use crate::display::{DayPhase, WeatherCondition};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForecastSource {
	Live,
	Demo,
	Mock,
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
	pub latitude: f32,
	pub longitude: f32,
	pub timezone: String,
	pub hours: Vec<HourForecast>,
	pub sunrise: Option<String>,
	pub sunset: Option<String>,
}

pub fn forecast_source_from_env() -> ForecastSource {
	match env::var("EINK_WEATHER_FORECAST_SOURCE") {
		Ok(value) if value.eq_ignore_ascii_case("demo") => ForecastSource::Demo,
		Ok(value) if value.eq_ignore_ascii_case("mock") => ForecastSource::Mock,
		_ => ForecastSource::Live,
	}
}

pub fn load_forecast_by_source(source: ForecastSource) -> Result<ForecastStrip, Box<dyn Error>> {
	match source {
		ForecastSource::Live => load_forecast_from_env(),
		ForecastSource::Demo => Ok(demo_forecast()),
		ForecastSource::Mock => load_mock_forecast_from_env(),
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
	latitude: f32,
	longitude: f32,
	timezone: String,
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

pub fn load_forecast_from_env() -> Result<ForecastStrip, Box<dyn Error>> {
	let latitude = env::var("EINK_WEATHER_LAT")
		.ok()
		.and_then(|value| value.parse::<f32>().ok())
		.unwrap_or(44.655215);

	let longitude = env::var("EINK_WEATHER_LON")
		.ok()
		.and_then(|value| value.parse::<f32>().ok())
		.unwrap_or(-63.590768);

	let timezone = env::var("EINK_WEATHER_TZ").unwrap_or_else(|_| "auto".to_owned());

	let hours = forecast_entry_count_from_env();

	fetch_open_meteo_forecast(latitude, longitude, &timezone, hours)
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

	// Use the first sunrise/sunset for today (if available)
	let (sunrise, sunset) = if let Some(daily) = &payload.daily {
		(
			daily.sunrise.get(0).cloned(),
			daily.sunset.get(0).cloned(),
		)
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

	// Use the first sunrise/sunset for today (if available)
	let (sunrise, sunset) = if let Some(daily) = &payload.daily {
		(
			daily.sunrise.get(0).cloned(),
			daily.sunset.get(0).cloned(),
		)
	} else {
		(None, None)
	};

	Ok(ForecastStrip {
		latitude: payload.latitude,
		longitude: payload.longitude,
		timezone: payload.timezone,
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
		latitude: 0.0,
		longitude: 0.0,
		timezone: "demo".to_owned(),
		hours,
		sunrise: None,
		sunset: None,
	}
}

pub fn load_mock_forecast_from_env() -> Result<ForecastStrip, Box<dyn Error>> {
	let preset_name = env::var("EINK_WEATHER_MOCK_PRESET").unwrap_or_else(|_| "thunder".to_owned());
	let mut preset = lookup_mock_preset(&preset_name).ok_or_else(|| {
		let names = mock_preset_names().join(", ");
		format!(
			"unknown EINK_WEATHER_MOCK_PRESET='{preset_name}'. Supported values: {names}"
		)
	})?;

	if let Ok(code) = env::var("EINK_WEATHER_MOCK_WEATHER_CODE") {
		if let Ok(parsed_code) = code.parse::<u16>() {
			preset.weather_code = parsed_code;
		}
	}

	if let Ok(precip) = env::var("EINK_WEATHER_MOCK_PRECIP") {
		if let Ok(parsed_precip) = precip.parse::<u8>() {
			preset.precipitation_probability = parsed_precip.min(100);
		}
	}

	if let Ok(temp) = env::var("EINK_WEATHER_MOCK_TEMP_C") {
		if let Ok(parsed_temp) = temp.parse::<f32>() {
			preset.base_temp_c = parsed_temp;
		}
	}

	let hours = forecast_entry_count_from_env();

	let is_night = env::var("EINK_WEATHER_MOCK_NIGHT")
		.map(|value| value.eq_ignore_ascii_case("true") || value == "1")
		.unwrap_or(false);

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
		let time_iso = format!("2026-05-30T{label}");
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

	Ok(ForecastStrip {
		latitude: 44.6488,
		longitude: -63.5752,
		timezone: if is_night {
			"mock/night".to_owned()
		} else {
			"mock/day".to_owned()
		},
		hours: entries,
		sunrise: None,
		sunset: None,
	})
}

fn forecast_entry_count_from_env() -> usize {
	env::var("EINK_WEATHER_HOURS")
		.ok()
		.and_then(|value| value.parse::<usize>().ok())
		.unwrap_or(12)
		.clamp(1, 12)
		.saturating_add(1)
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
		51 | 53 | 55 | 56 | 57 | 61 | 63 | 65 | 66 | 67 | 71 | 73 | 75 | 77 | 80 | 81 | 82
		| 85 | 86 => 0.35,
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
		61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 | 51 | 53 | 55 | 56 | 57 | 71 | 73 | 75 | 77
		| 85 | 86 => 20,
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

