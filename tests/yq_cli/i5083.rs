//! #5083: a plain scalar on a line of its own (the value of the `key:` above it) followed by a line holding
//! `key: value` is one folded scalar that ends at the `: `, and yq stops there with `mapping values are not allowed
//! in this context`. succinctly read `k:` / `  a` / `  z: 1` as `{"k":"a z"}` and dropped the `: 1` (#5064 fixed
//! the same shape for a value on the key's line). Each row is a live yq v4.53.3 capture; a `:` with no space after
//! it stays text.

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

/// Documents yq rejects with `mapping values are not allowed in this context`.
const REJECTED: &[&str] = &[
    "k:\n  a\n  z: 1\n",
    "k:\n  a\n  k: z\n",
    "k:\n  a\n - k: z\n",
    "k:\n  a\n  b\n  z: 1\n",
    "k:\n  a\n   z: 1\n",
    "k:\n  a z\n  q: 1\n",
    "k:\n  a\n  z:\n",
    "a:\n  k:\n    x\n    y: 1\n",
    "k:\r\n  a\r\n  z: 1\r\n",
    "k:\n  a\n  z:	1\n",
];

/// (document, `-o json -I0 .` output).
const ACCEPTED: &[(&str, &str)] = &[
    ("k:\n  a\n  b\n", "{\"k\":\"a b\"}\n"),
    ("k:\n  a\n  z:b\n", "{\"k\":\"a z:b\"}\n"),
    ("k:\n  a\nz: 1\n", "{\"k\":\"a\",\"z\":1}\n"),
    ("k:\n  a b\n  c\nm: 1\n", "{\"k\":\"a b c\",\"m\":1}\n"),
    ("k:\n  http://x\n  y\n", "{\"k\":\"http://x y\"}\n"),
    (
        "a:\n  k:\n    x\n    y\n  m: 1\n",
        "{\"a\":{\"k\":\"x y\",\"m\":1}}\n",
    ),
    ("k:\n  a # c\nm: 1\n", "{\"k\":\"a\",\"m\":1}\n"),
    ("k:\n  a\n  ? z\n", "{\"k\":\"a ? z\"}\n"),
];

#[test]
fn test_own_line_plain_scalar_continued_by_a_key_value_line_is_rejected_5083() -> Result<()> {
    for doc in REJECTED {
        let (stdout, stderr, code) = run_yq_stdin_with_stderr(".", doc, &[])?;
        assert_eq!((stdout.as_str(), code), ("", 1), "{doc:?}");
        assert!(
            stderr.contains("mapping values are not allowed in this context"),
            "{doc:?}: {stderr}"
        );
    }
    Ok(())
}

#[test]
fn test_own_line_plain_scalar_continuations_that_are_text_stay_5083() -> Result<()> {
    for (doc, expected) in ACCEPTED {
        let (stdout, code) = run_yq_stdin(".", doc, &["-o", "json", "-I0"])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?}");
    }
    Ok(())
}
