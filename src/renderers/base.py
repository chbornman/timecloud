"""Base renderer abstract class."""

from abc import ABC, abstractmethod
from typing import Any

from ..timecloud.config import Config
from ..timecloud.core import CloudState


class BaseRenderer(ABC):
    """Abstract base class for TimeCloud renderers.

    Renderers take CloudState objects and produce visual output.
    This could be video frames, terminal output, web canvas, etc.
    """

    def __init__(self, config: Config):
        """Initialize renderer with configuration.

        Args:
            config: Configuration object with visual settings.
        """
        self.config = config

    @abstractmethod
    def render_state(self, state: CloudState) -> Any:
        """Render a single CloudState.

        Args:
            state: The current state of the word cloud.

        Returns:
            Renderer-specific output (frame, text, etc.)
        """
        pass

    @abstractmethod
    def finalize(self) -> None:
        """Complete rendering and clean up.

        Called after all states have been rendered.
        For video renderers, this might encode the final video.
        """
        pass

    def render_all(self, states) -> None:
        """Render all states from an iterator.

        Args:
            states: Iterator of CloudState objects.
        """
        for state in states:
            self.render_state(state)
        self.finalize()
