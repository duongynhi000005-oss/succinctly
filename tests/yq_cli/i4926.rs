//! #4926: `sort`, `sort_by(f)`, `reverse`, `unique` and `unique_by(f)` of a document's own
//! sequence keep the comment block above it (the banner a config file starts with). yq reorders
//! the sequence node in place, so the node's head comment stays on top; the items, which stream
//! from their own cursors, carry only their line comments.

use anyhow::Result;

use crate::run_yq_stdin;

type Row = (
    &'static str,
    &'static [&'static str],
    &'static str,
    &'static str,
);

/// Every row is captured from yq v4.53.3: the document, the extra arguments, the filter and the
/// expected output. The rows at the end are controls that must not gain a head: an element read
/// out of the sorted sequence, and a `map`, which builds a new node in yq.
#[test]
fn test_4926_a_reordered_root_sequence_keeps_its_head_comment() -> Result<()> {
    let rows: &[Row] = &[
        (
            "# top\n- 2 # two\n- 1 # one\n",
            &[],
            "sort",
            "# top\n- 1 # one\n- 2 # two\n",
        ),
        (
            "# top\n- 2 # two\n- 1 # one\n",
            &[],
            "sort_by(.)",
            "# top\n- 1 # one\n- 2 # two\n",
        ),
        (
            "# top\n- 2 # two\n- 1 # one\n",
            &[],
            "reverse",
            "# top\n- 1 # one\n- 2 # two\n",
        ),
        (
            "# top\n- 2 # two\n- 1 # one\n",
            &[],
            "unique",
            "# top\n- 2 # two\n- 1 # one\n",
        ),
        (
            "# top\n- 2 # two\n- 1 # one\n",
            &[],
            "unique_by(.)",
            "# top\n- 2 # two\n- 1 # one\n",
        ),
        (
            "# top\n- 2 # two\n- 1 # one\n",
            &[],
            "(sort)",
            "# top\n- 1 # one\n- 2 # two\n",
        ),
        (
            "# top\n\n# second\n- 2\n- 1\n",
            &[],
            "sort",
            "# top\n\n# second\n- 1\n- 2\n",
        ),
        (
            "# top\n\n# second\n- 2\n- 1\n",
            &[],
            "reverse",
            "# top\n\n# second\n- 1\n- 2\n",
        ),
        (
            "---\n# banner\n- b\n- a\n",
            &[],
            "sort",
            "---\n# banner\n- a\n- b\n",
        ),
        (
            "---\n# banner\n- b\n- a\n",
            &[],
            "unique",
            "---\n# banner\n- b\n- a\n",
        ),
        (
            "# top\n- {b: 2}\n- {b: 1}\n",
            &[],
            "sort_by(.b)",
            "# top\n- {b: 1}\n- {b: 2}\n",
        ),
        ("# top\n- 2\n- 2\n- 1\n", &[], "unique", "# top\n- 2\n- 1\n"),
        (
            "# top\n- 3\n- 1\n- 2\n",
            &[],
            "sort",
            "# top\n- 1\n- 2\n- 3\n",
        ),
        (
            "# top\n- 3\n- 1\n- 2\n",
            &[],
            "reverse",
            "# top\n- 2\n- 1\n- 3\n",
        ),
        (
            "# top\n- 3\n- 1\n- 2\n",
            &["-I=4"],
            "sort",
            "# top\n- 1\n- 2\n- 3\n",
        ),
        (
            "# top\n- 2 # two\n- 1 # one\n",
            &["-o=json", "-I=0"],
            "sort",
            "[1,2]\n",
        ),
        ("# top\n- 2 # two\n- 1 # one\n", &[], "sort | .[0]", "1\n"),
        (
            "# top\n- 2 # two\n- 1 # one\n",
            &[],
            "map(.)",
            "- 2 # two\n- 1 # one\n",
        ),
    ];
    for (doc, extra, filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, doc, extra)?;
        assert_eq!(code, 0, "{doc:?} {extra:?} {filter}");
        assert_eq!(out, *want, "{doc:?} {extra:?} {filter}");
    }
    Ok(())
}
