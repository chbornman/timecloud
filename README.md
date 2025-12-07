# TimeCloud

Animated wordcloud generator with sliding window frequency tracking. Watch word frequencies evolve over time as text is processed chronologically.

## How It Works

TimeCloud processes text word-by-word, maintaining a sliding window of recent words. As new words enter, old words exit the window, causing word frequencies (and therefore sizes) to naturally evolve. This creates a dynamic visualization showing how language patterns change over time.

```
[word1, word2, word3, ..., wordN] ← sliding window
        ↓
   frequency count
        ↓
   wordcloud frame
```

## Installation

```bash
# Clone the repo
git clone https://github.com/chbornman/timecloud.git
cd timecloud

# Create virtual environment
python -m venv venv
source venv/bin/activate  # or `venv\Scripts\activate` on Windows

# Install dependencies
pip install -e .

# Ensure ffmpeg is installed (for video output)
# macOS: brew install ffmpeg
# Ubuntu: sudo apt install ffmpeg
# Windows: download from ffmpeg.org
```

## Quick Start

```bash
# 1. Scrape articles from a Substack
python main.py scrape https://example.substack.com/

# 2. Generate video
python main.py render --output my-wordcloud.mp4

# 3. (Optional) Test with debug mode first
python main.py render --debug
```

## CLI Reference

### `scrape` - Download articles

```bash
python main.py scrape <url> [options]
```

| Option | Description |
|--------|-------------|
| `url` | Substack base URL (required) |
| `--output-dir, -o` | Output directory (default: `articles/`) |

### `render` - Generate wordcloud video

```bash
python main.py render [options]
```

#### Input/Output Options

| Option | Default | Description |
|--------|---------|-------------|
| `--input-dir, -i` | `articles/` | Directory with .txt files |
| `--output, -o` | `output.mp4` | Output video path |
| `--frames-only DIR` | - | Save PNG frames instead of video |

#### Core Engine Options

| Option | Default | Description |
|--------|---------|-------------|
| `--queue-size` | 500 | Sliding window size (words in memory) |
| `--display-words` | 50 | Max words shown in cloud |

#### Tokenizer Options

| Option | Default | Description |
|--------|---------|-------------|
| `--no-lowercase` | false | Keep original case |
| `--no-stopwords` | false | Don't filter stopwords |
| `--stopwords-file` | `stopwords.txt` | Custom stopwords file |
| `--stemming` | false | Enable word stemming |
| `--min-word-length` | 2 | Minimum word length |

#### Video Options

| Option | Default | Description |
|--------|---------|-------------|
| `--words-per-frame` | 1 | Words processed per frame |
| `--fps` | 30 | Frames per second |
| `--width` | 1920 | Video width (pixels) |
| `--height` | 1080 | Video height (pixels) |

#### Visual Style Options

| Option | Default | Description |
|--------|---------|-------------|
| `--background-color` | `#FAF9F6` | Background color (hex) |
| `--word-color` | `#2C2C2C` | Word color (hex) |
| `--font-path` | system | Path to .ttf font file |
| `--size-scale` | `log` | Scaling: `log` or `linear` |
| `--min-font-size` | 12 | Minimum font size |
| `--max-font-size` | 120 | Maximum font size |

#### Debug Options

| Option | Default | Description |
|--------|---------|-------------|
| `--debug` | false | Terminal output instead of video |
| `--debug-every` | 100 | Show debug output every N words |

## Examples

### Basic usage

```bash
# Scrape and render with defaults
python main.py scrape https://mysubstack.substack.com/
python main.py render
```

### High-quality long video

```bash
python main.py render \
    --queue-size 2000 \
    --display-words 100 \
    --words-per-frame 1 \
    --fps 60 \
    --width 3840 \
    --height 2160
```

### Quick preview

```bash
python main.py render \
    --queue-size 200 \
    --words-per-frame 10 \
    --fps 24
```

### Custom styling

```bash
python main.py render \
    --background-color "#1a1a2e" \
    --word-color "#eaeaea" \
    --font-path "/path/to/Garamond.ttf" \
    --size-scale linear
```

### Debug/test mode

```bash
python main.py render --debug --debug-every 50
```

## Architecture

```
src/
├── scraper.py           # Substack article scraper
├── timecloud/
│   ├── config.py        # Configuration dataclass
│   ├── core.py          # TimeCloud engine (sliding window)
│   └── tokenizer.py     # Word processing
└── renderers/
    ├── base.py          # Abstract renderer
    ├── debug.py         # Terminal output
    └── video.py         # MP4/PNG output
```

The core engine (`TimeCloud`) is decoupled from rendering, making it easy to add new output formats (web, interactive, etc.).

## Programmatic Usage

```python
from src.timecloud import Config, TimeCloud, Tokenizer
from src.renderers.video import VideoRenderer

# Configure
config = Config(
    max_queue_size=1000,
    max_display_words=75,
    background_color="#FFFFFF",
)

# Tokenize your text
tokenizer = Tokenizer(config)
words = tokenizer.tokenize("Your text content here...")

# Process through engine
cloud = TimeCloud(config)
renderer = VideoRenderer(config)

for state in cloud.process_words(words):
    renderer.render_state(state)

renderer.finalize()
```

## Dependencies

- Python 3.11+
- requests
- beautifulsoup4
- wordcloud
- Pillow
- nltk (optional, for stemming)
- ffmpeg (system install, for video encoding)

## License

MIT
