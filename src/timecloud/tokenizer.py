"""Word tokenization and filtering."""

import re
from pathlib import Path

from .config import Config


class Tokenizer:
    """Tokenizes text into words with configurable filtering.

    Handles:
    - Lowercasing
    - Punctuation removal
    - Stopword filtering
    - Optional stemming
    - Minimum word length filtering
    """

    def __init__(self, config: Config):
        """Initialize tokenizer with configuration.

        Args:
            config: Config object with tokenizer settings.
        """
        self.config = config
        self.stopwords: set[str] = set()
        self.stemmer = None

        if config.filter_stopwords:
            self._load_stopwords()

        if config.enable_stemming:
            self._init_stemmer()

    def _load_stopwords(self) -> None:
        """Load stopwords from file."""
        stopwords_path = self.config.stopwords_file
        if not stopwords_path.exists():
            # Try relative to this file
            alt_path = Path(__file__).parent.parent.parent / "stopwords.txt"
            if alt_path.exists():
                stopwords_path = alt_path
            else:
                print(f"Warning: Stopwords file not found at {stopwords_path}")
                return

        text = stopwords_path.read_text(encoding="utf-8")
        self.stopwords = {word.strip().lower() for word in text.splitlines() if word.strip()}
        print(f"Loaded {len(self.stopwords)} stopwords")

    def _init_stemmer(self) -> None:
        """Initialize NLTK stemmer."""
        try:
            from nltk.stem import PorterStemmer
            self.stemmer = PorterStemmer()
        except ImportError:
            print("Warning: NLTK not available, stemming disabled")
            self.config.enable_stemming = False

    def tokenize(self, text: str) -> list[str]:
        """Tokenize text into a list of words.

        Args:
            text: Input text to tokenize.

        Returns:
            List of processed word tokens.
        """
        # Lowercase if configured
        if self.config.lowercase:
            text = text.lower()

        # Split on whitespace and punctuation, keeping only word characters
        # This regex finds sequences of letters (including unicode)
        words = re.findall(r"\b[a-zA-Z]+\b", text)

        # Filter by minimum length
        words = [w for w in words if len(w) >= self.config.min_word_length]

        # Filter stopwords
        if self.config.filter_stopwords:
            words = [w for w in words if w.lower() not in self.stopwords]

        # Apply stemming
        if self.config.enable_stemming and self.stemmer:
            words = [self.stemmer.stem(w) for w in words]

        return words

    def tokenize_file(self, filepath: Path) -> list[str]:
        """Tokenize an entire file.

        Args:
            filepath: Path to text file.

        Returns:
            List of word tokens from the file.
        """
        text = filepath.read_text(encoding="utf-8")
        return self.tokenize(text)

    def tokenize_files(self, filepaths: list[Path]) -> list[str]:
        """Tokenize multiple files in order.

        Args:
            filepaths: List of file paths to tokenize, in order.

        Returns:
            Combined list of word tokens from all files.
        """
        all_words = []
        for filepath in filepaths:
            words = self.tokenize_file(filepath)
            all_words.extend(words)
            print(f"Tokenized {filepath.name}: {len(words)} words")
        return all_words
