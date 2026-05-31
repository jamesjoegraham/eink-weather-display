mod weather;
mod svg_render;
mod ui;
mod template;
mod display;

#[cfg(target_os = "linux")]
use std::{env, error::Error, thread, time::Duration};

#[cfg(not(target_os = "linux"))]
use std::{env, error::Error, thread, time::Duration};

fn main() -> Result<(), Box<dyn Error>> {
    match dotenvy::dotenv() {
        Ok(path) => eprintln!("Loaded env from {}", path.display()),
        Err(_) => eprintln!("No .env found, using process environment/defaults."),
    }

    let refresh_secs = std::env::var("EINK_WEATHER_REFRESH_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let svg_arg = std::env::args().nth(1);

    if refresh_secs == 0 {
        run_once(svg_arg.as_deref())?;
        return Ok(());
    }

    if refresh_secs < 60 {
        eprintln!(
            "EINK_WEATHER_REFRESH_SECS={} is aggressive for e-paper. Consider >= 300.",
            refresh_secs
        );
    }

    eprintln!("Refresh loop enabled: every {refresh_secs}s");
    loop {
        if let Err(err) = run_once(svg_arg.as_deref()) {
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

fn run_once(svg_path_arg: Option<&str>) -> Result<(), Box<dyn Error>> {
    #[cfg(not(target_os = "linux"))]
    let _ = svg_path_arg;

    let theme = ui::UiTheme::from_env();
    let forecast_source = weather::forecast_source_from_env();

    let forecast = match weather::load_forecast_by_source(forecast_source) {
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

    let templated_svg = template::render_dashboard_svg(&forecast, theme)?;

    if let Ok(path) = env::var("EINK_WEATHER_PREVIEW_SVG_PATH") {
        ensure_parent_dir(&path)?;
        std::fs::write(&path, &templated_svg)?;
        eprintln!("Wrote rendered template SVG preview to {path}");
    }

    if let Ok(path) = env::var("EINK_WEATHER_PREVIEW_PNG_PATH") {
        ensure_parent_dir(&path)?;
        svg_render::rasterize_svg_to_png(
            &templated_svg,
            std::path::Path::new(&path),
            800,
            480,
        )?;
        eprintln!("Wrote rendered PNG preview to {path}");

        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&path).spawn();
        }
    }

    #[cfg(target_os = "linux")]
    {
        render_to_hardware(svg_path_arg, theme, &templated_svg)?;
    }

    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("ℹ️  Non-Linux platform detected.");
        eprintln!("💡 Hardware output disabled. To test rendering:");
            eprintln!("   Set EINK_WEATHER_PREVIEW_SVG_PATH=/path/to/output.svg");
            eprintln!("   Set EINK_WEATHER_PREVIEW_PNG_PATH=/path/to/output.png");
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

#[cfg(target_os = "linux")]
fn render_to_hardware(
    svg_path_arg: Option<&str>,
    theme: ui::UiTheme,
    templated_svg: &[u8],
) -> Result<(), Box<dyn Error>> {
    use embedded_graphics::prelude::*;
    use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
    use epd_waveshare::{
        color::OctColor,
        epd7in3f::{Display7in3f, Epd7in3f},
        prelude::WaveshareDisplay,
    };
    use linux_embedded_hal::{spidev::{self, SpidevOptions}, Delay, SpidevDevice};
    use std::path::Path;

    let mut spi = SpidevDevice::open("/dev/spidev0.0")
        .map_err(|e| std::io::Error::other(format!("open /dev/spidev0.0 failed: {e}")))?;
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(4_000_000)
        .mode(spidev::SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)
        .map_err(|e| std::io::Error::other(format!("configure /dev/spidev0.0 failed: {e}")))?;

    let busy = cdev_input_pin(24)?;
    let dc = cdev_output_pin(25, 1)?;
    let rst = cdev_output_pin(17, 1)?;

    let mut delay = Delay {};
    let mut epd = Epd7in3f::new(&mut spi, busy, dc, rst, &mut delay, None)
        .map_err(|e| std::io::Error::other(format!("initialize EPD controller failed: {e}")))?;

    let mut display = Box::new(Display7in3f::default());
    let layout = ui::overlay_layout();
    let clear_color = match theme {
        ui::UiTheme::Light => OctColor::White,
        ui::UiTheme::Dark => OctColor::Black,
    };
    display.clear(clear_color).expect("clear failed");

    let color_test_mode = env::var("EINK_WEATHER_COLOR_TEST")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            matches!(value.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false);

    if color_test_mode {
        let colors = [
            OctColor::Black,
            OctColor::White,
            OctColor::Green,
            OctColor::Blue,
            OctColor::Red,
            OctColor::Yellow,
            OctColor::Orange,
        ];

        for (idx, color) in colors.iter().enumerate() {
            let x0 = ((idx as i32) * 800) / (colors.len() as i32);
            let x1 = (((idx as i32) + 1) * 800) / (colors.len() as i32);
            Rectangle::new(Point::new(x0, 0), Size::new((x1 - x0) as u32, 480))
                .into_styled(PrimitiveStyle::with_fill(*color))
                .draw(display.as_mut())
                .map_err(|e| std::io::Error::other(format!("draw color test pattern failed: {e}")))?;
        }
    } else {
        svg_render::draw_svg_bytes(display.as_mut(), templated_svg, Point::new(0, 0), Size::new(800, 480))?;

        if let Some(svg_path) = svg_path_arg {
            svg_render::draw_svg_icon(
                display.as_mut(),
                Path::new(&svg_path),
                layout.origin,
                layout.size,
            )?;
        }
    }

    epd.update_and_display_frame(&mut spi, display.buffer(), &mut delay)
        .map_err(|e| std::io::Error::other(format!("send frame to EPD failed: {e}")))?;
    epd.sleep(&mut spi, &mut delay)
        .map_err(|e| std::io::Error::other(format!("put EPD to sleep failed: {e}")))?;

    if color_test_mode {
        println!("Frame sent to 7in3f display (hardware color test mode).");
    } else {
        println!("Frame sent to 7in3f display ({theme:?} theme, template mode).");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn cdev_input_pin(pin: u32) -> Result<linux_embedded_hal::CdevPin, Box<dyn Error>> {
    use linux_embedded_hal::gpio_cdev::{Chip, LineRequestFlags};
    use std::io;

    let mut chip = Chip::new("/dev/gpiochip0")
        .map_err(|e| io::Error::other(format!("open /dev/gpiochip0 failed: {e}")))?;
    let line = chip
        .get_line(pin)
        .map_err(|e| io::Error::other(format!("get gpio input line {pin} failed: {e}")))?;
    let handle = request_line_with_retry(pin, || {
        line.request(LineRequestFlags::INPUT, 0, "eink-writer")
    })?;
    linux_embedded_hal::CdevPin::new(handle)
        .map_err(|e| io::Error::other(format!("create input pin {pin} failed: {e}")))
        .map_err(|e| e.into())
}

#[cfg(target_os = "linux")]
fn cdev_output_pin(pin: u32, initial_value: u8) -> Result<linux_embedded_hal::CdevPin, Box<dyn Error>> {
    use linux_embedded_hal::gpio_cdev::{Chip, LineRequestFlags};
    use std::io;

    let mut chip = Chip::new("/dev/gpiochip0")
        .map_err(|e| io::Error::other(format!("open /dev/gpiochip0 failed: {e}")))?;
    let line = chip
        .get_line(pin)
        .map_err(|e| io::Error::other(format!("get gpio output line {pin} failed: {e}")))?;
    let handle = request_line_with_retry(pin, || {
        line.request(LineRequestFlags::OUTPUT, initial_value, "eink-writer")
    })?;
    linux_embedded_hal::CdevPin::new(handle)
        .map_err(|e| io::Error::other(format!("create output pin {pin} failed: {e}")))
        .map_err(|e| e.into())
}

#[cfg(target_os = "linux")]
fn request_line_with_retry<T, F>(pin: u32, mut request_fn: F) -> Result<T, Box<dyn Error>>
where
    F: FnMut() -> Result<T, linux_embedded_hal::gpio_cdev::Error>,
{
    use std::io;

    const RETRIES: usize = 5;
    const RETRY_DELAY_MS: u64 = 150;

    for attempt in 1..=RETRIES {
        match request_fn() {
            Ok(handle) => return Ok(handle),
            Err(err) => {
                let msg = err.to_string();
                let busy = msg.contains("EBUSY");
                if busy && attempt < RETRIES {
                    eprintln!(
                        "GPIO pin {pin} busy (attempt {attempt}/{RETRIES}), retrying..."
                    );
                    thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
                    continue;
                }
                return Err(io::Error::other(format!(
                    "request gpio line {pin} failed after {attempt} attempt(s): {msg}"
                ))
                .into());
            }
        }
    }

    Err(io::Error::other(format!(
        "request gpio line {pin} failed: retries exhausted"
    ))
    .into())
}


