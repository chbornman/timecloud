"""TimeCloud core engine - sliding window word frequency tracking."""

from collections import Counter, deque
from dataclasses import dataclass
from typing import Iterator

from .config import Config


@dataclass
class CloudState:
    """Represents the state of the word cloud at a point in time.

    This is the data structure passed to renderers.
    """

    word_frequencies: dict[str, int]
    """All word frequencies in the current sliding window."""

    top_words: list[tuple[str, int]]
    """Top N words by frequency, for display. List of (word, count) tuples."""

    total_words_processed: int
    """Total number of words processed so far (not just in window)."""

    current_queue_size: int
    """Current number of words in the sliding window."""

    latest_word: str | None
    """The most recently added word (for highlighting in renders)."""


class TimeCloud:
    """Core engine for tracking word frequencies over a sliding window.

    Words are added one at a time. The engine maintains a queue of the
    N most recent words and calculates frequencies based on that window.
    As new words are added, old words are evicted from the queue, causing
    their frequencies to decrease naturally.

    Example:
        config = Config(max_queue_size=1000, max_display_words=50)
        cloud = TimeCloud(config)

        for word in words:
            state = cloud.add_word(word)
            renderer.render_state(state)
    """

    def __init__(self, config: Config):
        """Initialize the TimeCloud engine.

        Args:
            config: Configuration object.
        """
        self.config = config
        self.queue: deque[str] = deque(maxlen=config.max_queue_size)
        self.frequencies: Counter[str] = Counter()
        self.total_words_processed: int = 0
        self.latest_word: str | None = None

    def add_word(self, word: str) -> CloudState:
        """Add a word to the sliding window and return current state.

        If the queue is at capacity, the oldest word is evicted and its
        frequency count is decremented.

        Args:
            word: The word to add.

        Returns:
            Current CloudState after adding the word.
        """
        # If queue is full, we need to evict the oldest word
        if len(self.queue) == self.queue.maxlen:
            evicted = self.queue[0]  # Will be removed when we append
            self.frequencies[evicted] -= 1
            if self.frequencies[evicted] <= 0:
                del self.frequencies[evicted]

        # Add new word
        self.queue.append(word)
        self.frequencies[word] += 1
        self.total_words_processed += 1
        self.latest_word = word

        return self.get_state()

    def get_state(self) -> CloudState:
        """Get the current state of the word cloud.

        Returns:
            CloudState with current frequencies and top words.
        """
        top_words = self.frequencies.most_common(self.config.max_display_words)

        return CloudState(
            word_frequencies=dict(self.frequencies),
            top_words=top_words,
            total_words_processed=self.total_words_processed,
            current_queue_size=len(self.queue),
            latest_word=self.latest_word,
        )

    def get_frequencies(self) -> dict[str, int]:
        """Get current word frequencies.

        Returns:
            Dictionary mapping words to their counts in the current window.
        """
        return dict(self.frequencies)

    def get_top_words(self, n: int | None = None) -> list[tuple[str, int]]:
        """Get the top N words by frequency.

        Args:
            n: Number of top words to return. Defaults to max_display_words.

        Returns:
            List of (word, count) tuples, sorted by frequency descending.
        """
        if n is None:
            n = self.config.max_display_words
        return self.frequencies.most_common(n)

    def reset(self) -> None:
        """Reset the engine to initial state."""
        self.queue.clear()
        self.frequencies.clear()
        self.total_words_processed = 0
        self.latest_word = None

    def process_words(self, words: list[str]) -> Iterator[CloudState]:
        """Process a list of words, yielding state after each addition.

        This is the main method for generating animation frames.

        Args:
            words: List of words to process in order.

        Yields:
            CloudState after each word is added.
        """
        for word in words:
            yield self.add_word(word)

    def process_words_batched(
        self, words: list[str], batch_size: int
    ) -> Iterator[CloudState]:
        """Process words in batches, yielding state after each batch.

        Useful for reducing the number of frames when there are many words.

        Args:
            words: List of words to process.
            batch_size: Number of words to process per yield.

        Yields:
            CloudState after each batch of words.
        """
        for i, word in enumerate(words, 1):
            self.add_word(word)
            if i % batch_size == 0 or i == len(words):
                yield self.get_state()
