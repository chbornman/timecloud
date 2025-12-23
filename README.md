# TimeCloud

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**Animated word cloud videos with physics-based layout.** Watch word frequencies evolve over time as text is processed chronologically—words grow, shrink, fade in, and drift apart based on their frequency in a sliding window.

![TimeCloud Demo](demo_shakespeare/demo.gif)

> *Shakespeare's plays visualized with background image transitions*

## Features

- **Sliding window frequency tracking** — Word sizes reflect recent usage, not total count
- **Physics-based animations** — Smooth transitions with configurable lerp speeds
- **Two layout modes** — Spiral (classic) or Amoeba (physics-based repulsion)
- **Substack scraper included** — Download articles and cover images automatically
- **Highly configurable** — Control every aspect of the visualization
- **Fast** — Parallel rendering, pipes directly to ffmpeg

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) 1.70+
- [ffmpeg](https://ffmpeg.org/) (for video encoding)
- Python 3.8+ (optional, for scraper)

### Build

```bash
git clone https://github.com/chbornman/timecloud.git
cd timecloud
cargo build --release
```

The binary will be at `target/release/timecloud`.

## Quick Start

```bash
# Option 1: Use your own text files
# Put .txt files in a directory, then:
cargo run --release -- -i my-texts/ -o output.mp4

# Option 2: Scrape a Substack
pip install -r requirements.txt
python scraper.py https://example.substack.com/
cargo run --release -- -i articles/ -o output.mp4
```

## Usage

```bash
timecloud -i <INPUT_DIR> [OPTIONS]
```

### Common Options

| Option | Default | Description |
|--------|---------|-------------|
| `-i, --input` | required | Directory containing .txt files |
| `-o, --output` | `output.mp4` | Output video path |
| `--window-size` | 2500 | Sliding window size (words tracked) |
| `--max-words` | 18 | Maximum words displayed |
| `--fps` | 60 | Frames per second |
| `--width` | 1920 | Video width |
| `--height` | 1080 | Video height |
| `--font` | system | Path to .ttf font file |
| `--background` | `#FAF9F6` | Background color (hex) |
| `--layout` | `amoeba` | Layout: `spiral` or `amoeba` |

<details>
<summary><strong>All Options</strong></summary>

### Engine
| Option | Default | Description |
|--------|---------|-------------|
| `--window-size` | 2500 | Sliding window size |
| `--max-words` | 18 | Max words displayed |
| `--words-per-frame` | 1 | Words processed per frame |

### Tokenizer
| Option | Default | Description |
|--------|---------|-------------|
| `--no-lowercase` | false | Keep original case |
| `--no-filter-stopwords` | false | Keep stopwords |
| `--enable-stemming` | false | Enable Porter stemming |
| `--min-word-length` | 2 | Minimum word length |
| `--stopwords` | `stopwords.txt` | Custom stopwords file |

### Visual
| Option | Default | Description |
|--------|---------|-------------|
| `--font` | system | Path to .ttf font |
| `--background` | `#FAF9F6` | Background color |
| `--min-font-size` | 24 | Minimum font size |
| `--max-font-size` | 200 | Maximum font size |

### Physics & Animation
| Option | Default | Description |
|--------|---------|-------------|
| `--physics-steps` | 10 | Physics steps per frame |
| `--preload` | 1500 | Words to preload |
| `--warmup-frames` | 0 | Warmup frames after preload |
| `--relayout-cooldown` | 2.0 | Seconds between relayouts |
| `--position-lerp-speed` | 0.001 | Position smoothing (0-1) |
| `--size-lerp-speed` | 0.01 | Size smoothing (0-1) |
| `--scale-speed` | 0.02 | Zoom speed (0-1) |
| `--fade-in-speed` | 0.02 | Fade in speed (0-1) |
| `--fade-out-speed` | 0.02 | Fade out speed (0-1) |

### Layout
| Option | Default | Description |
|--------|---------|-------------|
| `--layout` | `amoeba` | `spiral` or `amoeba` |
| `--word-padding-x` | 8 | Horizontal word padding |
| `--word-padding-y` | 8 | Vertical word padding |
| `--no-relayout` | true | Disable repositioning |
| `--repulsion-strength` | 150 | Amoeba repulsion force |
| `--ramp-seconds` | 0.75 | Word add interval during ramp |

</details>

## Examples

### Basic
```bash
cargo run --release -- -i articles -o output.mp4
```

### 4K with custom font
```bash
cargo run --release -- \
    -i articles \
    -o output.mp4 \
    --max-words 25 \
    --font /usr/share/fonts/TTF/DejaVuSerif.ttf \
    --width 3840 \
    --height 2160
```

### Quick preview (faster render)
```bash
cargo run --release -- \
    -i articles \
    --window-size 500 \
    --fps 30 \
    --preload 200
```

### Dark theme
```bash
cargo run --release -- \
    -i articles \
    --background "#1a1a2e"
```

## Substack Scraper

The included Python scraper downloads articles from any Substack:

```bash
pip install -r requirements.txt

# Download articles
python scraper.py https://example.substack.com/

# Download articles + cover images
python scraper.py https://example.substack.com/ --images

# Download only images (if articles exist)
python scraper.py https://example.substack.com/ --images-only
```

Articles are saved as `YYYY-MM-DD_slug.txt` in the `articles/` directory.

## How It Works

1. **Tokenization** — Text files are read and tokenized (lowercase, stopword removal, optional stemming)
2. **Sliding Window** — Words are added to a fixed-size queue; old words fall off as new ones enter
3. **Frequency Tracking** — Word frequencies are computed from the current window contents
4. **Layout** — Words are positioned using spiral or amoeba (physics-based) algorithms
5. **Rendering** — Each frame is rendered and piped directly to ffmpeg
6. **Animation** — Word positions, sizes, and opacity smoothly interpolate between states

## Project Structure

```
timecloud/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library exports
│   ├── config.rs        # Configuration structs
│   ├── core.rs          # Sliding window engine
│   ├── tokenizer.rs     # Text processing
│   └── physics/
│       └── renderer.rs  # Physics layout & video rendering
├── scraper.py           # Substack article scraper
├── stopwords.txt        # Default stopword list
├── Cargo.toml
└── requirements.txt     # Python dependencies (scraper)
```

## Contributing

Contributions are welcome! Feel free to:

- Report bugs or request features via [Issues](https://github.com/chbornman/timecloud/issues)
- Submit pull requests
- Share videos you've created

## License

[MIT](LICENSE)
