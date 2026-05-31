mod config;
mod api;
mod panel;
mod model;
mod render;

#[cfg(target_os = "linux")]
use std::{env, error::Error, thread, time::Duration};

#[cfg(not(target_os = "linux"))]
use std::{env, error::Error, thread, time::Duration};

use api::weather;

use crate::{panel::WeatherPanelViewModel, render::rasterize_svg_to_png};

fn main() -> Result<(), Box<dyn Error>> {
    // Load config from file specified by EINK_WEATHER_CONFIG_PATH
    let config_path = env::var("EINK_WEATHER_CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());
    let config = config::Config::from_path(&config_path).expect("Failed to load config");

    let refresh_secs = config.refresh_secs.unwrap_or(600);
    let svg_arg = std::env::args().nth(1);

    if refresh_secs == 0 {
        run_once(svg_arg.as_deref(), &config)?;
        return Ok(());
    }

    if refresh_secs < 60 {
        eprintln!(
            "refresh_secs={} is aggressive for e-paper. Consider >= 300.",
            refresh_secs
        );
    }

    eprintln!("Refresh loop enabled: every {refresh_secs}s");
    loop {
        if let Err(err) = run_once(svg_arg.as_deref(), &config) {
            eprintln!("Render loop error: {}", format_error_chain(err.as_ref()));
        }
        thread::sleep(Duration::from_secs(refresh_secs));
    }
}

fn format_error_chain(err: &dyn Error) -> String {
    let mut message = err.to_string();
    let mut current = err.source();
    while let Some(source) = current {
        message.push_str(": ");
        message.push_str(&source.to_string());
        current = source.source();
    }
    message
}

// currently just render the weather panel
fn run_once(svg_path_arg: Option<&str>, config: &config::Config) -> Result<(), Box<dyn Error>> {
    #[cfg(not(target_os = "linux"))]
    let _ = svg_path_arg;

    let theme = config.theme.as_deref().and_then(|s| s.parse().ok()).unwrap_or_default();
    let forecast_source = config.forecast_source.as_deref().and_then(|s| s.parse().ok()).unwrap_or_default();

    let forecast = match weather::load_forecast_by_source(forecast_source, config) {
        Ok(data) => data,
        Err(err) => match forecast_source {
            weather::ForecastSource::Live => {
                eprintln!("Open-Meteo fetch failed, using demo data: {err}");
                weather::demo_forecast()
            }
            weather::ForecastSource::Demo | weather::ForecastSource::Mock => return Err(err),
        },
    };

    eprintln!("Forecast source: {forecast_source:?}");

    let weather_panel = WeatherPanelViewModel::from_forecast(&forecast);
    let templated_svg = weather_panel.render_svg(theme)?;


    if let Some(path) = config.preview_svg_path.as_ref() {
        ensure_parent_dir(path)?;
        std::fs::write(path, &templated_svg)?;
        eprintln!("Wrote rendered template SVG preview to {path}");
    }

    if let Some(path) = config.preview_png_path.as_ref() {
        ensure_parent_dir(path)?;
        rasterize_svg_to_png(
            &templated_svg,
            std::path::Path::new(path),
            800,
            480,
        )?;
        eprintln!("Wrote rendered PNG preview to {path}");

        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(path).spawn();
        }
    }

    #[cfg(target_os = "linux")]
    {
        render::render_to_hardware(svg_path_arg, theme, &templated_svg)?;
    }

    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("ℹ️  Non-Linux platform detected.");
        eprintln!("💡 Hardware output disabled. To test rendering:");
        eprintln!("   Set preview_svg_path in config");
        eprintln!("   Set preview_png_path in config");
        eprintln!("✓ SVG template rendering succeeded.");
    }

    Ok(())
}

fn ensure_parent_dir(path: &str) -> Result<(), Box<dyn Error>> {
    let path = std::path::Path::new(path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}
