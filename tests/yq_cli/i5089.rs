//! #5089: a document after a `...` end marker needs its own `---`. Content that follows `...` with no
//! `---` read as a further document (`a` / `...` / `b` was `"a"` then `"b"`); yq prints the first and
//! stops with `did not find expected <document start>`, and so does `succinctly yq` now (exit 1, with
//! no output: yq has printed the first document before the error, which this does not). Each row is a
//! live yq v4.53.3 capture. A directive after `...` stays accepted (YAML 1.2 allows it, yq rejects it).

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

const FIRSTS: &[&str] = &["a", "\"q\"", "[1]", "a: 1", "- a", "|\n  t"];
const AFTERS: &[&str] = &[
    "b",
    "b: 1",
    "- b",
    "[x]",
    "\"r\"",
    "# c\nb",
    "\nb",
    "...\nb",
    "... # c\nb",
];

#[test]
fn test_content_after_a_document_end_marker_without_a_start_is_rejected_5089() -> Result<()> {
    for first in FIRSTS {
        for after in AFTERS {
            let doc = format!("{first}\n...\n{after}\n");
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

/// Accepted by yq: (document, `-o json -I0` output).
const ACCEPTED: &[(&str, &str)] = &[
    ("a\n...\n---\nb\n", "\"a\"\n\"b\"\n"),
    ("a\n...\n# c\n---\nb\n", "\"a\"\n\"b\"\n"),
    ("a\n...\n\n\n---\nb\n", "\"a\"\n\"b\"\n"),
    ("\"q\"\n...\n--- |\n  t\n", "\"q\"\n\"t\\n\"\n"),
    ("a: 1\n...\n--- b\n", "{\"a\":1}\n\"b\"\n"),
    ("a\n...\n", "\"a\"\n"),
    ("a\n... # c\n", "\"a\"\n"),
    ("a\n...\n# only a comment\n", "\"a\"\n"),
    ("a\n...\n...\n", "\"a\"\n"),
    ("a\n---\nb\n...\n---\nc\n", "\"a\"\n\"b\"\n\"c\"\n"),
    // A directive after `...` is rejected by yq and accepted here, as YAML 1.2 allows it.
    ("a\n...\n%YAML 1.2\n---\nb\n", "\"a\"\n\"b\"\n"),
];

#[test]
fn test_documents_separated_by_a_start_marker_after_an_end_marker_are_unchanged_5089() -> Result<()>
{
    for (doc, expected) in ACCEPTED {
        let (stdout, code) = run_yq_stdin(".", doc, &["-o", "json", "-I0"])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?}");
    }
    Ok(())
}

/// Recorded divergence: a stream whose first document is `--- text` followed by `...` and more
/// content. yq folds the `...` into the scalar (`"v... b"`); the document end marker ends it here,
/// so the content after it is the error. It read as two documents before (`"v"`, `"b"`); both
/// differ from yq. Pinned to what succinctly answers today; delete a row when it starts to fail.
const FOLDED_BY_YQ: &[&str] = &[
    "--- v\n...\nb\n",
    "--- v w\n...\n  b\n",
    "--- &a v\n...\nb\n",
];

#[test]
fn test_recorded_divergence_yq_folds_the_end_marker_into_an_inline_scalar_5089() -> Result<()> {
    for doc in FOLDED_BY_YQ {
        let (stdout, _stderr, code) = run_yq_stdin_with_stderr(".", doc, &[])?;
        assert_eq!((stdout.as_str(), code), ("", 1), "{doc:?}");
    }
    Ok(())
}
