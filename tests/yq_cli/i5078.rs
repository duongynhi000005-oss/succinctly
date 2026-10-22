//! #5078: a line with content below a block scalar value that is still deeper than the block holding the scalar
//! belongs to nothing. yq stops there with `did not find expected key` (`... '-' indicator` in a sequence);
//! succinctly read it as a key of its own (`a: |` / `   x` / `  y` / `b: 1` was `{"a":"x\n","y":"b"}`), or dropped it.
//! Blank lines, comment lines and a sibling at the parent's indent are fine. Each row is a live yq v4.53.3 capture.

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

/// (document, the part of yq's message that names what it expected).
const REJECTED: &[(&str, &str)] = &[
    ("a: |\n   x\n  y\nb: 1\n", "did not find expected key"),
    ("a: |\n   x\n y: 1\n", "did not find expected key"),
    (
        "- |\n   x\n  y\n- z\n",
        "did not find expected '-' indicator",
    ),
    (
        "a:\n  - |\n    x\n   y\n  - z\n",
        "did not find expected '-' indicator",
    ),
    ("a: |\n  x\n\n y\nb: 1\n", "did not find expected key"),
    ("a: >\n   x\n  y\n", "did not find expected key"),
    (
        "a:\n b: |\n    x\n   y\n c: 1\n",
        "did not find expected key",
    ),
    ("a: |2\n x\nb: 1\n", "did not find expected key"),
    ("- |2\n x\n- z\n", "did not find expected '-' indicator"),
    ("a: |4\n  x\nb: 1\n", "did not find expected key"),
    ("a: |-\n   x\n  y:\n", "did not find expected key"),
    ("? |\n   x\n  y\n: v\n", "did not find expected key"),
    ("- ? |\n    x\n   y\n  : v\n", "did not find expected key"),
    ("a: !!str |\n   x\n  y\nb: 1\n", "did not find expected key"),
    ("a: &x |\n   x\n  y\nb: 1\n", "did not find expected key"),
];

/// (document, `-o json -I0 .` output).
const ACCEPTED: &[(&str, &str)] = &[
    ("a: |\n   x\n  # c\nb: 1\n", "{\"a\":\"x\\n\",\"b\":1}\n"),
    ("a: |\n   x\n # c\nb: 1\n", "{\"a\":\"x\\n\",\"b\":1}\n"),
    ("a: |\n  x\nb: 1\n", "{\"a\":\"x\\n\",\"b\":1}\n"),
    ("- a: |\n    x\n  b: 1\n", "[{\"a\":\"x\\n\",\"b\":1}]\n"),
    ("- |\n  x\n- y\n", "[\"x\\n\",\"y\"]\n"),
    (
        "a:\n  b: |\n    x\n  c: 1\n",
        "{\"a\":{\"b\":\"x\\n\",\"c\":1}}\n",
    ),
    ("? |\n  x\n: v\n", "{\"x\\n\":\"v\"}\n"),
    (
        "- ? a\n  : |\n    x\n  b: 1\n",
        "[{\"a\":\"x\\n\",\"b\":1}]\n",
    ),
    ("a: |2\nx: y\n", "{\"a\":\"\",\"x\":\"y\"}\n"),
    ("a: |\n  x\n\n\nb: 1\n", "{\"a\":\"x\\n\",\"b\":1}\n"),
    ("? |\n  x\n: v\n", "{\"x\\n\":\"v\"}\n"),
    (
        "? |\n  x\n? |\n  y\n: v\n",
        "{\"x\\n\":null,\"y\\n\":\"v\"}\n",
    ),
    ("- ? |\n    x\n  : v\n", "[{\"x\\n\":\"v\"}]\n"),
    ("a: !!str |\n  x\nb: 1\n", "{\"a\":\"x\\n\",\"b\":1}\n"),
];

#[test]
fn test_line_deeper_than_the_block_after_a_block_scalar_is_rejected_5078() -> Result<()> {
    for (doc, expected) in REJECTED {
        let (stdout, stderr, code) = run_yq_stdin_with_stderr(".", doc, &[])?;
        assert_eq!((stdout.as_str(), code), ("", 1), "{doc:?}");
        assert!(stderr.contains(expected), "{doc:?}: {stderr}");
    }
    Ok(())
}

#[test]
fn test_sibling_comment_and_blank_lines_after_a_block_scalar_are_kept_5078() -> Result<()> {
    for (doc, expected) in ACCEPTED {
        let (stdout, code) = run_yq_stdin(".", doc, &["-o", "json", "-I0"])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?}");
    }
    Ok(())
}
