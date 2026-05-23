//! # Wallpaper Engine
//!
//! Loads, caches, and renders desktop wallpapers.
//!
//! Supports:
//! - Static images (PNG, JPEG, WebP, BMP)
//! - Per-output wallpapers (different image per monitor)
//! - Fit modes: fill, fit, stretch, center, tile
//! - Solid color fallback
//!
//! ## MDB Touch
//!
//! The default wallpaper is a procedurally generated MDB dimensional
//! visualization — a dark field with golden ratio spirals and dimensional
//! coordinate grid lines, rendered at native resolution.

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use crate::output::OutputId;

/// How to fit the wallpaper to the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallpaperMode {
    /// Scale to fill, cropping excess (default).
    Fill,
    /// Scale to fit inside, letterboxing.
    Fit,
    /// Stretch to exact output size.
    Stretch,
    /// Center at native size, no scaling.
    Center,
    /// Tile the image to fill the output.
    Tile,
}

/// Wallpaper configuration for a single output.
#[derive(Debug, Clone)]
pub struct WallpaperConfig {
    /// Image source (None = use solid color or procedural).
    pub image_path: Option<PathBuf>,
    /// Fit mode.
    pub mode: WallpaperMode,
    /// Solid color fallback [R, G, B, A] in 0.0–1.0.
    pub solid_color: [f32; 4],
    /// Use procedural MDB wallpaper (overrides image and color).
    pub procedural: bool,
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            image_path: None,
            mode: WallpaperMode::Fill,
            // Deep MDB dark blue
            solid_color: [0.05, 0.05, 0.12, 1.0],
            procedural: true,
        }
    }
}

/// Cached pixel data for a loaded wallpaper.
#[derive(Debug)]
pub struct WallpaperTexture {
    /// RGBA pixel data.
    pub pixels: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// The wallpaper engine.
pub struct WallpaperEngine {
    /// Per-output wallpaper config.
    configs: HashMap<OutputId, WallpaperConfig>,
    /// Global default config.
    default_config: WallpaperConfig,
    /// Cached textures per output.
    cache: HashMap<OutputId, WallpaperTexture>,
}

impl WallpaperEngine {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            default_config: WallpaperConfig::default(),
            cache: HashMap::new(),
        }
    }

    /// Set wallpaper config for a specific output.
    pub fn set_output_wallpaper(&mut self, output_id: OutputId, config: WallpaperConfig) {
        self.cache.remove(&output_id); // Invalidate cache
        self.configs.insert(output_id, config);
    }

    /// Set the default wallpaper config (used for outputs without specific config).
    pub fn set_default(&mut self, config: WallpaperConfig) {
        self.cache.clear(); // Invalidate all caches
        self.default_config = config;
    }

    /// Get the wallpaper config for an output.
    pub fn config_for(&self, output_id: OutputId) -> &WallpaperConfig {
        self.configs.get(&output_id).unwrap_or(&self.default_config)
    }

    /// Load or generate the wallpaper texture for an output.
    pub fn get_texture(
        &mut self,
        output_id: OutputId,
        output_width: u32,
        output_height: u32,
    ) -> &WallpaperTexture {
        if !self.cache.contains_key(&output_id) {
            let config = self.config_for(output_id).clone();
            let texture = if config.procedural {
                self.generate_mdb_wallpaper(output_width, output_height)
            } else if let Some(path) = &config.image_path {
                self.load_image(path, output_width, output_height, config.mode)
                    .unwrap_or_else(|e| {
                        log::warn!("Failed to load wallpaper {:?}: {}", path, e);
                        self.solid_color_texture(output_width, output_height, config.solid_color)
                    })
            } else {
                self.solid_color_texture(output_width, output_height, config.solid_color)
            };
            self.cache.insert(output_id, texture);
        }
        self.cache.get(&output_id).unwrap()
    }

    /// Invalidate cached texture for an output (e.g. on mode change).
    pub fn invalidate(&mut self, output_id: OutputId) {
        self.cache.remove(&output_id);
    }

    /// Invalidate all cached textures.
    pub fn invalidate_all(&mut self) {
        self.cache.clear();
    }

    // ── Texture generation ──────────────────────────────────────────────

    /// Generate the procedural MDB wallpaper.
    ///
    /// Dark background with:
    /// - Golden ratio spiral curves (φ)
    /// - Dimensional grid lines (D3/D4/D5 axes)
    /// - Subtle noise texture
    /// - MDB logo glow at center
    fn generate_mdb_wallpaper(&self, w: u32, h: u32) -> WallpaperTexture {
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let phi: f64 = 1.618_033_988_749_895;

        let cx = w as f64 / 2.0;
        let cy = h as f64 / 2.0;
        let max_r = (cx * cx + cy * cy).sqrt();

        for y in 0..h {
            for x in 0..w {
                let idx = ((y * w + x) * 4) as usize;
                let fx = x as f64 - cx;
                let fy = y as f64 - cy;
                let r = (fx * fx + fy * fy).sqrt();
                let theta = fy.atan2(fx);

                // Base: deep dark blue
                let mut red = 0.04f64;
                let mut green = 0.04;
                let mut blue = 0.10;

                // Golden ratio spiral — thin bright lines
                let spiral_r = 20.0 * (theta * phi).exp().abs() % max_r;
                let spiral_dist = (r - spiral_r).abs();
                if spiral_dist < 1.5 {
                    let intensity = (1.0 - spiral_dist / 1.5) * 0.15;
                    red += intensity * 0.0;
                    green += intensity * 0.85;
                    blue += intensity * 0.95;
                }

                // Dimensional grid — faint lines every 100px
                let grid_x = (x as f64 % 100.0 - 50.0).abs();
                let grid_y = (y as f64 % 100.0 - 50.0).abs();
                if grid_x < 0.5 || grid_y < 0.5 {
                    let fade = 1.0 - r / max_r;
                    red += 0.03 * fade;
                    green += 0.04 * fade;
                    blue += 0.06 * fade;
                }

                // Center glow — soft radial gradient
                let glow_r = r / (max_r * 0.3);
                if glow_r < 1.0 {
                    let glow = (1.0 - glow_r * glow_r) * 0.08;
                    green += glow * 0.7;
                    blue += glow;
                }

                // Subtle noise
                let noise = pseudo_noise(x, y) * 0.015;
                red += noise;
                green += noise;
                blue += noise;

                pixels[idx] = (red.clamp(0.0, 1.0) * 255.0) as u8;
                pixels[idx + 1] = (green.clamp(0.0, 1.0) * 255.0) as u8;
                pixels[idx + 2] = (blue.clamp(0.0, 1.0) * 255.0) as u8;
                pixels[idx + 3] = 255;
            }
        }

        WallpaperTexture { pixels, width: w, height: h }
    }

    /// Load an image file and scale it to the output size.
    fn load_image(
        &self,
        path: &Path,
        target_w: u32,
        target_h: u32,
        mode: WallpaperMode,
    ) -> Result<WallpaperTexture, String> {
        let img = image::open(path)
            .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
        let img = img.to_rgba8();
        let (src_w, src_h) = img.dimensions();

        let resized = match mode {
            WallpaperMode::Stretch => {
                image::imageops::resize(&img, target_w, target_h, image::imageops::FilterType::Lanczos3)
            }
            WallpaperMode::Fill => {
                // Scale to cover, then crop center
                let scale = (target_w as f64 / src_w as f64)
                    .max(target_h as f64 / src_h as f64);
                let scaled_w = (src_w as f64 * scale) as u32;
                let scaled_h = (src_h as f64 * scale) as u32;
                let scaled = image::imageops::resize(&img, scaled_w, scaled_h, image::imageops::FilterType::Lanczos3);
                let crop_x = (scaled_w.saturating_sub(target_w)) / 2;
                let crop_y = (scaled_h.saturating_sub(target_h)) / 2;
                image::imageops::crop_imm(&scaled, crop_x, crop_y, target_w, target_h).to_image()
            }
            WallpaperMode::Fit => {
                // Scale to fit inside, letterbox with solid color
                let scale = (target_w as f64 / src_w as f64)
                    .min(target_h as f64 / src_h as f64);
                let scaled_w = (src_w as f64 * scale) as u32;
                let scaled_h = (src_h as f64 * scale) as u32;
                let scaled = image::imageops::resize(&img, scaled_w, scaled_h, image::imageops::FilterType::Lanczos3);
                let mut canvas = image::RgbaImage::from_pixel(
                    target_w, target_h,
                    image::Rgba([12, 12, 30, 255]),
                );
                let offset_x = (target_w - scaled_w) / 2;
                let offset_y = (target_h - scaled_h) / 2;
                image::imageops::overlay(&mut canvas, &scaled, offset_x as i64, offset_y as i64);
                canvas
            }
            WallpaperMode::Center => {
                let mut canvas = image::RgbaImage::from_pixel(
                    target_w, target_h,
                    image::Rgba([12, 12, 30, 255]),
                );
                let offset_x = (target_w as i64 - src_w as i64) / 2;
                let offset_y = (target_h as i64 - src_h as i64) / 2;
                image::imageops::overlay(&mut canvas, &img, offset_x, offset_y);
                canvas
            }
            WallpaperMode::Tile => {
                let mut canvas = image::RgbaImage::new(target_w, target_h);
                for ty in (0..target_h).step_by(src_h as usize) {
                    for tx in (0..target_w).step_by(src_w as usize) {
                        image::imageops::overlay(&mut canvas, &img, tx as i64, ty as i64);
                    }
                }
                canvas
            }
        };

        Ok(WallpaperTexture {
            pixels: resized.into_raw(),
            width: target_w,
            height: target_h,
        })
    }

    /// Generate a solid color texture.
    fn solid_color_texture(&self, w: u32, h: u32, color: [f32; 4]) -> WallpaperTexture {
        let r = (color[0] * 255.0) as u8;
        let g = (color[1] * 255.0) as u8;
        let b = (color[2] * 255.0) as u8;
        let a = (color[3] * 255.0) as u8;
        let pixel_count = (w * h) as usize;
        let mut pixels = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            pixels.extend_from_slice(&[r, g, b, a]);
        }
        WallpaperTexture { pixels, width: w, height: h }
    }
}

/// Simple deterministic noise function (no external dependency).
fn pseudo_noise(x: u32, y: u32) -> f64 {
    let mut h = (x.wrapping_mul(374761393)).wrapping_add(y.wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h = h ^ (h >> 16);
    (h as f64 / u32::MAX as f64) - 0.5
}
