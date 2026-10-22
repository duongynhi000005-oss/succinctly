//! #5067 (found while fixing it): a block scalar with an explicit indent indicator whose first line is indented
//! less than that is empty, as an auto-detected one is. The end-of-block scan never looked at that first line, so a
//! `---` marker there was swallowed: `a: |2` / `---` / `b: 1` merged two documents into one, and `|2` / `---` at
//! the root lost the empty document after the marker. Each row is a live yq v4.53.3 capture.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, `-o json -I0 .` output).
const ROWS: &[(&str, &str)] = &[
    ("|2\n---\n", "\"\"\nnull\n"),
    ("a: |1\n---\nb: 1\n", "{\"a\":\"\"}\n{\"b\":1}\n"),
    ("a: |2\nb: 1\n", "{\"a\":\"\",\"b\":1}\n"),
    ("a: |2\n---\n", "{\"a\":\"\"}\nnull\n"),
    ("- |2\n- b\n", "[\"\",\"b\"]\n"),
    ("|2\r\n---\r\n", "\"\"\nnull\n"),
    ("a: |2\r\n---\r\nb: 1\r\n", "{\"a\":\"\"}\n{\"b\":1}\n"),
];

#[test]
fn test_explicit_indent_block_scalar_with_a_dedented_first_line_is_empty_5067() -> Result<()> {
    for (doc, expected) in ROWS {
        let (stdout, code) = run_yq_stdin(".", doc, &["-o", "json", "-I0"])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?}");
    }
    Ok(())
}
