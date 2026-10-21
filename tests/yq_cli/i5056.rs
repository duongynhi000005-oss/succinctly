//! #5056: a standalone comment block with a blank line above and below it, before a node that dedents out of the
//! block the previous node is in, is that previous node's foot in yq (printed under it), unless the comment sits at
//! the next node's own column, where it heads that node. Its blocks keep one blank line between them. A second block
//! after a node that already has a foot is left as it was. Each row is a live yq v4.53.3 capture.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, extra args, filter, expected output).
const ROWS: &[(&str, &[&str], &str, &str)] = &[
    (
        "b:\n  s:\n  - n: a\n    u: x\n\n  # C\n\nd: 1\n",
        &[],
        ".",
        "b:\n  s:\n    - n: a\n      u: x\n      # C\nd: 1\n",
    ),
    (
        "b:\n  s:\n  - n: a\n    u: x\n\n  # C\n\nd: 1\n",
        &["-P"],
        ".",
        "b:\n  s:\n    - n: a\n      u: x\n      # C\nd: 1\n",
    ),
    (
        "b:\n  s:\n  - n: a\n    u: x\n\n  # C\n\nd: 1\n",
        &[],
        ".z = 1",
        "b:\n  s:\n    - n: a\n      u: x\n      # C\nd: 1\nz: 1\n",
    ),
    (
        "a:\n  b:\n    c: 1\n\n    # C\n\n  d: 1\n",
        &[],
        ".",
        "a:\n  b:\n    c: 1\n    # C\n  d: 1\n",
    ),
    (
        "a:\n  b:\n    c: 1\n\n# C\n\n  d: 1\n",
        &[],
        ".",
        "a:\n  b:\n    c: 1\n    # C\n  d: 1\n",
    ),
    (
        "b:\n  s:\n    - 1\n\n    # C\n\nd: 1\n",
        &[],
        ".",
        "b:\n  s:\n    - 1\n    # C\nd: 1\n",
    ),
    (
        "k:\n  - a: 1\n    b:\n      - 2\n\n    # C\n\n    # D\n\nd: 1\n",
        &[],
        ".",
        "k:\n  - a: 1\n    b:\n      - 2\n      # C\n\n      # D\nd: 1\n",
    ),
    (
        "a:\n  b: &x\n    c: 1\n\n   # C\n\n   # D\n\n---\nz: 1\n",
        &[],
        ".",
        "a:\n  b: &x\n    c: 1\n    # C\n\n    # D\n---\nz: 1\n",
    ),
    (
        "a:\n  b:\n    c: 1\n\n  # C\n\n  d: 1\n",
        &[],
        ".",
        "a:\n  b:\n    c: 1\n  # C\n  d: 1\n",
    ),
    (
        "a:\n  b:\n    c: 1\n\n# C\n\nd: 1\n",
        &[],
        ".",
        "a:\n  b:\n    c: 1\n# C\nd: 1\n",
    ),
    (
        "- a:\n    b: 1\n    # C\n\n    # D\n\n- 9\n",
        &["-o", "json", "-I0"],
        "[.. | select(key != null) | [key, (key|head_comment), (key|foot_comment)]]",
        "[[0,\"\",\"\"],[\"a\",\"\",\"\"],[\"b\",\"\",\"C\"],[1,\"\",\"\"]]\n",
    ),
    (
        "b:\n  s:\n  - n: a\n    u: x\n\n  # C\n\nd: 1\n",
        &["-o", "json", "-I0"],
        "[(.d | key | head_comment), (.b | key | foot_comment)]",
        "[\"\",\"\"]\n",
    ),
];

#[test]
fn test_blank_separated_comment_before_a_dedent_is_the_previous_nodes_foot_5056() -> Result<()> {
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
