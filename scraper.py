"""Substack article scraper."""

import re
import time
from datetime import datetime
from pathlib import Path
from urllib.parse import urljoin, urlparse

import requests
from bs4 import BeautifulSoup


def get_archive_urls(base_url: str) -> list[dict]:
    """Fetch all article URLs from a Substack archive using the API.

    Args:
        base_url: Base URL of the Substack (e.g., 'https://example.substack.com')

    Returns:
        List of dicts with 'url', 'title', 'date' keys, sorted by date ascending.
    """
    base_url = base_url.rstrip("/")
    api_url = f"{base_url}/api/v1/archive"
    articles = []

    offset = 0
    limit = 50

    while True:
        url = f"{api_url}?sort=new&search=&offset={offset}&limit={limit}"
        print(f"Fetching archive (offset={offset})...")

        response = requests.get(url, timeout=30)
        response.raise_for_status()

        posts = response.json()

        if not posts:
            break

        for post in posts:
            post_url = f"{base_url}/p/{post['slug']}"
            articles.append({
                "url": post_url,
                "slug": post["slug"],
                "title": post.get("title", "Untitled"),
                "date": parse_date(post.get("post_date")),
            })

        offset += len(posts)
        time.sleep(0.5)  # Be polite

    articles.sort(key=lambda x: x["date"] or datetime.min)

    return articles


def parse_date(date_str: str | None) -> datetime | None:
    """Parse various date string formats."""
    if not date_str:
        return None

    # Common formats
    formats = [
        "%Y-%m-%dT%H:%M:%S.%fZ",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%d",
        "%B %d, %Y",
        "%b %d, %Y",
    ]

    for fmt in formats:
        try:
            return datetime.strptime(date_str.strip(), fmt)
        except ValueError:
            continue

    return None


def fetch_with_retry(url: str, max_retries: int = 5) -> requests.Response:
    """Fetch a URL with exponential backoff on rate limiting.

    Args:
        url: URL to fetch.
        max_retries: Maximum number of retries on 429 errors.

    Returns:
        Response object.

    Raises:
        requests.HTTPError: If request fails after all retries.
    """
    delay = 2.0

    for attempt in range(max_retries + 1):
        response = requests.get(url, timeout=30)

        if response.status_code == 429:
            if attempt < max_retries:
                print(f"  Rate limited, waiting {delay:.0f}s...")
                time.sleep(delay)
                delay *= 2  # Exponential backoff
                continue

        response.raise_for_status()
        return response

    # Should not reach here, but just in case
    response.raise_for_status()
    return response


def scrape_article(url: str) -> str:
    """Scrape the body text from a single Substack article.

    Args:
        url: Full URL of the article.

    Returns:
        Plain text content of the article body.
    """
    print(f"Scraping: {url}")

    response = fetch_with_retry(url)

    soup = BeautifulSoup(response.text, "html.parser")

    # Substack article body is typically in a div with class containing 'body'
    # or in the main content area
    body = soup.select_one('div[class*="body"]')
    if not body:
        body = soup.select_one("article")
    if not body:
        body = soup.select_one('div[class*="post-content"]')
    if not body:
        # Fallback: get main content
        body = soup.select_one("main")

    if not body:
        print(f"  Warning: Could not find article body for {url}")
        return ""

    # Remove script, style, and other non-content elements
    for tag in body.find_all(["script", "style", "nav", "footer", "button"]):
        tag.decompose()

    # Extract text
    text = body.get_text(separator=" ", strip=True)

    # Clean up whitespace
    text = re.sub(r"\s+", " ", text)

    return text


def slugify(text: str) -> str:
    """Convert text to a filesystem-safe slug."""
    text = text.lower()
    text = re.sub(r"[^\w\s-]", "", text)
    text = re.sub(r"[-\s]+", "-", text)
    return text.strip("-")[:50]


def scrape_substack(base_url: str, output_dir: Path) -> list[Path]:
    """Scrape all articles from a Substack and save to files.

    Args:
        base_url: Base URL of the Substack.
        output_dir: Directory to save article text files.

    Returns:
        List of paths to created files.
    """
    output_dir.mkdir(parents=True, exist_ok=True)

    articles = get_archive_urls(base_url)
    print(f"Found {len(articles)} articles")

    created_files = []

    skipped = 0
    for i, article in enumerate(articles, 1):
        # Create filename with date prefix for sorting
        date_prefix = "0000-00-00"
        if article["date"]:
            date_prefix = article["date"].strftime("%Y-%m-%d")

        slug = article["slug"]
        filename = f"{date_prefix}_{slug}.txt"
        filepath = output_dir / filename

        # Skip if already exists
        if filepath.exists():
            skipped += 1
            continue

        print(f"\n[{i}/{len(articles)}] {article['title']}")

        text = scrape_article(article["url"])
        if not text:
            continue

        filepath.write_text(text, encoding="utf-8")
        created_files.append(filepath)
        print(f"  Saved: {filename} ({len(text)} chars)")

        time.sleep(1.5)  # Be polite to avoid rate limiting

    if skipped:
        print(f"\nSkipped {skipped} already downloaded articles.")

    return created_files


def get_cover_images_from_api(base_url: str) -> dict[str, str]:
    """Fetch cover images for all articles from the Substack API.

    Args:
        base_url: Base URL of the Substack (e.g., 'https://example.substack.com')

    Returns:
        Dict mapping article slug to cover image URL.
    """
    base_url = base_url.rstrip("/")
    api_url = f"{base_url}/api/v1/archive"
    images_by_slug = {}

    offset = 0
    limit = 50

    while True:
        url = f"{api_url}?sort=new&offset={offset}&limit={limit}"
        print(f"Fetching archive for images (offset={offset})...")

        response = requests.get(url, timeout=30)
        response.raise_for_status()

        posts = response.json()

        if not posts:
            break

        for post in posts:
            slug = post.get("slug")
            cover = post.get("cover_image")
            if slug and cover:
                images_by_slug[slug] = cover

        offset += len(posts)
        time.sleep(0.5)

    print(f"Found cover images for {len(images_by_slug)} articles")
    return images_by_slug


def clean_image_url(url: str) -> str:
    """Extract the raw S3 image URL from a Substack CDN URL.

    Substack wraps images in their CDN like:
    https://substackcdn.com/image/fetch/...params.../https%3A%2F%2Fsubstack-post-media...

    This extracts the underlying S3 URL.
    """
    if "substack-post-media.s3.amazonaws.com" in url and "substackcdn.com" not in url:
        return url

    # Try to extract the S3 URL from CDN wrapper
    match = re.search(
        r'https%3A%2F%2Fsubstack-post-media\.s3\.amazonaws\.com[^"\'>\s&]+',
        url
    )
    if match:
        from urllib.parse import unquote
        return unquote(match.group(0))

    return url


def download_image(url: str, output_path: Path) -> bool:
    """Download an image to the specified path.

    Args:
        url: URL of the image.
        output_path: Path to save the image.

    Returns:
        True if successful, False otherwise.
    """
    try:
        response = fetch_with_retry(url)
        output_path.write_bytes(response.content)
        return True
    except Exception as e:
        print(f"  Error downloading {url}: {e}")
        return False


def scrape_images(
    base_url: str,
    output_dir: Path,
) -> dict[str, Path]:
    """Scrape cover images from a Substack and save next to article text files.

    Args:
        base_url: Base URL of the Substack.
        output_dir: Directory containing article .txt files (images saved alongside).

    Returns:
        Dict mapping article slug to downloaded image path.
    """
    output_dir.mkdir(parents=True, exist_ok=True)

    # Get cover images from API (gets all articles, not just recent 20)
    images_by_slug = get_cover_images_from_api(base_url)

    # Build mapping from slug to article file (to get the date prefix)
    slug_to_file_prefix = {}
    for txt_file in output_dir.glob("*.txt"):
        # Filename format: YYYY-MM-DD_slug.txt
        parts = txt_file.stem.split("_", 1)
        if len(parts) == 2:
            date_prefix, file_slug = parts
            slug_to_file_prefix[file_slug] = f"{date_prefix}_{file_slug}"

    downloaded = {}
    total_images = 0

    for slug, cover_url in images_by_slug.items():
        # Get the file prefix (date_slug) for this article
        file_prefix = slug_to_file_prefix.get(slug)
        if not file_prefix:
            print(f"  Warning: No matching article file for slug '{slug}', skipping")
            continue

        clean_url = clean_image_url(cover_url)
        ext = Path(urlparse(clean_url).path).suffix or ".jpg"
        cover_path = output_dir / f"{file_prefix}{ext}"

        if not cover_path.exists():
            print(f"Downloading cover for '{slug}'...")
            if download_image(clean_url, cover_path):
                downloaded[slug] = cover_path
                total_images += 1
            time.sleep(0.3)

    print(f"\nDownloaded {total_images} cover images")
    return downloaded


if __name__ == "__main__":
    import sys

    if len(sys.argv) < 2:
        print("Usage: python scraper.py <substack_url> [--images]")
        print("       python scraper.py <substack_url> --images-only")
        sys.exit(1)

    base_url = sys.argv[1]
    articles_path = Path("articles")

    if "--images-only" in sys.argv:
        # Only scrape images (saves alongside existing .txt files)
        scrape_images(base_url, articles_path)
    elif "--images" in sys.argv:
        # Scrape articles and images
        scrape_substack(base_url, articles_path)
        scrape_images(base_url, articles_path)
    else:
        # Just scrape articles
        scrape_substack(base_url, articles_path)
