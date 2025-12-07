#!/usr/bin/env python3
"""TimeCloud CLI - Animated wordcloud generator with sliding window."""

import argparse
import sys
from pathlib import Path

from src.scraper import scrape_substack
from src.timecloud import Config, TimeCloud, Tokenizer
from src.renderers.debug import DebugRenderer, ProgressRenderer
from src.renderers.video import VideoRenderer, FrameRenderer


def create_parser() -> argparse.ArgumentParser:
    """Create the CLI argument parser."""
    parser = argparse.ArgumentParser(
        prog="timecloud",
        description="Animated wordcloud generator with sliding window frequency tracking.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Scrape articles from a Substack
  python main.py scrape https://example.substack.com/

  # Generate video with default settings
  python main.py render --output output.mp4

  # Custom settings
  python main.py render --queue-size 1000 --display-words 75 --fps 30 --output output.mp4

  # Debug mode (no video)
  python main.py render --debug
        """,
    )

    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    # === Scrape command ===
    scrape_parser = subparsers.add_parser("scrape", help="Scrape articles from Substack")
    scrape_parser.add_argument("url", help="Substack base URL (e.g., https://example.substack.com)")
    scrape_parser.add_argument(
        "--output-dir", "-o",
        type=Path,
        default=Path("articles"),
        help="Directory to save article files (default: articles/)",
    )

    # === Render command ===
    render_parser = subparsers.add_parser("render", help="Generate wordcloud video")

    # Input/output
    render_parser.add_argument(
        "--input-dir", "-i",
        type=Path,
        default=Path("articles"),
        help="Directory containing article text files (default: articles/)",
    )
    render_parser.add_argument(
        "--output", "-o",
        type=Path,
        default=Path("output.mp4"),
        help="Output video file path (default: output.mp4)",
    )
    render_parser.add_argument(
        "--frames-only",
        type=Path,
        metavar="DIR",
        help="Save individual frames to directory instead of video",
    )

    # Core engine options
    render_parser.add_argument(
        "--queue-size",
        type=int,
        default=500,
        help="Sliding window size (default: 500)",
    )
    render_parser.add_argument(
        "--display-words",
        type=int,
        default=50,
        help="Maximum words shown in cloud (default: 50)",
    )

    # Tokenizer options
    render_parser.add_argument(
        "--no-lowercase",
        action="store_true",
        help="Don't convert words to lowercase",
    )
    render_parser.add_argument(
        "--no-stopwords",
        action="store_true",
        help="Don't filter stopwords",
    )
    render_parser.add_argument(
        "--stopwords-file",
        type=Path,
        default=Path("stopwords.txt"),
        help="Path to stopwords file (default: stopwords.txt)",
    )
    render_parser.add_argument(
        "--stemming",
        action="store_true",
        help="Enable word stemming (requires NLTK)",
    )
    render_parser.add_argument(
        "--min-word-length",
        type=int,
        default=2,
        help="Minimum word length to include (default: 2)",
    )

    # Video output options
    render_parser.add_argument(
        "--words-per-frame",
        type=int,
        default=1,
        help="Words to process per frame (default: 1)",
    )
    render_parser.add_argument(
        "--fps",
        type=int,
        default=30,
        help="Frames per second (default: 30)",
    )
    render_parser.add_argument(
        "--width",
        type=int,
        default=1920,
        help="Video width in pixels (default: 1920)",
    )
    render_parser.add_argument(
        "--height",
        type=int,
        default=1080,
        help="Video height in pixels (default: 1080)",
    )

    # Visual style options
    render_parser.add_argument(
        "--background-color",
        default="#FAF9F6",
        help="Background color hex (default: #FAF9F6, off-white)",
    )
    render_parser.add_argument(
        "--word-color",
        default="#2C2C2C",
        help="Word color hex (default: #2C2C2C, dark grey)",
    )
    render_parser.add_argument(
        "--font-path",
        type=Path,
        help="Path to .ttf font file",
    )
    render_parser.add_argument(
        "--size-scale",
        choices=["log", "linear"],
        default="log",
        help="Word size scaling method (default: log)",
    )
    render_parser.add_argument(
        "--min-font-size",
        type=int,
        default=12,
        help="Minimum font size (default: 12)",
    )
    render_parser.add_argument(
        "--max-font-size",
        type=int,
        default=120,
        help="Maximum font size (default: 120)",
    )

    # Debug options
    render_parser.add_argument(
        "--debug",
        action="store_true",
        help="Debug mode - print to terminal instead of rendering video",
    )
    render_parser.add_argument(
        "--debug-every",
        type=int,
        default=100,
        help="In debug mode, show output every N words (default: 100)",
    )

    return parser


def cmd_scrape(args) -> int:
    """Handle the scrape command."""
    print(f"Scraping articles from: {args.url}")
    print(f"Output directory: {args.output_dir}")

    files = scrape_substack(args.url, args.output_dir)

    print(f"\nDone! Scraped {len(files)} articles.")
    return 0


def cmd_render(args) -> int:
    """Handle the render command."""
    # Build config from args
    config = Config(
        max_queue_size=args.queue_size,
        max_display_words=args.display_words,
        lowercase=not args.no_lowercase,
        filter_stopwords=not args.no_stopwords,
        stopwords_file=args.stopwords_file,
        enable_stemming=args.stemming,
        min_word_length=args.min_word_length,
        words_per_frame=args.words_per_frame,
        fps=args.fps,
        frame_width=args.width,
        frame_height=args.height,
        background_color=args.background_color,
        word_color=args.word_color,
        font_path=args.font_path,
        size_scale=args.size_scale,
        min_font_size=args.min_font_size,
        max_font_size=args.max_font_size,
        output_path=args.output,
        articles_dir=args.input_dir,
    )

    # Find and sort article files
    input_dir = args.input_dir
    if not input_dir.exists():
        print(f"Error: Input directory not found: {input_dir}")
        return 1

    files = sorted(input_dir.glob("*.txt"))
    if not files:
        print(f"Error: No .txt files found in {input_dir}")
        return 1

    print(f"Found {len(files)} article files")

    # Tokenize
    print("\nTokenizing...")
    tokenizer = Tokenizer(config)
    words = tokenizer.tokenize_files(files)
    print(f"Total words after filtering: {len(words)}")

    if not words:
        print("Error: No words to process after filtering")
        return 1

    # Create engine
    cloud = TimeCloud(config)

    # Create renderer
    if args.debug:
        print("\nRunning in debug mode...")
        renderer = DebugRenderer(config, show_every=args.debug_every)
    elif args.frames_only:
        print(f"\nRendering frames to: {args.frames_only}")
        renderer = FrameRenderer(config, args.frames_only)
    else:
        print(f"\nRendering video to: {args.output}")
        renderer = VideoRenderer(config)

    # Process words
    print(f"\nProcessing {len(words)} words (batch size: {config.words_per_frame})...")

    if config.words_per_frame == 1:
        states = cloud.process_words(words)
    else:
        states = cloud.process_words_batched(words, config.words_per_frame)

    # Render
    renderer.render_all(states)

    print("\nDone!")
    return 0


def main() -> int:
    """Main entry point."""
    parser = create_parser()
    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return 1

    if args.command == "scrape":
        return cmd_scrape(args)
    elif args.command == "render":
        return cmd_render(args)
    else:
        parser.print_help()
        return 1


if __name__ == "__main__":
    sys.exit(main())
