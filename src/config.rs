use std::path::PathBuf;

/// Layout algorithm for word placement
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LayoutMode {
    /// Spiral placement from center outward
    Spiral,
    /// Physics-based repulsion where words push each other apart
    #[default]
    Amoeba,
}

/// Configuration for TimeCloud
#[derive(Debug, Clone)]
pub struct Config {
    // Engine
    pub max_queue_size: usize,
    pub max_display_words: usize,

    // Tokenizer
    pub lowercase: bool,
    pub filter_stopwords: bool,
    pub enable_stemming: bool,
    pub min_word_length: usize,
    pub stopwords_path: Option<PathBuf>,

    // Video
    pub words_per_frame: usize,
    pub fps: u32,
    pub frame_width: u32,
    pub frame_height: u32,

    // Visual
    pub background_color: String,
    pub color_palette: Vec<String>,
    pub font_path: Option<PathBuf>,
    pub min_font_size: f32,
    pub max_font_size: f32,

    // Physics
    pub physics_steps_per_frame: u32,
    pub relayout_cooldown_secs: f32,

    // Animation
    pub position_lerp_speed: f32,
    pub size_lerp_speed: f32,
    pub scale_speed: f32,
    pub fade_in_speed: f32,
    pub fade_out_speed: f32,

    // Layout
    pub word_padding_x: f32,
    pub word_padding_y: f32,
    /// If true, never reposition words - reserve max space for each word
    pub no_relayout: bool,
    /// Layout algorithm to use
    pub layout_mode: LayoutMode,
    /// Repulsion strength for amoeba layout (higher = words push harder)
    pub repulsion_strength: f32,
    /// Frames between adding each word during ramp-up (0 = no ramp, instant full)
    pub ramp_frames_per_word: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Engine
            max_queue_size: 2500,
            max_display_words: 18,

            // Tokenizer
            lowercase: true,
            filter_stopwords: true,
            enable_stemming: false,
            min_word_length: 2,
            stopwords_path: Some(PathBuf::from("stopwords.txt")),

            // Video
            words_per_frame: 1,
            fps: 60,
            frame_width: 1920,
            frame_height: 1080,

            // Visual
            background_color: "#FAF9F6".to_string(),
            color_palette: vec![
                "#2a9d8f".to_string(), // Teal
                "#9b8ac4".to_string(), // Lavender
                "#e9c46a".to_string(), // Yellow
                "#f4a261".to_string(), // Orange
                "#e76f51".to_string(), // Red-orange
                "#9b2226".to_string(), // Dark red
            ],
            font_path: None,
            min_font_size: 24.0,
            max_font_size: 200.0,

            // Physics
            physics_steps_per_frame: 10,
            relayout_cooldown_secs: 2.0,

            // Animation
            position_lerp_speed: 0.001,
            size_lerp_speed: 0.01,
            scale_speed: 0.02,
            fade_in_speed: 0.02,
            fade_out_speed: 0.02,

            // Layout
            word_padding_x: 8.0,
            word_padding_y: 8.0,
            no_relayout: true,
            layout_mode: LayoutMode::default(),
            repulsion_strength: 150.0,
            ramp_frames_per_word: 0,
        }
    }
}
