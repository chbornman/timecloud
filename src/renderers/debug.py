"""Debug renderer for terminal output."""

import sys
from typing import Any

from ..timecloud.config import Config
from ..timecloud.core import CloudState
from .base import BaseRenderer


class DebugRenderer(BaseRenderer):
    """Renders CloudState to terminal for debugging.

    Displays top words and their frequencies, useful for testing
    the core engine without generating video.
    """

    def __init__(self, config: Config, show_every: int = 100):
        """Initialize debug renderer.

        Args:
            config: Configuration object.
            show_every: Only display output every N states (to reduce noise).
        """
        super().__init__(config)
        self.show_every = show_every
        self.state_count = 0

    def render_state(self, state: CloudState) -> Any:
        """Render state to terminal.

        Args:
            state: Current cloud state.

        Returns:
            None
        """
        self.state_count += 1

        if self.state_count % self.show_every != 0:
            return None

        # Clear line and print status
        sys.stdout.write("\r" + " " * 80 + "\r")

        print(f"\n{'='*60}")
        print(f"State #{self.state_count}")
        print(f"Words processed: {state.total_words_processed}")
        print(f"Queue size: {state.current_queue_size}/{self.config.max_queue_size}")
        print(f"Latest word: '{state.latest_word}'")
        print(f"Unique words in window: {len(state.word_frequencies)}")
        print(f"\nTop {min(20, len(state.top_words))} words:")
        print("-" * 40)

        for i, (word, count) in enumerate(state.top_words[:20], 1):
            bar = "#" * min(count, 30)
            print(f"{i:3}. {word:<20} {count:4} {bar}")

        return None

    def finalize(self) -> None:
        """Print final summary."""
        print(f"\n{'='*60}")
        print(f"Debug rendering complete. Total states: {self.state_count}")
        print(f"{'='*60}")


class ProgressRenderer(BaseRenderer):
    """Minimal renderer that just shows progress."""

    def __init__(self, config: Config, total_words: int | None = None):
        """Initialize progress renderer.

        Args:
            config: Configuration object.
            total_words: Total expected words (for percentage display).
        """
        super().__init__(config)
        self.total_words = total_words
        self.state_count = 0

    def render_state(self, state: CloudState) -> Any:
        """Update progress display.

        Args:
            state: Current cloud state.

        Returns:
            None
        """
        self.state_count += 1

        if self.total_words:
            pct = (state.total_words_processed / self.total_words) * 100
            sys.stdout.write(
                f"\rProcessing: {state.total_words_processed}/{self.total_words} "
                f"({pct:.1f}%) | Queue: {state.current_queue_size} | "
                f"Latest: {state.latest_word or '':<15}"
            )
        else:
            sys.stdout.write(
                f"\rProcessing: {state.total_words_processed} words | "
                f"Queue: {state.current_queue_size} | "
                f"Latest: {state.latest_word or '':<15}"
            )
        sys.stdout.flush()

        return None

    def finalize(self) -> None:
        """Print completion message."""
        print(f"\nComplete! Processed {self.state_count} states.")
