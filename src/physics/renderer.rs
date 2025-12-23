#![allow(deprecated)]

use ab_glyph::{FontRef, PxScale};
use anyhow::{Context, Result};
use exif::{In, Reader as ExifReader, Tag};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use super::engine::PhysicsEngine;
use crate::config::Config;
use crate::core::{ArticleInfo, CloudState};

/// Physics-based renderer that generates frames and encodes to video
pub struct PhysicsRenderer {
    engine: PhysicsEngine,
    config: Config,
    font: FontRef<'static>,
    ffmpeg_process: Option<Child>,
    output_path: Option<std::path::PathBuf>,
    frame_count: u32,
    background_color: Rgba<u8>,
    current_article: Option<ArticleInfo>,
    /// Cache of loaded and pre-processed background images (already scaled to fill)
    /// None value means the image failed to load
    background_cache: HashMap<String, Option<RgbaImage>>,
    /// Currently active background image (persists until a new one is found)
    current_background: Option<RgbaImage>,
    /// Previous background for crossfade transition
    previous_background: Option<RgbaImage>,
    /// Crossfade progress (0.0 = showing previous, 1.0 = showing current)
    crossfade_progress: f32,
    /// Crossfade speed per frame (higher = faster transition)
    crossfade_speed: f32,
    /// Darkening overlay opacity (0.0 = no darkening, 1.0 = fully black)
    overlay_opacity: f32,
}

impl PhysicsRenderer {
    pub fn new(config: Config) -> Result<Self> {
        // Load font
        let font_data = match &config.font_path {
            Some(path) => std::fs::read(path)
                .with_context(|| format!("Failed to read font file: {:?}", path))?,
            None => {
                // Try to find a system font
                let font_paths = [
                    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                    "/usr/share/fonts/TTF/DejaVuSans.ttf",
                    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
                    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
                    "/System/Library/Fonts/Helvetica.ttc",
                    "C:\\Windows\\Fonts\\arial.ttf",
                ];

                let mut found = None;
                for path in font_paths {
                    if let Ok(data) = std::fs::read(path) {
                        found = Some(data);
                        break;
                    }
                }

                found.context("No font file specified and no system font found")?
            }
        };

        let font = FontRef::try_from_slice(Box::leak(font_data.into_boxed_slice()))
            .context("Failed to parse font file")?;

        let background_color = parse_hex_color(&config.background_color);

        Ok(Self {
            engine: PhysicsEngine::new(&config, font.clone()),
            config,
            font,
            ffmpeg_process: None,
            output_path: None,
            frame_count: 0,
            background_color,
            current_article: None,
            background_cache: HashMap::new(),
            current_background: None,
            previous_background: None,
            crossfade_progress: 1.0, // Start fully transitioned
            crossfade_speed: 0.02,   // ~50 frames for full transition at 60fps (~0.8 seconds)
            overlay_opacity: 0.5,    // 50% darkening by default
        })
    }

    /// Start the ffmpeg process for piping frames
    fn start_ffmpeg(&mut self, output_path: &Path) -> Result<()> {
        if self.ffmpeg_process.is_some() {
            return Ok(()); // Already started
        }

        let process = Command::new("ffmpeg")
            .args([
                "-y",
                "-f", "rawvideo",
                "-pixel_format", "rgba",
                "-video_size", &format!("{}x{}", self.config.frame_width, self.config.frame_height),
                "-framerate", &self.config.fps.to_string(),
                "-i", "-",  // Read from stdin
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                "-crf", "18",
                "-preset", "medium",
                output_path.to_str().unwrap(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start ffmpeg process")?;

        self.ffmpeg_process = Some(process);
        self.output_path = Some(output_path.to_path_buf());

        Ok(())
    }

    /// Load and scale an image to fill the frame (zoom to fill, crop excess)
    fn load_background_image(&mut self, path: &str) -> Option<RgbaImage> {
        // Check cache first (including failed loads cached as None)
        if let Some(cached) = self.background_cache.get(path) {
            return cached.clone();
        }

        // Load the image
        let img = match image::open(path) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("Warning: Failed to load background image '{}': {}", path, e);
                // Cache the failure so we don't keep retrying
                self.background_cache.insert(path.to_string(), None);
                return None;
            }
        };

        // Apply EXIF orientation
        let img = self.apply_exif_orientation(path, img);

        // Scale to fill (cover) - zoom so image fills entire frame, crop excess
        let processed = self.scale_to_fill(img);

        // Cache for reuse
        self.background_cache.insert(path.to_string(), Some(processed.clone()));

        Some(processed)
    }

    /// Read EXIF orientation and rotate/flip image accordingly
    fn apply_exif_orientation(&self, path: &str, img: DynamicImage) -> DynamicImage {
        let orientation = match File::open(path) {
            Ok(file) => {
                let mut bufreader = BufReader::new(&file);
                match ExifReader::new().read_from_container(&mut bufreader) {
                    Ok(exif) => {
                        exif.get_field(Tag::Orientation, In::PRIMARY)
                            .and_then(|f| f.value.get_uint(0))
                            .unwrap_or(1)
                    }
                    Err(_) => 1, // No EXIF data, assume normal orientation
                }
            }
            Err(_) => 1,
        };

        // Apply orientation transform
        // See: https://exiftool.org/TagNames/EXIF.html (Orientation values)
        match orientation {
            1 => img,                                           // Normal
            2 => img.fliph(),                                   // Flipped horizontally
            3 => img.rotate180(),                               // Rotated 180°
            4 => img.flipv(),                                   // Flipped vertically
            5 => img.rotate90().fliph(),                        // Rotated 90° CW + flipped horizontally
            6 => img.rotate90(),                                // Rotated 90° CW
            7 => img.rotate270().fliph(),                       // Rotated 270° CW + flipped horizontally
            8 => img.rotate270(),                               // Rotated 270° CW
            _ => img,                                           // Unknown, leave as-is
        }
    }

    /// Scale image to fill the frame completely (zoom to fill / cover mode)
    /// This crops the image to fill the entire frame without letterboxing
    fn scale_to_fill(&self, img: DynamicImage) -> RgbaImage {
        let (img_w, img_h) = img.dimensions();
        let frame_w = self.config.frame_width;
        let frame_h = self.config.frame_height;

        // Calculate scale factor to fill (cover) - use the larger scale
        let scale_x = frame_w as f32 / img_w as f32;
        let scale_y = frame_h as f32 / img_h as f32;
        let scale = scale_x.max(scale_y);

        // Calculate new dimensions after scaling
        let new_w = (img_w as f32 * scale).ceil() as u32;
        let new_h = (img_h as f32 * scale).ceil() as u32;

        // Resize the image
        let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);

        // Calculate crop offsets to center the image
        let crop_x = (new_w.saturating_sub(frame_w)) / 2;
        let crop_y = (new_h.saturating_sub(frame_h)) / 2;

        // Crop to exact frame size
        let cropped = resized.crop_imm(crop_x, crop_y, frame_w, frame_h);

        cropped.to_rgba8()
    }

    /// Apply darkening overlay to an image
    fn apply_darkening_overlay(&self, img: &mut RgbaImage) {
        let overlay_alpha = (self.overlay_opacity * 255.0) as u8;

        for pixel in img.pixels_mut() {
            // Blend with black using overlay opacity
            let r = ((pixel[0] as u16 * (255 - overlay_alpha) as u16) / 255) as u8;
            let g = ((pixel[1] as u16 * (255 - overlay_alpha) as u16) / 255) as u8;
            let b = ((pixel[2] as u16 * (255 - overlay_alpha) as u16) / 255) as u8;
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
        }
    }

    /// Render a single state (one frame)
    pub fn render_state(&mut self, state: &CloudState, output_path: &Path) -> Result<()> {
        // Start ffmpeg if not already running
        self.start_ffmpeg(output_path)?;

        // Update current article info
        self.current_article = state.current_article.clone();

        // Update background image if article has one (persists until next image found)
        if let Some(ref article) = state.current_article {
            if let Some(ref image_path) = article.image_path {
                if let Some(bg) = self.load_background_image(image_path) {
                    // Only start crossfade if this is a different image
                    let is_new_image = match &self.current_background {
                        Some(current) => current.as_raw() != bg.as_raw(),
                        None => true,
                    };
                    if is_new_image {
                        // Move current to previous for crossfade
                        self.previous_background = self.current_background.take();
                        self.current_background = Some(bg);
                        self.crossfade_progress = 0.0; // Start crossfade
                    }
                }
            }
        }

        // Advance crossfade
        if self.crossfade_progress < 1.0 {
            self.crossfade_progress = (self.crossfade_progress + self.crossfade_speed).min(1.0);
        }

        // Update physics with new frequencies
        self.engine.update_frequencies(&state.top_words);

        // Run physics steps for smooth animation
        let dt = 1.0 / 60.0;
        for _ in 0..self.config.physics_steps_per_frame {
            self.engine.step(dt);
        }

        // Render frame
        self.render_frame()?;
        self.frame_count += 1;

        Ok(())
    }

    fn render_frame(&mut self) -> Result<()> {
        // Start with background - handle crossfade between images
        let mut img = match (&self.current_background, &self.previous_background) {
            (Some(current), Some(previous)) if self.crossfade_progress < 1.0 => {
                // Crossfading between two images
                let mut blended = RgbaImage::new(self.config.frame_width, self.config.frame_height);
                let alpha = self.crossfade_progress;

                for (x, y, pixel) in blended.enumerate_pixels_mut() {
                    let prev_pixel = previous.get_pixel(x, y);
                    let curr_pixel = current.get_pixel(x, y);

                    // Linear interpolation between pixels
                    let r = (prev_pixel[0] as f32 * (1.0 - alpha) + curr_pixel[0] as f32 * alpha) as u8;
                    let g = (prev_pixel[1] as f32 * (1.0 - alpha) + curr_pixel[1] as f32 * alpha) as u8;
                    let b = (prev_pixel[2] as f32 * (1.0 - alpha) + curr_pixel[2] as f32 * alpha) as u8;
                    let a = (prev_pixel[3] as f32 * (1.0 - alpha) + curr_pixel[3] as f32 * alpha) as u8;

                    *pixel = Rgba([r, g, b, a]);
                }

                self.apply_darkening_overlay(&mut blended);
                blended
            }
            (Some(current), _) => {
                // Just current background (no crossfade or crossfade complete)
                let mut bg_copy = current.clone();
                self.apply_darkening_overlay(&mut bg_copy);
                bg_copy
            }
            (None, Some(previous)) if self.crossfade_progress < 1.0 => {
                // Fading from previous to solid color
                let mut blended = RgbaImage::new(self.config.frame_width, self.config.frame_height);
                let alpha = self.crossfade_progress;

                for (x, y, pixel) in blended.enumerate_pixels_mut() {
                    let prev_pixel = previous.get_pixel(x, y);

                    let r = (prev_pixel[0] as f32 * (1.0 - alpha) + self.background_color[0] as f32 * alpha) as u8;
                    let g = (prev_pixel[1] as f32 * (1.0 - alpha) + self.background_color[1] as f32 * alpha) as u8;
                    let b = (prev_pixel[2] as f32 * (1.0 - alpha) + self.background_color[2] as f32 * alpha) as u8;

                    *pixel = Rgba([r, g, b, 255]);
                }

                self.apply_darkening_overlay(&mut blended);
                blended
            }
            _ => {
                // No background images - solid color
                RgbaImage::from_pixel(
                    self.config.frame_width,
                    self.config.frame_height,
                    self.background_color,
                )
            }
        };

        // Get visible words sorted by font size (render small first, large on top)
        let mut words = self.engine.get_visible_words();
        words.sort_by(|a, b| a.font_size.partial_cmp(&b.font_size).unwrap_or(std::cmp::Ordering::Equal));

        for wb in &words {
            // Apply opacity to color
            let mut color = wb.color;
            color[3] = (wb.opacity * 255.0) as u8;

            // Apply scale to font size and dimensions for zoom effect
            let scaled_font_size = wb.font_size * wb.scale;
            let scaled_half_width = wb.half_width * wb.scale;
            let scaled_half_height = wb.half_height * wb.scale;

            // Calculate text position (center the text on position)
            let text_x = (wb.x - scaled_half_width) as i32;
            let text_y = (wb.y - scaled_half_height) as i32;

            draw_text_mut(
                &mut img,
                Rgba(color),
                text_x,
                text_y,
                PxScale::from(scaled_font_size),
                &self.font,
                &wb.word,
            );
        }

        // Draw article info in bottom-left corner (date on top, title below)
        if let Some(article) = &self.current_article {
            let title_font_size = 72.0;
            let date_font_size = 56.0;
            let margin = 40;
            let line_spacing = 12;

            // Format text
            let title_text = if !article.title.is_empty() {
                if article.title.len() > 50 {
                    format!("{}...", &article.title[..47])
                } else {
                    article.title.clone()
                }
            } else {
                String::new()
            };
            let date_text = if !article.date.is_empty() {
                article.date.clone()
            } else {
                String::new()
            };

            if !title_text.is_empty() || !date_text.is_empty() {
                // Use white text when we have a darkened background image, black otherwise
                let text_color = if self.current_background.is_some() {
                    Rgba([255, 255, 255, 255])
                } else {
                    Rgba([0, 0, 0, 255])
                };
                let text_x = margin;

                // Calculate Y position from bottom
                let mut total_height = 0;
                if !title_text.is_empty() {
                    total_height += title_font_size as i32;
                }
                if !date_text.is_empty() {
                    if !title_text.is_empty() {
                        total_height += line_spacing;
                    }
                    total_height += date_font_size as i32;
                }

                let mut text_y = self.config.frame_height as i32 - margin - total_height;

                // Draw date first (on top)
                if !date_text.is_empty() {
                    draw_text_mut(
                        &mut img,
                        text_color,
                        text_x,
                        text_y,
                        PxScale::from(date_font_size),
                        &self.font,
                        &date_text,
                    );
                    text_y += date_font_size as i32 + line_spacing;
                }

                // Draw title below
                if !title_text.is_empty() {
                    draw_text_mut(
                        &mut img,
                        text_color,
                        text_x,
                        text_y,
                        PxScale::from(title_font_size),
                        &self.font,
                        &title_text,
                    );
                }
            }
        }

        // Pipe raw RGBA data to ffmpeg
        if let Some(ref mut process) = self.ffmpeg_process {
            if let Some(ref mut stdin) = process.stdin {
                stdin.write_all(img.as_raw())
                    .context("Failed to write frame to ffmpeg")?;
            }
        }

        Ok(())
    }

    /// Finalize rendering - close stdin and wait for ffmpeg to finish
    pub fn finalize(&mut self) -> Result<()> {
        if self.frame_count == 0 {
            anyhow::bail!("No frames to encode");
        }

        println!("Finalizing video encoding...");

        // Take ownership of the process and close stdin to signal EOF
        if let Some(mut process) = self.ffmpeg_process.take() {
            // Drop stdin to close the pipe and signal EOF to ffmpeg
            drop(process.stdin.take());

            // Wait for ffmpeg to finish
            let status = process.wait()
                .context("Failed to wait for ffmpeg process")?;

            if !status.success() {
                anyhow::bail!("ffmpeg encoding failed with status: {}", status);
            }
        }

        if let Some(ref output_path) = self.output_path {
            println!("Video saved to: {:?}", output_path);
        }

        Ok(())
    }

    /// Get current frame count
    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }
}

fn parse_hex_color(hex: &str) -> Rgba<u8> {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(250);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(249);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(246);
    Rgba([r, g, b, 255])
}
