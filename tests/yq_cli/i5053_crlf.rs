//! #5053 (item 3): in a CRLF file, a blank line inside the first document's leading comment block is `# \r` in yq's
//! output -- the `\r` is comment text to go-yaml -- where an LF file keeps the bare blank line. Every comment line of
//! that block keeps its `\r`. Each row is a live yq v4.53.3 capture, on the streaming route (`.`), the DOM route
//! (`-P`) and a write (`.z = 1`); the LF and no-blank rows were already right.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, filter, flags, expected output).
const ROWS: &[(&str, &str, &[&str], &str)] = &[
    ("# c\r\n\r\na: 1\r\n", ".", &[], "# c\r\n# \r\na: 1\n"),
    ("# c\r\n\r\na: 1\r\n", ".", &["-P"], "# c\r\n# \r\na: 1\n"),
    (
        "# c\r\n\r\na: 1\r\n",
        ".z = 1",
        &[],
        "# c\r\n# \r\na: 1\nz: 1\n",
    ),
    (
        "# c\r\n\r\n\r\na: 1\r\n",
        ".",
        &[],
        "# c\r\n# \r\n# \r\na: 1\n",
    ),
    (
        "# c\r\n\r\n\r\na: 1\r\n",
        ".",
        &["-P"],
        "# c\r\n# \r\n# \r\na: 1\n",
    ),
    (
        "# c\r\n\r\n\r\na: 1\r\n",
        ".z = 1",
        &[],
        "# c\r\n# \r\n# \r\na: 1\nz: 1\n",
    ),
    ("\r\n# c\r\na: 1\r\n", ".", &[], "# \r\n# c\r\na: 1\n"),
    ("\r\n# c\r\na: 1\r\n", ".", &["-P"], "# \r\n# c\r\na: 1\n"),
    (
        "\r\n# c\r\na: 1\r\n",
        ".z = 1",
        &[],
        "# \r\n# c\r\na: 1\nz: 1\n",
    ),
    ("---\r\n\r\na: 1\r\n", ".", &[], "---\r\n# \r\na: 1\n"),
    ("---\r\n\r\na: 1\r\n", ".", &["-P"], "---\r\n# \r\na: 1\n"),
    (
        "---\r\n\r\na: 1\r\n",
        ".z = 1",
        &[],
        "---\r\n# \r\na: 1\nz: 1\n",
    ),
    ("---\r\n# c\r\na: 1\r\n", ".", &[], "---\r\n# c\r\na: 1\n"),
    (
        "---\r\n# c\r\na: 1\r\n",
        ".",
        &["-P"],
        "---\r\n# c\r\na: 1\n",
    ),
    (
        "---\r\n# c\r\na: 1\r\n",
        ".z = 1",
        &[],
        "---\r\n# c\r\na: 1\nz: 1\n",
    ),
    (
        "# c\n\r\n# d\n\na: 1\n",
        ".",
        &[],
        "# c\n# \r\n# d\n\na: 1\n",
    ),
    (
        "# c\n\r\n# d\n\na: 1\n",
        ".",
        &["-P"],
        "# c\n# \r\n# d\n\na: 1\n",
    ),
    (
        "# c\n\r\n# d\n\na: 1\n",
        ".z = 1",
        &[],
        "# c\n# \r\n# d\n\na: 1\nz: 1\n",
    ),
    (
        "# c\r\n\r\n---\r\na: 1\r\n",
        ".",
        &[],
        "# c\r\n# \r\n---\r\na: 1\n",
    ),
    (
        "# c\r\n\r\n---\r\na: 1\r\n",
        ".",
        &["-P"],
        "# c\r\n# \r\n---\r\na: 1\n",
    ),
    (
        "# c\r\n\r\n---\r\na: 1\r\n",
        ".z = 1",
        &[],
        "# c\r\n# \r\n---\r\na: 1\nz: 1\n",
    ),
    (
        "# c\r\n\r\n# d\r\n---\r\na: 1\r\n",
        ".",
        &[],
        "# c\r\n# \r\n# d\r\n---\r\na: 1\n",
    ),
    (
        "# c\r\n\r\n# d\r\n---\r\na: 1\r\n",
        ".",
        &["-P"],
        "# c\r\n# \r\n# d\r\n---\r\na: 1\n",
    ),
    (
        "# c\r\n\r\n# d\r\n---\r\na: 1\r\n",
        ".z = 1",
        &[],
        "# c\r\n# \r\n# d\r\n---\r\na: 1\nz: 1\n",
    ),
    ("# c\n\na: 1\n", ".", &[], "# c\n\na: 1\n"),
    ("# c\n\na: 1\n", ".", &["-P"], "# c\n\na: 1\n"),
    ("# c\n\na: 1\n", ".z = 1", &[], "# c\n\na: 1\nz: 1\n"),
    ("# c\r\na: 1\r\n", ".", &[], "# c\r\na: 1\n"),
    ("# c\r\na: 1\r\n", ".", &["-P"], "# c\r\na: 1\n"),
    ("# c\r\na: 1\r\n", ".z = 1", &[], "# c\r\na: 1\nz: 1\n"),
    ("# c\r\n\r\n- 1\r\n", ".", &[], "# c\r\n# \r\n- 1\n"),
    ("# c\r\n\r\n- 1\r\n", ".", &["-P"], "# c\r\n# \r\n- 1\n"),
];

#[test]
fn test_blank_line_in_a_crlf_documents_leading_comment_is_a_comment_line_5053() -> Result<()> {
    for (doc, filter, flags, expected) in ROWS {
        let (stdout, code) = run_yq_stdin(filter, doc, flags)?;
        assert_eq!(
            (stdout.as_str(), code),
            (*expected, 0),
            "{doc:?} {filter} {flags:?}"
        );
    }
    Ok(())
}
