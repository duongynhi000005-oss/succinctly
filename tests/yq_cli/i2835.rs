//! #2835 (part 1): a comment floated off an absent bare item heads the next item of the same
//! sequence when that item is a compact mapping or a nested sequence, the node `.[i]` resolves to,
//! not the item's first key or inner item. The head hook for such an item (`attach_item_head_at`)
//! stood aside whenever a comment was floated, to keep a float from a sequence that has since
//! closed (`k:` / `  - - # c1` / `  - - # c2`) out of the stem's head; it now stands aside only
//! then.
//!
//! Every expected output is captured from yq v4.53.3.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, `yq .` output).
const ROWS: &[(&str, &str)] = &[
    ("- # c\n- b: 1\n", "-\n# c\n- b: 1\n"),
    ("- # c\n- - 1\n", "-\n# c\n- - 1\n"),
    ("- # c\n- - 1\n- 2\n", "-\n# c\n- - 1\n- 2\n"),
    (
        "a:\n  - # c\n  - b: 1\nz: 2\n",
        "a:\n  -\n  # c\n  - b: 1\nz: 2\n",
    ),
    (
        "k:\n  - - # c1\n  - - # c2\n",
        "k:\n  - -\n  - -\n# c1\n# c2\n",
    ),
    ("- # c\n- # d\n- b: 1\n", "-\n-\n# c\n# d\n- b: 1\n"),
    (
        "- # c\n- b: 1\n  d: 2\n- e: 3\n",
        "-\n# c\n- b: 1\n  d: 2\n- e: 3\n",
    ),
    ("-\n- # c\n- b: 1\n", "-\n-\n# c\n- b: 1\n"),
    ("- 1\n# h\n- b: 1\n", "- 1\n# h\n- b: 1\n"),
    ("- # c\n- b: 1\n# f\n", "-\n# c\n- b: 1\n# f\n"),
    ("- - # c\n  - b: 1\n- 2\n", "- -\n  # c\n  - b: 1\n- 2\n"),
    (
        "k:\n  - # c\n  - - 1\nz: 2\n",
        "k:\n  -\n  # c\n  - - 1\nz: 2\n",
    ),
    ("- # c\n  # d\n- b: 1\n", "-\n# c\n# d\n- b: 1\n"),
    ("- # c\n\n- b: 1\n", "-\n# c\n- b: 1\n"),
    ("- # c\n- &a b: 1\n", "-\n# c\n- &a b: 1\n"),
    ("- # c\n- !!map\n  b: 1\n", "-\n- !!map\n  # c\n  b: 1\n"),
    ("- # c\n- [1]\n", "-\n# c\n- [1]\n"),
    ("- # c\n- {a: 1}\n", "-\n# c\n- {a: 1}\n"),
];

#[test]
fn test_2835_a_a_floated_comment_heads_the_compact_sibling() -> Result<()> {
    for (input, expected) in ROWS {
        let (out, code) = run_yq_stdin(".", input, &[])?;
        assert_eq!(code, 0, "{input:?}");
        assert_eq!(&out, expected, "{input:?}");
    }
    Ok(())
}

/// The getters the issue lists: the sibling has the head, its first key / inner item has none.
#[test]
fn test_2835_b_the_head_is_the_siblings_not_its_first_keys() -> Result<()> {
    let getters: &[(&str, &str, &str)] = &[
        ("- # c\n- b: 1\n", ".[1] | head_comment", "c\n"),
        ("- # c\n- b: 1\n", ".[1].b | key | head_comment", "\n"),
        ("- # c\n- - 1\n", ".[1] | head_comment", "c\n"),
        ("- # c\n- - 1\n", ".[1][0] | head_comment", "\n"),
        (
            "k:\n  - - # c1\n  - - # c2\n",
            ".k | key | foot_comment",
            "c1\n",
        ),
    ];
    for (input, filter, expected) in getters {
        let (out, code) = run_yq_stdin(filter, input, &[])?;
        assert_eq!(code, 0, "{input:?} {filter}");
        assert_eq!(&out, expected, "{input:?} {filter}");
    }
    Ok(())
}

/// Neighbouring shapes that still differ from yq, pinned with this tool's answer so a change is
/// deliberate (recorded in `limitations.md`): a second absent item between the float and the
/// sibling, a float from a nested sequence that closed at the sibling, an explicit-key sibling.
#[test]
fn test_2835_c_neighbouring_shapes_still_differ() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        ("- # c\n-\n- b: 1\n", "-\n-\n- b: 1\n  # c\n"),
        ("- - # c\n- b: 1\n", "- -\n- # c\n  b: 1\n"),
        ("- # c\n- ? k\n  : v\n", "-\n- # c\n  k: v\n"),
    ];
    for (input, ours) in rows {
        let (out, code) = run_yq_stdin(".", input, &[])?;
        assert_eq!(code, 0, "{input:?}");
        assert_eq!(&out, ours, "{input:?}");
    }
    Ok(())
}
