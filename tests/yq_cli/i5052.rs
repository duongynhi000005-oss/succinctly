//! #5052: a document's trailing comment blocks keep one blank line between them, as yq's foot comment does
//! (`a: 1` / blank / `# A` / blank / `# B` prints `# A`, a blank line, `# B`, and answers `. | foot_comment` as
//! `"A\n\nB"`), on the `.`, `-P` and write routes and across a `---`. A block attached to the last key (no blank
//! line before it) is that key's foot and is unchanged. Each row is a live yq v4.53.3 capture.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, extra args, filter, expected output).
const ROWS: &[(&str, &[&str], &str, &str)] = &[
    ("a: 1\n\n# A\n\n# B\n", &[], ".", "a: 1\n# A\n\n# B\n"),
    ("a: 1\n\n# A\n\n# B\n", &["-P"], ".", "a: 1\n# A\n\n# B\n"),
    (
        "a: 1\n\n# A\n\n# B\n",
        &[],
        ".z = 1",
        "a: 1\nz: 1\n# A\n\n# B\n",
    ),
    (
        "a: 1\n\n# A\n\n\n# B\n\n# C\n",
        &[],
        ".",
        "a: 1\n# A\n\n# B\n\n# C\n",
    ),
    (
        "a: 1\n\n# A\n# A2\n\n# B\n",
        &[],
        ".",
        "a: 1\n# A\n# A2\n\n# B\n",
    ),
    (
        "a:\n  b: 1\n\n# A\n\n# B\n",
        &["-P"],
        ".",
        "a:\n  b: 1\n# A\n\n# B\n",
    ),
    ("- 1\n\n# A\n\n# B\n", &[], ".", "- 1\n# A\n\n# B\n"),
    (
        "a: 1\n\n# A\n\n# B\n---\nb: 2\n",
        &[],
        ".",
        "a: 1\n# A\n\n# B\n---\nb: 2\n",
    ),
    (
        "a: 1\n\n# A\n\n# B\n---\nb: 2\n",
        &[],
        ".z = 1",
        "a: 1\nz: 1\n# A\n\n# B\n---\nb: 2\nz: 1\n",
    ),
    ("a: 1\n\n# A\n\n# B\n", &[], ". | foot_comment", "A\n\nB\n"),
    ("a: 1\n# A\n\n# B\n", &[], ".", "a: 1\n# A\n# B\n"),
    (
        "a: 1\n# A\n\n# B\n",
        &[],
        ".z = 1",
        "a: 1\n# A\n\nz: 1\n# B\n",
    ),
];

#[test]
fn test_document_foot_keeps_one_blank_line_between_comment_blocks_5052() -> Result<()> {
    for (doc, args, filter, expected) in ROWS {
        let (stdout, code) = run_yq_stdin(filter, doc, args)?;
        assert_eq!(
            (stdout.as_str(), code),
            (*expected, 0),
            "{doc:?} {args:?} {filter}"
        );
    }
    Ok(())
}
