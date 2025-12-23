use anyhow::Result;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::path::PathBuf;

use timecloud::{ArticleInfo, Config, LayoutMode, PhysicsRenderer, TimeCloud, Tokenizer};

// Default values as constants for comparison
const DEFAULT_OUTPUT: &str = "output.mp4";
const DEFAULT_WINDOW_SIZE: usize = 2500;
const DEFAULT_MAX_WORDS: usize = 18;
const DEFAULT_WORDS_PER_FRAME: usize = 1;
const DEFAULT_FPS: u32 = 60;
const DEFAULT_WIDTH: u32 = 1920;
const DEFAULT_HEIGHT: u32 = 1080;
const DEFAULT_MIN_WORD_LENGTH: usize = 2;
const DEFAULT_BACKGROUND: &str = "#FAF9F6";
const DEFAULT_MIN_FONT_SIZE: f32 = 24.0;
const DEFAULT_MAX_FONT_SIZE: f32 = 200.0;
const DEFAULT_PHYSICS_STEPS: u32 = 10;
const DEFAULT_PRELOAD_WORDS: usize = 1500;
const DEFAULT_WARMUP_FRAMES: u32 = 0;
const DEFAULT_RELAYOUT_COOLDOWN: f32 = 2.0;
const DEFAULT_POSITION_LERP_SPEED: f32 = 0.001;
const DEFAULT_SIZE_LERP_SPEED: f32 = 0.01;
const DEFAULT_SCALE_SPEED: f32 = 0.02;
const DEFAULT_FADE_IN_SPEED: f32 = 0.02;
const DEFAULT_FADE_OUT_SPEED: f32 = 0.02;
const DEFAULT_WORD_PADDING_X: f32 = 8.0;
const DEFAULT_WORD_PADDING_Y: f32 = 8.0;
const DEFAULT_REPULSION_STRENGTH: f32 = 150.0;
const DEFAULT_LAYOUT: &str = "amoeba";
const DEFAULT_RAMP_SECONDS: f32 = 0.75;
const DEFAULT_COLOR_PALETTE: &[&str] = &[
    "#2a9d8f", "#9b8ac4", "#e9c46a", "#f4a261", "#e76f51", "#9b2226",
];

#[derive(Parser)]
#[command(name = "timecloud")]
#[command(about = "Animated wordcloud generator with physics-based layout")]
#[command(version)]
struct Cli {
    /// Input directory containing text files
    #[arg(short, long)]
    input: PathBuf,

    /// Output video file
    #[arg(short, long, default_value = DEFAULT_OUTPUT)]
    output: PathBuf,

    // Engine options
    /// Sliding window size (number of words to track)
    #[arg(long, default_value_t = DEFAULT_WINDOW_SIZE)]
    window_size: usize,

    /// Maximum number of words to display
    #[arg(long, default_value_t = DEFAULT_MAX_WORDS)]
    max_words: usize,

    /// Words to process per frame
    #[arg(long, default_value_t = DEFAULT_WORDS_PER_FRAME)]
    words_per_frame: usize,

    // Video options
    /// Frames per second
    #[arg(long, default_value_t = DEFAULT_FPS)]
    fps: u32,

    /// Frame width in pixels
    #[arg(long, default_value_t = DEFAULT_WIDTH)]
    width: u32,

    /// Frame height in pixels
    #[arg(long, default_value_t = DEFAULT_HEIGHT)]
    height: u32,

    // Tokenizer options
    /// Disable lowercase conversion
    #[arg(long)]
    no_lowercase: bool,

    /// Disable stopword filtering
    #[arg(long)]
    no_filter_stopwords: bool,

    /// Enable Porter stemming
    #[arg(long)]
    enable_stemming: bool,

    /// Minimum word length
    #[arg(long, default_value_t = DEFAULT_MIN_WORD_LENGTH)]
    min_word_length: usize,

    /// Path to stopwords file
    #[arg(long, default_value = "stopwords.txt")]
    stopwords: PathBuf,

    // Visual options
    /// Path to TrueType font file
    #[arg(long)]
    font: Option<PathBuf>,

    /// Background color (hex)
    #[arg(long, default_value = DEFAULT_BACKGROUND)]
    background: String,

    /// Minimum font size
    #[arg(long, default_value_t = DEFAULT_MIN_FONT_SIZE)]
    min_font_size: f32,

    /// Maximum font size
    #[arg(long, default_value_t = DEFAULT_MAX_FONT_SIZE)]
    max_font_size: f32,

    /// Physics simulation steps per frame
    #[arg(long, default_value_t = DEFAULT_PHYSICS_STEPS)]
    physics_steps: u32,

    /// Number of words to preload before rendering (warm start)
    #[arg(long, default_value_t = DEFAULT_PRELOAD_WORDS)]
    preload: usize,

    /// Number of warmup frames to render after preloading (for animation)
    #[arg(long, default_value_t = DEFAULT_WARMUP_FRAMES)]
    warmup_frames: u32,

    /// Minimum seconds between full relayouts (prevents rapid repositioning)
    #[arg(long, default_value_t = DEFAULT_RELAYOUT_COOLDOWN)]
    relayout_cooldown: f32,

    // Animation options
    /// Position interpolation speed (0-1, higher = faster movement toward target)
    #[arg(long, default_value_t = DEFAULT_POSITION_LERP_SPEED)]
    position_lerp_speed: f32,

    /// Size interpolation speed (0-1, higher = faster font size changes)
    #[arg(long, default_value_t = DEFAULT_SIZE_LERP_SPEED)]
    size_lerp_speed: f32,

    /// Zoom scale speed (0-1, higher = faster zoom in/out per frame)
    #[arg(long, default_value_t = DEFAULT_SCALE_SPEED)]
    scale_speed: f32,

    /// Fade in speed (0-1, higher = faster appearance, reaches full opacity quicker)
    #[arg(long, default_value_t = DEFAULT_FADE_IN_SPEED)]
    fade_in_speed: f32,

    /// Fade out speed (0-1, higher = faster disappearance)
    #[arg(long, default_value_t = DEFAULT_FADE_OUT_SPEED)]
    fade_out_speed: f32,

    // Layout options
    /// Horizontal padding between words in pixels (higher = more spacing)
    #[arg(long, default_value_t = DEFAULT_WORD_PADDING_X)]
    word_padding_x: f32,

    /// Vertical padding between words in pixels (higher = more spacing)
    #[arg(long, default_value_t = DEFAULT_WORD_PADDING_Y)]
    word_padding_y: f32,

    /// Disable repositioning - words stay where first placed, space reserved for growth
    #[arg(long, default_value_t = true)]
    no_relayout: bool,

    /// Layout algorithm: "spiral" or "amoeba" (physics-based repulsion)
    #[arg(long, default_value = DEFAULT_LAYOUT)]
    layout: String,

    /// Repulsion strength for amoeba layout (higher = words push harder)
    #[arg(long, default_value_t = DEFAULT_REPULSION_STRENGTH)]
    repulsion_strength: f32,

    /// Seconds between adding each word during ramp-up (0 = use queue fill ratio)
    #[arg(long, default_value_t = DEFAULT_RAMP_SECONDS)]
    ramp_seconds: f32,
}

/// Format a value with default/custom indicator
fn fmt_val<T: std::fmt::Display + PartialEq>(value: T, default: T) -> String {
    if value == default {
        format!("{}  ", value)
    } else {
        format!("{} *", value)
    }
}

fn fmt_bool(value: bool, default: bool, true_str: &str, false_str: &str) -> String {
    let display = if value { true_str } else { false_str };
    if value == default {
        format!("{}  ", display)
    } else {
        format!("{} *", display)
    }
}

fn fmt_option<T: std::fmt::Debug>(value: &Option<T>) -> String {
    match value {
        Some(v) => format!("{:?} *", v),
        None => "none  ".to_string(),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Parse layout mode
    let layout_mode = match cli.layout.to_lowercase().as_str() {
        "amoeba" => LayoutMode::Amoeba,
        _ => LayoutMode::Spiral,
    };

    // Build config from CLI args
    let config = Config {
        max_queue_size: cli.window_size,
        max_display_words: cli.max_words,
        lowercase: !cli.no_lowercase,
        filter_stopwords: !cli.no_filter_stopwords,
        enable_stemming: cli.enable_stemming,
        min_word_length: cli.min_word_length,
        stopwords_path: Some(cli.stopwords.clone()),
        words_per_frame: cli.words_per_frame,
        fps: cli.fps,
        frame_width: cli.width,
        frame_height: cli.height,
        background_color: cli.background.clone(),
        font_path: cli.font.clone(),
        min_font_size: cli.min_font_size,
        max_font_size: cli.max_font_size,
        physics_steps_per_frame: cli.physics_steps,
        relayout_cooldown_secs: cli.relayout_cooldown,
        position_lerp_speed: cli.position_lerp_speed,
        size_lerp_speed: cli.size_lerp_speed,
        scale_speed: cli.scale_speed,
        fade_in_speed: cli.fade_in_speed,
        fade_out_speed: cli.fade_out_speed,
        word_padding_x: cli.word_padding_x,
        word_padding_y: cli.word_padding_y,
        no_relayout: cli.no_relayout,
        layout_mode,
        repulsion_strength: cli.repulsion_strength,
        ..Default::default()
    };

    println!("TimeCloud - Physics-based wordcloud generator");
    println!("==============================================");
    println!();

    println!("Input/Output:");
    println!("  input:              {:?}", cli.input);
    println!("  output:             {} ", fmt_val(cli.output.display().to_string(), DEFAULT_OUTPUT.to_string()));
    println!();

    println!("Engine:");
    println!("  window_size:        {}", fmt_val(cli.window_size, DEFAULT_WINDOW_SIZE));
    println!("  max_words:          {}", fmt_val(cli.max_words, DEFAULT_MAX_WORDS));
    println!("  words_per_frame:    {}", fmt_val(cli.words_per_frame, DEFAULT_WORDS_PER_FRAME));
    println!();

    println!("Video:");
    println!("  fps:                {}", fmt_val(cli.fps, DEFAULT_FPS));
    println!("  width:              {}", fmt_val(cli.width, DEFAULT_WIDTH));
    println!("  height:             {}", fmt_val(cli.height, DEFAULT_HEIGHT));
    println!();

    println!("Tokenizer:");
    println!("  lowercase:          {}", fmt_bool(!cli.no_lowercase, true, "yes", "no"));
    println!("  filter_stopwords:   {}", fmt_bool(!cli.no_filter_stopwords, true, "yes", "no"));
    println!("  enable_stemming:    {}", fmt_bool(cli.enable_stemming, false, "yes", "no"));
    println!("  min_word_length:    {}", fmt_val(cli.min_word_length, DEFAULT_MIN_WORD_LENGTH));
    println!("  stopwords_file:     {}", cli.stopwords.display());
    println!();

    println!("Visual:");
    println!("  font:               {}", fmt_option(&cli.font));
    println!("  background:         {}", fmt_val(cli.background.as_str(), DEFAULT_BACKGROUND));
    println!("  min_font_size:      {}", fmt_val(cli.min_font_size, DEFAULT_MIN_FONT_SIZE));
    println!("  max_font_size:      {}", fmt_val(cli.max_font_size, DEFAULT_MAX_FONT_SIZE));
    println!();

    println!("Physics:");
    println!("  physics_steps:      {}", fmt_val(cli.physics_steps, DEFAULT_PHYSICS_STEPS));
    println!("  preload:            {}", fmt_val(cli.preload, DEFAULT_PRELOAD_WORDS));
    println!("  warmup_frames:      {}", fmt_val(cli.warmup_frames, DEFAULT_WARMUP_FRAMES));
    println!("  relayout_cooldown:  {}s", fmt_val(cli.relayout_cooldown, DEFAULT_RELAYOUT_COOLDOWN));
    println!();

    println!("Animation:");
    println!("  position_lerp:      {}", fmt_val(cli.position_lerp_speed, DEFAULT_POSITION_LERP_SPEED));
    println!("  size_lerp:          {}", fmt_val(cli.size_lerp_speed, DEFAULT_SIZE_LERP_SPEED));
    println!("  scale_speed:        {}", fmt_val(cli.scale_speed, DEFAULT_SCALE_SPEED));
    println!("  fade_in_speed:      {}", fmt_val(cli.fade_in_speed, DEFAULT_FADE_IN_SPEED));
    println!("  fade_out_speed:     {}", fmt_val(cli.fade_out_speed, DEFAULT_FADE_OUT_SPEED));
    println!();

    println!("Layout:");
    println!("  layout_mode:        {}", fmt_val(cli.layout.as_str(), DEFAULT_LAYOUT));
    println!("  word_padding_x:     {}", fmt_val(cli.word_padding_x, DEFAULT_WORD_PADDING_X));
    println!("  word_padding_y:     {}", fmt_val(cli.word_padding_y, DEFAULT_WORD_PADDING_Y));
    println!("  no_relayout:        {}", fmt_bool(cli.no_relayout, true, "yes", "no"));
    if layout_mode == LayoutMode::Amoeba {
        println!("  repulsion_strength: {}", fmt_val(cli.repulsion_strength, DEFAULT_REPULSION_STRENGTH));
    }
    println!();

    println!("Colors:");
    print!("  palette:            ");
    for (i, color) in DEFAULT_COLOR_PALETTE.iter().enumerate() {
        if i > 0 {
            print!(", ");
        }
        print!("{}", color);
    }
    println!("  ");
    println!();
    println!("  (* = custom value)");
    println!();

    println!("==============================================");
    println!();

    // Create tokenizer
    println!("Initializing tokenizer...");
    let tokenizer = Tokenizer::new(&config)?;

    // Read and tokenize input files
    println!("Tokenizing input files...");
    let words = tokenizer.tokenize_files_with_metadata(&cli.input)?;
    let words_len = words.len();
    println!("Total words: {}", words_len);

    if words.is_empty() {
        anyhow::bail!("No words found in input files");
    }

    // Calculate expected frame count
    let expected_frames = words.len() / config.words_per_frame;
    println!("Expected frames: ~{}", expected_frames);
    println!();

    // Create engine and renderer
    let mut engine = TimeCloud::new(config.max_queue_size, config.max_display_words);

    // Set up ramp if requested (convert seconds to frames)
    if cli.ramp_seconds > 0.0 {
        let ramp_frames = (cli.ramp_seconds * cli.fps as f32) as u32;
        engine.set_ramp_frames_per_word(ramp_frames);
    }

    let mut renderer = PhysicsRenderer::new(config.clone())?;

    // Preload words if requested (warm start)
    let mut words_iter = words.into_iter().peekable();
    let preload_count = cli.preload.min(words_len);

    if preload_count > 0 {
        println!("Preloading {} words...", preload_count);
        let mut current_article = (String::new(), String::new(), None::<String>);

        for _ in 0..preload_count {
            if let Some(tw) = words_iter.next() {
                // Update article info when it changes
                if (tw.article_date.as_str(), tw.article_title.as_str()) != (current_article.0.as_str(), current_article.1.as_str()) {
                    current_article = (tw.article_date.clone(), tw.article_title.clone(), tw.article_image.clone());
                    engine.set_article(ArticleInfo {
                        date: tw.article_date.clone(),
                        title: tw.article_title.clone(),
                        image_path: tw.article_image.clone(),
                    });
                }
                engine.add_word(tw.word);
            }
        }

        // Render warmup frames to let preloaded words animate in
        if cli.warmup_frames > 0 {
            println!("Rendering {} warmup frames...", cli.warmup_frames);
            for _ in 0..cli.warmup_frames {
                engine.advance_ramp();
                let state = engine.get_state();
                renderer.render_state(&state, &cli.output)?;
            }
        }
    }

    // Process remaining words with progress bar
    let remaining_words = words_len - preload_count;
    println!("Rendering frames...");
    let pb = ProgressBar::new(remaining_words as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} words ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut word_count = 0;
    let batch_size = config.words_per_frame;
    let mut current_article = (String::new(), String::new(), None::<String>);

    for tw in words_iter {
        // Update article info when it changes
        if (tw.article_date.as_str(), tw.article_title.as_str()) != (current_article.0.as_str(), current_article.1.as_str()) {
            current_article = (tw.article_date.clone(), tw.article_title.clone(), tw.article_image.clone());
            engine.set_article(ArticleInfo {
                date: tw.article_date.clone(),
                title: tw.article_title.clone(),
                image_path: tw.article_image.clone(),
            });
        }

        engine.add_word(tw.word);
        word_count += 1;

        if word_count >= batch_size {
            engine.advance_ramp();
            let state = engine.get_state();
            renderer.render_state(&state, &cli.output)?;
            word_count = 0;
        }

        pb.inc(1);
    }

    // Render final state if there are remaining words
    if word_count > 0 {
        let state = engine.get_state();
        renderer.render_state(&state, &cli.output)?;
    }

    pb.finish_with_message("Frames complete");
    println!();

    // Finalize video encoding
    renderer.finalize()?;

    // Write params file
    let params_path = cli.output.with_extension("txt");
    let mut params_file = std::fs::File::create(&params_path)?;

    writeln!(params_file, "TimeCloud Parameters")?;
    writeln!(params_file, "====================")?;
    writeln!(params_file)?;
    writeln!(params_file, "Input/Output:")?;
    writeln!(params_file, "  input:              {:?}", cli.input)?;
    writeln!(params_file, "  output:             {}", fmt_val(cli.output.display().to_string(), DEFAULT_OUTPUT.to_string()))?;
    writeln!(params_file)?;
    writeln!(params_file, "Engine:")?;
    writeln!(params_file, "  window_size:        {}", fmt_val(cli.window_size, DEFAULT_WINDOW_SIZE))?;
    writeln!(params_file, "  max_words:          {}", fmt_val(cli.max_words, DEFAULT_MAX_WORDS))?;
    writeln!(params_file, "  words_per_frame:    {}", fmt_val(cli.words_per_frame, DEFAULT_WORDS_PER_FRAME))?;
    writeln!(params_file)?;
    writeln!(params_file, "Video:")?;
    writeln!(params_file, "  fps:                {}", fmt_val(cli.fps, DEFAULT_FPS))?;
    writeln!(params_file, "  width:              {}", fmt_val(cli.width, DEFAULT_WIDTH))?;
    writeln!(params_file, "  height:             {}", fmt_val(cli.height, DEFAULT_HEIGHT))?;
    writeln!(params_file)?;
    writeln!(params_file, "Tokenizer:")?;
    writeln!(params_file, "  lowercase:          {}", fmt_bool(!cli.no_lowercase, true, "yes", "no"))?;
    writeln!(params_file, "  filter_stopwords:   {}", fmt_bool(!cli.no_filter_stopwords, true, "yes", "no"))?;
    writeln!(params_file, "  enable_stemming:    {}", fmt_bool(cli.enable_stemming, false, "yes", "no"))?;
    writeln!(params_file, "  min_word_length:    {}", fmt_val(cli.min_word_length, DEFAULT_MIN_WORD_LENGTH))?;
    writeln!(params_file, "  stopwords_file:     {}", cli.stopwords.display())?;
    writeln!(params_file)?;
    writeln!(params_file, "Visual:")?;
    writeln!(params_file, "  font:               {}", fmt_option(&cli.font))?;
    writeln!(params_file, "  background:         {}", fmt_val(cli.background.as_str(), DEFAULT_BACKGROUND))?;
    writeln!(params_file, "  min_font_size:      {}", fmt_val(cli.min_font_size, DEFAULT_MIN_FONT_SIZE))?;
    writeln!(params_file, "  max_font_size:      {}", fmt_val(cli.max_font_size, DEFAULT_MAX_FONT_SIZE))?;
    writeln!(params_file)?;
    writeln!(params_file, "Physics:")?;
    writeln!(params_file, "  physics_steps:      {}", fmt_val(cli.physics_steps, DEFAULT_PHYSICS_STEPS))?;
    writeln!(params_file, "  preload:            {}", fmt_val(cli.preload, DEFAULT_PRELOAD_WORDS))?;
    writeln!(params_file, "  warmup_frames:      {}", fmt_val(cli.warmup_frames, DEFAULT_WARMUP_FRAMES))?;
    writeln!(params_file, "  relayout_cooldown:  {}s", fmt_val(cli.relayout_cooldown, DEFAULT_RELAYOUT_COOLDOWN))?;
    writeln!(params_file)?;
    writeln!(params_file, "Animation:")?;
    writeln!(params_file, "  position_lerp:      {}", fmt_val(cli.position_lerp_speed, DEFAULT_POSITION_LERP_SPEED))?;
    writeln!(params_file, "  size_lerp:          {}", fmt_val(cli.size_lerp_speed, DEFAULT_SIZE_LERP_SPEED))?;
    writeln!(params_file, "  scale_speed:        {}", fmt_val(cli.scale_speed, DEFAULT_SCALE_SPEED))?;
    writeln!(params_file, "  fade_in_speed:      {}", fmt_val(cli.fade_in_speed, DEFAULT_FADE_IN_SPEED))?;
    writeln!(params_file, "  fade_out_speed:     {}", fmt_val(cli.fade_out_speed, DEFAULT_FADE_OUT_SPEED))?;
    writeln!(params_file)?;
    writeln!(params_file, "Layout:")?;
    writeln!(params_file, "  layout_mode:        {}", fmt_val(cli.layout.as_str(), DEFAULT_LAYOUT))?;
    writeln!(params_file, "  word_padding_x:     {}", fmt_val(cli.word_padding_x, DEFAULT_WORD_PADDING_X))?;
    writeln!(params_file, "  word_padding_y:     {}", fmt_val(cli.word_padding_y, DEFAULT_WORD_PADDING_Y))?;
    writeln!(params_file, "  no_relayout:        {}", fmt_bool(cli.no_relayout, true, "yes", "no"))?;
    if layout_mode == LayoutMode::Amoeba {
        writeln!(params_file, "  repulsion_strength: {}", fmt_val(cli.repulsion_strength, DEFAULT_REPULSION_STRENGTH))?;
    }
    writeln!(params_file)?;
    writeln!(params_file, "Colors:")?;
    write!(params_file, "  palette:            ")?;
    for (i, color) in DEFAULT_COLOR_PALETTE.iter().enumerate() {
        if i > 0 {
            write!(params_file, ", ")?;
        }
        write!(params_file, "{}", color)?;
    }
    writeln!(params_file, "  ")?;
    writeln!(params_file)?;
    writeln!(params_file, "(* = custom value)")?;
    writeln!(params_file)?;
    writeln!(params_file, "Results:")?;
    writeln!(params_file, "  total_words:        {}", words_len)?;
    writeln!(params_file, "  frames_generated:   {}", renderer.frame_count())?;

    println!();
    println!("Done! Generated {} frames", renderer.frame_count());
    println!("Params saved to: {:?}", params_path);

    Ok(())
}
