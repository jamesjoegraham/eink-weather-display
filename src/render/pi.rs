
use std::time::Duration;
use std::thread;
use std::error::Error;
use std::env;
use super::rasterize::{draw_svg_bytes, draw_svg_icon};
use super::UiTheme;

use embedded_graphics::prelude::*;


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OverlayLayout {
    pub origin: Point,
    pub size: Size,
}

pub fn overlay_layout() -> OverlayLayout {
    OverlayLayout {
        origin: Point::new(620, 40),
        size: Size::new(140, 140),
    }
}

pub fn render_to_hardware(
    svg_path_arg: Option<&str>,
    theme: UiTheme,
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
    let layout = overlay_layout();
    let clear_color = match theme {
        UiTheme::Light => OctColor::White,
        UiTheme::Dark => OctColor::Black,
    };
    display.clear(clear_color).expect("clear failed");

    let color_test_mode = env::var("EINK_WEATHER_COLOR_TEST")
        .ok()
        .map(|value: String| {
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
        draw_svg_bytes(display.as_mut(), templated_svg, Point::new(0, 0), Size::new(800, 480))?;

        if let Some(svg_path) = svg_path_arg {
            draw_svg_icon(
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


