"""Substack article scraper."""

import re
import time
from datetime import datetime
from pathlib import Path
from urllib.parse import urljoin

import requests
from bs4 import BeautifulSoup


def get_archive_urls(base_url: str) -> list[dict]:
    """Fetch all article URLs from a Substack archive.

    Args:
        base_url: Base URL of the Substack (e.g., 'https://example.substack.com')

    Returns:
        List of dicts with 'url', 'title', 'date' keys, sorted by date ascending.
    """
    archive_url = urljoin(base_url.rstrip("/") + "/", "archive")
    articles = []

    # Substack archive pages can have pagination, but most small substacks
    # show all posts on a single page. We'll handle basic pagination.
    page = 1
    while True:
        url = f"{archive_url}?page={page}" if page > 1 else archive_url
        print(f"Fetching archive page {page}...")

        response = requests.get(url, timeout=30)
        response.raise_for_status()

        soup = BeautifulSoup(response.text, "html.parser")

        # Find article links - Substack uses various class patterns
        post_links = soup.select('a[data-testid="post-preview-title"]')
        if not post_links:
            # Try alternative selector
            post_links = soup.select("a.post-preview-title")
        if not post_links:
            # Try finding links within post preview containers
            post_links = soup.select('div[class*="post-preview"] a[href*="/p/"]')

        if not post_links:
            break

        found_new = False
        for link in post_links:
            href = link.get("href", "")
            if "/p/" not in href:
                continue

            full_url = urljoin(base_url, href)
            if any(a["url"] == full_url for a in articles):
                continue

            found_new = True
            title = link.get_text(strip=True)

            # Try to find the date - look in parent containers
            date_str = None
            parent = link.find_parent(class_=re.compile(r"post-preview"))
            if parent:
                time_elem = parent.find("time")
                if time_elem:
                    date_str = time_elem.get("datetime") or time_elem.get_text(strip=True)

            articles.append({
                "url": full_url,
                "title": title,
                "date_str": date_str,
            })

        if not found_new:
            break

        page += 1
        time.sleep(0.5)  # Be polite

    # Parse dates and sort
    for article in articles:
        article["date"] = parse_date(article.get("date_str"))

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


def scrape_article(url: str) -> str:
    """Scrape the body text from a single Substack article.

    Args:
        url: Full URL of the article.

    Returns:
        Plain text content of the article body.
    """
    print(f"Scraping: {url}")

    response = requests.get(url, timeout=30)
    response.raise_for_status()

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

    for i, article in enumerate(articles, 1):
        print(f"\n[{i}/{len(articles)}] {article['title']}")

        text = scrape_article(article["url"])
        if not text:
            continue

        # Create filename with date prefix for sorting
        date_prefix = "0000-00-00"
        if article["date"]:
            date_prefix = article["date"].strftime("%Y-%m-%d")

        slug = slugify(article["title"])
        filename = f"{date_prefix}_{slug}.txt"
        filepath = output_dir / filename

        filepath.write_text(text, encoding="utf-8")
        created_files.append(filepath)
        print(f"  Saved: {filename} ({len(text)} chars)")

        time.sleep(0.5)  # Be polite

    return created_files


if __name__ == "__main__":
    import sys

    if len(sys.argv) < 2:
        print("Usage: python scraper.py <substack_url>")
        sys.exit(1)

    scrape_substack(sys.argv[1], Path("articles"))
