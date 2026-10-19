//! #4856: a yq query that is a `,` of a node of the document and something else (an absent key, a
//! literal, a computation) keeps the node's comments, style and anchors, as a comma of nodes does
//! (#1361). Each branch is answered on its own and the answers are printed in order.

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

type Row = (
    &'static str,
    &'static [&'static str],
    &'static str,
    &'static str,
);

/// Every row is captured from yq v4.53.3: the document, the extra arguments, the filter and the
/// expected output. `.x`'s `# note` comment and `x: &a`'s anchor are what the old route dropped.
#[test]
fn test_4856_a_comma_with_a_non_node_branch_keeps_the_nodes_presentation() -> Result<()> {
    let rows: &[Row] = &[
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            ".x, .zz",
            "p: 1 # note\nnull\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            ".zz, .x",
            "null\np: 1 # note\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            ".x, 1",
            "p: 1 # note\n1\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            "1, .x",
            "1\np: 1 # note\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            ".x, (.y | length)",
            "p: 1 # note\n1\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            ".x, \"s\"",
            "p: 1 # note\ns\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            ".x, .zz, .y",
            "p: 1 # note\nnull\n2\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            ".x, (.zz, .y)",
            "p: 1 # note\nnull\n2\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            "(.x, .zz)",
            "p: 1 # note\nnull\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            ".x, [1]",
            "p: 1 # note\n- 1\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            ".x, {\"a\": 1}",
            "p: 1 # note\na: 1\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            ".zz, .x, .zz",
            "null\np: 1 # note\nnull\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            ".x, .y",
            "p: 1 # note\n2\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &[],
            ".x, .x",
            "p: 1 # note\np: 1 # note\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &["-I=4"],
            ".x, .zz",
            "p: 1 # note\nnull\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &["-o=json", "-I=0"],
            ".x, .zz",
            "{\"p\":1}\nnull\n",
        ),
        (
            "x:\n  p: 1 # note\ny: 2\n",
            &["-o=json", "-I=0"],
            ".x, 1",
            "{\"p\":1}\n1\n",
        ),
        (
            "# head\nx: &a\n  p: 1 # note\nz: *a\nlist: [1, 2] # tail\nq: \"dq\"\ns: 'sq'\n",
            &[],
            ".x, .zz",
            "&a\np: 1 # note\nnull\n",
        ),
        (
            "# head\nx: &a\n  p: 1 # note\nz: *a\nlist: [1, 2] # tail\nq: \"dq\"\ns: 'sq'\n",
            &[],
            ".list, .zz",
            "[1, 2] # tail\nnull\n",
        ),
        (
            "# head\nx: &a\n  p: 1 # note\nz: *a\nlist: [1, 2] # tail\nq: \"dq\"\ns: 'sq'\n",
            &[],
            ".list, 1, .q",
            "[1, 2] # tail\n1\ndq\n",
        ),
        (
            "# head\nx: &a\n  p: 1 # note\nz: *a\nlist: [1, 2] # tail\nq: \"dq\"\ns: 'sq'\n",
            &[],
            ".q, .s, 1",
            "dq\nsq\n1\n",
        ),
        (
            "# head\nx: &a\n  p: 1 # note\nz: *a\nlist: [1, 2] # tail\nq: \"dq\"\ns: 'sq'\n",
            &[],
            ".x.p, .zz, .list[0]",
            "1\nnull\n1\n",
        ),
        (
            "# head\nx: &a\n  p: 1 # note\nz: *a\nlist: [1, 2] # tail\nq: \"dq\"\ns: 'sq'\n",
            &[],
            ".x // .zz, .list",
            "&a\np: 1 # note\n[1, 2] # tail\n",
        ),
        (
            "# head\nx: &a\n  p: 1 # note\nz: *a\nlist: [1, 2] # tail\nq: \"dq\"\ns: 'sq'\n",
            &[],
            ".zz // .x, .list",
            "&a\np: 1 # note\n[1, 2] # tail\n",
        ),
        (
            "# head\nx: &a\n  p: 1 # note\nz: *a\nlist: [1, 2] # tail\nq: \"dq\"\ns: 'sq'\n",
            &[],
            ".x, (.list | length), .s",
            "&a\np: 1 # note\n2\nsq\n",
        ),
        (
            "# head\nx: &a\n  p: 1 # note\nz: *a\nlist: [1, 2] # tail\nq: \"dq\"\ns: 'sq'\n",
            &[],
            "(.x, 1), .list",
            "&a\np: 1 # note\n1\n[1, 2] # tail\n",
        ),
        (
            "# head\nx: &a\n  p: 1 # note\nz: *a\nlist: [1, 2] # tail\nq: \"dq\"\ns: 'sq'\n",
            &[],
            ".x, .zz | length",
            "&a\np: 1 # note\n0\n",
        ),
    ];
    for (doc, extra, filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, doc, extra)?;
        assert_eq!(code, 0, "{doc:?} {extra:?} {filter}");
        assert_eq!(out, *want, "{doc:?} {extra:?} {filter}");
    }
    Ok(())
}

/// An error in any branch prints nothing, whichever branch it is (yq discards the prefix), and the
/// message is the branch's own.
#[test]
fn test_4856_an_error_in_any_branch_prints_nothing() -> Result<()> {
    let doc = "x:\n  p: 1 # note\ny: 2\n";
    for filter in [
        ".x, error(\"boom\")",
        "error(\"boom\"), .x",
        ".x, .zz, error(\"boom\")",
        ".x, (.y | error(\"boom\"))",
    ] {
        let (out, err, code) = run_yq_stdin_with_stderr(filter, doc, &[])?;
        assert_eq!(code, 1, "{filter}");
        assert_eq!(out, "", "{filter}");
        assert!(err.contains("boom"), "{filter}: {err}");
    }
    Ok(())
}
