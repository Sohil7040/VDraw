//! Industrial Laser Raster Engraving Subsystem.
//!
//! Provides pixel manipulation (grayscale, brightness, contrast, gamma, invert),
//! high-fidelity dithering algorithms (Threshold, Bayer 4x4, Bayer 8x8, Floyd-Steinberg, Atkinson),
//! DPI-controlled scanline generation, bidirectional scanning, overscan acceleration ramps,
//! and run-length encoded toolpath synthesis.

use core_geom::Point2;
use serde::{Deserialize, Serialize};

/// Supported dithering algorithms for laser photo & graphic etching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DitherAlgorithm {
    Threshold(u8),
    Bayer4x4,
    Bayer8x8,
    #[default]
    FloydSteinberg,
    Atkinson,
}

/// A grayscale raster image positioned in physical millimeter space.
///
/// Pixel values are 0..=255, where:
/// - 0 represents pure black (maximum laser burn / 100% power)
/// - 255 represents pure white (no burn / 0% power / laser off)
#[derive(Debug, Clone, PartialEq)]
pub struct RasterImage {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
    pub origin: Point2,
    pub width_mm: f64,
    pub height_mm: f64,
}

impl RasterImage {
    /// Create from existing grayscale 8-bit buffer.
    pub fn new_grayscale(
        width: usize,
        height: usize,
        data: Vec<u8>,
        origin: Point2,
        width_mm: f64,
        height_mm: f64,
    ) -> Self {
        assert_eq!(
            data.len(),
            width * height,
            "RasterImage data length must match width * height"
        );
        Self {
            width,
            height,
            data,
            origin,
            width_mm,
            height_mm,
        }
    }

    /// Create from 32-bit RGBA buffer, converting to standard luminance grayscale.
    pub fn from_rgba(
        width: usize,
        height: usize,
        rgba: &[u8],
        origin: Point2,
        width_mm: f64,
        height_mm: f64,
    ) -> Self {
        assert_eq!(rgba.len(), width * height * 4);
        let mut data = Vec::with_capacity(width * height);
        for chunk in rgba.chunks_exact(4) {
            let r = chunk[0] as f64;
            let g = chunk[1] as f64;
            let b = chunk[2] as f64;
            let a = chunk[3] as f64 / 255.0;

            // Standard ITU-R BT.601 luminance with alpha blending over white
            let lum = (0.299 * r + 0.587 * g + 0.114 * b) * a + 255.0 * (1.0 - a);
            data.push(lum.round().clamp(0.0, 255.0) as u8);
        }

        Self {
            width,
            height,
            data,
            origin,
            width_mm,
            height_mm,
        }
    }

    /// Adjust image brightness (-1.0 to 1.0) and contrast (-1.0 to 1.0).
    pub fn adjust_brightness_contrast(&mut self, brightness: f64, contrast: f64) {
        let b = brightness.clamp(-1.0, 1.0) * 255.0;
        let c = contrast.clamp(-1.0, 1.0);
        let factor = (1.0 + c) / (1.0 - c.min(0.99));

        for val in &mut self.data {
            let v = *val as f64;
            let adjusted = factor * (v - 128.0) + 128.0 + b;
            *val = adjusted.round().clamp(0.0, 255.0) as u8;
        }
    }

    /// Apply gamma correction (`gamma > 0.0`). Standard gamma is 1.0.
    pub fn adjust_gamma(&mut self, gamma: f64) {
        if gamma <= 0.0 || (gamma - 1.0).abs() < 1e-4 {
            return;
        }
        let inv_gamma = 1.0 / gamma;
        for val in &mut self.data {
            let norm = *val as f64 / 255.0;
            let corrected = norm.powf(inv_gamma) * 255.0;
            *val = corrected.round().clamp(0.0, 255.0) as u8;
        }
    }

    /// Invert image tones.
    pub fn invert(&mut self) {
        for val in &mut self.data {
            *val = 255 - *val;
        }
    }

    /// Perform dithering to produce a binary 1-bit etching pattern (0 or 255).
    pub fn dither(&self, algo: DitherAlgorithm) -> Self {
        let mut dithered = self.clone();
        let w = self.width;
        let h = self.height;

        match algo {
            DitherAlgorithm::Threshold(thresh) => {
                for v in &mut dithered.data {
                    *v = if *v < thresh { 0 } else { 255 };
                }
            }
            DitherAlgorithm::Bayer4x4 => {
                #[rustfmt::skip]
                const BAYER4: [f64; 16] = [
                     0.0 / 16.0,  8.0 / 16.0,  2.0 / 16.0, 10.0 / 16.0,
                    12.0 / 16.0,  4.0 / 16.0, 14.0 / 16.0,  6.0 / 16.0,
                     3.0 / 16.0, 11.0 / 16.0,  1.0 / 16.0,  9.0 / 16.0,
                    15.0 / 16.0,  7.0 / 16.0, 13.0 / 16.0,  5.0 / 16.0,
                ];
                for y in 0..h {
                    for x in 0..w {
                        let idx = y * w + x;
                        let threshold = (BAYER4[(y % 4) * 4 + (x % 4)] * 255.0).round() as u8;
                        dithered.data[idx] = if dithered.data[idx] < threshold {
                            0
                        } else {
                            255
                        };
                    }
                }
            }
            DitherAlgorithm::Bayer8x8 => {
                #[rustfmt::skip]
                const BAYER8: [f64; 64] = [
                     0.0/64.0, 32.0/64.0,  8.0/64.0, 40.0/64.0,  2.0/64.0, 34.0/64.0, 10.0/64.0, 42.0/64.0,
                    48.0/64.0, 16.0/64.0, 56.0/64.0, 24.0/64.0, 50.0/64.0, 18.0/64.0, 58.0/64.0, 26.0/64.0,
                    12.0/64.0, 44.0/64.0,  4.0/64.0, 36.0/64.0, 14.0/64.0, 46.0/64.0,  6.0/64.0, 38.0/64.0,
                    60.0/64.0, 28.0/64.0, 52.0/64.0, 20.0/64.0, 62.0/64.0, 30.0/64.0, 54.0/64.0, 22.0/64.0,
                     3.0/64.0, 35.0/64.0, 11.0/64.0, 43.0/64.0,  1.0/64.0, 33.0/64.0,  9.0/64.0, 41.0/64.0,
                    51.0/64.0, 19.0/64.0, 59.0/64.0, 27.0/64.0, 49.0/64.0, 17.0/64.0, 57.0/64.0, 25.0/64.0,
                    15.0/64.0, 47.0/64.0,  7.0/64.0, 39.0/64.0, 13.0/64.0, 45.0/64.0,  5.0/64.0, 37.0/64.0,
                    63.0/64.0, 31.0/64.0, 55.0/64.0, 23.0/64.0, 61.0/64.0, 29.0/64.0, 53.0/64.0, 21.0/64.0,
                ];
                for y in 0..h {
                    for x in 0..w {
                        let idx = y * w + x;
                        let threshold = (BAYER8[(y % 8) * 8 + (x % 8)] * 255.0).round() as u8;
                        dithered.data[idx] = if dithered.data[idx] < threshold {
                            0
                        } else {
                            255
                        };
                    }
                }
            }
            DitherAlgorithm::FloydSteinberg => {
                let mut buf: Vec<f64> = self.data.iter().map(|&v| v as f64).collect();
                for y in 0..h {
                    for x in 0..w {
                        let idx = y * w + x;
                        let old_val = buf[idx];
                        let new_val = if old_val < 128.0 { 0.0 } else { 255.0 };
                        buf[idx] = new_val;
                        let err = old_val - new_val;

                        if x + 1 < w {
                            buf[idx + 1] += err * (7.0 / 16.0);
                        }
                        if y + 1 < h {
                            if x > 0 {
                                buf[(y + 1) * w + (x - 1)] += err * (3.0 / 16.0);
                            }
                            buf[(y + 1) * w + x] += err * (5.0 / 16.0);
                            if x + 1 < w {
                                buf[(y + 1) * w + (x + 1)] += err * (1.0 / 16.0);
                            }
                        }
                    }
                }
                for (dst, &src) in dithered.data.iter_mut().zip(&buf) {
                    *dst = if src < 128.0 { 0 } else { 255 };
                }
            }
            DitherAlgorithm::Atkinson => {
                let mut buf: Vec<f64> = self.data.iter().map(|&v| v as f64).collect();
                for y in 0..h {
                    for x in 0..w {
                        let idx = y * w + x;
                        let old_val = buf[idx];
                        let new_val = if old_val < 128.0 { 0.0 } else { 255.0 };
                        buf[idx] = new_val;
                        let err = (old_val - new_val) / 8.0;

                        if x + 1 < w {
                            buf[idx + 1] += err;
                        }
                        if x + 2 < w {
                            buf[idx + 2] += err;
                        }
                        if y + 1 < h {
                            if x > 0 {
                                buf[(y + 1) * w + (x - 1)] += err;
                            }
                            buf[(y + 1) * w + x] += err;
                            if x + 1 < w {
                                buf[(y + 1) * w + (x + 1)] += err;
                            }
                        }
                        if y + 2 < h {
                            buf[(y + 2) * w + x] += err;
                        }
                    }
                }
                for (dst, &src) in dithered.data.iter_mut().zip(&buf) {
                    *dst = if src < 128.0 { 0 } else { 255 };
                }
            }
        }

        dithered
    }
}

/// A contiguous burn span on a single raster scanline.
#[derive(Debug, Clone, PartialEq)]
pub struct RasterSpan {
    pub x_start_mm: f64,
    pub x_end_mm: f64,
    pub power_pct: f64,
}

/// A discrete horizontal raster scanline with overscan margins.
#[derive(Debug, Clone, PartialEq)]
pub struct RasterScanline {
    pub y_mm: f64,
    pub ramp_start_x: f64,
    pub ramp_end_x: f64,
    pub direction_left_to_right: bool,
    pub spans: Vec<RasterSpan>,
}

/// Complete planned raster engraving job.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RasterJobPlan {
    pub scanlines: Vec<RasterScanline>,
    pub total_scan_length_mm: f64,
    pub total_burn_length_mm: f64,
    pub estimated_duration_s: f64,
}

/// Generate optimized raster scanlines from a dithered image.
pub fn generate_raster_plan(
    image: &RasterImage,
    dpi: f64,
    bidirectional: bool,
    overscan_mm: f64,
    power_pct: f64,
    speed_mm_s: f64,
) -> RasterJobPlan {
    if image.width == 0 || image.height == 0 || image.width_mm <= 0.0 || image.height_mm <= 0.0 {
        return RasterJobPlan::default();
    }

    let line_pitch_mm = 25.4 / dpi.max(10.0);
    let total_lines = (image.height_mm / line_pitch_mm).ceil() as usize;
    let mut scanlines = Vec::with_capacity(total_lines);

    let mut total_scan_len = 0.0;
    let mut total_burn_len = 0.0;

    let pixel_w_mm = image.width_mm / image.width as f64;

    for line_idx in 0..total_lines {
        let y_mm = image.origin.y + (line_idx as f64) * line_pitch_mm;
        if y_mm > image.origin.y + image.height_mm {
            break;
        }

        // Map line_idx to image pixel row
        let img_y = ((line_idx as f64 / total_lines as f64) * image.height as f64) as usize;
        let img_y = img_y.min(image.height - 1);

        let row_start = img_y * image.width;
        let row = &image.data[row_start..row_start + image.width];

        // Determine if line has any burn pixels (val == 0)
        let mut spans = Vec::new();
        let mut in_burn = false;
        let mut span_start_x = 0.0;

        for (x, &val) in row.iter().enumerate() {
            let cur_x_mm = image.origin.x + (x as f64) * pixel_w_mm;
            if val == 0 {
                // Burn pixel
                if !in_burn {
                    in_burn = true;
                    span_start_x = cur_x_mm;
                }
            } else if in_burn {
                in_burn = false;
                let span_len = cur_x_mm - span_start_x;
                total_burn_len += span_len;
                spans.push(RasterSpan {
                    x_start_mm: span_start_x,
                    x_end_mm: cur_x_mm,
                    power_pct,
                });
            }
        }

        if in_burn {
            let end_x = image.origin.x + image.width_mm;
            let span_len = end_x - span_start_x;
            total_burn_len += span_len;
            spans.push(RasterSpan {
                x_start_mm: span_start_x,
                x_end_mm: end_x,
                power_pct,
            });
        }

        // Skip completely blank scanlines
        if spans.is_empty() {
            continue;
        }

        let l2r = if bidirectional {
            line_idx % 2 == 0
        } else {
            true
        };

        let min_x = image.origin.x;
        let max_x = image.origin.x + image.width_mm;
        let (ramp_start, ramp_end) = if l2r {
            (min_x - overscan_mm, max_x + overscan_mm)
        } else {
            (max_x + overscan_mm, min_x - overscan_mm)
        };

        let line_len = (ramp_end - ramp_start).abs();
        total_scan_len += line_len;

        scanlines.push(RasterScanline {
            y_mm,
            ramp_start_x: ramp_start,
            ramp_end_x: ramp_end,
            direction_left_to_right: l2r,
            spans,
        });
    }

    // Estimate duration: scan length / speed + ramp acceleration allowances
    let duration = if speed_mm_s > 0.0 {
        total_scan_len / speed_mm_s + (scanlines.len() as f64 * 0.02)
    } else {
        0.0
    };

    RasterJobPlan {
        scanlines,
        total_scan_length_mm: total_scan_len,
        total_burn_length_mm: total_burn_len,
        estimated_duration_s: duration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grayscale_and_dithering() {
        // Create 4x4 gradient test image
        let mut data = Vec::with_capacity(16);
        for i in 0..16 {
            data.push((i * 16) as u8);
        }

        let img = RasterImage::new_grayscale(4, 4, data, Point2::ZERO, 10.0, 10.0);
        assert_eq!(img.width, 4);
        assert_eq!(img.height, 4);

        // Floyd-Steinberg dither
        let fs = img.dither(DitherAlgorithm::FloydSteinberg);
        for &val in &fs.data {
            assert!(val == 0 || val == 255);
        }

        // Atkinson dither
        let atk = img.dither(DitherAlgorithm::Atkinson);
        for &val in &atk.data {
            assert!(val == 0 || val == 255);
        }

        // Bayer dither
        let bayer = img.dither(DitherAlgorithm::Bayer4x4);
        for &val in &bayer.data {
            assert!(val == 0 || val == 255);
        }
    }

    #[test]
    fn test_raster_scanline_generation() {
        // 10x10 image with a 4x4 black square in the middle
        let mut data = vec![255u8; 100];
        for y in 3..7 {
            for x in 3..7 {
                data[y * 10 + x] = 0;
            }
        }

        let img = RasterImage::new_grayscale(10, 10, data, Point2::ZERO, 10.0, 10.0);
        let plan = generate_raster_plan(&img, 25.4, true, 2.0, 50.0, 200.0);

        assert!(!plan.scanlines.is_empty());
        assert!(plan.total_burn_length_mm > 0.0);
        assert!(plan.total_scan_length_mm > plan.total_burn_length_mm);
    }
}
