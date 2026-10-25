//! Deterministic candle and volume chart rendering.
//!
//! The shell renders a chart from the candles the data engine retained; it does
//! not compute an indicator. The image is drawn into a pixel buffer with an
//! embedded bitmap font and encoded as PNG. No system font, clock, locale, or
//! unordered collection participates, so the same candles produce the same PNG
//! bytes on every run.
//!
//! The fetch chart shows the last candles of the requested window, not the full
//! series when the series is longer than the `--candles` selection. Price and
//! volume panels share the same x-axis. The market chart adds four aligned axis
//! panels over the Layer 2 bars; see [`market`].

mod font;
mod market;

pub(crate) use market::render_market_png;

use std::fmt;

use quantrule_data::{Candle, Timeframe, format_utc_timestamp};

use crate::source::DateRange;

/// The rendered image width in pixels.
const WIDTH: u32 = 1_280;
/// The rendered image height in pixels.
const HEIGHT: u32 = 720;
/// The left edge of the plot area.
const LEFT: i32 = 88;
/// The right edge of the plot area.
const RIGHT: i32 = 1_260;
/// The top of the price panel.
const PRICE_TOP: i32 = 72;
/// The bottom of the price panel.
const PRICE_BOTTOM: i32 = 492;
/// The top of the volume panel.
const VOLUME_TOP: i32 = 522;
/// The bottom of the volume panel.
const VOLUME_BOTTOM: i32 = 640;
/// The baseline of the x-axis labels.
const LABEL_Y: i32 = 652;
/// The plot width.
const PLOT_WIDTH: i32 = RIGHT - LEFT;
/// The price panel height.
const PRICE_HEIGHT: i32 = PRICE_BOTTOM - PRICE_TOP;
/// The volume panel height.
const VOLUME_HEIGHT: i32 = VOLUME_BOTTOM - VOLUME_TOP;

/// The image background.
const BACKGROUND: Rgb = Rgb::new(15, 20, 25);
/// The plot panel background.
const PANEL: Rgb = Rgb::new(20, 26, 32);
/// Grid and border lines.
const GRID: Rgb = Rgb::new(43, 52, 61);
/// Primary text.
const TEXT: Rgb = Rgb::new(216, 222, 233);
/// Secondary labels.
const MUTED: Rgb = Rgb::new(138, 148, 158);
/// A rising candle.
const UP: Rgb = Rgb::new(38, 166, 154);
/// A falling candle.
const DOWN: Rgb = Rgb::new(239, 83, 80);
/// A candle whose open equals its close.
const DOJI: Rgb = Rgb::new(176, 190, 197);

/// The number of price grid divisions.
const PRICE_DIVISIONS: i32 = 4;
/// The number of x-axis labels.
const TIME_LABELS: usize = 5;
/// The widest candle body, so a short chart does not fill the plot with one bar.
const MAX_BODY_WIDTH: i32 = 24;

/// An RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rgb {
    red: u8,
    green: u8,
    blue: u8,
}

impl Rgb {
    /// Builds a color from its three channels.
    const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

/// An RGB pixel canvas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Canvas {
    /// Creates a canvas filled with one color.
    fn new(width: u32, height: u32, background: Rgb) -> Self {
        let length = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .map(|height| width * height * 3)
            })
            .unwrap_or(0);
        let mut pixels = Vec::with_capacity(length);
        for _ in 0..length / 3 {
            pixels.extend_from_slice(&[background.red, background.green, background.blue]);
        }

        Self {
            width,
            height,
            pixels,
        }
    }

    /// Sets one pixel, ignoring a coordinate outside the canvas.
    fn set(&mut self, x: i32, y: i32, color: Rgb) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as usize;
        let y = y as usize;
        let width = self.width as usize;
        let height = self.height as usize;
        if x >= width || y >= height {
            return;
        }

        let offset = (y * width + x) * 3;
        if let Some(pixel) = self.pixels.get_mut(offset..offset + 3) {
            pixel.copy_from_slice(&[color.red, color.green, color.blue]);
        }
    }

    /// Returns one pixel, or `None` when the coordinate is outside the canvas.
    #[cfg(test)]
    fn pixel(&self, x: i32, y: i32) -> Option<Rgb> {
        if x < 0 || y < 0 {
            return None;
        }
        let x = x as usize;
        let y = y as usize;
        let width = self.width as usize;
        let height = self.height as usize;
        if x >= width || y >= height {
            return None;
        }

        let offset = (y * width + x) * 3;
        let pixel = self.pixels.get(offset..offset + 3)?;
        Some(Rgb::new(pixel[0], pixel[1], pixel[2]))
    }

    /// Fills a rectangle, clipped to the canvas.
    fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: Rgb) {
        if width <= 0 || height <= 0 {
            return;
        }
        let end_x = x.saturating_add(width);
        let end_y = y.saturating_add(height);
        for row in y..end_y {
            for column in x..end_x {
                self.set(column, row, color);
            }
        }
    }

    /// Draws a one-pixel line with Bresenham's algorithm.
    fn line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: Rgb) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            self.set(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let doubled = 2 * error;
            if doubled >= dy {
                error += dy;
                x0 += sx;
            }
            if doubled <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    /// Draws text with the embedded bitmap font.
    ///
    /// The glyph is eight pixels wide and eight pixels tall at scale 1, with one
    /// pixel of spacing. Characters outside printable ASCII render as blank.
    fn text(&mut self, mut x: i32, y: i32, text: &str, color: Rgb, scale: u32) {
        if scale == 0 {
            return;
        }
        let scale = i32::try_from(scale).unwrap_or(1);

        for character in text.chars() {
            let glyph = glyph(character);
            for (row, bits) in glyph.iter().enumerate() {
                for column in 0..8_u32 {
                    if bits & (1 << column) != 0 {
                        self.fill_rect(
                            x + (column as i32) * scale,
                            y + (row as i32) * scale,
                            scale,
                            scale,
                            color,
                        );
                    }
                }
            }
            x += 9 * scale;
        }
    }

    /// Encodes the canvas as an RGB PNG.
    fn encode_png(&self) -> Result<Vec<u8>, png::EncodingError> {
        let mut output = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut output, self.width, self.height);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(&self.pixels)?;
            writer.finish()?;
        }
        Ok(output)
    }
}

/// The width of a text run in pixels.
fn text_width(text: &str, scale: u32) -> i32 {
    let count = i32::try_from(text.chars().count()).unwrap_or(0);
    if count == 0 {
        return 0;
    }
    count * 9 * i32::try_from(scale).unwrap_or(1) - i32::try_from(scale).unwrap_or(1)
}

/// Draws text whose right edge is at `right`.
fn text_right(canvas: &mut Canvas, right: i32, y: i32, text: &str, color: Rgb, scale: u32) {
    canvas.text(right - text_width(text, scale), y, text, color, scale);
}

/// A chart could not be rendered.
#[derive(Debug)]
pub(crate) enum ChartError {
    /// No candle was supplied.
    Empty,
    /// The PNG encoder failed.
    Encode(png::EncodingError),
}

impl fmt::Display for ChartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("cannot render an empty chart"),
            Self::Encode(source) => write!(formatter, "cannot encode PNG: {source}"),
        }
    }
}

impl std::error::Error for ChartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Empty => None,
            Self::Encode(source) => Some(source),
        }
    }
}

/// Renders one PNG from the supplied candles.
///
/// The candles are assumed to be in chronological order. The caller selects the
/// window to draw; this function draws exactly that slice.
///
/// # Errors
///
/// Returns [`ChartError::Empty`] when the slice is empty and
/// [`ChartError::Encode`] when the PNG encoder fails.
pub(crate) fn render_png(
    candles: &[Candle],
    symbol: &str,
    range: &DateRange,
    timeframe: Timeframe,
) -> Result<Vec<u8>, ChartError> {
    let mut canvas = Canvas::new(WIDTH, HEIGHT, BACKGROUND);
    draw_chart(&mut canvas, candles, symbol, range, timeframe)?;
    canvas.encode_png().map_err(ChartError::Encode)
}

fn draw_chart(
    canvas: &mut Canvas,
    candles: &[Candle],
    symbol: &str,
    range: &DateRange,
    timeframe: Timeframe,
) -> Result<(), ChartError> {
    let Some(first) = candles.first() else {
        return Err(ChartError::Empty);
    };
    let last = candles.last().unwrap_or(first);

    draw_panels(canvas);

    let (min_price, max_price) = price_bounds(candles);
    let max_volume = volume_max(candles);

    draw_price_grid(canvas, min_price, max_price);
    draw_volume_grid(canvas, max_volume);
    draw_time_grid(canvas, candles);

    draw_candles(canvas, candles, min_price, max_price);
    draw_volumes(canvas, candles, max_volume);

    draw_borders(canvas);
    draw_labels(canvas, first, last, candles.len(), symbol, range, timeframe);

    Ok(())
}

fn draw_panels(canvas: &mut Canvas) {
    canvas.fill_rect(LEFT, PRICE_TOP, PLOT_WIDTH, PRICE_HEIGHT, PANEL);
    canvas.fill_rect(LEFT, VOLUME_TOP, PLOT_WIDTH, VOLUME_HEIGHT, PANEL);
}

fn price_bounds(candles: &[Candle]) -> (f64, f64) {
    let mut min_price = f64::INFINITY;
    let mut max_price = f64::NEG_INFINITY;
    for candle in candles {
        min_price = min_price.min(candle.low());
        max_price = max_price.max(candle.high());
    }

    if max_price <= min_price {
        let center = min_price;
        let padding = if center == 0.0 {
            1.0
        } else {
            center.abs() * 0.01
        };
        min_price = center - padding;
        max_price = center + padding;
    } else {
        let padding = (max_price - min_price) * 0.05;
        min_price -= padding;
        max_price += padding;
    }

    (min_price, max_price)
}

fn volume_max(candles: &[Candle]) -> f64 {
    candles
        .iter()
        .filter_map(Candle::volume)
        .fold(0.0_f64, f64::max)
}

fn price_y(price: f64, min_price: f64, max_price: f64) -> i32 {
    let span = max_price - min_price;
    if span <= 0.0 {
        return PRICE_TOP + PRICE_HEIGHT / 2;
    }
    let offset = (max_price - price) / span * f64::from(PRICE_HEIGHT);
    (f64::from(PRICE_TOP) + offset).round() as i32
}

fn draw_price_grid(canvas: &mut Canvas, min_price: f64, max_price: f64) {
    let precision = price_precision(max_price - min_price);
    for division in 0..=PRICE_DIVISIONS {
        let ratio = f64::from(division) / f64::from(PRICE_DIVISIONS);
        let price = max_price - (max_price - min_price) * ratio;
        let y = PRICE_TOP + (PRICE_HEIGHT * division) / PRICE_DIVISIONS;
        canvas.line(LEFT, y, RIGHT, y, GRID);
        text_right(
            canvas,
            LEFT - 8,
            y - 4,
            &format!("{price:.precision$}"),
            TEXT,
            1,
        );
    }
}

fn draw_volume_grid(canvas: &mut Canvas, max_volume: f64) {
    for division in 0..=2 {
        let y = VOLUME_TOP + (VOLUME_HEIGHT * division) / 2;
        canvas.line(LEFT, y, RIGHT, y, GRID);
    }

    let label = if max_volume >= 100.0 {
        format!("{max_volume:.0}")
    } else if max_volume > 0.0 {
        format!("{max_volume:.2}")
    } else {
        String::from("0")
    };
    text_right(canvas, LEFT - 8, VOLUME_TOP + 4, &label, MUTED, 1);
}

fn draw_time_grid(canvas: &mut Canvas, candles: &[Candle]) {
    let count = candles.len();
    let span_seconds = candles
        .last()
        .zip(candles.first())
        .map_or(0, |(last, first)| last.timestamp() - first.timestamp());
    let date_only = span_seconds > 7 * 86_400;

    for index in 0..TIME_LABELS {
        let candle_index = if count <= 1 {
            0
        } else {
            index * (count - 1) / (TIME_LABELS - 1)
        };
        let x = if count <= 1 {
            LEFT + PLOT_WIDTH / 2
        } else {
            LEFT + i32::try_from(candle_index)
                .unwrap_or(0)
                .saturating_mul(PLOT_WIDTH)
                / i32::try_from(count - 1).unwrap_or(1)
        };

        canvas.line(x, PRICE_TOP, x, PRICE_BOTTOM, GRID);
        canvas.line(x, VOLUME_TOP, x, VOLUME_BOTTOM, GRID);

        let label = time_label(candles[candle_index].timestamp(), date_only);
        let width = text_width(&label, 1);
        let label_x = (x - width / 2).clamp(LEFT - 20, RIGHT - width);
        canvas.text(label_x, LABEL_Y, &label, MUTED, 1);
    }
}

fn draw_candles(canvas: &mut Canvas, candles: &[Candle], min_price: f64, max_price: f64) {
    let count = candles.len().max(1);
    let step = f64::from(PLOT_WIDTH) / count as f64;
    let body_width = ((step * 0.7).floor() as i32).clamp(1, MAX_BODY_WIDTH);
    let half = body_width / 2;

    for (index, candle) in candles.iter().enumerate() {
        let center = f64::from(LEFT) + (index as f64 + 0.5) * step;
        let x = center.round() as i32;
        let color = candle_color(candle);
        let high_y = price_y(candle.high(), min_price, max_price);
        let low_y = price_y(candle.low(), min_price, max_price);
        let open_y = price_y(candle.open(), min_price, max_price);
        let close_y = price_y(candle.close(), min_price, max_price);

        canvas.line(x, high_y, x, low_y, color);

        let top = open_y.min(close_y);
        let bottom = open_y.max(close_y);
        let body_height = (bottom - top).max(1);
        canvas.fill_rect(x - half, top, body_width, body_height, color);
    }
}

fn draw_volumes(canvas: &mut Canvas, candles: &[Candle], max_volume: f64) {
    let count = candles.len().max(1);
    let step = f64::from(PLOT_WIDTH) / count as f64;
    let body_width = ((step * 0.7).floor() as i32).clamp(1, MAX_BODY_WIDTH);
    let half = body_width / 2;

    for (index, candle) in candles.iter().enumerate() {
        let Some(volume) = candle.volume() else {
            continue;
        };
        let center = f64::from(LEFT) + (index as f64 + 0.5) * step;
        let x = center.round() as i32;
        let height = if max_volume > 0.0 {
            ((volume / max_volume) * f64::from(VOLUME_HEIGHT))
                .round()
                .max(1.0) as i32
        } else {
            1
        };
        canvas.fill_rect(
            x - half,
            VOLUME_BOTTOM - height.min(VOLUME_HEIGHT),
            body_width,
            height.min(VOLUME_HEIGHT),
            candle_color(candle),
        );
    }
}

fn draw_borders(canvas: &mut Canvas) {
    for (top, bottom) in [(PRICE_TOP, PRICE_BOTTOM), (VOLUME_TOP, VOLUME_BOTTOM)] {
        canvas.line(LEFT, top, RIGHT, top, GRID);
        canvas.line(LEFT, bottom, RIGHT, bottom, GRID);
        canvas.line(LEFT, top, LEFT, bottom, GRID);
        canvas.line(RIGHT, top, RIGHT, bottom, GRID);
    }
}

fn draw_labels(
    canvas: &mut Canvas,
    first: &Candle,
    last: &Candle,
    count: usize,
    symbol: &str,
    range: &DateRange,
    timeframe: Timeframe,
) {
    let title = format!(
        "{} {} {} to {} UTC",
        symbol,
        timeframe.code(),
        range.start_label(),
        range.end_label()
    );
    canvas.text(18, 16, &title, TEXT, 2);

    let subtitle = format!(
        "last {count} candles | {} to {} | bid",
        format_utc_timestamp(first.timestamp()),
        format_utc_timestamp(last.timestamp())
    );
    canvas.text(18, 44, &subtitle, MUTED, 1);

    canvas.text(8, PRICE_TOP - 14, "PRICE", MUTED, 1);
    canvas.text(8, VOLUME_TOP - 14, "VOLUME", MUTED, 1);
}

fn candle_color(candle: &Candle) -> Rgb {
    if candle.close() > candle.open() {
        UP
    } else if candle.close() < candle.open() {
        DOWN
    } else {
        DOJI
    }
}

fn price_precision(span: f64) -> usize {
    if span >= 1_000.0 {
        0
    } else if span >= 10.0 {
        2
    } else if span >= 1.0 {
        3
    } else {
        5
    }
}

fn time_label(timestamp: i64, date_only: bool) -> String {
    let text = format_utc_timestamp(timestamp);
    if date_only {
        text[..10].to_owned()
    } else {
        format!("{} {}", &text[5..10], &text[11..16])
    }
}

fn glyph(character: char) -> &'static [u8; 8] {
    let code = character as u32;
    if (0x20..=0x7e).contains(&code) {
        &font::GLYPHS[(code - 0x20) as usize]
    } else {
        &font::GLYPHS[0]
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use quantrule_data::{Candle, Timeframe};

    use super::{
        BACKGROUND, Canvas, ChartError, LEFT, PRICE_BOTTOM, PRICE_TOP, RIGHT, UP, WIDTH,
        render_png, text_width,
    };
    use crate::source::DateRange;

    fn candle(
        series_index: u64,
        timestamp: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: Option<f64>,
    ) -> Candle {
        Candle::new(series_index, timestamp, open, high, low, close, volume)
            .expect("the test candle is valid")
    }

    fn candles(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|index| {
                let timestamp = 1_735_689_600 + index as i64 * 60;
                let open = 2_600.0 + index as f64;
                candle(
                    index as u64,
                    timestamp,
                    open,
                    open + 1.0,
                    open - 1.0,
                    open + 0.5,
                    Some(index as f64),
                )
            })
            .collect()
    }

    #[test]
    fn canvas_clips_every_primitive() {
        let mut canvas = Canvas::new(WIDTH, 20, BACKGROUND);

        canvas.set(-1, 0, UP);
        canvas.set(0, -1, UP);
        canvas.set(WIDTH as i32, 0, UP);
        canvas.fill_rect(-5, -5, 4, 4, UP);
        canvas.line(-10, -10, 3, 3, UP);
        canvas.text(-100, -100, "clipped", UP, 2);

        assert_eq!(canvas.pixel(7, 7), Some(BACKGROUND));
        assert!(canvas.pixel(-1, 0).is_none());
        assert!(canvas.pixel(WIDTH as i32, 0).is_none());
    }

    #[test]
    fn text_advance_uses_eight_pixels_plus_spacing() {
        assert_eq!(text_width("", 1), 0);
        assert_eq!(text_width("A", 1), 8);
        assert_eq!(text_width("AB", 1), 17);
        assert_eq!(text_width("A", 2), 16);
    }

    #[test]
    fn a_rising_candle_uses_the_up_color() {
        let mut canvas = Canvas::new(WIDTH, 720, BACKGROUND);
        let candle = candle(0, 0, 1.0, 2.0, 0.5, 1.5, Some(1.0));

        super::draw_candles(&mut canvas, &[candle], 0.5, 2.0);

        let pixel = canvas
            .pixel(LEFT + 586, 300)
            .expect("the candle body is inside the canvas");
        assert_eq!(pixel, UP);
    }

    #[test]
    fn repeated_renders_are_byte_identical() {
        let candles = candles(20);
        let range = DateRange::parse("2025-01-01", "2025-01-01").expect("the date range is valid");

        let first =
            render_png(&candles, "XAU/USD", &range, Timeframe::M1).expect("the chart is rendered");
        let second =
            render_png(&candles, "XAU/USD", &range, Timeframe::M1).expect("the chart is rendered");

        assert_eq!(first, second);
    }

    #[test]
    fn the_timeframe_changes_the_rendered_title() {
        let candles = candles(20);
        let range = DateRange::parse("2025-01-01", "2025-01-01").expect("the date range is valid");

        let one_minute =
            render_png(&candles, "XAU/USD", &range, Timeframe::M1).expect("the chart is rendered");
        let one_hour =
            render_png(&candles, "XAU/USD", &range, Timeframe::H1).expect("the chart is rendered");

        assert_ne!(one_minute, one_hour);
    }

    #[test]
    fn png_has_the_expected_dimensions_and_color_type() {
        let candles = candles(2);
        let range = DateRange::parse("2025-01-01", "2025-01-01").expect("the date range is valid");
        let bytes =
            render_png(&candles, "XAU/USD", &range, Timeframe::M1).expect("the chart is rendered");

        let decoder = png::Decoder::new(Cursor::new(bytes));
        let mut reader = decoder.read_info().expect("the PNG header is valid");
        assert_eq!(reader.info().width, WIDTH);
        assert_eq!(reader.info().height, 720);
        assert_eq!(reader.info().color_type, png::ColorType::Rgb);

        let mut buffer = vec![
            0;
            reader
                .output_buffer_size()
                .expect("the buffer size is known")
        ];
        reader
            .next_frame(&mut buffer)
            .expect("the PNG frame is valid");
    }

    #[test]
    fn a_single_flat_zero_volume_candle_renders() {
        let candle = candle(
            0,
            1_735_689_600,
            2_600.0,
            2_600.0,
            2_600.0,
            2_600.0,
            Some(0.0),
        );
        let range = DateRange::parse("2025-01-01", "2025-01-01").expect("the date range is valid");

        let bytes =
            render_png(&[candle], "XAU/USD", &range, Timeframe::M1).expect("the chart is rendered");

        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn empty_chart_is_an_error() {
        let range = DateRange::parse("2025-01-01", "2025-01-01").expect("the date range is valid");

        assert!(matches!(
            render_png(&[], "XAU/USD", &range, Timeframe::M1),
            Err(ChartError::Empty)
        ));
    }

    #[test]
    fn chart_contains_price_and_volume_panels() {
        let candles = candles(10);
        let range = DateRange::parse("2025-01-01", "2025-01-01").expect("the date range is valid");
        let bytes =
            render_png(&candles, "XAU/USD", &range, Timeframe::M1).expect("the chart is rendered");
        let decoder = png::Decoder::new(Cursor::new(bytes));
        let mut reader = decoder.read_info().expect("the PNG header is valid");
        let mut buffer = vec![
            0;
            reader
                .output_buffer_size()
                .expect("the buffer size is known")
        ];
        let frame = reader
            .next_frame(&mut buffer)
            .expect("the PNG frame is valid");
        let pixels = &buffer[..frame.buffer_size()];
        let pixel = |x: i32, y: i32| {
            let offset = ((y as usize) * WIDTH as usize + x as usize) * 3;
            &pixels[offset..offset + 3]
        };

        assert_eq!(pixel(LEFT, PRICE_TOP), &[43, 52, 61]);
        assert_eq!(pixel(LEFT, PRICE_BOTTOM), &[43, 52, 61]);
        assert_eq!(pixel(RIGHT, PRICE_TOP), &[43, 52, 61]);
    }
}
