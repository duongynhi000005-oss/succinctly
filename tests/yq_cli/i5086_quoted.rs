//! #5086: a complete quoted or flow value (`"a"`, `[1]`, `{a: 1}`) followed by a line deeper than the block that
//! holds it. The value cannot be continued, so the line belongs to nothing, and yq stops there with
//! `did not find expected key` (`... '-' indicator` in a sequence, `mapping values are not allowed ...` where its
//! scanner reads a key). succinctly read it as a key of its own or dropped it (`k:` / `  "a"` / `  z: 1` was
//! `{"k":"a"}`, `- "a"` / `  z: 1` was `["a",{"z":1}]`). A sibling at the block's indent, a comment and blank
//! lines are fine. Each row is a live yq v4.53.3 capture; the token is the part of yq's message succinctly's
//! also carries (`Error` where the wording differs).

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

/// (document, the part of the error message both sides carry).
const REJECTED: &[(&str, &str)] = &[
    ("- \"a\"\n  z: 1\n", "did not find expected '-' indicator"),
    ("k:\n  \"a\"\n  z: 1\n", "did not find expected key"),
    (
        "-\n  \"a\"\n  z: 1\n",
        "did not find expected '-' indicator",
    ),
    ("k:\n  [1]\n  z: 1\n", "did not find expected key"),
    ("k:\n  {a: 1}\n  z: 1\n", "did not find expected key"),
    ("-\n  [1]\n  z: 1\n", "did not find expected '-' indicator"),
    ("k:\n  &x \"a\"\n  z: 1\n", "did not find expected key"),
    ("k: &x\n  \"a\"\n  z: 1\n", "did not find expected key"),
    ("k: \"a\"\n  z: 1\n", "did not find expected key"),
    ("k: \"a\"\n z\n", "did not find expected key"),
    ("- k: \"a\"\n    z: 1\n", "did not find expected key"),
    ("k: [1]\n  z: 1\n", "did not find expected key"),
    ("- [1]\n  z: 1\n", "did not find expected '-' indicator"),
    ("k: {a: 1}\n  z: 1\n", "did not find expected key"),
    ("k: \"a\" # c\n  z: 1\n", "did not find expected key"),
    ("? k\n: \"a\"\n  z: 1\n", "did not find expected key"),
    ("k: 'a'\n  z: 1\n", "did not find expected key"),
    ("- \"a\"\n  - b\n", "did not find expected '-' indicator"),
    ("k: [a, b]\n  z\n", "Error"),
    ("- a # c\n- \"q\"\n  z\n", "Error"),
    ("- a # c\n- [1]\n  z\n", "Error"),
    ("k: a # c\nj: \"q\"\n  z\n", "Error"),
    ("k: !!str \"a\"\n  z: 1\n", "did not find expected key"),
    ("k: !!str [1]\n  z: 1\n", "did not find expected key"),
    ("k: \"a\"\r\n  z: 1\r\n", "did not find expected key"),
    (
        "- \"a\"\r\n  z: 1\r\n",
        "did not find expected '-' indicator",
    ),
];

/// (document, `-o json -I0 .` output).
const ACCEPTED: &[(&str, &str)] = &[
    ("k:\n  \"a\"\nz: 1\n", "{\"k\":\"a\",\"z\":1}\n"),
    ("- \"a\"\n- b\n", "[\"a\",\"b\"]\n"),
    ("-\n  \"a\"\n- b\n", "[\"a\",\"b\"]\n"),
    ("k:\n  \"a\" # c\nm: 1\n", "{\"k\":\"a\",\"m\":1}\n"),
    (
        "a:\n  b:\n    \"x\"\n  c: 1\n",
        "{\"a\":{\"b\":\"x\",\"c\":1}}\n",
    ),
    ("k: \"a\"\n# c\n\nm: 1\n", "{\"k\":\"a\",\"m\":1}\n"),
    ("- [1]\n- [2]\n", "[[1],[2]]\n"),
    (
        "a:\n  b: {x: 1}\n  c: 1\n",
        "{\"a\":{\"b\":{\"x\":1},\"c\":1}}\n",
    ),
    ("- k: \"a\"\n  z: 1\n", "[{\"k\":\"a\",\"z\":1}]\n"),
    ("k: \"a\"\r\nm: 1\r\n", "{\"k\":\"a\",\"m\":1}\n"),
    ("- \"a\"\n---\n- [1]\n", "[\"a\"]\n[[1]]\n"),
    ("k: \"a\"\n...\n", "{\"k\":\"a\"}\n"),
];

#[test]
fn test_line_deeper_than_the_block_after_a_quoted_or_flow_value_is_rejected_5086() -> Result<()> {
    for (doc, token) in REJECTED {
        let (stdout, stderr, code) = run_yq_stdin_with_stderr(".", doc, &[])?;
        assert_eq!((stdout.as_str(), code), ("", 1), "{doc:?}");
        assert!(stderr.contains(token), "{doc:?}: {stderr}");
    }
    Ok(())
}

#[test]
fn test_siblings_comments_and_blank_lines_after_a_quoted_or_flow_value_stay_5086() -> Result<()> {
    for (doc, expected) in ACCEPTED {
        let (stdout, code) = run_yq_stdin(".", doc, &["-o", "json", "-I0"])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?}");
    }
    Ok(())
}
