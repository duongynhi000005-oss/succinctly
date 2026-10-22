//! #5068: a plain scalar that a comment ended is not continued by the next line, and a line that is
//! neither a sibling nor a continuation has nothing to attach to: yq stops (`did not find expected
//! '-' indicator`, `could not find expected ':'`) where succinctly read the line as a further item
//! or dropped it. Each row is a live yq v4.53.3 capture.

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

/// Documents yq rejects.
const REJECTED: &[&str] = &[
    "- a # c\n  z\n",
    "k: a # c\n  z\n",
    "- a # c\n  z: 1\n",
    "- a # c\n  - z\n",
    "k: a # c\n  z: 1\n",
    "k: a # c\n  - z\n",
    // A comment line between the scalar and the line.
    "- a\n  # c\n  z: 1\n",
    "- a\n  # c\n  - z\n",
    "k:\n  a\n  # c\n  z: 1\n",
    "k:\n  a\n  # c\n  z\n",
    // The value on a line of its own.
    "k:\n  a # c\n  z\n",
    // Nested owners.
    "k:\n  - a # c\n    z\n",
    "k:\n  j: a # c\n    z\n",
    "- k: a # c\n    z\n",
    "- - a # c\n    z\n",
    // A bare line led by node properties.
    "- a\n&x z\n",
    "k: a\n!!str z\n",
    "k: a # c\n&x z\n",
    "k:\n  a\n&x z\n",
    "- a\n  # c\n!!str z\n",
    // CRLF line breaks.
    "- a # c\r\n  z\r\n",
    "- a\r\n  # c\r\n  z: 1\r\n",
    "k: a\r\nz\r\n",
    // A document whose first line is only node properties is not a bare scalar root.
    "&a\n- x # c\n  y\n",
    // A bare line that is not a continuation at all.
    "- a\nz\n",
    "k: a\nz\n",
    "k:\n  - a\n  z\n",
];

/// Documents yq accepts, unchanged: (document, `-o json -I0` output).
const ACCEPTED: &[(&str, &str)] = &[
    ("- a # c\n- z\n", "[\"a\",\"z\"]\n"),
    ("k: a # c\nj: z\n", "{\"k\":\"a\",\"j\":\"z\"}\n"),
    ("- k: a # c\n  j: z\n", "[{\"k\":\"a\",\"j\":\"z\"}]\n"),
    ("- a # c\n\n- z\n", "[\"a\",\"z\"]\n"),
    ("- a # c\n  # d\n- z\n", "[\"a\",\"z\"]\n"),
    ("k:\n  a\n  z\n", "{\"k\":\"a z\"}\n"),
    ("k:\n  a # c\n", "{\"k\":\"a\"}\n"),
    ("- a\n  b\n", "[\"a b\"]\n"),
    ("k: a\n  b\n", "{\"k\":\"a b\"}\n"),
    ("k:\n  - a # c\n  - z\n", "{\"k\":[\"a\",\"z\"]}\n"),
    (
        "k:\n  j: a # c\n  i: z\n",
        "{\"k\":{\"j\":\"a\",\"i\":\"z\"}}\n",
    ),
    (
        "- a # c\n- k: z\n  j: 1\n",
        "[\"a\",{\"k\":\"z\",\"j\":1}]\n",
    ),
    ("a: 1 # c\nb: 2 # d\n", "{\"a\":1,\"b\":2}\n"),
    ("- a # c\r\n- z\r\n", "[\"a\",\"z\"]\n"),
    ("k:\r\n  a\r\n  z\r\n", "{\"k\":\"a z\"}\n"),
];

#[test]
fn test_line_after_a_plain_scalar_a_comment_ended_is_rejected_5068() -> Result<()> {
    for doc in REJECTED {
        let (stdout, _stderr, code) = run_yq_stdin_with_stderr(".", doc, &[])?;
        assert_eq!((stdout.as_str(), code), ("", 1), "{doc:?}");
    }
    Ok(())
}

#[test]
fn test_a_comment_after_a_plain_scalar_leaves_valid_documents_alone_5068() -> Result<()> {
    for (doc, expected) in ACCEPTED {
        let (stdout, code) = run_yq_stdin(".", doc, &["-o", "json", "-I0"])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?}");
    }
    Ok(())
}

/// Recorded gaps: documents yq rejects that are still read, because the check leaves a bare plain
/// scalar root and a flow value alone (a root followed by more content is #5070). Each row is
/// pinned to what succinctly answers today; delete a row when it starts to fail.
const RECORDED_GAPS: &[(&str, &str)] = &[
    ("text\n- b # c\n  d\n", "\"text\"\n[\"b\",\"d\"]\n"),
    ("&a text\n- b # c\n  d\n", "\"text\"\n[\"b\",\"d\"]\n"),
    ("--- a\n- b # c\n  d\n", "\"a\"\n[\"b\",\"d\"]\n"),
    ("--- &a text\n- b # c\n  d\n", "\"text\"\n[\"b\",\"d\"]\n"),
    ("k: [a, b]\n  z\n", "{\"k\":[\"a\",\"b\"]}\n"),
];

#[test]
fn test_recorded_gaps_after_a_root_scalar_or_a_flow_value_5068() -> Result<()> {
    for (doc, expected) in RECORDED_GAPS {
        let (stdout, code) = run_yq_stdin(".", doc, &["-o", "json", "-I0"])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?}");
    }
    Ok(())
}
