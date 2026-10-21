//! #5067: a document-root block scalar (`|`, `>`) whose content holds a `: ` is that scalar and nothing
//! else. It went down the plain-scalar scanner, which ends at the `: `, so what followed (`y` of `x: y`)
//! printed as a second value. Each row is a live yq v4.53.3 capture; a block scalar as a value or a
//! sequence item was right already and stays so.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, extra args, expected output).
const ROWS: &[(&str, &[&str], &str)] = &[
    ("|\n  x: y\n", &["-o", "json", "-I0"], "\"x: y\\n\"\n"),
    (">\n  x: y\n", &["-o", "json", "-I0"], "\"x: y\\n\"\n"),
    ("|-\n  x: y\n", &["-o", "json", "-I0"], "\"x: y\"\n"),
    (
        "|\n  a\n  x: y\n",
        &["-o", "json", "-I0"],
        "\"a\\nx: y\\n\"\n",
    ),
    ("|\n  x: y z\n", &["-o", "json", "-I0"], "\"x: y z\\n\"\n"),
    (
        ">-\n  x: y\n\n  z: w\n",
        &["-o", "json", "-I0"],
        "\"x: y\\nz: w\"\n",
    ),
    ("|2-\n  x: y\n", &["-o", "json", "-I0"], "\"x: y\"\n"),
    ("|1\n  x: y\n", &["-o", "json", "-I0"], "\" x: y\\n\"\n"),
    ("|\n\n  x: y\n", &["-o", "json", "-I0"], "\"\\nx: y\\n\"\n"),
    (
        "|\n  x: y\n  # c\n",
        &["-o", "json", "-I0"],
        "\"x: y\\n# c\\n\"\n",
    ),
    ("| # c\n  x: y\n", &["-o", "json", "-I0"], "\"x: y\\n\"\n"),
    ("&a |\n  x: y\n", &["-o", "json", "-I0"], "\"x: y\\n\"\n"),
    (
        "!!str >\n  x: y\n  z: w\n",
        &["-o", "json", "-I0"],
        "\"x: y z: w\\n\"\n",
    ),
    ("|\n  x: y\n...\n", &["-o", "json", "-I0"], "\"x: y\\n\"\n"),
    (
        "|\n  x: y\n---\nb: 1\n",
        &["-o", "json", "-I0"],
        "\"x: y\\n\"\n{\"b\":1}\n",
    ),
    ("|\n  x:y\n", &["-o", "json", "-I0"], "\"x:y\\n\"\n"),
    (
        "a: |\n  x: y\n",
        &["-o", "json", "-I0"],
        "{\"a\":\"x: y\\n\"}\n",
    ),
    ("- |\n  x: y\n", &["-o", "json", "-I0"], "[\"x: y\\n\"]\n"),
    ("&a\n|\n  x: y\n", &["-o", "json", "-I0"], "\"x: y\\n\"\n"),
    (
        "&a # c\n|\n  x: y\n",
        &["-o", "json", "-I0"],
        "\"x: y\\n\"\n",
    ),
    (
        "!!str\n>\n  x: y\n",
        &["-o", "json", "-I0"],
        "\"x: y\\n\"\n",
    ),
    (
        "|+\n  x: y\n\n\n",
        &["-o", "json", "-I0"],
        "\"x: y\\n\\n\\n\"\n",
    ),
    (
        "|+\n  x: y\n\n\n---\nb\n",
        &["-o", "json", "-I0"],
        "\"x: y\\n\\n\\n\"\n\"b\"\n",
    ),
    (
        "# lead\n|\n  x: y\n",
        &["-o", "json", "-I0"],
        "\"x: y\\n\"\n",
    ),
    (
        "|\n  x: y\n#foot\n",
        &["-o", "json", "-I0"],
        "\"x: y\\n\"\n",
    ),
    ("|\n  x: y\n", &[], "x: y\n\n"),
];

#[test]
fn test_document_root_block_scalar_with_a_colon_line_is_one_value_5067() -> Result<()> {
    for (doc, args, expected) in ROWS {
        let (stdout, code) = run_yq_stdin(".", doc, args)?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?} {args:?}");
    }
    Ok(())
}
