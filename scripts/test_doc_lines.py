#!/usr/bin/env python3
"""Unit tests for scripts/doc_lines.py (#5048)."""

import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import doc_lines  # noqa: E402

SCRIPT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "doc_lines.py")

# A paragraph's first line: the tested line after it continues the paragraph.
LEAD = "Lead line.\n"


class SplitTests(unittest.TestCase):
    def fix(self, text, limit=40):
        return doc_lines.fix_text(text, limit)

    def assert_unchanged(self, text, limit=30):
        self.assertEqual(self.fix(text, limit), text)

    def test_splits_an_overlong_continuation_at_sentence_ends_keeping_its_indent(self):
        text = "- Item.\n   First sentence here. Second one follows. Third ends it."
        self.assertEqual(
            self.fix(text),
            "- Item.\n   First sentence here.\n   Second one follows.\n   Third ends it.",
        )

    def test_leaves_short_lines_byte_for_byte(self):
        self.assert_unchanged(LEAD + "Short. Lines. Stay.\n  Indented. Too.", limit=100)

    def test_does_not_split_inside_a_code_span(self):
        self.assertEqual(
            self.fix(LEAD + "Read `a. B. c` as one span here. Then the next sentence."),
            LEAD + "Read `a. B. c` as one span here.\nThen the next sentence.",
        )

    def test_double_backtick_span_hides_a_single_backtick(self):
        self.assertEqual(
            self.fix(LEAD + "Read ``a ` b. C`` here as code. Then the next sentence."),
            LEAD + "Read ``a ` b. C`` here as code.\nThen the next sentence.",
        )

    def test_does_not_split_inside_an_angle_bracket_link_destination(self):
        self.assert_unchanged(LEAD + "See [x](<a b. C d>) then more text running long.")

    def test_needs_a_sentence_start_after_the_period(self):
        self.assert_unchanged(LEAD + "Calls .a. then .b. and keeps going past forty chars here.")

    def test_abbreviations_and_numbers_do_not_end_a_sentence(self):
        out = self.fix(LEAD + "See e.g. Foo and jq 1.7.1 here. Then version 1. Next is fine.").split("\n")
        self.assertEqual(out[1], "See e.g. Foo and jq 1.7.1 here.")
        self.assertEqual(out[2], "Then version 1. Next is fine.")

    def test_never_starts_a_line_with_a_block_marker_or_thematic_break(self):
        for nxt in ("- item text", "1. item text", "# head text", "> quote text", "| cell text", "+ x text",
                    "***", "___", "_ _ _", "- - -", "<div> text", "[r]: /u"):
            text = LEAD + f"An opening sentence that runs long enough. {nxt}"
            out = self.fix(text)
            self.assertNotIn("\n" + nxt, out, nxt)

    def test_splits_an_overlong_sentence_at_semicolons_and_colons(self):
        self.assertEqual(
            self.fix(LEAD + "One clause runs here; another clause runs there: and a last clause ends it.", limit=30),
            LEAD + "One clause runs here;\nanother clause runs there:\nand a last clause ends it.",
        )

    def test_does_not_split_a_paragraphs_first_line_or_a_container_line(self):
        for text in (
            "A first line of a paragraph. Second sentence of it here.",
            "- # Heading one is long. Second sentence of it.",
            "> Quoted line runs long here. Second sentence of it.",
            "Lead.\n> Quoted line runs long here. Second sentence of it.",
            "> | a cell. With sentences. | b |",
            "Lead.\n- An item runs long enough here. Second sentence of it.",
        ):
            self.assert_unchanged(text)

    def test_does_not_split_indented_code(self):
        for text in (
            "    code line one is long. Second code sentence here.\n    more code. Still code here ok.",
            "\tcode line one is long. Second code sentence here.\n\tmore code. Still code here ok.",
            "- item\n\n      code line one is long. Second code sentence.\n      more code. Still code here ok.",
        ):
            self.assert_unchanged(text)

    def test_tracks_fences_by_character_and_length(self):
        for text in (
            "````md\n```\ncode sentence one is here. Second sentence.\n```\n````",
            "```\n~~~\ncode sentence one is here. Second sentence.\n```",
            "- ```\n  code sentence one is here. Second sentence.\n  ```",
            "> ```\n> code sentence one is here. Second sentence.\n> ```",
        ):
            self.assert_unchanged(text)
            self.assertEqual(doc_lines.long_lines(text, 30), [])

    def test_inline_triple_backticks_do_not_open_a_fence(self):
        text = "Lead ```a``` x.\nAnd prose follows here long. With a second sentence."
        self.assertEqual(
            self.fix(text),
            "Lead ```a``` x.\nAnd prose follows here long.\nWith a second sentence.",
        )

    def test_does_not_split_html_blocks(self):
        self.assert_unchanged("<pre>\nPreformatted text is long here. With a second sentence.\n</pre>")

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

    def test_check_counts_a_line_that_starts_with_an_issue_reference(self):
        text = "#1193's fix was a long sentence in a paragraph."
        self.assertEqual(doc_lines.long_lines(text, 30), [(1, len(text))])

    def test_check_reports_overlong_prose_lines(self):
        text = "short\n" + "x" * 50 + "\nshort"
        self.assertEqual(doc_lines.long_lines(text, 40), [(2, 50)])

    def test_fix_is_idempotent(self):
        text = "- Item.\n   First sentence here. Second one follows. Third ends it."
        once = self.fix(text)
        self.assertEqual(self.fix(once), once)


class ResolveTests(unittest.TestCase):
    """A branch that appended to a long line before it was split rebases onto the split."""

    def git(self, *args):
        return subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false",
                               *args], cwd=self.repo, capture_output=True, text=True)

    def run_script(self, *args):
        return subprocess.run([sys.executable, SCRIPT, *args], cwd=self.repo, capture_output=True, text=True)

    def write(self, text, name="lim.md"):
        with open(os.path.join(self.repo, name), "w", encoding="utf-8") as fh:
            fh.write(text)

    def read(self):
        with open(os.path.join(self.repo, "lim.md"), encoding="utf-8") as fh:
            return fh.read()

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = self.tmp.name
        self.git("init", "-q", "-b", "main")
        self.long = "   Alpha one is here. Beta two is here. Gamma three is here. Delta four is here."
        self.write("# Title\n\n- Item.\n" + self.long + "\n")
        self.git("add", ".")
        self.git("commit", "-qm", "base")
        # The branch inserts a sentence after Alpha in the old long line.
        self.git("checkout", "-qb", "topic")
        self.write("# Title\n\n- Item.\n" + self.long.replace("here. Beta", "here. Branch added this. Beta") + "\n")
        self.git("commit", "-qam", "branch")
        # main splits the line and then inserts its own sentence after Gamma.
        self.git("checkout", "-q", "main")
        self.assertEqual(self.run_script("fix", "--limit", "40", "lim.md").returncode, 0)
        self.write(self.read().replace("Gamma three is here.\n", "Gamma three is here.\n   Main added this.\n"))
        self.git("commit", "-qam", "split and insert")

    def tearDown(self):
        self.tmp.cleanup()

    def test_resolve_merges_independent_insertions_after_a_split(self):
        self.git("checkout", "-q", "topic")
        self.assertNotEqual(self.git("rebase", "main").returncode, 0, "the rebase should conflict")
        done = self.run_script("resolve", "--limit", "40", "lim.md")
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(
            self.read(),
            "# Title\n\n- Item.\n   Alpha one is here.\n   Branch added this.\n   Beta two is here.\n"
            "   Gamma three is here.\n   Main added this.\n   Delta four is here.\n",
        )
        self.assertEqual(self.git("diff", "--cached", "--name-only", "--diff-filter=U").stdout, "")

    def test_resolve_exit_codes_outside_a_conflict(self):
        self.assertEqual(self.run_script("resolve", "lim.md").returncode, 0)
        self.assertEqual(self.run_script("resolve", "missing.md").returncode, 2)
        self.write("<<<<<<< ours\na\n=======\nb\n>>>>>>> theirs\n", name="marked.md")
        self.assertEqual(self.run_script("resolve", "marked.md").returncode, 1)
        self.assertNotEqual(self.run_script("resolve").returncode, 0)


if __name__ == "__main__":
    unittest.main()
