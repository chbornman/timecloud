#!/usr/bin/env python3
"""Split Shakespeare's complete works into individual plays with dates."""

import re
from pathlib import Path

# Approximate dates of composition for chronological ordering
PLAY_DATES = {
    "THE TWO GENTLEMEN OF VERONA": "1589",
    "THE TAMING OF THE SHREW": "1590",
    "THE SECOND PART OF KING HENRY THE SIXTH": "1591",
    "THE THIRD PART OF KING HENRY THE SIXTH": "1591",
    "THE FIRST PART OF HENRY THE SIXTH": "1592",
    "THE TRAGEDY OF TITUS ANDRONICUS": "1592",
    "KING RICHARD THE THIRD": "1593",
    "THE COMEDY OF ERRORS": "1594",
    "LOVE'S LABOUR'S LOST": "1594",
    "THE TRAGEDY OF ROMEO AND JULIET": "1595",
    "KING RICHARD THE SECOND": "1595",
    "A MIDSUMMER NIGHT'S DREAM": "1595",
    "THE LIFE AND DEATH OF KING JOHN": "1596",
    "THE MERCHANT OF VENICE": "1596",
    "THE FIRST PART OF KING HENRY THE FOURTH": "1597",
    "THE SECOND PART OF KING HENRY THE FOURTH": "1598",
    "MUCH ADO ABOUT NOTHING": "1598",
    "THE LIFE OF KING HENRY THE FIFTH": "1599",
    "THE TRAGEDY OF JULIUS CAESAR": "1599",
    "AS YOU LIKE IT": "1599",
    "THE MERRY WIVES OF WINDSOR": "1600",
    "THE TRAGEDY OF HAMLET, PRINCE OF DENMARK": "1600",
    "TWELFTH NIGHT; OR, WHAT YOU WILL": "1601",
    "TROILUS AND CRESSIDA": "1602",
    "ALL'S WELL THAT ENDS WELL": "1602",
    "MEASURE FOR MEASURE": "1604",
    "THE TRAGEDY OF OTHELLO, THE MOOR OF VENICE": "1604",
    "THE TRAGEDY OF KING LEAR": "1605",
    "THE TRAGEDY OF MACBETH": "1606",
    "THE TRAGEDY OF ANTONY AND CLEOPATRA": "1606",
    "THE TRAGEDY OF CORIOLANUS": "1607",
    "THE LIFE OF TIMON OF ATHENS": "1607",
    "PERICLES, PRINCE OF TYRE": "1608",
    "CYMBELINE": "1609",
    "THE WINTER'S TALE": "1610",
    "THE TEMPEST": "1611",
    "KING HENRY THE EIGHTH": "1613",
    "THE TWO NOBLE KINSMEN": "1613",
    # Poetry
    "VENUS AND ADONIS": "1593",
    "THE RAPE OF LUCRECE": "1594",
    "THE SONNETS": "1609",
    "A LOVER'S COMPLAINT": "1609",
    "THE PASSIONATE PILGRIM": "1599",
    "THE PHOENIX AND THE TURTLE": "1601",
}

def slugify(title):
    """Convert title to filename-safe slug."""
    slug = title.lower()
    slug = re.sub(r'[^\w\s-]', '', slug)
    slug = re.sub(r'[-\s]+', '-', slug)
    return slug.strip('-')[:60]

def split_works(input_file, output_dir):
    """Split complete works into individual files."""
    output_dir = Path(output_dir)
    output_dir.mkdir(exist_ok=True)

    with open(input_file, 'r', encoding='utf-8') as f:
        content = f.read()

    # Find start of actual content (after Project Gutenberg header)
    start_marker = "*** START OF THE PROJECT GUTENBERG EBOOK"
    start_idx = content.find(start_marker)
    if start_idx != -1:
        content = content[start_idx:]

    # Find end marker
    end_marker = "*** END OF THE PROJECT GUTENBERG EBOOK"
    end_idx = content.find(end_marker)
    if end_idx != -1:
        content = content[:end_idx]

    # Split by play titles (all caps, starts a section)
    works_written = 0

    for title, date in sorted(PLAY_DATES.items(), key=lambda x: x[1]):
        # Find the title in the content
        # Titles appear as standalone lines in all caps
        pattern = rf'\n{re.escape(title)}\n'
        match = re.search(pattern, content)

        if not match:
            print(f"Warning: Could not find '{title}'")
            continue

        # Find the start of this work
        start = match.start()

        # Find the end (next work title or end of content)
        # Look for next title from our list
        next_start = len(content)
        for other_title in PLAY_DATES.keys():
            if other_title == title:
                continue
            other_pattern = rf'\n{re.escape(other_title)}\n'
            other_match = re.search(other_pattern, content[start + len(title) + 2:])
            if other_match:
                pos = start + len(title) + 2 + other_match.start()
                if pos < next_start:
                    next_start = pos

        # Extract the work
        work_text = content[start:next_start].strip()

        if len(work_text) < 1000:
            print(f"Warning: '{title}' seems too short ({len(work_text)} chars)")
            continue

        # Write to file with date prefix
        slug = slugify(title)
        filename = f"{date}_{slug}.txt"
        filepath = output_dir / filename

        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(work_text)

        print(f"Wrote: {filename} ({len(work_text):,} chars)")
        works_written += 1

    print(f"\nTotal works written: {works_written}")

if __name__ == "__main__":
    split_works("shakespeare_complete.txt", "plays")
