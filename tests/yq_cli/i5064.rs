//! #5064 (shape 1): a plain scalar continued by a more-indented line holding `key: value` is not a node of its own;
//! yq stops at the `:` with `mapping values are not allowed in this context` where succinctly read `["a z", 1]`.
//! Each row is a live yq v4.53.3 capture. A `:` with no space after it stays text.

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

/// Documents yq rejects with `mapping values are not allowed in this context`: (document, extra args).
const REJECTED: &[(&str, &[&str])] = &[
    ("- a\n  z: 1\n", &[]),
    ("k:\n  - a\n    z: 1\n", &[]),
    ("- a\n  z:\n", &[]),
    ("- a\n  b c\n  z: 1\n", &[]),
    ("- a\n  z: 1 # c\n", &[]),
    ("a\nz: 1\n", &[]),
    ("a b\nc\nd: e\n", &[]),
    ("&x a\nz: 1\n", &[]),
    ("a\n b: 1\n", &[]),
    ("- a\n  z: 1\n", &["-o", "json", "-I0"]),
    ("- a\n  z: 1\n", &["-P"]),
];

/// Documents yq accepts (a `:` not followed by a space is text): (document, `-o json -I0` output).
const ACCEPTED: &[(&str, &str)] = &[
    ("- a\n  z:b\n", "[\"a z:b\"]\n"),
    ("- a\n  http://x\n", "[\"a http://x\"]\n"),
    ("a\nz:b\n", "\"a z:b\"\n"),
    ("- a # c: d\n- b\n", "[\"a\",\"b\"]\n"),
    ("- a\n  b c\n- d\n", "[\"a b c\",\"d\"]\n"),
    ("k:\n  - a\n    z\n", "{\"k\":[\"a z\"]}\n"),
    ("- |\n  x: y\n", "[\"x: y\\n\"]\n"),
    ("a: |\n  x: y\n", "{\"a\":\"x: y\\n\"}\n"),
];

#[test]
fn test_plain_scalar_continued_by_a_key_value_line_is_rejected_5064() -> Result<()> {
    for (doc, args) in REJECTED {
        let (stdout, stderr, code) = run_yq_stdin_with_stderr(".", doc, args)?;
        assert_eq!((stdout.as_str(), code), ("", 1), "{doc:?} {args:?}");
        assert!(
            stderr.contains("mapping values are not allowed in this context"),
            "{doc:?} {args:?}: {stderr}"
        );
    }
    Ok(())
}

#[test]
fn test_plain_scalar_continuation_with_a_colon_without_a_space_is_text_5064() -> Result<()> {
    for (doc, expected) in ACCEPTED {
        let (stdout, code) = run_yq_stdin(".", doc, &["-o", "json", "-I0"])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?}");
    }
    Ok(())
}
