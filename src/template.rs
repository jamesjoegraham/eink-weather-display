use crate::display::Icon;
use crate::ui::UiTheme;
use crate::weather::ForecastStrip;
use chrono::Local;
use include_dir::{include_dir, Dir};
use minijinja::{context, Environment};
use std::error::Error;

static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");
static ICONS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/icons");

fn load_icon_svg(icon_name: &str) -> Option<&'static str> {
    ICONS_DIR
        .get_file(icon_name)
        .and_then(|f| f.contents_utf8())
        .or_else(|| ICONS_DIR.get_file("unknown.svg").and_then(|f| f.contents_utf8()))
}

fn extract_attr(opening_tag: &str, attr_name: &str) -> Option<String> {
    let needle = format!("{attr_name}=\"");
    let start = opening_tag.find(&needle)? + needle.len();
    let rest = &opening_tag[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn strip_attr_value(mut input: String, attr_name: &str) -> String {
    let needle = format!(" {attr_name}=\"");
    loop {
        let Some(start) = input.find(&needle) else {
            break;
        };
        let value_start = start + needle.len();
        let Some(value_end_rel) = input[value_start..].find('"') else {
            break;
        };
        let value_end = value_start + value_end_rel;
        input.replace_range(start..=value_end, "");
    }
    input
}

fn icon_markup(icon_name: &str, x: i32, y: i32, width: i32, height: i32) -> String {
    let Some(raw_svg) = load_icon_svg(icon_name) else {
        return String::new();
    };

    let Some(svg_start) = raw_svg.find("<svg") else {
        return String::new();
    };
    let Some(open_end_rel) = raw_svg[svg_start..].find('>') else {
        return String::new();
    };

    let open_end = svg_start + open_end_rel;
    let opening_tag = &raw_svg[svg_start..=open_end];
    let closing_idx = raw_svg.rfind("</svg>").unwrap_or(raw_svg.len());
    let inner = raw_svg[open_end + 1..closing_idx].trim();
    let inner = strip_attr_value(inner.to_owned(), "sketch:type");
    let view_box = extract_attr(opening_tag, "viewBox").unwrap_or_else(|| "0 0 128 128".to_owned());

    format!(
        "<svg x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" viewBox=\"{view_box}\" preserveAspectRatio=\"xMidYMid meet\">{inner}</svg>"
    )
}

pub fn render_dashboard_svg(
    forecast: &ForecastStrip,
    theme: UiTheme,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let template_name = match theme {
        UiTheme::Light => "dashboard-full-template.svg.j2",
        UiTheme::Dark => "dashboard-full-template-dark.svg.j2",
    };

    let source = TEMPLATE_DIR
        .get_file(template_name)
        .ok_or_else(|| format!("missing template file: {template_name}"))?
        .contents_utf8()
        .ok_or_else(|| format!("template is not valid UTF-8: {template_name}"))?;

    let mut env = Environment::new();
    
    // Register custom truncate filter
    env.add_filter("truncate", |s: String, length: usize, _: bool, end_str: String| {
        if s.len() > length {
            format!("{}{}", &s[..length], end_str)
        } else {
            s
        }
    });
    
    env.add_template(template_name, source)?;
    let template = env.get_template(template_name)?;

    let now = Local::now();
    let current = forecast.hours.first();
    let header_date = now.format("%a, %b %-d").to_string();

    let current_icon = current
        .map(|h| icon_markup(h.condition.icon(&h.day_phase), 142, 114, 108, 108))
        .unwrap_or_default();

    // Determine temperature color
    let current_temp_c = current.map(|h| h.temperature_c).unwrap_or(0.0);

    let temp_color = if current_temp_c < 0.0 {
        "#0074D9" // blue
    } else if current_temp_c < 5.0 {
        "#008000" // green
    } else if current_temp_c < 15.0 {
        "#111111" // black
    } else if current_temp_c < 20.0 {
        "#FFFF00" // yellow
    } else if current_temp_c < 26.0 {
        "#FFA500" // orange
    } else {
        "#FF0000" // red
    };

    let selected_hours: Vec<_> = if forecast.hours.len() >= 13 {
        forecast.hours.iter().skip(1).step_by(2).take(6).collect()
    } else {
        forecast.hours.iter().skip(1).take(6).collect()
    };

    let hours: Vec<_> = selected_hours
        .iter()
        .map(|h| {
            // add am/pm to hour labels for better readability
            let hour_label = h.hour_label.split(':').next().and_then(|s| s.parse::<u8>().ok()).map(|hour| {
                let am_pm = if hour < 12 { "am" } else { "pm" };
                let hour_12 = if hour == 0 {
                    12
                } else if hour > 12 {
                    hour - 12
                } else {
                    hour
                };
                format!("{}{}", hour_12, am_pm)
            }).unwrap_or_else(|| h.hour_label.clone());
            context! {
                time => hour_label,
                cond => h.condition.summary(),
                temp => format!("{:.0}°C", h.temperature_c),
                wind => format!("{:.0}", h.wind_speed_kmh),
                rain => format!("{}%", h.precipitation_probability),
                humidity => format!("{}%", h.humidity_percent),
                uv => format!("{:.0}", h.uv_index),
                icon => icon_markup(h.condition.icon(&h.day_phase), 0, 0, 64, 64),
                wind_badge_icon => icon_markup(Icon::Wind.filename(), 0, 0, 14, 14),
                rain_badge_icon => icon_markup(Icon::Rain.filename(), 0, 0, 14, 14),
                uv_badge_icon => icon_markup(Icon::UVIndex.filename(), 0, 0, 14, 14),
                humidity_badge_icon => icon_markup(Icon::Humidity.filename(), 0, 0, 14, 14),
            }
        })
        .collect();

    // Compute connectors between adjacent hours for sunrise/dusk transitions
    let mut connectors = Vec::new();
    for i in 0..(selected_hours.len() - 1) {
        let h1 = selected_hours[i];
        let h2 = selected_hours[i + 1];
        if h1.day_phase != h2.day_phase {
            // Determine if this is sunrise or dusk
            let (from, to) = (h1.day_phase, h2.day_phase);
            let kind = if from.is_night() && to.is_day() {
                "sunrise"
            } else if from.is_day() && to.is_night() {
                "dusk"
            } else {
                continue;
            };
            connectors.push(context! {
                index => i,
                kind => kind,
            });
        }
    }

    let current_humidity = current.map(|h| h.humidity_percent).unwrap_or(0);
    let current_wind = current.map(|h| h.wind_speed_kmh).unwrap_or(0.0);
    let current_uv = current.map(|h| h.uv_index).unwrap_or(0.0);
    let current_pressure = current.map(|h| h.pressure_hpa).unwrap_or(0.0);
    let current_rain = current.map(|h| h.precipitation_probability).unwrap_or(0);
    let current_visibility = current.map(|h| h.visibility_km).unwrap_or(0.0);

    let metrics_values = [
        (format!("{}%", current_humidity), Icon::Humidity.filename()),
        (format!("{:.0} km h⁻¹", current_wind), Icon::Wind.filename()),
        (format!("{:.0}", current_uv), Icon::UVIndex.filename()),
        (format!("{:.0} hPa", current_pressure), Icon::Pressure.filename()),
        (format!("{}%", current_rain), Icon::Rain.filename()),
        (format!("{:.0} km", current_visibility), Icon::Visibility.filename()),
    ];

    // Metric panel layout knobs. Change these values to quickly tune panel size and spacing.
    let panel_x = 445;
    let panel_y = 100;
    let card_w = 165;
    let card_h = 50;
    let col_gap = 8;
    let row_gap = 4;
    let columns = 2;

    let metrics: Vec<_> = metrics_values
        .iter()
        .enumerate()
        .map(|(idx, (value, icon_name))| {
            let col = idx % columns;
            let row = idx / columns;
            let x = panel_x + (col as i32 * (card_w + col_gap));
            let y = panel_y + (row as i32 * (card_h + row_gap));

            context! {
                value => value.clone(),
                icon => icon_markup(icon_name, 0, 0, 22, 22),
                x => x,
                y => y,
                w => card_w,
                h => card_h,
                icon_x => x + 12,
                icon_y => y + 12,
                value_x => x + card_w - 12,
                value_y => y + 30,
            }
        })
        .collect();

    let rendered = template.render(context! {
        date => header_date,
        now_time => now.format("%-I:%M %p").to_string(),
        current_temp => format!("{:.0}", current_temp_c),
        temp_unit => "°C",
        temp_color => temp_color,
        current_wind => format!("{:.0} km/h", current.map(|h| h.wind_speed_kmh).unwrap_or(0.0)),
        current_rain => format!("{}%", current.map(|h| h.precipitation_probability).unwrap_or(0)),
        current_cond => current.map(|h| h.condition.summary()).unwrap_or("Unknown"),
        current_code => current.map(|h| h.weather_code).unwrap_or(0),
        current_icon => current_icon,
        metrics_title_x => panel_x,
        metrics_title_y => panel_y - 5,
        metrics => metrics,
        hours => hours,
        connectors => connectors,
    })?;

    Ok(rendered.into_bytes())
}
