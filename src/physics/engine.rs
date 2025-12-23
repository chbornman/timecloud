use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use rand::Rng;
use std::collections::{HashMap, HashSet};

use crate::config::{Config, LayoutMode};

/// A word with position and rendering info
#[derive(Debug, Clone)]
pub struct WordBody {
    pub word: String,
    pub frequency: u32,
    pub x: f32,
    pub y: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub font_size: f32,
    pub target_font_size: f32,
    pub opacity: f32,
    pub scale: f32, // 0.0 to 1.0, multiplied with font_size for zoom effect
    pub color: [u8; 4],
    pub half_width: f32,
    pub half_height: f32,
    pub target_half_width: f32,
    pub target_half_height: f32,
    pub fading_out: bool,
}

/// Simple word cloud engine with spiral placement
pub struct PhysicsEngine {
    pub words: HashMap<String, WordBody>,
    width: f32,
    height: f32,
    center: (f32, f32),
    min_font_size: f32,
    max_font_size: f32,
    pub color_palette: Vec<[u8; 4]>,
    font: FontRef<'static>,
    /// Track previous top words to detect when full relayout is needed
    last_top_words: Vec<String>,
    /// Frame counter for cooldown tracking
    frame_count: u32,
    /// Frame number of last full relayout
    last_relayout_frame: u32,
    /// Minimum frames between full relayouts
    relayout_cooldown: u32,
    /// Position interpolation speed (0-1, higher = faster movement)
    position_lerp_speed: f32,
    /// Size interpolation speed (0-1, higher = faster font size changes)
    size_lerp_speed: f32,
    /// Zoom scale speed (0-1, higher = faster zoom in/out)
    scale_speed: f32,
    /// Fade in speed (0-1, higher = faster appearance)
    fade_in_speed: f32,
    /// Fade out speed (0-1, higher = faster disappearance)
    fade_out_speed: f32,
    /// Horizontal padding between words in pixels
    word_padding_x: f32,
    /// Vertical padding between words in pixels
    word_padding_y: f32,
    /// If true, never reposition words
    no_relayout: bool,
    /// Layout algorithm to use
    layout_mode: LayoutMode,
    /// Repulsion strength for amoeba layout
    repulsion_strength: f32,
}

impl PhysicsEngine {
    pub fn new(config: &Config, font: FontRef<'static>) -> Self {
        let width = config.frame_width as f32;
        let height = config.frame_height as f32;

        let color_palette: Vec<[u8; 4]> = config
            .color_palette
            .iter()
            .map(|c| parse_hex_color(c))
            .collect();

        // Convert cooldown from seconds to frames
        let relayout_cooldown = (config.fps as f32 * config.relayout_cooldown_secs) as u32;

        Self {
            words: HashMap::new(),
            width,
            height,
            center: (width / 2.0, height / 2.0),
            min_font_size: config.min_font_size,
            max_font_size: config.max_font_size,
            color_palette,
            font,
            last_top_words: Vec::new(),
            frame_count: 0,
            last_relayout_frame: 0,
            relayout_cooldown,
            position_lerp_speed: config.position_lerp_speed,
            size_lerp_speed: config.size_lerp_speed,
            scale_speed: config.scale_speed,
            fade_in_speed: config.fade_in_speed,
            fade_out_speed: config.fade_out_speed,
            word_padding_x: config.word_padding_x,
            word_padding_y: config.word_padding_y,
            no_relayout: config.no_relayout,
            layout_mode: config.layout_mode,
            repulsion_strength: config.repulsion_strength,
        }
    }

    /// Measure text dimensions using font metrics
    fn measure_text(&self, word: &str, font_size: f32) -> (f32, f32) {
        let scale = PxScale::from(font_size);
        let scaled_font = self.font.as_scaled(scale);

        let mut text_width = 0.0f32;
        for c in word.chars() {
            let glyph_id = self.font.glyph_id(c);
            text_width += scaled_font.h_advance(glyph_id);
        }

        let text_height = scaled_font.ascent() - scaled_font.descent();

        (text_width / 2.0, text_height / 2.0)
    }

    fn frequency_to_font_size(&self, frequency: u32, max_freq: u32) -> f32 {
        let scale = self.width.min(self.height) / 1080.0;
        let min_size = self.min_font_size * scale;
        let max_size = self.max_font_size * scale;

        if max_freq == 0 {
            return min_size;
        }

        // Use a minimum threshold for max_freq to prevent early words from being huge
        // When max_freq is low (early in processing), words should be smaller
        let effective_max_freq = max_freq.max(15);
        let ratio = frequency as f32 / effective_max_freq as f32;
        min_size + ratio.min(1.0) * (max_size - min_size)
    }

    fn get_word_color(&self) -> [u8; 4] {
        let mut rng = rand::thread_rng();
        self.color_palette[rng.gen_range(0..self.color_palette.len())]
    }

    /// Check if two rectangles overlap (with padding for better spacing)
    fn overlaps(
        &self,
        x1: f32,
        y1: f32,
        hw1: f32,
        hh1: f32,
        x2: f32,
        y2: f32,
        hw2: f32,
        hh2: f32,
    ) -> bool {
        let dx = (x1 - x2).abs();
        let dy = (y1 - y2).abs();
        dx < (hw1 + hw2 + self.word_padding_x) && dy < (hh1 + hh2 + self.word_padding_y)
    }

    /// Find position using Archimedean spiral, checking against a list of placed rectangles
    /// Uses elliptical spiral (wider horizontally) for a more natural word cloud shape
    fn find_spiral_position_against(
        &self,
        placed: &[(f32, f32, f32, f32)],
        hw: f32,
        hh: f32,
    ) -> Option<(f32, f32)> {
        let mut angle = 0.0f32;
        let angle_step = 0.3; // radians - smaller steps for finer search
        let radius_step = 2.0; // pixels per radian - smaller for denser search
        let horizontal_stretch = 1.6; // Stretch factor to create horizontal ellipse

        // Check collision against placed words only
        let collides = |x: f32, y: f32| -> bool {
            for &(px, py, phw, phh) in placed {
                if self.overlaps(x, y, hw, hh, px, py, phw, phh) {
                    return true;
                }
            }
            false
        };

        // Try center first
        if !collides(self.center.0, self.center.1) {
            return Some(self.center);
        }

        // Spiral outward with horizontal ellipse bias
        // More iterations to cover larger area with max-size reservations
        for _ in 0..5000 {
            angle += angle_step;
            let radius = radius_step * angle;
            let x = self.center.0 + radius * angle.cos() * horizontal_stretch;
            let y = self.center.1 + radius * angle.sin();

            // Check bounds
            if x - hw < 0.0 || x + hw > self.width || y - hh < 0.0 || y + hh > self.height {
                continue;
            }

            if !collides(x, y) {
                return Some((x, y));
            }
        }

        // No valid position found
        None
    }

    /// Check if layout needs full recalculation
    /// Returns true if word set changed significantly (new words entered top positions)
    /// and cooldown has elapsed
    fn needs_full_relayout(&self, top_words: &[(String, u32)]) -> bool {
        // If no_relayout is enabled, never do full relayout (except first time)
        if self.no_relayout && !self.last_top_words.is_empty() {
            return false;
        }

        let current: Vec<_> = top_words.iter().map(|(w, _)| w.clone()).collect();

        // If this is the first update, do full layout
        if self.last_top_words.is_empty() {
            return true;
        }

        // Check cooldown - don't relayout too frequently
        let frames_since_relayout = self.frame_count.saturating_sub(self.last_relayout_frame);
        if frames_since_relayout < self.relayout_cooldown {
            return false;
        }

        // Check if any new words appeared in top half of the list
        // (these would need good positions near center)
        let top_half = current.len() / 2;
        for word in current.iter().take(top_half.max(5)) {
            if !self.words.contains_key(word) {
                return true;
            }
        }

        // Check if top 3 words changed order significantly
        let old_top3: Vec<_> = self.last_top_words.iter().take(3).collect();
        let new_top3: Vec<_> = current.iter().take(3).collect();
        if old_top3 != new_top3 {
            // Top words reshuffled - need relayout to put biggest in center
            return true;
        }

        false
    }

    /// Update with new word frequencies
    /// Uses stable positioning when possible, full relayout only when needed
    pub fn update_frequencies(&mut self, top_words: &[(String, u32)]) {
        if top_words.is_empty() {
            for wb in self.words.values_mut() {
                wb.fading_out = true;
            }
            return;
        }

        let max_freq = top_words[0].1;
        let new_words: HashSet<_> = top_words.iter().map(|(w, _)| w.clone()).collect();

        // Mark removed words as fading out
        for (word, wb) in &mut self.words {
            if !new_words.contains(word) {
                wb.fading_out = true;
            }
        }

        let needs_relayout = self.needs_full_relayout(top_words);

        if needs_relayout {
            // Full relayout - place words in frequency order (largest first)
            self.do_full_relayout(top_words, max_freq);
            self.last_relayout_frame = self.frame_count;
        } else {
            // Stable update - keep existing positions, only place new words
            self.do_stable_update(top_words, max_freq);
        }

        self.frame_count += 1;

        // Update last_top_words for next comparison
        self.last_top_words = top_words.iter().map(|(w, _)| w.clone()).collect();
    }

    /// Full relayout - rebuild all positions from scratch
    fn do_full_relayout(&mut self, top_words: &[(String, u32)], max_freq: u32) {
        let max_font_size = self.frequency_to_font_size(max_freq, max_freq);
        let mut placed: Vec<(f32, f32, f32, f32)> = Vec::new();

        for (word, freq) in top_words {
            let font_size = self.frequency_to_font_size(*freq, max_freq);
            let (hw, hh) = self.measure_text(word, font_size);

            // Determine position based on layout mode
            let (x, y) = if self.layout_mode == LayoutMode::Amoeba {
                // Amoeba mode: start at random position biased toward center
                let mut rng = rand::thread_rng();
                // Use normal-ish distribution by averaging two random values (central limit theorem)
                let rand_x = (rng.gen_range(0.0..self.width) + rng.gen_range(0.0..self.width)) / 2.0;
                let rand_y = (rng.gen_range(0.0..self.height) + rng.gen_range(0.0..self.height)) / 2.0;
                (rand_x, rand_y)
            } else {
                // Spiral mode: find position using spiral algorithm
                // When no_relayout, place using max possible size to reserve growth space
                let (place_hw, place_hh) = if self.no_relayout {
                    self.measure_text(word, max_font_size)
                } else {
                    (hw, hh)
                };

                // Skip word if no valid position found (screen is full)
                let Some(pos) = self.find_spiral_position_against(&placed, place_hw, place_hh) else {
                    continue;
                };
                placed.push((pos.0, pos.1, place_hw, place_hh));
                pos
            };

            let color = self.get_word_color();

            if let Some(wb) = self.words.get_mut(word) {
                wb.frequency = *freq;
                wb.target_font_size = font_size;
                wb.target_half_width = hw;
                wb.target_half_height = hh;
                wb.fading_out = false;
                // In amoeba mode, don't force position update for existing words
                if self.layout_mode != LayoutMode::Amoeba {
                    wb.target_x = x;
                    wb.target_y = y;
                }
            } else {
                self.words.insert(
                    word.clone(),
                    WordBody {
                        word: word.clone(),
                        frequency: *freq,
                        x,
                        y,
                        target_x: x,
                        target_y: y,
                        font_size,
                        target_font_size: font_size,
                        opacity: 0.0,
                        scale: 0.3, // Start small for zoom-in effect
                        color,
                        half_width: hw,
                        half_height: hh,
                        target_half_width: hw,
                        target_half_height: hh,
                        fading_out: false,
                    },
                );
            }
        }
    }

    /// Stable update - existing words keep positions, new words find gaps
    fn do_stable_update(&mut self, top_words: &[(String, u32)], max_freq: u32) {
        // First pass: update existing words (keep their positions, just update size)
        for (word, freq) in top_words {
            let font_size = self.frequency_to_font_size(*freq, max_freq);
            let (hw, hh) = self.measure_text(word, font_size);

            if let Some(wb) = self.words.get_mut(word) {
                wb.frequency = *freq;
                wb.target_font_size = font_size;
                wb.target_half_width = hw;
                wb.target_half_height = hh;
                wb.fading_out = false;
                // Keep existing target position - don't move!
            }
        }

        // Collect existing word positions for collision detection
        // When no_relayout is enabled, use max possible size (as if word had max frequency)
        // to reserve space for potential growth
        // Include fading-out words too since they're still visible
        let max_font_size = self.frequency_to_font_size(max_freq, max_freq);
        let mut placed: Vec<(f32, f32, f32, f32)> = self
            .words
            .values()
            .filter(|wb| wb.opacity > 0.1) // Include any visible word
            .map(|wb| {
                if self.no_relayout {
                    // Reserve space for max possible size
                    let (max_hw, max_hh) = self.measure_text(&wb.word, max_font_size);
                    (wb.target_x, wb.target_y, max_hw, max_hh)
                } else {
                    (
                        wb.target_x,
                        wb.target_y,
                        wb.half_width.max(wb.target_half_width),
                        wb.half_height.max(wb.target_half_height),
                    )
                }
            })
            .collect();

        // Second pass: place only new words
        for (word, freq) in top_words {
            if self.words.contains_key(word) {
                continue; // Already handled above
            }

            let font_size = self.frequency_to_font_size(*freq, max_freq);
            let (hw, hh) = self.measure_text(word, font_size);

            // Determine position based on layout mode
            let (x, y) = if self.layout_mode == LayoutMode::Amoeba {
                // Amoeba mode: start at random position biased toward center
                let mut rng = rand::thread_rng();
                // Use normal-ish distribution by averaging two random values (central limit theorem)
                let rand_x = (rng.gen_range(0.0..self.width) + rng.gen_range(0.0..self.width)) / 2.0;
                let rand_y = (rng.gen_range(0.0..self.height) + rng.gen_range(0.0..self.height)) / 2.0;
                (rand_x, rand_y)
            } else {
                // Spiral mode
                // When no_relayout, place using max possible size to reserve growth space
                let (place_hw, place_hh) = if self.no_relayout {
                    self.measure_text(word, max_font_size)
                } else {
                    (hw, hh)
                };

                // Skip word if no valid position found (screen is full)
                let Some(pos) = self.find_spiral_position_against(&placed, place_hw, place_hh) else {
                    continue;
                };
                placed.push((pos.0, pos.1, place_hw, place_hh));
                pos
            };

            let color = self.get_word_color();

            self.words.insert(
                word.clone(),
                WordBody {
                    word: word.clone(),
                    frequency: *freq,
                    x,
                    y,
                    target_x: x,
                    target_y: y,
                    font_size,
                    target_font_size: font_size,
                    opacity: 0.0,
                    scale: 0.3, // Start small for zoom-in effect
                    color,
                    half_width: hw,
                    half_height: hh,
                    target_half_width: hw,
                    target_half_height: hh,
                    fading_out: false,
                },
            );
        }
    }

    /// Apply amoeba physics - words repel each other and are attracted to center
    fn apply_amoeba_physics(&mut self) {
        // Collect word data to avoid borrow issues
        let word_data: Vec<(String, f32, f32, f32, f32, bool)> = self
            .words
            .iter()
            .map(|(k, w)| {
                let mass = (w.target_half_width * w.target_half_height).sqrt();
                (k.clone(), w.target_x, w.target_y, mass, w.opacity, w.fading_out)
            })
            .collect();

        // Calculate forces for each word
        let mut forces: HashMap<String, (f32, f32)> = HashMap::new();

        for (i, (word_i, x1, y1, mass1, opacity1, fading1)) in word_data.iter().enumerate() {
            // Skip fading out words
            if *fading1 || *opacity1 < 0.1 {
                continue;
            }

            let mut force_x = 0.0f32;
            let mut force_y = 0.0f32;

            // Repulsion from other words
            for (j, (_, x2, y2, mass2, opacity2, fading2)) in word_data.iter().enumerate() {
                if i == j || *fading2 || *opacity2 < 0.1 {
                    continue;
                }

                let dx = x1 - x2;
                let dy = y1 - y2;
                let dist_sq = (dx * dx + dy * dy).max(100.0); // Min distance to prevent explosion
                let dist = dist_sq.sqrt();

                // Repulsion force - inverse square law
                let force_magnitude = self.repulsion_strength * mass1 * mass2 / dist_sq;
                force_x += force_magnitude * dx / dist;
                force_y += force_magnitude * dy / dist;
            }

            // Weak center attraction to keep words from flying off
            let dx_center = self.center.0 - x1;
            let dy_center = self.center.1 - y1;
            let _center_dist = (dx_center * dx_center + dy_center * dy_center).sqrt().max(1.0);
            let center_strength = 0.0005 * mass1;
            force_x += dx_center * center_strength;
            force_y += dy_center * center_strength;

            // Boundary repulsion - push away from edges
            let margin = 50.0;
            let boundary_strength = self.repulsion_strength * 2.0;

            // Left edge
            if *x1 < margin {
                force_x += boundary_strength * (margin - x1) / margin;
            }
            // Right edge
            if *x1 > self.width - margin {
                force_x -= boundary_strength * (x1 - (self.width - margin)) / margin;
            }
            // Top edge
            if *y1 < margin {
                force_y += boundary_strength * (margin - y1) / margin;
            }
            // Bottom edge
            if *y1 > self.height - margin {
                force_y -= boundary_strength * (y1 - (self.height - margin)) / margin;
            }

            // Bottom-left reserved zone (for date/title overlay)
            // Push words up and right if they enter this zone
            let reserved_width = 600.0;
            let reserved_height = 150.0;
            let reserved_strength = boundary_strength * 5.0;
            if *x1 < reserved_width && *y1 > self.height - reserved_height {
                let dist_from_right = reserved_width - x1;
                let dist_from_top = y1 - (self.height - reserved_height);
                // Push both up and right
                force_x += reserved_strength * dist_from_right / reserved_width;
                force_y -= reserved_strength * dist_from_top / reserved_height;
            }

            forces.insert(word_i.clone(), (force_x, force_y));
        }

        // Apply forces to target positions
        let force_scale = 0.01; // Damping factor
        for (word, (fx, fy)) in forces {
            if let Some(wb) = self.words.get_mut(&word) {
                wb.target_x += fx * force_scale;
                wb.target_y += fy * force_scale;

                // Clamp to bounds (ensure min <= max to avoid panic)
                let min_x = wb.target_half_width + self.word_padding_x;
                let max_x = self.width - wb.target_half_width - self.word_padding_x;
                if min_x < max_x {
                    wb.target_x = wb.target_x.clamp(min_x, max_x);
                } else {
                    wb.target_x = self.center.0; // Center if word is too wide
                }

                let min_y = wb.target_half_height + self.word_padding_y;
                let max_y = self.height - wb.target_half_height - self.word_padding_y;
                if min_y < max_y {
                    wb.target_y = wb.target_y.clamp(min_y, max_y);
                } else {
                    wb.target_y = self.center.1; // Center if word is too tall
                }
            }
        }
    }

    /// Animate one step - interpolate positions, fade in/out
    pub fn step(&mut self, _dt: f32) {
        // Apply amoeba physics if in amoeba mode
        if self.layout_mode == LayoutMode::Amoeba {
            self.apply_amoeba_physics();
        }

        let mut to_remove = Vec::new();

        for (word, wb) in &mut self.words {
            // Interpolate position
            wb.x += (wb.target_x - wb.x) * self.position_lerp_speed;
            wb.y += (wb.target_y - wb.y) * self.position_lerp_speed;

            // Interpolate font size and dimensions together for smooth scaling
            wb.font_size += (wb.target_font_size - wb.font_size) * self.size_lerp_speed;
            wb.half_width += (wb.target_half_width - wb.half_width) * self.size_lerp_speed;
            wb.half_height += (wb.target_half_height - wb.half_height) * self.size_lerp_speed;

            // Zoom and fade in/out
            if wb.fading_out {
                // Zoom out and fade out (lerp toward 0)
                wb.scale += (0.0 - wb.scale) * self.scale_speed;
                wb.opacity += (0.0 - wb.opacity) * self.fade_out_speed;
                if wb.opacity <= 0.01 || wb.scale <= 0.01 {
                    to_remove.push(word.clone());
                }
            } else {
                // Zoom in and fade in (lerp toward 1)
                wb.scale += (1.0 - wb.scale) * self.scale_speed;
                wb.opacity += (1.0 - wb.opacity) * self.fade_in_speed;
            }
        }

        for word in to_remove {
            self.words.remove(&word);
        }
    }

    /// Get visible words for rendering
    pub fn get_visible_words(&self) -> Vec<&WordBody> {
        self.words.values().filter(|wb| wb.opacity > 0.0).collect()
    }
}

fn parse_hex_color(hex: &str) -> [u8; 4] {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    [r, g, b, 255]
}
