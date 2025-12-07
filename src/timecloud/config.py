"""Configuration dataclass for TimeCloud."""

from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class Config:
    """Configuration for TimeCloud engine and renderers.

    All visual, processing, and output options are configurable here.
    See PLAN.md for full documentation of each option.
    """

    # === Core Engine Options ===
    max_queue_size: int = 500
    """Sliding window size - number of raw words kept in memory."""

    max_display_words: int = 50
    """Maximum number of words shown in the cloud at once."""

    # === Tokenizer Options ===
    lowercase: bool = True
    """Convert all words to lowercase."""

    filter_stopwords: bool = True
    """Filter common English stopwords."""

    stopwords_file: Path = field(default_factory=lambda: Path("stopwords.txt"))
    """Path to custom stopwords file (one word per line)."""

    enable_stemming: bool = False
    """Reduce words to stems (e.g., 'running' -> 'run')."""

    min_word_length: int = 2
    """Minimum word length to include."""

    # === Video Output Options ===
    words_per_frame: int = 1
    """Number of words to process before rendering a frame."""

    fps: int = 30
    """Frames per second in output video."""

    frame_width: int = 1920
    """Video width in pixels."""

    frame_height: int = 1080
    """Video height in pixels."""

    # === Visual Style Options ===
    background_color: str = "#FAF9F6"
    """Background color (hex or color name). Default is off-white."""

    word_color: str = "#2C2C2C"
    """Word color (hex or color name). Default is dark grey."""

    font_path: Path | None = None
    """Path to .ttf font file. None uses system default."""

    font_name: str = "serif"
    """Font family fallback when font_path is None."""

    size_scale: str = "log"
    """Word size scaling method: 'log' or 'linear'."""

    min_font_size: int = 12
    """Minimum font size in pixels."""

    max_font_size: int = 120
    """Maximum font size in pixels."""

    # === Output Options ===
    output_path: Path = field(default_factory=lambda: Path("output.mp4"))
    """Output video file path."""

    articles_dir: Path = field(default_factory=lambda: Path("articles"))
    """Directory containing article text files."""

    def __post_init__(self):
        """Convert string paths to Path objects if needed."""
        if isinstance(self.stopwords_file, str):
            self.stopwords_file = Path(self.stopwords_file)
        if isinstance(self.font_path, str):
            self.font_path = Path(self.font_path)
        if isinstance(self.output_path, str):
            self.output_path = Path(self.output_path)
        if isinstance(self.articles_dir, str):
            self.articles_dir = Path(self.articles_dir)

        # Validate size_scale
        if self.size_scale not in ("log", "linear"):
            raise ValueError(f"size_scale must be 'log' or 'linear', got '{self.size_scale}'")
