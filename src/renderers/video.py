"""Video renderer using wordcloud library and ffmpeg."""

import math
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from PIL import Image
from wordcloud import WordCloud

from ..timecloud.config import Config
from ..timecloud.core import CloudState
from .base import BaseRenderer


class VideoRenderer(BaseRenderer):
    """Renders CloudState frames to video using wordcloud + ffmpeg.

    Generates individual frames as PNG images, then combines them
    into an MP4 video using ffmpeg.
    """

    def __init__(self, config: Config):
        """Initialize video renderer.

        Args:
            config: Configuration object with visual and output settings.
        """
        super().__init__(config)

        # Create temp directory for frames
        self.temp_dir = Path(tempfile.mkdtemp(prefix="timecloud_"))
        self.frame_count = 0

        # Check ffmpeg availability
        if not shutil.which("ffmpeg"):
            raise RuntimeError(
                "ffmpeg not found. Please install ffmpeg to generate video output."
            )

        # Configure wordcloud generator
        self.wc_kwargs = {
            "width": config.frame_width,
            "height": config.frame_height,
            "background_color": config.background_color,
            "color_func": lambda *args, **kwargs: config.word_color,
            "min_font_size": config.min_font_size,
            "max_font_size": config.max_font_size,
            "prefer_horizontal": 0.7,
            "relative_scaling": 0.5,
            "margin": 10,
        }

        # Add font if specified
        if config.font_path and config.font_path.exists():
            self.wc_kwargs["font_path"] = str(config.font_path)

        print(f"VideoRenderer initialized. Frames will be saved to: {self.temp_dir}")

    def _scale_frequencies(self, frequencies: dict[str, int]) -> dict[str, float]:
        """Scale word frequencies for display.

        Applies logarithmic or linear scaling based on config.

        Args:
            frequencies: Raw word frequencies.

        Returns:
            Scaled frequencies for wordcloud generation.
        """
        if not frequencies:
            return {}

        max_freq = max(frequencies.values())
        if max_freq == 0:
            return {}

        scaled = {}
        for word, freq in frequencies.items():
            if self.config.size_scale == "log":
                # Log scaling: log(freq + 1) to handle freq=1
                scaled[word] = math.log(freq + 1) / math.log(max_freq + 1)
            else:
                # Linear scaling
                scaled[word] = freq / max_freq

        return scaled

    def render_state(self, state: CloudState) -> Any:
        """Render a single frame.

        Args:
            state: Current cloud state.

        Returns:
            Path to the generated frame image.
        """
        self.frame_count += 1

        # Progress output
        if self.frame_count % 100 == 0:
            sys.stdout.write(f"\rRendering frame {self.frame_count}...")
            sys.stdout.flush()

        # Handle empty state
        if not state.top_words:
            # Create blank frame
            img = Image.new(
                "RGB",
                (self.config.frame_width, self.config.frame_height),
                self.config.background_color,
            )
        else:
            # Build frequency dict from top words
            freq_dict = dict(state.top_words)

            # Scale frequencies
            scaled = self._scale_frequencies(freq_dict)

            # Convert scaled values back to integers for wordcloud
            # (wordcloud needs positive integers)
            int_freqs = {w: max(1, int(v * 1000)) for w, v in scaled.items()}

            try:
                wc = WordCloud(**self.wc_kwargs)
                wc.generate_from_frequencies(int_freqs)
                img = wc.to_image()
            except ValueError as e:
                # wordcloud can fail with too few words
                print(f"\nWarning: WordCloud generation failed: {e}")
                img = Image.new(
                    "RGB",
                    (self.config.frame_width, self.config.frame_height),
                    self.config.background_color,
                )

        # Save frame
        frame_path = self.temp_dir / f"frame_{self.frame_count:08d}.png"
        img.save(frame_path)

        return frame_path

    def finalize(self) -> None:
        """Encode frames to video using ffmpeg."""
        print(f"\n\nEncoding {self.frame_count} frames to video...")

        output_path = self.config.output_path
        output_path.parent.mkdir(parents=True, exist_ok=True)

        # ffmpeg command
        cmd = [
            "ffmpeg",
            "-y",  # Overwrite output
            "-framerate", str(self.config.fps),
            "-i", str(self.temp_dir / "frame_%08d.png"),
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",  # Compatibility
            "-crf", "18",  # High quality
            "-preset", "medium",
            str(output_path),
        ]

        print(f"Running: {' '.join(cmd)}")

        result = subprocess.run(cmd, capture_output=True, text=True)

        if result.returncode != 0:
            print(f"ffmpeg error: {result.stderr}")
            raise RuntimeError(f"ffmpeg failed with code {result.returncode}")

        print(f"\nVideo saved to: {output_path}")
        print(f"Duration: {self.frame_count / self.config.fps:.1f} seconds")

        # Cleanup temp directory
        self._cleanup()

    def _cleanup(self) -> None:
        """Remove temporary frame files."""
        print(f"Cleaning up {self.temp_dir}...")
        shutil.rmtree(self.temp_dir, ignore_errors=True)


class FrameRenderer(BaseRenderer):
    """Renders CloudState to individual PNG frames without video encoding.

    Useful for custom post-processing or when ffmpeg is not available.
    """

    def __init__(self, config: Config, output_dir: Path):
        """Initialize frame renderer.

        Args:
            config: Configuration object.
            output_dir: Directory to save frame images.
        """
        super().__init__(config)
        self.output_dir = output_dir
        self.output_dir.mkdir(parents=True, exist_ok=True)
        self.frame_count = 0

        # Configure wordcloud (same as VideoRenderer)
        self.wc_kwargs = {
            "width": config.frame_width,
            "height": config.frame_height,
            "background_color": config.background_color,
            "color_func": lambda *args, **kwargs: config.word_color,
            "min_font_size": config.min_font_size,
            "max_font_size": config.max_font_size,
            "prefer_horizontal": 0.7,
            "relative_scaling": 0.5,
            "margin": 10,
        }

        if config.font_path and config.font_path.exists():
            self.wc_kwargs["font_path"] = str(config.font_path)

    def render_state(self, state: CloudState) -> Any:
        """Render and save a single frame.

        Args:
            state: Current cloud state.

        Returns:
            Path to saved frame.
        """
        self.frame_count += 1

        if not state.top_words:
            img = Image.new(
                "RGB",
                (self.config.frame_width, self.config.frame_height),
                self.config.background_color,
            )
        else:
            freq_dict = dict(state.top_words)
            max_freq = max(freq_dict.values()) if freq_dict else 1

            if self.config.size_scale == "log":
                scaled = {
                    w: max(1, int(math.log(f + 1) / math.log(max_freq + 1) * 1000))
                    for w, f in freq_dict.items()
                }
            else:
                scaled = {w: max(1, int(f / max_freq * 1000)) for w, f in freq_dict.items()}

            try:
                wc = WordCloud(**self.wc_kwargs)
                wc.generate_from_frequencies(scaled)
                img = wc.to_image()
            except ValueError:
                img = Image.new(
                    "RGB",
                    (self.config.frame_width, self.config.frame_height),
                    self.config.background_color,
                )

        frame_path = self.output_dir / f"frame_{self.frame_count:08d}.png"
        img.save(frame_path)

        if self.frame_count % 100 == 0:
            print(f"Saved frame {self.frame_count}")

        return frame_path

    def finalize(self) -> None:
        """Print summary."""
        print(f"\nSaved {self.frame_count} frames to {self.output_dir}")
