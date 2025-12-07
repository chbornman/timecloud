# TimeCloud Project Plan

## Overview

A reusable engine for creating animated wordclouds that show word frequency evolution over time using a sliding window approach.

## Design Principles

- **Parameterize everything**: All visual, processing, and output options are configurable
- **Separation of concerns**: Core engine is independent of rendering
- **CLI-first**: Runnable from command line with sensible defaults
- **Well-documented**: All config options clearly documented

## Architecture

```
articles/                  # Scraped text files
  2025-01-15_article-slug.txt
  2025-01-22_article-slug.txt
  ...

src/
  scraper.py              # Substack → text files
  timecloud/
    __init__.py
    core.py               # TimeCloud engine (sliding window, frequency tracking)
    tokenizer.py          # Word processing (lowercase, stopwords, stemming)
    config.py             # Configuration dataclass
  renderers/
    __init__.py
    base.py               # Abstract base renderer
    video.py              # MP4 output via PIL + ffmpeg
    debug.py              # Terminal output for testing

main.py                   # CLI entry point
stopwords.txt             # Configurable stopword list
```

## Component Details

### 1. Scraper (`src/scraper.py`)

- Input: Substack archive URL
- Output: `articles/YYYY-MM-DD_slug.txt` files
- Extracts article body text only (no headers, footers, nav)
- Sorts by date ascending

### 2. Tokenizer (`src/timecloud/tokenizer.py`)

- `tokenize(text: str, config: Config) -> list[str]`
- Lowercase all words
- Strip punctuation
- Filter against stopword list
- Optional: stem words (using NLTK or similar)
- Optional: handle plurals (simple 's' removal or stemmer)

### 3. TimeCloud Core (`src/timecloud/core.py`)

```python
class TimeCloud:
    def __init__(self, config: Config):
        self.queue: deque[str]          # Sliding window of raw words
        self.max_queue_size: int        # Max words in sliding window
        self.max_display_words: int     # Max words shown in visual

    def add_word(self, word: str) -> CloudState:
        """Add word, potentially evict oldest, return current state."""

    def get_frequencies(self) -> dict[str, int]:
        """Current word frequencies from queue."""

    def get_top_words(self, n: int) -> list[tuple[str, int]]:
        """Top N words by frequency."""

    def __iter__(self) -> Iterator[CloudState]:
        """Iterate through states as words are added."""

@dataclass
class CloudState:
    word_frequencies: dict[str, int]    # All frequencies in current window
    top_words: list[tuple[str, int]]    # Top N for display
    total_words_processed: int
    current_queue_size: int
```

### 4. Config (`src/timecloud/config.py`)

```python
@dataclass
class Config:
    # Queue settings
    max_queue_size: int = 500           # Sliding window size
    max_display_words: int = 50         # Words shown in cloud

    # Tokenizer settings
    lowercase: bool = True
    filter_stopwords: bool = True
    stopwords_file: str = "stopwords.txt"
    enable_stemming: bool = False

    # Renderer settings
    words_per_frame: int = 1            # How many words advance per frame
    fps: int = 30
    frame_width: int = 1920
    frame_height: int = 1080

    # Visual settings
    background_color: str = "#FAF9F6"   # Off-white
    word_color: str = "#2C2C2C"         # Dark grey
    font_path: str | None = None        # Path to .ttf, None = default
    font_name: str = "serif"            # Fallback font family
    size_scale: str = "log"             # "log" or "linear"
    min_font_size: int = 12
    max_font_size: int = 120
```

### 5. Base Renderer (`src/renderers/base.py`)

```python
class BaseRenderer(ABC):
    @abstractmethod
    def render_state(self, state: CloudState) -> Any:
        """Render a single state."""

    @abstractmethod
    def finalize(self) -> None:
        """Complete rendering (e.g., write video file)."""
```

### 6. Video Renderer (`src/renderers/video.py`)

- Uses `wordcloud` library for layout generation
- PIL for frame composition
- ffmpeg (subprocess) for MP4 encoding
- Outputs frames to temp directory, then combines

### 7. Debug Renderer (`src/renderers/debug.py`)

- Prints top words to terminal
- Useful for testing without generating video

## CLI Interface

```bash
# Scrape articles
python main.py scrape https://amybornman.substack.com/

# Generate video with defaults
python main.py render --output timecloud.mp4

# Custom settings
python main.py render \
    --queue-size 1000 \
    --display-words 75 \
    --words-per-frame 5 \
    --fps 30 \
    --stemming \
    --output timecloud.mp4

# Visual customization
python main.py render \
    --background-color "#FFFFFF" \
    --word-color "#000000" \
    --font-path "/path/to/font.ttf" \
    --size-scale linear \
    --min-font-size 10 \
    --max-font-size 150 \
    --output timecloud.mp4

# Debug mode (no video)
python main.py render --debug
```

## Implementation Order

1. [ ] Set up project structure and dependencies
2. [ ] Implement scraper for Substack
3. [ ] Implement tokenizer with stopwords
4. [ ] Implement TimeCloud core engine
5. [ ] Implement debug renderer (test the core)
6. [ ] Implement video renderer
7. [ ] Add stemming support
8. [ ] CLI polish and error handling

## Dependencies

```
requests
beautifulsoup4
wordcloud
Pillow
nltk (optional, for stemming)
```

Plus `ffmpeg` installed on system.

## Configuration Reference

All options can be set via CLI flags, config file, or programmatically.

### Core Engine Options

| Option | CLI Flag | Default | Description |
|--------|----------|---------|-------------|
| `max_queue_size` | `--queue-size` | 500 | Sliding window size (raw words kept in memory) |
| `max_display_words` | `--display-words` | 50 | Maximum words shown in the cloud at once |

### Tokenizer Options

| Option | CLI Flag | Default | Description |
|--------|----------|---------|-------------|
| `lowercase` | `--no-lowercase` | True | Convert all words to lowercase |
| `filter_stopwords` | `--no-stopwords` | True | Filter common English stopwords |
| `stopwords_file` | `--stopwords-file` | stopwords.txt | Path to custom stopwords file (one word per line) |
| `enable_stemming` | `--stemming` | False | Reduce words to stems (running→run) |

### Video Output Options

| Option | CLI Flag | Default | Description |
|--------|----------|---------|-------------|
| `words_per_frame` | `--words-per-frame` | 1 | Words to process before rendering a frame |
| `fps` | `--fps` | 30 | Frames per second in output video |
| `frame_width` | `--width` | 1920 | Video width in pixels |
| `frame_height` | `--height` | 1080 | Video height in pixels |

### Visual Style Options

| Option | CLI Flag | Default | Description |
|--------|----------|---------|-------------|
| `background_color` | `--background-color` | #FAF9F6 | Background color (hex or name) |
| `word_color` | `--word-color` | #2C2C2C | Word color (hex or name) |
| `font_path` | `--font-path` | None | Path to .ttf font file |
| `font_name` | `--font-name` | serif | Font family fallback |
| `size_scale` | `--size-scale` | log | Word size scaling: "log" or "linear" |
| `min_font_size` | `--min-font-size` | 12 | Minimum font size in pixels |
| `max_font_size` | `--max-font-size` | 120 | Maximum font size in pixels |

## Future Extensions

- Interactive web renderer (d3.js / canvas)
- Color themes based on sentiment or article
- Word positioning that's more deterministic/structured
- Support for other sources (RSS, local markdown, etc.)
- Config file support (YAML/JSON)
