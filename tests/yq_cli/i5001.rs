//! #5001: a comment after a block scalar's `|`/`>` indicator is the header line's comment. It was written after
//! the scalar's last line, where inside a block scalar it is content, so a round trip changed the string
//! (`"DROP TABLE x # sql\n"`). yq keeps it on the header line, on `.`, `-P` and writes, for every chomping or
//! indentation indicator, anchor and tag.
//!
//! Every output is captured from yq v4.53.3. A value the writer cannot keep as a block scalar (`x  `
//! with trailing spaces, an empty body) is written quoted and keeps the comment after it, as any scalar does.

use anyhow::Result;

use crate::run_yq_stdin;

/// (filter, extra args, document, expected output).
const ROWS: &[(&str, &[&str], &str, &str)] = &[
    (
        ".",
        &[],
        "steps:\n  - down: | # sql\n      DROP TABLE x\n",
        "steps:\n  - down: | # sql\n      DROP TABLE x\n",
    ),
    (".", &[], "a: |- # c\n  x\n", "a: |- # c\n  x\n"),
    (".", &[], "a: >- # c\n  x\n  y\n", "a: >- # c\n  x y\n"),
    (
        ".",
        &[],
        "a: |+ # c\n  x\n\nb: 1\n",
        "a: |+ # c\n  x\n\nb: 1\n",
    ),
    (".", &[], "a: |2 # c\n   x\n", "a: |2 # c\n   x\n"),
    (
        ".",
        &[],
        "a: &x !!str | # c\n  x\n",
        "a: &x !!str | # c\n  x\n",
    ),
    (
        ".",
        &[],
        "- | # c\n  x\n- &a !!str > # c4\n  y\n- z # k\n",
        "- | # c\n  x\n- &a !!str > # c4\n  y\n\n- z # k\n",
    ),
    (
        ".",
        &[],
        "a:\n  - >2 # c\n     x\n",
        "a:\n  - >2 # c\n     x\n",
    ),
    (
        ".",
        &["-P"],
        "a: | # c\n  x\n  y\nb: >- # d\n  y\n  z\n",
        "a: | # c\n  x\n  y\nb: y z # d\n",
    ),
    (
        ".",
        &["-I4"],
        "a:\n  b: | # c\n    x\n",
        "a:\n    b: | # c\n        x\n",
    ),
    (
        ".a = \"n\"",
        &[],
        "a: | # c\n  x\nb: >- # d\n  y\n",
        "a: |- # c\n  n\nb: >- # d\n  y\n",
    ),
    (
        ".c = 2",
        &[],
        "a: | # c\n  x\nb: >- # d\n  y\nc: 1\n",
        "a: | # c\n  x\nb: >- # d\n  y\nc: 2\n",
    ),
    (
        ".[1] = \"w\"",
        &[],
        "- | # c\n  x\n- y\n",
        "- | # c\n  x\n- w\n",
    ),
    (
        ".",
        &[],
        "a: | # c\n  x # not a comment\n",
        "a: | # c\n  x # not a comment\n",
    ),
    (".", &[], "a:\n  | # h\n  text\n", "a: | # h\n  text\n"),
    (".", &[], "a: | # c\n  x  \n", "a: \"x  \\n\" # c\n"),
    (".", &[], "a: |- # c\n", "a: \"\" # c\n"),
    (".", &[], "a: !!str |- # c\n", "a: !!str \"\" # c\n"),
    (
        ".",
        &[],
        "a:\n  - &x | # c\n    v\n  - *x\n",
        "a:\n  - &x | # c\n    v\n  - *x\n",
    ),
    (".", &[], "a: |1 # c\n  x\n", "a: |2 # c\n   x\n"),
    (
        ".",
        &[],
        "a: >- # c\n  x\n\n  y\n",
        "a: >- # c\n  x\n\n  y\n",
    ),
    (".a", &[], "a: | # c\n  x\n", "x\n\n"),
];

#[test]
fn test_5001_a_block_scalar_header_comment_stays_on_the_header_line() -> Result<()> {
    for (filter, args, input, expected) in ROWS {
        let (out, code) = run_yq_stdin(filter, input, args)?;
        assert_eq!(code, 0, "{filter} {args:?} {input:?}");
        assert_eq!(out, *expected, "{filter} {args:?} {input:?}");
    }
    Ok(())
}
