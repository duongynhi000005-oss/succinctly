//! #5053 (item 1): a trailing comment block separated from the `---`/`...` marker by a blank line, with no node
//! it is attached to, is the closing document's foot, as it is in yq, and not the next document's head. Each row is a
//! live yq v4.53.3 capture; the controls (a comment-only first document, a block attached to its key, a block
//! flush against the marker) were already right.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, filter, flags, expected output).
const ROWS: &[(&str, &str, &[&str], &str)] = &[
    (
        "a: 1\n\n# A\n\n---\nb: 2\n",
        ".",
        &[],
        "a: 1\n# A\n---\nb: 2\n",
    ),
    (
        "a: 1\n\n# A\n\n---\nb: 2\n",
        ".",
        &["-P"],
        "a: 1\n# A\n---\nb: 2\n",
    ),
    (
        "a: 1\n\n# A\n\n---\nb: 2\n",
        ".z = 1",
        &[],
        "a: 1\nz: 1\n# A\n---\nb: 2\nz: 1\n",
    ),
    (
        "a: 1\n\n# A\n\n---\nb: 2\n\n# B\n\n---\nc: 3\n",
        ".",
        &[],
        "a: 1\n# A\n---\nb: 2\n# B\n---\nc: 3\n",
    ),
    (
        "a:\n  b: 1\n\n# A\n\n---\nb: 2\n",
        ".",
        &[],
        "a:\n  b: 1\n# A\n---\nb: 2\n",
    ),
    (
        "a:\n  b: 1\n\n  # A\n\n---\nb: 2\n",
        ".",
        &[],
        "a:\n  b: 1\n  # A\n---\nb: 2\n",
    ),
    (
        "- 1\n\n# A\n\n...\n---\nb: 2\n",
        ".",
        &[],
        "- 1\n# A\n---\nb: 2\n",
    ),
    (
        "- 1\n\n# A\n\n---\nb: 2\n",
        ".",
        &[],
        "- 1\n# A\n---\nb: 2\n",
    ),
    ("# A\n\n---\nb: 2\n", ".", &[], "# A\n\n---\nb: 2\n"),
    (
        "a: 1\n# A\n\n---\nb: 2\n",
        ".",
        &[],
        "a: 1\n# A\n---\nb: 2\n",
    ),
    (
        "a: 1\n\n# A\n---\nb: 2\n",
        ".",
        &[],
        "a: 1\n# A\n---\nb: 2\n",
    ),
    // Controls with a comment-only document around the marker, which keep the ordinary path.
    (
        "a: 1\n---\n# A\n\n---\nb: 2\n",
        ".",
        &[],
        "a: 1\n# A\n---\n\n---\nb: 2\n",
    ),
    (
        "a: 1\n\n# A\n\n---\n# B\n\n---\nc: 3\n",
        ".",
        &[],
        "a: 1\n# B\n---\n\n---\nc: 3\n",
    ),
    (
        "a: 1\n---\n# O\n\n...\n---\n",
        ".",
        &[],
        "a: 1\n# O\n---\n\n---\n\n",
    ),
];

#[test]
fn test_trailing_comment_block_before_a_blank_separated_marker_is_the_closing_documents_foot_5053(
) -> Result<()> {
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
