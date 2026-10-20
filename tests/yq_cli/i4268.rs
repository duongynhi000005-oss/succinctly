//! #4268: the rest of a bare `-` item's comments in front of a scalar or flow value.
//!
//! * Comment lines between the dash and a scalar or flow value on a later line (`-` / `  # f` /
//!   `  z`) head that value, as they head a block mapping's first key or a block sequence's first
//!   item. The bare-item arm leaves them pending "for the item's first node", and a scalar was not
//!   given them, so they rode on to the next item.
//! * A flow collection item written `- # c` / `  [1]` answers `c` as its `head_comment`, as a
//!   scalar item does since #4810 (the getter excluded every container, flow ones included).
//!
//! Every expected output is captured from yq v4.53.3.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, `yq .` output).
const ROWS: &[(&str, &str)] = &[
    ("-\n  # f\n  z\n- y\n", "# f\n- z\n- y\n"),
    ("a:\n  -\n    # f\n    z\n", "a:\n  # f\n  - z\n"),
    (
        "a:\n  - 0\n  -\n    # f\n    z\n  - y\n",
        "a:\n  - 0\n  # f\n  - z\n  - y\n",
    ),
    ("-\n  # f\n  [1]\n- y\n", "# f\n- [1]\n- y\n"),
    ("-\n  # f\n  \"q\"\n- y\n", "# f\n- \"q\"\n- y\n"),
    ("-\n  # f\n  k: v\n- y\n", "- # f\n  k: v\n- y\n"),
    ("-\n  # f\n  - x\n- y\n", "- # f\n  - x\n- y\n"),
    ("-\n  # f\n  # g\n  z\n- y\n", "# f\n# g\n- z\n- y\n"),
    ("# h\n-\n  # f\n  z\n- y\n", "# h\n# f\n- z\n- y\n"),
    (
        "a:\n  - 1\n  # g\n  -\n    # f\n    z\nb: 2\n",
        "a:\n  - 1\n  # g\n  # f\n  - z\nb: 2\n",
    ),
    (
        "-\n  # f\n  z\n  # after\n- y\n",
        "# f\n- z\n# after\n- y\n",
    ),
    (
        "a:\n  - # c\n    [1]\n  - y\n",
        "a:\n  # c\n  - [1]\n  - y\n",
    ),
    ("- # c\n  {k: 1}\n- 2\n", "# c\n- {k: 1}\n- 2\n"),
];

#[test]
fn test_4268_a_comment_lines_before_a_scalar_or_flow_value_head_it() -> Result<()> {
    for (input, expected) in ROWS {
        let (out, code) = run_yq_stdin(".", input, &[])?;
        assert_eq!(code, 0, "{input:?}");
        assert_eq!(&out, expected, "{input:?}");
    }
    Ok(())
}

#[test]
fn test_4268_b_the_getters_follow() -> Result<()> {
    let getters: &[(&str, &str, &str)] = &[
        ("-\n  # f\n  z\n- y\n", ".[0] | head_comment", "f\n"),
        ("-\n  # f\n  z\n- y\n", ".[1] | head_comment", "\n"),
        ("-\n  # f\n  [1]\n- y\n", ".[0] | head_comment", "f\n"),
        ("-\n  # f\n  [1]\n- y\n", ".[1] | head_comment", "\n"),
        (
            "a:\n  - # c\n    [1]\n  - y\n",
            ".a[0] | head_comment",
            "c\n",
        ),
        ("- # c\n  {k: 1}\n- 2\n", ".[0] | head_comment", "c\n"),
        ("- # c\n  [1]\n- 2\n", ".[0] | head_comment", "c\n"),
        ("- # c\n  [1]\n- 2\n", ".[1] | head_comment", "\n"),
        ("- # c\n  plain\n- 2\n", ".[0] | head_comment", "c\n"),
        ("- # c\n  # f\n  z\n- y\n", ".[0] | head_comment", "c\nf\n"),
        ("-   # t\n  # f\n  z\n", ".[0] | head_comment", "t\nf\n"),
    ];
    for (input, filter, expected) in getters {
        let (out, code) = run_yq_stdin(filter, input, &[])?;
        assert_eq!(code, 0, "{input:?} {filter}");
        assert_eq!(&out, expected, "{input:?} {filter}");
    }
    Ok(())
}

/// Shapes that still differ from yq, pinned with this tool's answer so a change is deliberate
/// (recorded in `limitations.md`): a blank line between the comment and the value (the comment is
/// settled backwards as the previous item's foot), and a comment on the dash line as well as one
/// below it (the printed order is the own-line one first, yq's the dash one first; the `head_comment` getter has the yq order).
#[test]
fn test_4268_c_neighbouring_shapes_still_differ() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        ("-\n  # f\n\n  z\n- y\n", "- z\n# f\n\n- y\n"),
        ("- # c\n  # f\n  z\n- y\n", "# f\n# c\n- z\n- y\n"),
    ];
    for (input, ours) in rows {
        let (out, code) = run_yq_stdin(".", input, &[])?;
        assert_eq!(code, 0, "{input:?}");
        assert_eq!(&out, ours, "{input:?}");
    }
    Ok(())
}
