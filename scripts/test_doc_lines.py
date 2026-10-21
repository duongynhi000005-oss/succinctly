#!/usr/bin/env python3
"""Unit tests for scripts/doc_lines.py (#5048)."""

import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import doc_lines  # noqa: E402


class SplitTests(unittest.TestCase):
    def fix(self, text, limit=40):
        return doc_lines.fix_text(text, limit)

    def test_splits_an_overlong_line_at_sentence_ends_keeping_its_indent(self):
        line = "   First sentence here. Second one follows. Third ends it."
        self.assertEqual(
            self.fix(line),
            "   First sentence here.\n   Second one follows.\n   Third ends it.",
        )

    def test_leaves_short_lines_byte_for_byte(self):
        text = "Short. Lines. Stay.\n  Indented. Too."
        self.assertEqual(self.fix(text, limit=100), text)

    def test_does_not_split_inside_a_code_span(self):
        line = "Read `a. B. c` as one span here. Then the next sentence."
        self.assertEqual(
            self.fix(line),
            "Read `a. B. c` as one span here.\nThen the next sentence.",
        )

    def test_double_backtick_span_hides_a_single_backtick(self):
        line = "Read ``a ` b. C`` here as code. Then the next sentence."
        self.assertEqual(self.fix(line), "Read ``a ` b. C`` here as code.\nThen the next sentence.")

    def test_needs_a_sentence_start_after_the_period(self):
        line = "Calls .a. then .b. and keeps going past forty chars here."
        self.assertEqual(self.fix(line), line)

    def test_abbreviations_and_numbers_do_not_end_a_sentence(self):
        line = "See e.g. Foo and jq 1.7.1 here. Then version 1. Next is fine."
        out = self.fix(line).split("\n")
        self.assertEqual(out[0], "See e.g. Foo and jq 1.7.1 here.")
        self.assertEqual(out[1], "Then version 1. Next is fine.")

    def test_never_starts_a_line_with_a_block_marker(self):
        for nxt in ("- item", "1. item", "# head", "> quote", "| cell", "+ x"):
            line = f"An opening sentence that runs long enough. {nxt} continues the text here."
            self.assertNotIn("\n" + nxt, self.fix(line), nxt)

    def test_splits_an_overlong_sentence_at_semicolons_and_colons(self):
        line = "One clause runs here; another clause runs there: and a last clause ends it."
        self.assertEqual(
            self.fix(line, limit=30),
            "One clause runs here;\nanother clause runs there:\nand a last clause ends it.",
        )

    def test_skips_tables_headings_and_fences(self):
        text = "\n".join([
            "| A long cell. With sentences. In a table row that is long. |",
            "# A long heading. With sentences. That is long enough here.",
            "```",
            "A long code line. With sentences. That is long enough here.",
            "```",
        ])
        self.assertEqual(self.fix(text), text)
        self.assertEqual(doc_lines.long_lines(text, 40), [])

    def test_check_reports_overlong_prose_lines(self):
        text = "short\n" + "x" * 50 + "\nshort"
        self.assertEqual(doc_lines.long_lines(text, 40), [(2, 50)])

    def test_fix_is_idempotent(self):
        line = "   First sentence here. Second one follows. Third ends it."
        once = self.fix(line)
        self.assertEqual(self.fix(once), once)


class ResolveTests(unittest.TestCase):
    """A branch that appended to a long line before it was split rebases onto the split."""

    SCRIPT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "doc_lines.py")

    def git(self, *args):
        return subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false",
                               *args], cwd=self.repo, capture_output=True, text=True)

    def write(self, text):
        with open(os.path.join(self.repo, "lim.md"), "w", encoding="utf-8") as fh:
            fh.write(text)

    def read(self):
        with open(os.path.join(self.repo, "lim.md"), encoding="utf-8") as fh:
            return fh.read()

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = self.tmp.name
        self.git("init", "-q", "-b", "main")
        self.long = "   Alpha one is here. Beta two is here. Gamma three is here. Delta four is here."
        self.write("# Title\n\n" + self.long + "\n")
        self.git("add", ".")
        self.git("commit", "-qm", "base")
        # The branch inserts a sentence after Alpha in the old long line.
        self.git("checkout", "-qb", "topic")
        self.write("# Title\n\n" + self.long.replace("here. Beta", "here. Branch added this. Beta") + "\n")
        self.git("commit", "-qam", "branch")
        # main splits the line and then inserts its own sentence after Gamma.
        self.git("checkout", "-q", "main")
        subprocess.run([sys.executable, self.SCRIPT, "fix", "--limit", "40", "lim.md"], cwd=self.repo, check=True)
        text = self.read().replace("Gamma three is here.\n", "Gamma three is here.\n   Main added this.\n")
        self.write(text)
        self.git("commit", "-qam", "split and insert")

    def tearDown(self):
        self.tmp.cleanup()

    def test_resolve_merges_independent_insertions_after_a_split(self):
        self.git("checkout", "-q", "topic")
        self.assertNotEqual(self.git("rebase", "main").returncode, 0, "the rebase should conflict")
        done = subprocess.run([sys.executable, self.SCRIPT, "resolve", "--limit", "40", "lim.md"],
                              cwd=self.repo, capture_output=True, text=True)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(
            self.read(),
            "# Title\n\n   Alpha one is here.\n   Branch added this.\n   Beta two is here.\n"
            "   Gamma three is here.\n   Main added this.\n   Delta four is here.\n",
        )
        self.assertEqual(self.git("diff", "--cached", "--name-only", "--diff-filter=U").stdout, "")


if __name__ == "__main__":
    unittest.main()
