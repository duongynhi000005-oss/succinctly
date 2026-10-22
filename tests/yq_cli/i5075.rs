//! #5075: a block scalar at an indented document root read as the empty string when its content was not
//! indented deeper than the header. A root node's parent is the document, so its content only has to be
//! indented more than column 0, wherever the header sits. Each row is a live yq v4.53.3 capture.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, extra args, expected output).
const ROWS: &[(&str, &[&str], &str)] = &[
    ("  |\n  x\n", &["-o", "json", "-I0"], "\"x\\n\"\n"),
    ("  >\n  a\n  b\n", &["-o", "json", "-I0"], "\"a b\\n\"\n"),
    ("  |\n x: y\n", &["-o", "json", "-I0"], "\"x: y\\n\"\n"),
    ("  |\n   x: y\n", &["-o", "json", "-I0"], "\"x: y\\n\"\n"),
    (" |\n x\n", &["-o", "json", "-I0"], "\"x\\n\"\n"),
    ("    |\n  x\n", &["-o", "json", "-I0"], "\"x\\n\"\n"),
    ("  |\n  x\n  y\n", &["-o", "json", "-I0"], "\"x\\ny\\n\"\n"),
    ("  |2\n  x\n", &["-o", "json", "-I0"], "\"x\\n\"\n"),
    ("  |+\n  x\n\n", &["-o", "json", "-I0"], "\"x\\n\\n\"\n"),
    ("  >-\n  a\n\n  b\n", &["-o", "json", "-I0"], "\"a\\nb\"\n"),
    ("  &a |\n  x\n", &["-o", "json", "-I0"], "\"x\\n\"\n"),
    ("&a\n  |\n  x\n", &["-o", "json", "-I0"], "\"x\\n\"\n"),
    ("# c\n  |\n  x\n", &["-o", "json", "-I0"], "\"x\\n\"\n"),
    ("---\n  |\n  x\n", &["-o", "json", "-I0"], "\"x\\n\"\n"),
    (
        "  |\n  x\n---\n  >\n  y\n",
        &["-o", "json", "-I0"],
        "\"x\\n\"\n\"y\\n\"\n",
    ),
    ("  |\n  x\n", &[], "x\n\n"),
    ("  |\n  x\n", &["length"], "2\n"),
];

#[test]
fn test_indented_document_root_block_scalar_reads_its_content_5075() -> Result<()> {
    for (doc, args, expected) in ROWS {
        let (filter, rest) = match args.first() {
            Some(&"length") => ("length", &args[1..]),
            _ => (".", *args),
        };
        let (stdout, code) = run_yq_stdin(filter, doc, rest)?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?} {args:?}");
    }
    Ok(())
}
