#!/usr/bin/env python3
"""Keep long prose lines in the compliance limitation pages one sentence per line (#5048).

The limitation pages grow by a sentence per PR. A paragraph kept on one source line makes any two
PRs that each add a sentence to it conflict, however independent the sentences are, because git
merges line by line. One sentence per line (semantic line breaks) keeps such insertions apart.
CommonMark renders a newline inside a paragraph as a space, so the page reads the same.

    python3 scripts/doc_lines.py check [FILE...]   # exit 1 if a prose line is longer than --limit
    python3 scripts/doc_lines.py fix [FILE...]     # split overlong prose lines at sentence ends

With no FILE, both act on DEFAULT_FILES. `fix` splits only lines longer than --limit, and only at a
sentence end outside a code span whose next line would still be paragraph text; every other line is
left byte for byte. Table rows, headings and fenced code are never split, and `check` does not
count them.

    python3 scripts/doc_lines.py resolve FILE...  # in a conflicted merge/rebase: fix all three
                                                  # sides, then merge them line by line

A branch written before a paragraph was split conflicts with the split once when it is rebased:
its commit edits the old long line. `resolve` reads the conflicted file's base, ours and theirs
from the index, runs `fix` on each, and merges the three with `git merge-file`, so sentences the
two sides added independently merge cleanly. It writes the result and stages it; where a
conflict remains, the markers are left for a hand resolution (exit 1).
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
]

# Longest prose line `check` accepts. A sentence longer than this is split at its own `; ` and `: `.
DEFAULT_LIMIT = 1000

# Abbreviations whose period does not end a sentence.
ABBREVIATIONS = ("e.g", "i.e", "vs", "cf", "etc", "approx", "incl", "resp", "No", "Fig")

# A line that starts with one of these (after indentation) would open a block of its own rather
# than continue the paragraph: a list item, heading, quote, table row, fence, HTML block or
# setext underline.
BLOCK_START = re.compile(r"(?:[-*+]\s|#|>|\||```|~~~|<|=+\s*$|-+\s*$|\d{1,9}[.)]\s)")

FENCE = re.compile(r"^\s*(```|~~~)")


def is_table_row(line):
    return line.lstrip().startswith("|")


def is_heading(line):
    return line.lstrip().startswith("#")


def code_span_mask(text):
    """For each index, whether it lies inside an inline code span (CommonMark backtick runs)."""
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
    return inside


def sentence_ends(text, mask, separators):
    """Indices just past each candidate sentence end: a separator outside a code span, then one
    space. A period ends a sentence only when the next word starts like a sentence does; a `;` or
    `:` (asked for only inside an overlong sentence) ends a clause wherever it stands."""
    ends = []
    for i, ch in enumerate(text):
        if ch not in separators or mask[i]:
            continue
        if i + 2 >= len(text) or text[i + 1] != " " or text[i + 2] == " ":
            continue
        if ch in ";:":
            ends.append(i + 1)
            continue
        nxt = text[i + 2]
        if not (nxt.isupper() or nxt in "`[(*_\""):
            continue
        word = re.search(r"([A-Za-z.]+)$", text[: i])
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
    """Join back any piece that would open a block if it began a line of its own."""
    out = [pieces[0]]
    for piece in pieces[1:]:
        if BLOCK_START.match(piece) or not piece:
            out[-1] = out[-1] + " " + piece
        else:
            out.append(piece)
    return out


def split_line(line, limit):
    indent = line[: len(line) - len(line.lstrip(" "))]
    body = line[len(indent):]
    mask = code_span_mask(body)
    pieces = safe_pieces(split_at(body, sentence_ends(body, mask, ".?!")))
    result = []
    for piece in pieces:
        if len(indent) + len(piece) > limit:
            sub_mask = code_span_mask(piece)
            result.extend(safe_pieces(split_at(piece, sentence_ends(piece, sub_mask, ";:"))))
        else:
            result.append(piece)
    return [indent + piece for piece in result]


def prose_lines(lines):
    """Yields (index, line) for each line that is paragraph prose: not fenced code, a table row
    or a heading."""
    fenced = False
    for i, line in enumerate(lines):
        if FENCE.match(line):
            fenced = not fenced
            continue
        if fenced or is_table_row(line) or is_heading(line):
            continue
        yield i, line


def fix_text(text, limit):
    lines = text.split("\n")
    replace = {}
    for i, line in prose_lines(lines):
        if len(line) > limit:
            replace[i] = split_line(line, limit)
    out = []
    for i, line in enumerate(lines):
        out.extend(replace.get(i, [line]))
    return "\n".join(out)


def long_lines(text, limit):
    return [(i + 1, len(line)) for i, line in prose_lines(text.split("\n")) if len(line) > limit]


def resolve(path, limit):
    """Merge a conflicted `path` from its index stages after fixing each (see the module doc)."""
    stages = {}
    for stage in (1, 2, 3):
        shown = subprocess.run(["git", "show", f":{stage}:{path}"], capture_output=True, text=True)
        if shown.returncode != 0:
            print(f"{path}: not conflicted on both sides of a common base; left alone")
            return 0
        stages[stage] = fix_text(shown.stdout, limit)
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
    files = args.files or DEFAULT_FILES
    if args.command == "resolve":
        return max(resolve(path, args.limit) for path in files)
    failed = False
    for path in files:
        with open(path, encoding="utf-8") as fh:
            text = fh.read()
        if args.command == "fix":
            fixed = fix_text(text, args.limit)
            if fixed != text:
                with open(path, "w", encoding="utf-8") as fh:
                    fh.write(fixed)
        else:
            fixed = text
        for lineno, length in long_lines(fixed, args.limit):
            failed = True
            print(f"{path}:{lineno}: prose line is {length} characters (limit {args.limit}); "
                  f"run `python3 scripts/doc_lines.py fix {path}`, or break it by hand at a sentence end")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
