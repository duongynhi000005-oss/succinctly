//! #4997 (item 2 of the #4099 sweep): a write that makes a document's root another kind -- a
//! comment-only file, `null`, `~` or a scalar written to a mapping or sequence, or back -- kept the
//! preamble only and dropped the comments around the document. yq keeps its head comment and, for
//! a container result, its foot. Each row is a live yq v4.53.3 capture.
//!
//! Not matched (the yq limitations): a non-first document's head, and `# c` / `!!null`.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, filter, expected output).
const ROWS: &[(&str, &str, &str)] = &[
    ("# c\n", ".a = 1", "# c\na: 1\n"),
    ("# c\n", ". = [1]", "# c\n- 1\n"),
    ("# c\n\n", ".a = 1", "# c\n\na: 1\n"),
    ("# c\n# d\n", ".a = 1", "# c\n# d\na: 1\n"),
    ("# c\nnull\n", ".a = 1", "# c\na: 1\n"),
    ("# c\n~\n", ".a = 1", "# c\na: 1\n"),
    ("# c\n5\n", ". = [1]", "# c\n- 1\n"),
    ("# c\n5 # l\n# f\n", ". = {\"a\":1}", "# c\na: 1\n# f\n"),
    ("# c\n[1]\n", ". = {\"a\":1}", "# c\na: 1\n"),
    ("# c\n{a: 1}\n", ". = [1]", "# c\n- 1\n"),
    ("# c\n{a: 1}\n# f\n", ". = [1]", "# c\n- 1\n# f\n"),
    ("# c\n[1]\n# f\n", ". = 5", "# c\n5\n"),
    ("# c\nnull\n# f\n", ".a = 1", "# c\na: 1\n# f\n"),
    ("# c\n# d\n", ".[0] = 1", "# c\n# d\n- 1\n"),
    ("# c\n\n# d\n", ".a = 1", "# c\n\n# d\na: 1\n"),
    (
        "# c\nnull\n---\n# e\nx: 1\n",
        ".a = 1",
        "# c\na: 1\n---\n# e\nx: 1\na: 1\n",
    ),
    ("x: 1\n---\n# c\n", ".a = 1", "x: 1\na: 1\n# c\n---\na: 1\n"),
    ("---\n# c\nnull\n", ".a = 1", "---\n# c\na: 1\n"),
    (
        "# top\n---\n# c\nnull\n",
        ".a = 1",
        "# top\n---\n# c\na: 1\n",
    ),
    ("# c\nnull # l\n", ".a = 1", "# c\na: 1\n"),
    ("# c\n&x null\n", ".a = 1", "# c\n&x\na: 1\n"),
];

#[test]
fn test_root_kind_change_keeps_the_comments_around_the_document_4997() -> Result<()> {
    for (doc, filter, expected) in ROWS {
        let (stdout, code) = run_yq_stdin(filter, doc, &[])?;
        assert_eq!(
            (stdout.as_str(), code),
            (*expected, 0),
            "{filter} over {doc:?}"
        );
    }
    Ok(())
}
