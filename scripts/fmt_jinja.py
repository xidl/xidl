#!/usr/bin/env python3
"""Format Jinja templates by normalizing blank lines.

Templates carry whitespace that minijinja copies verbatim into generated
files, so every line's indentation is author-owned and this tool never
re-indents anything: it collapses runs of blank lines into one blank
line, drops leading blank lines, and guarantees a single trailing
newline. A `jinja-fmt: skip` marker inside the leading comments
disables formatting for the whole file.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

SKIP_MARKER = "jinja-fmt: skip"


def format_jinja_source(source: str) -> str:
    """Return `source` with normalized blank lines."""
    if should_skip_jinja_format(source):
        return source
    return normalize_blank_lines(source)


def should_skip_jinja_format(source: str) -> bool:
    """Report whether leading comment lines contain the skip marker."""
    in_block_comment = False
    for line in source.split("\n"):
        trimmed = line.strip()
        if not trimmed:
            continue
        if in_block_comment:
            if SKIP_MARKER in trimmed:
                return True
            if "#}" in trimmed:
                in_block_comment = False
            continue
        if trimmed.startswith("##"):
            if SKIP_MARKER in trimmed:
                return True
            continue
        if trimmed.startswith("{#"):
            if SKIP_MARKER in trimmed:
                return True
            if "#}" not in trimmed:
                in_block_comment = True
            continue
        return False
    return False


def normalize_blank_lines(source: str) -> str:
    """Drop leading blanks, collapse runs, and keep one line before content."""
    out = []
    started = False
    blank_run = 0
    pending_blank = False
    for raw in source.split("\n"):
        line = raw.rstrip("\r")
        if not line.strip():
            if not started:
                continue
            blank_run += 1
            if blank_run > 1:
                continue
            pending_blank = True
            continue
        if pending_blank and line.strip() != "}":
            out.append("")
        pending_blank = False
        started = True
        blank_run = 0
        out.append(line)
    return ensure_trailing_newline("\n".join(out))


def ensure_trailing_newline(source: str) -> str:
    return source.rstrip() + "\n"


def format_files(paths: list[Path], inplace: bool) -> bool:
    """Format every path; return False when a file cannot be processed."""
    for path in paths:
        try:
            source = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError) as error:
            print(f"error: cannot read {path}: {error}", file=sys.stderr)
            return False
        formatted = format_jinja_source(source)
        if inplace:
            if formatted != source:
                path.write_text(formatted, encoding="utf-8")
        else:
            sys.stdout.write(formatted)
    return True


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="fmt_jinja",
        description="Format Jinja templates by normalizing blank lines.",
    )
    parser.add_argument("files", nargs="+", type=Path, help="template files to format")
    parser.add_argument(
        "-i",
        "--inplace",
        action="store_true",
        help="rewrite files in place instead of printing to stdout",
    )
    args = parser.parse_args(argv)
    return 0 if format_files(args.files, args.inplace) else 1


if __name__ == "__main__":
    raise SystemExit(main())
