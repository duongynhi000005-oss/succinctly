//! #5053 (item 2): a comment block that follows another, one blank line between them, at the column of the open block
//! the first one closed, before a key that dedents past it: the first block is the last key's foot, and yq gives the
//! second to the key that owns the closed block (`a:` / `  b: 1` / `  # A` / blank / `  # B` / `c: 2` is `# B` on
//! `.a`'s key, printed at column 0 with the blank line that separates it from `c`). Two blank lines, a first block at
//! an odd column, a following key at the same level and a block at the end of the document are not moved
//! (the rows below that stay as they were). Each row is
//! a live yq v4.53.3 capture.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, filter, flags, expected output).
const ROWS: &[(&str, &str, &[&str], &str)] = &[
    (
        "a:\n  b: 1\n  # A\n\n  # B\nc: 2\n",
        ".",
        &[],
        "a:\n  b: 1\n  # A\n# B\n\nc: 2\n",
    ),
    (
        "a:\n  b: 1\n  # A\n\n  # B\nc: 2\n",
        ".",
        &["-P"],
        "a:\n  b: 1\n  # A\n# B\n\nc: 2\n",
    ),
    (
        "a:\n  b: 1\n  # A\n\n  # B\nc: 2\n",
        ".z = 1",
        &[],
        "a:\n  b: 1\n  # A\n# B\n\nc: 2\nz: 1\n",
    ),
    (
        "a:\n  b: 1\n  # A\n\n  # B\n\n---\nb: 2\n",
        ".",
        &[],
        "a:\n  b: 1\n  # A\n# B\n---\nb: 2\n",
    ),
    (
        "a:\n  b: 1\n  # A\n\n  # B\n",
        ".",
        &[],
        "a:\n  b: 1\n  # A\n# B\n",
    ),
    (
        "a:\n  b: 1\n  # A\n  # A2\n\n  # B\nc: 2\n",
        ".",
        &[],
        "a:\n  b: 1\n  # A\n  # A2\n# B\n\nc: 2\n",
    ),
    (
        "a:\n  b: 1\n  # A\n\n  # B\n  d: 3\nc: 2\n",
        ".",
        &[],
        "a:\n  b: 1\n  # A\n\n  # B\n  d: 3\nc: 2\n",
    ),
    (
        "a:\n  b: 1\n  # A\n\n# B\nc: 2\n",
        ".",
        &[],
        "a:\n  b: 1\n  # A\n# B\nc: 2\n",
    ),
    (
        "a:\n  b: 1\n\n  # B\nc: 2\n",
        ".",
        &[],
        "a:\n  b: 1\n  # B\nc: 2\n",
    ),
    (
        "a:\n  b:\n    c: 1\n   # A\n\n    # B\nd: 2\n",
        ".",
        &[],
        "a:\n  b:\n    c: 1\n    # A\n    # B\nd: 2\n",
    ),
    (
        "a: 1\n# A\n\n# B\nc: 2\n",
        ".",
        &[],
        "a: 1\n# A\n\n# B\nc: 2\n",
    ),
    (
        "a:\n  - 1\n  # A\n\n  # B\nc: 2\n",
        ".",
        &[],
        "a:\n  - 1\n  # A\n# B\n\nc: 2\n",
    ),
];

#[test]
fn test_second_foot_block_after_a_blank_belongs_to_the_outer_key_5053() -> Result<()> {
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
