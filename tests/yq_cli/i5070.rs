//! #5070: nothing follows a document's root scalar, flow collection or block scalar before the next
//! `---`: yq prints the root and stops with `did not find expected <document start>`, where succinctly
//! read the line as a further document (`"a"` / `b` was `"a"` then `"b"`). A root plain scalar also
//! goes on across a line that opens like an item (`text` / `- b` is the one string `text - b`).
//! Each row is a live yq v4.53.3 capture; yq's stdout before its error (the first node) is not
//! reproduced, only the refusal.

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

/// Roots and what follows them, as (root, following line(s)).
const ROOTS: &[&str] = &[
    "\"q\"",
    "'q'",
    "[1]",
    "{x: 1}",
    "[1,\n  2]",
    "\"q\n  r\"",
    "&x a # c",
    "!!str \"q\"",
    "|\n  t",
    ">\n  t",
    "a # c",
];
const FOLLOWING: &[&str] = &[
    "b", "  b", "- b", "b: 1", "? b", "[x]", "\"r\"", "|\n  t", "# c\nb", "\nb", "&y b", ": v",
];

#[test]
fn test_content_after_a_complete_document_root_is_rejected_5070() -> Result<()> {
    for root in ROOTS {
        for following in FOLLOWING {
            // An indented line after a block scalar header is its content.
            if root.starts_with(['|', '>']) && following.starts_with("  b") {
                continue;
            }
            let doc = format!("{root}\n{following}\n");
            let (stdout, stderr, code) = run_yq_stdin_with_stderr(".", &doc, &[])?;
            assert_eq!((stdout.as_str(), code), ("", 1), "{doc:?}");
            assert!(
                stderr.contains("did not find expected <document start>"),
                "{doc:?}: {stderr}"
            );
        }
    }
    Ok(())
}

/// Rejected for the `...` / comment-led shapes the loop above does not build.
const REJECTED_OTHER: &[&str] = &[
    "text\n- b # c\n  d\n",
    "&a text\n- b # c\n  d\n",
    "--- a\n- b # c\n  d\n",
    "--- &a text\n- b # c\n  d\n",
    "--- \"q\"\nb\n",
    "--- [1]\n- b\n",
    "[a, b]\n: value\n",
    // A flow key that spans lines is a root with a line after it.
    "[a,\n b]: v\n",
    // A folded item-like line does not hide a key after it.
    "text\n- b\n- c: d\n",
    // CRLF line breaks.
    "\"a\"\r\nb\r\n",
    // The error is in a later document: yq has printed the earlier ones, this prints nothing.
    "x: 1\n---\n\"a\"\nb\n",
];

#[test]
fn test_content_after_a_root_in_other_shapes_is_rejected_5070() -> Result<()> {
    for doc in REJECTED_OTHER {
        let (stdout, _stderr, code) = run_yq_stdin_with_stderr(".", doc, &[])?;
        assert_eq!((stdout.as_str(), code), ("", 1), "{doc:?}");
    }
    Ok(())
}

/// Accepted, unchanged or newly read as yq does: (document, `-o json -I0` output).
const ACCEPTED: &[(&str, &str)] = &[
    // A root plain scalar goes on across a line that opens like an item.
    ("text\n- b\n", "\"text - b\"\n"),
    ("a b\n- c\n- d\n", "\"a b - c - d\"\n"),
    ("&x a\n- b\n", "\"a - b\"\n"),
    ("!!str a\n- b\n", "\"a - b\"\n"),
    // Documents separated by a marker.
    ("\"a\"\n---\n\"b\"\n", "\"a\"\n\"b\"\n"),
    ("[1]\n---\n[2]\n", "[1]\n[2]\n"),
    ("|\n  t\n---\n|\n  u\n", "\"t\\n\"\n\"u\\n\"\n"),
    ("a\n---\nb\n", "\"a\"\n\"b\"\n"),
    // Blank lines and comments after a root are not content.
    ("\"q\"\n\n# c\n\n", "\"q\"\n"),
    ("[1] # c\n", "[1]\n"),
    ("\"q\"\r\n\r\n", "\"q\"\n"),
    // A block collection root is not a scalar root.
    ("a: 1\nb: 2\n", "{\"a\":1,\"b\":2}\n"),
    ("- a\n- b\n", "[\"a\",\"b\"]\n"),
    ("? a\n: b\n", "{\"a\":\"b\"}\n"),
];

#[test]
fn test_roots_and_separators_that_yq_accepts_are_unchanged_5070() -> Result<()> {
    for (doc, expected) in ACCEPTED {
        let (stdout, code) = run_yq_stdin(".", doc, &["-o", "json", "-I0"])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?}");
    }
    Ok(())
}

/// Recorded gap: content after a `...` marker with no `---` is a second document here; yq prints the
/// first and stops (`did not find expected <document start>`). Pinned to what succinctly answers
/// today; delete a row when it starts to fail.
const RECORDED_GAPS: &[(&str, &str)] = &[
    ("\"a\"\n...\nb\n", "\"a\"\n\"b\"\n"),
    ("a\n...\nb\n", "\"a\"\n\"b\"\n"),
    ("--- \"a\"\n... # c\n--- b\n", "\"a\"\n\"b\"\n"),
];

#[test]
fn test_recorded_gap_content_after_a_document_end_marker_without_a_start_5070() -> Result<()> {
    for (doc, expected) in RECORDED_GAPS {
        let (stdout, code) = run_yq_stdin(".", doc, &["-o", "json", "-I0"])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?}");
    }
    Ok(())
}
