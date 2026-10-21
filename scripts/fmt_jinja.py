#!/usr/bin/env python3
"""Format Jinja templates by normalizing indentation and blank lines.

Replacement for the jinja formatter that used to be exposed through
`xidlc fmt --lang jinja`. Control tags (`{% if %}`, `{% for %}`, ...)
open one indent level per block, lines between control tags follow
`{`/`}` brace depth, and every indent level is four spaces. A
`jinja-fmt: skip` marker inside the leading comments disables
formatting for the whole file.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

INDENT = "    "
SKIP_MARKER = "jinja-fmt: skip"

CONTROL_STARTS = frozenset(
    {"for", "if", "with", "call", "macro", "filter", "block", "trans", "autoescape"}
)
CONTROL_MIDS = frozenset({"elif", "else", "pluralize"})
CONTROL_ENDS = frozenset(
    {
        "endfor",
        "endif",
        "endblock",
        "endwith",
        "endfilter",
        "endmacro",
        "endcall",
        "endset",
        "endtrans",
        "endautoescape",
    }
)


def format_jinja_source(source: str) -> str:
    """Return `source` with normalized indentation and blank lines."""
    if should_skip_jinja_format(source):
        return source
    return normalize_blank_lines(normalize_jinja_indentation(source))


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


def normalize_jinja_indentation(source: str) -> str:
    """Re-indent every line by control-tag depth plus brace depth."""
    out = []
    control_depth = 0
    content_brace_depth = 0
    for raw in source.split("\n"):
        line = raw.rstrip("\r")
        if not line.strip():
            out.append("")
            continue
        content = strip_leading_indent(line)
        tag = parse_control_tag(content.strip())
        if tag is not None:
            statement, _ = tag
            if statement in CONTROL_ENDS or statement in CONTROL_MIDS:
                control_depth = max(control_depth - 1, 0)
            line_depth = content_brace_depth
        else:
            leading_closing = leading_closing_braces(content)
            line_depth = max(content_brace_depth - leading_closing, 0)
        out.append(INDENT * max(control_depth + line_depth, 0) + content)
        if tag is not None:
            statement, _ = tag
            if statement in CONTROL_MIDS or opens_control(tag):
                control_depth += 1
        else:
            content_brace_depth = next_content_brace_depth(content, content_brace_depth)
    return "\n".join(out)


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


def parse_control_tag(line: str) -> tuple[str, str] | None:
    """Return `(statement, body)` for a line holding one `{% %}` tag."""
    if not (line.startswith("{%") and line.endswith("%}")):
        return None
    body = line
    for prefix in ("{%-", "{%+"):
        while body.startswith(prefix):
            body = body[len(prefix) :]
    while body.startswith("{%"):
        body = body[2:]
    for suffix in ("-%}", "+%}"):
        while body.endswith(suffix):
            body = body[: -len(suffix)]
    while body.endswith("%}"):
        body = body[:-2]
    body = body.strip()
    if not body:
        return None
    if body.startswith("#"):
        body = body[1:].strip()
    words = body.split()
    if not words:
        return None
    return words[0], body


def opens_control(tag: tuple[str, str]) -> bool:
    """Report whether the tag opens a new block."""
    statement, body = tag
    return statement in CONTROL_STARTS or (statement == "set" and "=" not in body)


def next_content_brace_depth(line: str, current: int) -> int:
    """Return brace depth of `line` ignoring quotes and escape sequences."""
    depth = current
    in_string = False
    in_char = False
    index = 0
    while index < len(line):
        char = line[index]
        if in_string or in_char:
            quote = '"' if in_string else "'"
            if char == "\\" and index + 1 < len(line):
                index += 2
                continue
            if char == quote:
                in_string = False
                in_char = False
            index += 1
            continue
        if char == '"':
            in_string = True
        elif char == "'":
            in_char = True
        elif char == "{":
            depth += 1
        elif char == "}":
            depth = max(depth - 1, 0)
        index += 1
    return depth


def strip_leading_indent(line: str) -> str:
    return line.lstrip(" \t")


def leading_closing_braces(content: str) -> int:
    tail = content.lstrip()
    return len(tail) - len(tail.lstrip("}"))


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
        description="Format Jinja templates by normalizing indentation and blank lines.",
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
