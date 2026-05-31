#[derive(Debug, Clone)]
pub struct MetricCardViewModel {
    pub value: String,
    pub icon: WeatherIcon,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub icon_x: i32,
    pub icon_y: i32,
    pub value_x: i32,
    pub value_y: i32,
}

#[derive(Debug, Clone)]
pub struct HourlyForecastViewModel {
    pub time: String,
    pub temp: String,
    pub wind: String,
    pub rain: String,
    pub humidity: String,
    pub uv: String,
    pub icon: WeatherCondition,
}

#[derive(Debug, Clone)]
pub struct ConnectorViewModel {
    pub index: usize,
    pub kind: String, // "sunrise" or "dusk"
}

use minijinja::context;

use crate::api::weather::ForecastStrip;
use crate::model::weather::{DayPhase, WeatherCondition, WeatherIcon};
use crate::render::{icon_markup, render_template_from_ctx};
use crate::render::UiTheme;

#[derive(Debug, Clone)]
pub struct WeatherPanelViewModel {
    pub date: String,
    pub now_time: String,
    pub current_day_phase: DayPhase,
    pub current_temp: String,
    pub temp_unit: String,
    pub temp_color: String,
    pub current_wind: String,
    pub current_rain: String,
    pub current_cond: String,
    pub current_code: u16,
    pub current_icon: WeatherCondition,
    pub metrics: Vec<MetricCardViewModel>,
    pub hours: Vec<HourlyForecastViewModel>,
    pub connectors: Vec<ConnectorViewModel>,
    pub daynight_bar_color: String,
    pub daynight_bar_width: i32,
}

impl WeatherPanelViewModel {
    pub fn from_forecast(forecast: &ForecastStrip)-> Self {
        use chrono::{Local, NaiveTime};
        let now = Local::now();
        let date = now.format("%a, %b %-d").to_string();
        let current = forecast.hours.first();
        let current_temp_c = current.map(|h| h.temperature_c).unwrap_or(0.0);
        let temp_color = if current_temp_c < 0.0 {
            "#0074D9"
        } else if current_temp_c < 5.0 {
            "#008000"
        } else if current_temp_c < 15.0 {
            "#111111"
        } else if current_temp_c < 20.0 {
            "#FFFF00"
        } else if current_temp_c < 26.0 {
            "#FFA500"
        } else {
            "#FF0000"
        };
        let temp_unit = "°C".to_string();
        let current_temp = format!("{:.0}", current_temp_c);
        let current_wind = format!("{:.0} km/h", current.map(|h| h.wind_speed_kmh).unwrap_or(0.0));
        let current_rain = format!("{}%", current.map(|h| h.precipitation_probability).unwrap_or(0));
        let current_cond = current.map(|h| h.condition.summary().to_string()).unwrap_or("Unknown".to_string());
        let current_code = current.map(|h| h.weather_code).unwrap_or(0);
        let current_icon = current
            .map(|h| h.condition)
            .unwrap_or(WeatherCondition::Clear);

        // Metrics
        let current_humidity = current.map(|h| h.humidity_percent).unwrap_or(0);
        let current_wind_val = current.map(|h| h.wind_speed_kmh).unwrap_or(0.0);
        let current_uv = current.map(|h| h.uv_index).unwrap_or(0.0);
        let current_pressure = current.map(|h| h.pressure_hpa).unwrap_or(0.0);
        let current_rain_val = current.map(|h| h.precipitation_probability).unwrap_or(0);
        let current_visibility = current.map(|h| h.visibility_km).unwrap_or(0.0);
        let panel_x = 445;
        let panel_y = 100;
        let card_w = 165;
        let card_h = 50;
        let col_gap = 8;
        let row_gap = 4;
        let columns = 2;
        let metrics_values = [
            (format!("{}%", current_humidity), WeatherIcon::Humidity),
            (format!("{:.0} km h⁻¹", current_wind_val), WeatherIcon::Wind),
            (format!("{:.0}", current_uv), WeatherIcon::UVIndex),
            (format!("{:.0} hPa", current_pressure), WeatherIcon::Pressure),
            (format!("{}%", current_rain_val), WeatherIcon::Rain),
            (format!("{:.0} km", current_visibility), WeatherIcon::Visibility),
        ];
        let metrics = metrics_values
            .into_iter()
            .enumerate()
            .map(|(idx, (value, icon_name))| {
                let col = idx % columns;
                let row = idx / columns;
                let x = panel_x + (col as i32 * (card_w + col_gap));
                let y = panel_y + (row as i32 * (card_h + row_gap));
                MetricCardViewModel {
                    value: value.clone(),
                    icon: icon_name,
                    x,
                    y,
                    w: card_w,
                    h: card_h,
                    icon_x: x + 12,
                    icon_y: y + 12,
                    value_x: x + card_w - 12,
                    value_y: y + 30,
                }
            })
            .collect();

        // helper for parsing sunrise/sunset times
        let parse_time = |s: &str| {
                if let Some(t) = s.split('T').nth(1) {
                    NaiveTime::parse_from_str(t, "%H:%M").or_else(|_| NaiveTime::parse_from_str(t, "%H:%M:%S")).ok()
                } else { None }
            };
        let sunrise = forecast.sunrise.as_deref().and_then(parse_time);
        let sunset = forecast.sunset.as_deref().and_then(parse_time);

        // Day/Night Bar
        let now_time = now.time();
        let (daynight_bar_color, daynight_bar_width);
        if let (Some(sunrise), Some(sunset)) = (sunrise, sunset) {
            if now_time >= sunrise && now_time < sunset {
                let total = (sunset - sunrise).num_seconds().max(1) as f32;
                let elapsed = (now_time - sunrise).num_seconds().max(0) as f32;
                let percent = (elapsed / total).clamp(0.0, 1.0);

                daynight_bar_color = "#ff8000".to_string();
                daynight_bar_width = (800.0f32 * percent).round() as i32;
            } else {
                let (night_start, night_end) = if now_time < sunrise {
                    (sunset, sunrise)
                } else {
                    (sunset, sunrise + chrono::Duration::days(1))
                };
                let total = (night_end - night_start).num_seconds().max(1) as f32;
                let elapsed = (now_time - night_start).num_seconds().rem_euclid(total as i64) as f32;
                let percent = (elapsed / total).clamp(0.0, 1.0);

                daynight_bar_color = "#0000ff".to_string();
                daynight_bar_width = (800.0f32 * percent).round() as i32;
            }
        } else {
            daynight_bar_color = "".to_string();
            daynight_bar_width = 0;
        }

        // Hourly Forecast
        let selected_hours: Vec<_> = if forecast.hours.len() >= 13 {
            forecast.hours.iter().skip(1).step_by(2).take(6).collect()
        } else {
            forecast.hours.iter().skip(1).take(6).collect()
        };
        let hours = selected_hours
            .iter()
            .map(|h| {
                let hour_label = h.hour_label.split(':').next().and_then(|s| s.parse::<u8>().ok()).map(|hour| {
                    let am_pm = if hour < 12 { "am" } else { "pm" };
                    let hour_12 = if hour == 0 { 12 } else if hour > 12 { hour - 12 } else { hour };
                    format!("{}{}", hour_12, am_pm)
                }).unwrap_or_else(|| h.hour_label.clone());

                
                HourlyForecastViewModel {
                    time: hour_label,
                    temp: format!("{:.0}°C", h.temperature_c),
                    wind: format!("{}", h.wind_speed_kmh as i32),
                    rain: format!("{}", h.precipitation_probability),
                    humidity: format!("{}", h.humidity_percent),
                    uv: format!("{:.0}", h.uv_index),
                    icon: h.condition,
                }
            })
            .collect();

        // Connectors
        let mut connectors = Vec::new();
        let first_time = selected_hours.first().map(|h| &h.time_iso);
        let last_time = selected_hours.last().map(|h| &h.time_iso);

        let sunrise_index = match (first_time, last_time, forecast.sunrise.as_deref()) {
            (Some(first), Some(last), Some(sunrise)) => {
                if sunrise >= first.as_str() && sunrise <= last.as_str() {
                    selected_hours.iter().position(|h| h.time_iso.as_str() >= sunrise)
                } else {
                    None
                }
            },
            _ => None
        };

        let sunset_index = match (first_time, last_time, forecast.sunset.as_deref()) {
            (Some(first), Some(last), Some(sunset)) => {
                if sunset >= first.as_str() && sunset <= last.as_str() {
                    selected_hours.iter().position(|h| h.time_iso.as_str() >= sunset)
                } else {
                    None
                }
            },
            _ => None
        };
        if let Some(i) = sunrise_index {
            connectors.push(ConnectorViewModel { index: i.saturating_sub(1), kind: "sunrise".to_string() });
        }
        if let Some(i) = sunset_index {
            connectors.push(ConnectorViewModel { index: i.saturating_sub(1), kind: "dusk".to_string() });
        }

        let current_day_phase = forecast.hours.first().map(|h| h.day_phase).unwrap_or(DayPhase::Day);

        WeatherPanelViewModel {
            date,
            now_time: now_time.format("%-I:%M %p").to_string(),
            current_day_phase,
            current_temp,
            temp_unit: temp_unit.to_string(),
            temp_color: temp_color.to_string(),
            current_wind,
            current_rain,
            current_cond,
            current_code,
            current_icon,
            metrics,
            hours,
            connectors,
            daynight_bar_color,
            daynight_bar_width,
        }
    }

    pub fn render_svg(&self, theme: UiTheme) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        return Ok(
            render_template_from_ctx(theme, context! {
                date => self.date,
                now_time => self.now_time,
                current_temp => self.current_temp,
                temp_unit => self.temp_unit,
                temp_color => self.temp_color,
                current_wind => self.current_wind,
                current_rain => self.current_rain,
                current_cond => self.current_cond,
                current_code => self.current_code,
                current_icon => icon_markup(self.current_icon.icon(&self.current_day_phase), 142, 114, 108, 108),
                metrics => self.metrics.iter().map(|m| context! {
                    value => m.value.clone(),
                    icon => icon_markup(m.icon.filename(), 0, 0, 22, 22),
                    x => m.x,
                    y => m.y,
                    w => m.w,
                    h => m.h,
                    icon_x => m.icon_x,
                    icon_y => m.icon_y,
                    value_x => m.value_x,
                    value_y => m.value_y,
                }).collect::<Vec<_>>(),
                hours => self.hours.iter().map(|h| {
                    let day_phase = h.time.split(':').next().and_then(|s| s.parse::<u8>().ok()).map(|hour| {
                        if hour >= 6 && hour < 18 {
                            DayPhase::Day
                        } else {
                            DayPhase::Night
                        }
                    }).unwrap_or(DayPhase::Day);
                    context! {
                        rain_badge_icon => icon_markup(WeatherIcon::Rain.filename(), 0, 0, 14, 14),
                        wind_badge_icon => icon_markup(WeatherIcon::Wind.filename(), 0, 0, 14, 14),
                        uv_badge_icon => icon_markup(WeatherIcon::UVIndex.filename(), 0, 0, 14, 14),
                        humidity_badge_icon => icon_markup(WeatherIcon::Humidity.filename(), 0, 0, 14, 14),
                        time => h.time.clone(),
                        icon => icon_markup(h.icon.icon(&day_phase), 0, 0, 64, 64),
                        temp => h.temp.clone(),
                        wind => h.wind.clone(),
                        rain => h.rain.clone(),
                        humidity => h.humidity.clone(),
                        uv => h.uv.clone(),
                    }
                }
                ).collect::<Vec<_>>(),
                connectors => self.connectors.iter().map(|c| context! {
                    index => c.index,
                    kind => c.kind.clone(),
                }).collect::<Vec<_>>(),
                daynight_bar_color => self.daynight_bar_color,
                daynight_bar_width => self.daynight_bar_width,
            })?)

}

}
