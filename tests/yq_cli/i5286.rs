//! #5286: a write behind a static navigation runs on the navigated node as a document of its own,
//! so the subtree keeps its comments, flow collections and quoting, as in yq. Every expected output
//! is what yq v4.53.3 prints.

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

const DOC: &str = "a:\n  # above b\n  b: 1 # b trail\n  c: [1, 2] # flow\n  d: {x: 1}\n  e: \"q\"\n  f: 'r'\nz: 3\n";

#[test]
fn test_write_behind_static_navigation_keeps_the_subtree_presentation_5286() -> Result<()> {
    for (filter, expected) in [
        (
            ".a | .b = 5",
            "# above b\nb: 5 # b trail\nc: [1, 2] # flow\nd: {x: 1}\ne: \"q\"\nf: 'r'\n",
        ),
        (
            ".a | (.b = 5)",
            "# above b\nb: 5 # b trail\nc: [1, 2] # flow\nd: {x: 1}\ne: \"q\"\nf: 'r'\n",
        ),
        (
            ".a | .b |= 5",
            "# above b\nb: 5 # b trail\nc: [1, 2] # flow\nd: {x: 1}\ne: \"q\"\nf: 'r'\n",
        ),
        (
            ".a | del(.b)",
            "c: [1, 2] # flow\nd: {x: 1}\ne: \"q\"\nf: 'r'\n",
        ),
        (
            ".a | .z = 9",
            "# above b\nb: 1 # b trail\nc: [1, 2] # flow\nd: {x: 1}\ne: \"q\"\nf: 'r'\nz: 9\n",
        ),
        (
            ".a | .c[0] = 7",
            "# above b\nb: 1 # b trail\nc: [7, 2] # flow\nd: {x: 1}\ne: \"q\"\nf: 'r'\n",
        ),
        (
            ".a | sort_keys(.)",
            "# above b\nb: 1 # b trail\nc: [1, 2] # flow\nd: {x: 1}\ne: \"q\"\nf: 'r'\n",
        ),
    ] {
        let (out, err, code) = run_yq_stdin_with_stderr(filter, DOC, &[])?;
        assert_eq!((out.as_str(), code), (expected, 0), "{filter}: {err}");
    }
    Ok(())
}

/// A sequence item keeps the comment after its `-`, which the sequence's walk holds, and a pipe of
/// navigations reaches the node.
#[test]
fn test_write_behind_navigation_to_a_sequence_item_5286() -> Result<()> {
    for (filter, input, expected) in [
        (
            ".[0] | .b = 2",
            "- # c\n  b: 1 # old\n  c: [1]\n- z\n",
            "# c\nb: 2 # old\nc: [1]\n",
        ),
        (
            ".x | .[1] | .k = 0",
            "x:\n  - 1\n  - # d\n    k: 'v'\n",
            "# d\nk: 0\n",
        ),
        (
            ".a.b | .c[1] = 9",
            "a:\n  b:\n    c: [1, 2] # t\n",
            "c: [1, 9] # t\n",
        ),
    ] {
        let (out, err, code) = run_yq_stdin_with_stderr(filter, input, &[])?;
        assert_eq!((out.as_str(), code), (expected, 0), "{filter}: {err}");
    }
    Ok(())
}

/// What the route does not take keeps today's answer: a plain read behind a navigation, and
/// `-o json` (no presentation to keep).
#[test]
fn test_reads_and_json_output_keep_their_route_5286() -> Result<()> {
    let (out, code) = run_yq_stdin(".a | .b", DOC, &[])?;
    assert_eq!((out.as_str(), code), ("1\n", 0));

    let (out, code) = run_yq_stdin(".a | .b = 5", DOC, &["-o", "json", "-I0"])?;
    assert_eq!(
        (out.as_str(), code),
        (
            "{\"b\":5,\"c\":[1,2],\"d\":{\"x\":1},\"e\":\"q\",\"f\":\"r\"}\n",
            0
        )
    );
    Ok(())
}

/// The route stays off for a document with an alias, and for a write that reads the node's own
/// context (`key`): they keep the old route's block-style, comment-free answer (the loss
/// `limitations.md` records), which a regression of either gate would change.
#[test]
fn test_the_route_gates_keep_the_old_answer_5286() -> Result<()> {
    let plain = "b: 5\nc:\n  - 1\n  - 2\nd:\n  x: 1\ne: q\nf: r\n";
    let (out, code) = run_yq_stdin(".a | .b = (key | length)", DOC, &[])?;
    assert_eq!(code, 0);
    assert!(!out.contains("# flow"), "{out}");

    let aliased = format!("{DOC}y: &k [1]\nw: *k\n");
    let (out, code) = run_yq_stdin(".a | .b = 5", &aliased, &[])?;
    assert_eq!((out.as_str(), code), (plain, 0));
    Ok(())
}
