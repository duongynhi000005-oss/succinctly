#!/usr/bin/env python3
"""Keep long prose lines in the compliance limitation pages one sentence per line (#5048).

The limitation pages grow by a sentence per PR. A paragraph kept on one source line makes any two
PRs that each add a sentence to it conflict, however independent the sentences are, because git
merges line by line. One sentence per line (semantic line breaks) keeps such insertions apart.
CommonMark renders a newline inside a paragraph as a space, so the page reads the same.

    python3 scripts/doc_lines.py check [FILE...]    # exit 1 if a prose line is longer than --limit
    python3 scripts/doc_lines.py fix [FILE...]      # split overlong prose lines at sentence ends
    python3 scripts/doc_lines.py resolve FILE...    # in a conflicted merge/rebase: fix all three
                                                    # sides, then merge them line by line

With no FILE, `check` and `fix` act on DEFAULT_FILES.

`fix` splits only a line longer than --limit that continues a paragraph: the line before it is
text of the same paragraph, so the line cannot open a block of its own (a list item, quote,
heading, table row, fence, HTML block or indented code), and each piece keeps the line's
indentation, so it continues the paragraph too. It splits at sentence ends outside code spans and
angle-bracket link destinations, and never where a piece would open a block (or be a thematic
break) if it began a line. Every other line is left byte for byte. `check` counts every line
outside fenced code, tables and headings, so a long line `fix` cannot split is still reported, to
be broken by hand.

A branch written before a paragraph was split conflicts with the split once when it is rebased:
its commit edits the old long line. `resolve` reads the conflicted file's base, ours and theirs
from the index, runs `fix` on each, and merges the three with `git merge-file`, so sentences the
two sides added independently merge cleanly. It overwrites the working copy (any hand edits to
the conflict are lost) and stages the result; where a conflict remains, the markers are left for
a hand resolution (exit 1).
"""

import argparse
import os
import re
import subprocess
import sys
import tempfile

DEFAULT_FILES = [
    "docs/compliance/jq/limitations.md",
    "docs/compliance/yq/limitations.md",
    "docs/compliance/yaml/limitations.md",
]

# Longest prose line `check` accepts. A sentence longer than this is split at its own `; ` and `: `.
DEFAULT_LIMIT = 1000

# Abbreviations whose period does not end a sentence.
ABBREVIATIONS = ("e.g", "i.e", "vs", "cf", "etc", "approx", "incl", "resp", "No", "Fig")

# A piece that starts with one of these (after indentation) would open a block of its own rather
# than continue the paragraph: a list item, heading, quote, table row, fence, HTML block, link
# reference definition or setext underline.
BLOCK_START = re.compile(r"(?:[-*+](?:\s|$)|#{1,6}(?:\s|$)|>|\||```|~~~|<|\[[^\]]*\]:|=+\s*$|-+\s*$|\d{1,9}[.)](?:\s|$))")

# A thematic break: three or more `*`, `-` or `_`, optionally spaced.
THEMATIC_BREAK = re.compile(r"^ {0,3}([*_-])(?:[ \t]*\1){2,}[ \t]*$")

# Container prefixes a fence may sit behind (list items, block quotes), then the fence itself.
FENCE_OPEN = re.compile(r"^(?:[ \t]*(?:>|[-*+](?=[ \t])|\d{1,9}[.)](?=[ \t])))*[ \t]*(`{3,}|~{3,})(.*)$")
HEADING = re.compile(r"^[ \t]{0,3}#{1,6}(?:[ \t]|$)")


def fence_open(line):
    """`(char, length)` of the fence `line` opens, or None. A backtick fence's info string holds no
    backtick (```` ```a``` x ```` is an inline code span)."""
    m = FENCE_OPEN.match(line)
    if not m:
        return None
    run, info = m.group(1), m.group(2)
    if run[0] == "`" and "`" in info:
        return None
    return run[0], len(run)


def fence_closes(line, fence):
    char, length = fence
    stripped = line.strip(" \t>")
    return (len(stripped) >= length and set(stripped) == {char})


def classify(lines):
    """Yields (index, kind) for each line: `fence` (a fence line or fenced code), `table`, `heading`,
    `html` (an HTML block), `blank`, `continuation` (paragraph text whose previous line is text of
    the same paragraph) or `other` (any other line)."""
    fence = None
    html = False
    in_paragraph = False
    for i, line in enumerate(lines):
        if fence is not None:
            if fence_closes(line, fence):
                fence = None
            yield i, "fence"
            continue
        stripped = line.strip()
        if not stripped:
            html = False
            in_paragraph = False
            yield i, "blank"
            continue
        opened = fence_open(line)
        if opened:
            fence = opened
            in_paragraph = False
            yield i, "fence"
            continue
        if html:
            yield i, "html"
            continue
        if stripped.startswith("|"):
            in_paragraph = False
            yield i, "table"
            continue
        if HEADING.match(line):
            in_paragraph = False
            yield i, "heading"
            continue
        body = line.lstrip(" \t")
        if body.startswith("<"):
            # Conservatively an HTML block to the next blank line, whatever the tag.
            html = True
            in_paragraph = False
            yield i, "html"
            continue
        indent = len(line) - len(body)
        if not in_paragraph and (indent >= 4 or "\t" in line[:indent]):
            # Maybe indented code (or a list item's own paragraph): never read as a paragraph the
            # next line continues.
            yield i, "other"
            continue
        if THEMATIC_BREAK.match(line) or (in_paragraph and re.match(r"^ {0,3}(=+|-+)[ \t]*$", line)):
            in_paragraph = False
            yield i, "other"
            continue
        if in_paragraph and not BLOCK_START.match(body):
            yield i, "continuation"
            continue
        # A paragraph's first line, a list item or a quote: text the next line may continue.
        in_paragraph = True
        yield i, "other"


def masked_spans(text):
    """For each index, whether it lies inside an inline code span (CommonMark backtick runs) or an
    angle-bracket link destination (`](<...>)`), where a newline would change what renders."""
    inside = [False] * len(text)
    i = 0
    n = len(text)
    while i < n:
        if text[i] == "\\" and i + 1 < n:
            i += 2
            continue
        if text[i] != "`":
            i += 1
            continue
        j = i
        while j < n and text[j] == "`":
            j += 1
        run = j - i
        # The closing run is the next backtick run of exactly the same length.
        k = j
        close = -1
        while k < n:
            if text[k] != "`":
                k += 1
                continue
            m = k
            while m < n and text[m] == "`":
                m += 1
            if m - k == run:
                close = k
                break
            k = m
        if close < 0:
            # An unmatched run is literal backticks.
            i = j
            continue
        for p in range(i, close + run):
            inside[p] = True
        i = close + run
    for m in re.finditer(r"\]\(<[^>\n]*>", text):
        for p in range(m.start(), m.end()):
            inside[p] = True
    return inside


def sentence_ends(text, mask, separators):
    """Indices just past each candidate sentence end: a separator outside a masked span, then one
    space. A period ends a sentence only when the next word starts like a sentence does; a `;` or
    `:` (asked for only inside an overlong sentence) ends a clause wherever it stands."""
    ends = []
    for i, ch in enumerate(text):
        if ch not in separators or mask[i]:
            continue
        if i + 2 >= len(text) or text[i + 1] != " " or text[i + 2] == " " or mask[i + 1]:
            continue
        if ch in ";:":
            ends.append(i + 1)
            continue
        nxt = text[i + 2]
        if not (nxt.isupper() or nxt in "`[(*_\""):
            continue
        word = re.search(r"([A-Za-z.]+)$", text[:i])
        if word and word.group(1) in ABBREVIATIONS:
            continue
        # `1.7.1. Next` ends at the last period; `1. Next` mid-sentence is an ordinal: keep it.
        if text[i - 1].isdigit() and re.search(r"(?:^|\s)\d+$", text[:i]):
            continue
        ends.append(i + 1)
    return ends


def split_at(text, ends):
    pieces = []
    start = 0
    for end in ends:
        pieces.append(text[start:end])
        start = end + 1  # drop the one space after the separator
    pieces.append(text[start:])
    return pieces


def safe_pieces(pieces):
    """Join back any piece that would open a block, or be a thematic break, if it began a line."""
    out = [pieces[0]]
    for piece in pieces[1:]:
        if not piece or BLOCK_START.match(piece) or THEMATIC_BREAK.match(piece):
            out[-1] = out[-1] + " " + piece
        else:
            out.append(piece)
    return out


def split_line(line, limit):
    indent = line[: len(line) - len(line.lstrip(" "))]
    body = line[len(indent):]
    pieces = safe_pieces(split_at(body, sentence_ends(body, masked_spans(body), ".?!")))
    result = []
    for piece in pieces:
        if len(indent) + len(piece) > limit:
            result.extend(safe_pieces(split_at(piece, sentence_ends(piece, masked_spans(piece), ";:"))))
        else:
            result.append(piece)
    return [indent + piece for piece in result]


def fix_text(text, limit):
    lines = text.split("\n")
    replace = {}
    for i, kind in classify(lines):
        if kind == "continuation" and len(lines[i]) > limit:
            replace[i] = split_line(lines[i], limit)
    out = []
    for i, line in enumerate(lines):
        out.extend(replace.get(i, [line]))
    return "\n".join(out)


def long_lines(text, limit):
    lines = text.split("\n")
    return [
        (i + 1, len(lines[i]))
        for i, kind in classify(lines)
        if kind not in ("fence", "table", "heading") and len(lines[i]) > limit
    ]


def has_conflict_markers(path):
    with open(path, encoding="utf-8") as fh:
        return any(line.startswith(("<<<<<<< ", ">>>>>>> ")) for line in fh)


def resolve(path, limit):
    """Merge a conflicted `path` from its index stages after fixing each (see the module doc)."""
    if not os.path.exists(path):
        print(f"{path}: no such file", file=sys.stderr)
        return 2
    stages = {}
    for stage in (1, 2, 3):
        shown = subprocess.run(["git", "show", f":{stage}:{path}"], capture_output=True, text=True)
        if shown.returncode == 0:
            stages[stage] = fix_text(shown.stdout, limit)
    if len(stages) != 3:
        if stages or has_conflict_markers(path):
            print(f"{path}: conflicted without a common base (stages {sorted(stages)}); resolve by hand")
            return 1
        print(f"{path}: not conflicted; left alone")
        return 0
    with tempfile.TemporaryDirectory() as tmp:
        files = {}
        for stage, name in ((1, "base"), (2, "ours"), (3, "theirs")):
            files[stage] = os.path.join(tmp, name)
            with open(files[stage], "w", encoding="utf-8") as fh:
                fh.write(stages[stage])
        merged = subprocess.run(
            ["git", "merge-file", "-p", "-L", "ours", "-L", "base", "-L", "theirs",
             files[2], files[1], files[3]],
            capture_output=True, text=True,
        )
    if merged.returncode < 0 or merged.returncode > 127:
        print(f"{path}: git merge-file failed: {merged.stderr.strip()}", file=sys.stderr)
        return 2
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(merged.stdout)
    if merged.returncode != 0:
        print(f"{path}: {merged.returncode} conflict(s) left after fixing all three sides; resolve by hand")
        return 1
    subprocess.run(["git", "add", path], check=True)
    print(f"{path}: merged and staged")
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("command", choices=["check", "fix", "resolve"])
    ap.add_argument("files", nargs="*")
    ap.add_argument("--limit", type=int, default=DEFAULT_LIMIT)
    args = ap.parse_intermixed_args(argv)
    if args.command == "resolve":
        if not args.files:
            ap.error("resolve needs the conflicted FILE(s)")
        return max(resolve(path, args.limit) for path in args.files)
    failed = False
    for path in args.files or DEFAULT_FILES:
        with open(path, encoding="utf-8") as fh:
            text = fh.read()
        if args.command == "fix":
            fixed = fix_text(text, args.limit)
            if fixed != text:
                with open(path, "w", encoding="utf-8") as fh:
                    fh.write(fixed)
            hint = "`fix` found no sentence end to split it at; break it by hand"
        else:
            fixed = text
            hint = f"run `python3 scripts/doc_lines.py fix {path}`, or break it by hand at a sentence end"
        for lineno, length in long_lines(fixed, args.limit):
            failed = True
            print(f"{path}:{lineno}: prose line is {length} characters (limit {args.limit}); {hint}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
