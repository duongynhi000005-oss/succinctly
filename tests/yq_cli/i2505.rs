//! #2505: an alias mark follows a plain `=` onto the shapes #2497's pass left plain: a target
//! under a parent the write created (`.c.d = .b`, `.c[0] = .b`), a comma of targets
//! (`(.c, .d) = .b`), a negative index on the source (`.c = .b[-1]`), and a source container
//! that holds an alias (`.c = .b` on `b: [*x]`).
//! Every expected output is what yq v4.53.3 prints for the same input and filter.
//!
//! The rows #2505 lists that stay plain (a constructed or computed right side, a compound
//! assignment, a comment copied from the source, a block copied with its flow style, a later
//! write through the anchor) are recorded in `docs/compliance/yq/limitations.md`.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, filter, expected output).
const ROWS: &[(&str, &str, &str)] = &[
    (
        "a: &x 1\nb: *x\n",
        ".c.d = .b",
        "a: &x 1\nb: *x\nc:\n  d: *x\n",
    ),
    (
        "a: &x 1\nb: *x\n",
        ".c[0] = .b",
        "a: &x 1\nb: *x\nc:\n  - *x\n",
    ),
    (
        "a: &x 1\nb: *x\n",
        ".c.d.e = .b",
        "a: &x 1\nb: *x\nc:\n  d:\n    e: *x\n",
    ),
    (
        "a: &x 1\nb: *x\n",
        ".c[1] = .b",
        "a: &x 1\nb: *x\nc:\n  - null\n  - *x\n",
    ),
    (
        "a: &x 1\nb: *x\n",
        ".c.d = .b | .e = .c.d",
        "a: &x 1\nb: *x\nc:\n  d: *x\ne: *x\n",
    ),
    (
        "a: &x 1\nb: [0, *x]\n",
        ".c = .b[-1]",
        "a: &x 1\nb: [0, *x]\nc: *x\n",
    ),
    (
        "a: &x 1\nb: [0, *x]\n",
        ".c = .b[-2]",
        "a: &x 1\nb: [0, *x]\nc: 0\n",
    ),
    (
        "a: &x 1\nb: [0, *x]\n",
        ".c = .b[1]",
        "a: &x 1\nb: [0, *x]\nc: *x\n",
    ),
    (
        "a: &x 1\nb: [0, *x]\n",
        ".c = .b[0]",
        "a: &x 1\nb: [0, *x]\nc: 0\n",
    ),
    (
        "a: &x 1\nb: *x\n",
        "(.c, .d) = .b",
        "a: &x 1\nb: *x\nc: *x\nd: *x\n",
    ),
    (
        "a: &x 1\nb: *x\n",
        "(.c, .d.e, .f[0]) = .b",
        "a: &x 1\nb: *x\nc: *x\nd:\n  e: *x\nf:\n  - *x\n",
    ),
    (
        "a: &x 1\nb: *x\n",
        "(.c, .d) = .a",
        "a: &x 1\nb: *x\nc: 1\nd: 1\n",
    ),
    (
        "a: &x 1\nb: *x\n",
        ".c = .b | (.d, .e) = .c",
        "a: &x 1\nb: *x\nc: *x\nd: *x\ne: *x\n",
    ),
    (
        "a: &x 1\nb:\n  - *x\n  - 2\n",
        ".c = .b",
        "a: &x 1\nb:\n  - *x\n  - 2\nc:\n  - *x\n  - 2\n",
    ),
    (
        "a: &x 1\nb:\n  p: *x\n  q: 2\n",
        ".c = .b",
        "a: &x 1\nb:\n  p: *x\n  q: 2\nc:\n  p: *x\n  q: 2\n",
    ),
    (
        "a: &x 1\nb:\n  p:\n    - *x\n    - 3\n  q: 2\n",
        ".c = .b",
        "a: &x 1\nb:\n  p:\n    - *x\n    - 3\n  q: 2\nc:\n  p:\n    - *x\n    - 3\n  q: 2\n",
    ),
    (
        "a: &x 1\nb:\n  - 2\n  - 3\n",
        ".c = .b",
        "a: &x 1\nb:\n  - 2\n  - 3\nc:\n  - 2\n  - 3\n",
    ),
    (
        "a: &x 1\nb:\n  - *x\n",
        ".c.d = .b",
        "a: &x 1\nb:\n  - *x\nc:\n  d:\n    - *x\n",
    ),
    (
        "a: &x 1\nc:\n  - *x\nb:\n  - 5\n",
        ".c = .b",
        "a: &x 1\nc:\n  - 5\nb:\n  - 5\n",
    ),
    (
        "a: &x 1\nc: *x\nb: 5\n",
        "(.c, .d) = .b",
        "a: &x 1\nc: 5\nb: 5\nd: 5\n",
    ),
    (
        "a: &x 1\nb: *x\n",
        ".c = .b | .c = 7",
        "a: &x 1\nb: *x\nc: 7\n",
    ),
];

#[test]
fn test_alias_mark_follows_a_plain_assignment_onto_the_residual_shapes_2505() -> Result<()> {
    for &(input, filter, expected) in ROWS {
        let (output, exit_code) = run_yq_stdin(filter, input, &[])?;
        assert_eq!(exit_code, 0, "filter: {filter} on {input:?}");
        assert_eq!(output, expected, "filter: {filter} on {input:?}");
    }
    Ok(())
}

/// The value never changes, only how it is spelled: `-o=json` expands every alias.
#[test]
fn test_alias_mark_propagation_leaves_the_values_alone_2505() -> Result<()> {
    for &(input, filter, _) in ROWS {
        let (plain, exit_code) = run_yq_stdin(filter, input, &["-o=json", "-I=0"])?;
        assert_eq!(exit_code, 0, "filter: {filter} on {input:?}");
        let (yaml_as_json, exit_code) = run_yq_stdin(filter, input, &["-o=yaml"])?;
        assert_eq!(exit_code, 0, "filter: {filter} on {input:?}");
        // The same program printed as JSON and as YAML names the same document.
        let (roundtrip, exit_code) = run_yq_stdin(".", &yaml_as_json, &["-o=json", "-I=0"])?;
        assert_eq!(exit_code, 0, "filter: {filter} on {input:?}");
        assert_eq!(plain, roundtrip, "filter: {filter} on {input:?}");
    }
    Ok(())
}

/// Real yq prints `*x` with no `&x` left anywhere for these, which it cannot read back
/// (ADR-0018 rule 4(a)); a `*name` is written only while its `&name` survives, as for
/// `.a = .b` on this document (#2497), so every alias drops to its value.
#[test]
fn test_alias_mark_dropped_when_the_write_destroys_the_anchor_2505() -> Result<()> {
    let (output, exit_code) = run_yq_stdin("(.a, .c) = .b", "a: &x 1\nb: *x\n", &[])?;
    assert_eq!(exit_code, 0);
    assert_eq!(output, "a: 1\nb: 1\nc: 1\n");
    Ok(())
}
