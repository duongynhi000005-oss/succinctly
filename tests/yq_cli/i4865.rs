//! #4865: a single-quoted or plain scalar holding U+2028 / U+2029 is spelled the way yq's emitter
//! writes it: single quotes across lines, the break followed by the indent (a plain scalar, which a
//! break refuses, becomes single-quoted).
//!
//! Every row is captured from yq v4.53.3: the source document, the extra arguments, the filter and
//! the expected output. One test per writer, so that disabling one of them fails its own test.

use anyhow::Result;

use crate::run_yq_stdin;

type Row = (
    &'static str,
    &'static [&'static str],
    &'static str,
    &'static str,
);

fn check(rows: &[Row]) -> Result<()> {
    for (input, extra, filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, input, extra)?;
        assert_eq!(code, 0, "{input:?} {extra:?} {filter}");
        assert_eq!(out, *want, "{input:?} {extra:?} {filter}");
    }
    Ok(())
}

/// A block scalar on the streaming route (`.`).
#[test]
fn test_4865_stream_block() -> Result<()> {
    check(&[
        ("a: 'x\u{2028}y'\n", &[], ".", "a: 'x\u{2028}  y'\n"),
        ("a: 'x\u{2028}y'\n", &["-I=4"], ".", "a: 'x\u{2028}    y'\n"),
        ("a: 'x\u{2028}'\n", &[], ".", "a: 'x\u{2028}'\n"),
        ("a: '\u{2028}x'\n", &[], ".", "a: '\u{2028}  x'\n"),
        (
            "a: 'x\u{2028}y\u{2029}z'\n",
            &[],
            ".",
            "a: 'x\u{2028}  y\u{2029}  z'\n",
        ),
        (
            "a: 'it''s\u{2028}ok'\n",
            &[],
            ".",
            "a: 'it''s\u{2028}  ok'\n",
        ),
        (
            "b:\n  a: 'x\u{2028}y'\n",
            &[],
            ".",
            "b:\n  a: 'x\u{2028}    y'\n",
        ),
        ("- 'x\u{2029}y'\n- z\n", &[], ".", "- 'x\u{2029}  y'\n- z\n"),
        (
            "a: &n 'x\u{2028}y'\nb: *n\n",
            &[],
            ".",
            "a: &n 'x\u{2028}  y'\nb: *n\n",
        ),
        ("a: \"x\u{2028}y\"\n", &[], ".", "a: \"x\\Ly\"\n"),
    ])
}

/// A flow-collection item on the streaming route, which writes it with the indent of the collection
/// it sits in (the first flow level only: a flow collection nested in another is #4913).
#[test]
fn test_4865_stream_flow() -> Result<()> {
    check(&[
        ("- [x\u{2029}y]\n", &[], ".", "- ['x\u{2029}    y']\n"),
        ("a: [x\u{2029}y]\n", &[], ".", "a: ['x\u{2029}    y']\n"),
        (
            "a: [x\u{2029}y]\n",
            &["-I=4"],
            ".",
            "a: ['x\u{2029}        y']\n",
        ),
        (
            "b:\n  a: [x\u{2029}y, 'p\u{2028}q']\n",
            &[],
            ".",
            "b:\n  a: ['x\u{2029}      y', 'p\u{2028}      q']\n",
        ),
        (
            "a: {k: x\u{2028}y}\n",
            &[],
            ".",
            "a: {k: 'x\u{2028}    y'}\n",
        ),
        (
            "a: [x\u{2028}y, z]\nb: {c: 'p\u{2029}q'}\n",
            &[],
            ".",
            "a: ['x\u{2028}    y', z]\nb: {c: 'p\u{2029}    q'}\n",
        ),
    ])
}

/// A block scalar on the DOM write route (`.z = 1`), which spells it through different code.
#[test]
fn test_4865_write_block() -> Result<()> {
    check(&[
        (
            "a: 'x\u{2028}y'\n",
            &[],
            ".z = 1",
            "a: 'x\u{2028}  y'\nz: 1\n",
        ),
        (
            "a: 'x\u{2028}y'\n",
            &["-I=4"],
            ".z = 1",
            "a: 'x\u{2028}    y'\nz: 1\n",
        ),
        (
            "- 'x\u{2029}y'\n- z\n",
            &[],
            ".[2] = 1",
            "- 'x\u{2029}  y'\n- z\n- 1\n",
        ),
    ])
}

/// A flow-collection item on the DOM write route.
#[test]
fn test_4865_write_flow() -> Result<()> {
    check(&[
        (
            "a: [x\u{2029}y]\n",
            &[],
            ".z = 1",
            "a: ['x\u{2029}    y']\nz: 1\n",
        ),
        (
            "a: [x\u{2029}y]\n",
            &["-I=4"],
            ".z = 1",
            "a: ['x\u{2029}        y']\nz: 1\n",
        ),
        (
            "b:\n  a: [x\u{2029}y, 'p\u{2028}q']\n",
            &[],
            ".z = 1",
            "b:\n  a: ['x\u{2029}      y', 'p\u{2028}      q']\nz: 1\n",
        ),
        (
            "a: {k: x\u{2028}y}\n",
            &[],
            ".z = 1",
            "a: {k: 'x\u{2028}    y'}\nz: 1\n",
        ),
        (
            "{a: 'x\u{2028}y'}\n",
            &[],
            ".z = 1",
            "{a: 'x\u{2028}    y', z: 1}\n",
        ),
        (
            "a: [[[x\u{2028}y]]]\n",
            &[],
            ".z = 1",
            "a: [[['x\u{2028}        y']]]\nz: 1\n",
        ),
        (
            "a: {b: [x\u{2028}y, {c: 'p\u{2029}q'}]}\n",
            &[],
            ".z = 1",
            "a: {b: ['x\u{2028}      y', {c: 'p\u{2029}        q'}]}\nz: 1\n",
        ),
        (
            "a: [x\u{2028}y, z]\nb: {c: 'p\u{2029}q'}\n",
            &[],
            ".z = 1",
            "a: ['x\u{2028}    y', z]\nb: {c: 'p\u{2029}    q'}\nz: 1\n",
        ),
    ])
}

/// JSON output is unchanged: the characters are escaped, not re-indented.
#[test]
fn test_4865_json() -> Result<()> {
    check(&[
        (
            "a: 'x\u{2028}y'\n",
            &["-o=json"],
            ".",
            "{\n  \"a\": \"x\\u2028y\"\n}\n",
        ),
        (
            "a: [x\u{2029}y]\n",
            &["-o=json"],
            ".",
            "{\n  \"a\": [\n    \"x\\u2029y\"\n  ]\n}\n",
        ),
    ])
}
