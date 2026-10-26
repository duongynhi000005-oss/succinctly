//! #2679: a metadata write behind a static navigation (`.a | .b line_comment = "hi"`) is applied
//! on the navigated node, which keeps its own presentation, as real yq does. Every expected output
//! is what yq v4.53.3 prints, bar the two rows `limitations.md` records.

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

#[test]
fn test_write_behind_static_navigation_keeps_the_subtree_presentation_2679() -> Result<()> {
    for (filter, input, expected) in [
        // The navigated node's own comments, flow collections and quoting survive the write.
        (
            ".a | .b line_comment = \"hi\"",
            "# head\na: # on a\n  # above b\n  b: 1 # old\n  c: [1, 2] # flow\n  d: {x: 1, y: 2}\nz: 3\n",
            "# above b\nb: 1 # hi\nc: [1, 2] # flow\nd: {x: 1, y: 2}\n",
        ),
        (
            ".a | .c style = \"single\"",
            "a:\n  b: \"q\"\n  c: x\n",
            "b: \"q\"\nc: 'x'\n",
        ),
        // A node that declares an anchor keeps it; a write may declare one.
        (
            ".x | .m line_comment = \"kk\"",
            "x: &k\n  m: 1\ny: *k\n",
            "&k\nm: 1 # kk\n",
        ),
        (
            ".a | .b anchor = \"nm\"",
            "a:\n  b: 1\n  c: 2\n",
            "b: &nm 1\nc: 2\n",
        ),
        // A sequence item, a navigation of several stages, and a clearing write.
        (
            ".[1] | .a line_comment = \"hi\"",
            "- a: 0\n- a: 1 # old\n  b: [1]\n",
            "a: 1 # hi\nb: [1]\n",
        ),
        (
            ".a | .b | .c style = \"double\"",
            "a:\n  b:\n    c: 1\n",
            "c: \"1\"\n",
        ),
        (
            ".a | .b line_comment = \"\"",
            "a:\n  b: 1 # old\n  c: 2\n",
            "b: 1\nc: 2\n",
        ),
        // A write after another write in the navigated document.
        (
            ".a | .z = 9 | .b line_comment = \"n\"",
            "a:\n  b: 1\n",
            "b: 1 # n\nz: 9\n",
        ),
    ] {
        let (out, err, code) = run_yq_stdin_with_stderr(filter, input, &[])?;
        assert_eq!((out.as_str(), code), (expected, 0), "{filter}: {err}");
    }
    Ok(())
}

/// A navigation to a node that is not there (yq writes onto the `null` it leaves) and one that
/// answers several nodes are still refused rather than dropped.
#[test]
fn test_write_behind_absent_or_fanning_navigation_is_refused_2679() -> Result<()> {
    for (filter, input) in [
        (".zz | .b line_comment = \"x\"", "a: 1\n"),
        (".a.zz | .b line_comment = \"x\"", "a:\n  b: 1\n"),
        (".[] | .b line_comment = \"x\"", "- b: 1\n- b: 2\n"),
        (".a[] | .b line_comment = \"x\"", "a:\n  - b: 1\n"),
    ] {
        let (out, err, code) = run_yq_stdin_with_stderr(filter, input, &[])?;
        assert_eq!((out.as_str(), code), ("", 1), "{filter}: {err}");
        assert!(err.contains("#2679"), "{filter}: {err}");
    }
    Ok(())
}

/// JSON output has no comments to place, so the write changes nothing there; and a write that
/// lands on an alias prints the node it resolves to, where yq prints `*k` (`limitations.md`).
#[test]
fn test_write_behind_navigation_json_and_alias_rows_2679() -> Result<()> {
    let (out, code) = run_yq_stdin(
        ".a | .b line_comment = \"hi\"",
        "a:\n  b: 1\n",
        &["-o", "json", "-I0"],
    )?;
    assert_eq!((out.trim(), code), (r#"{"b":1}"#, 0));

    let (out, code) = run_yq_stdin(
        ".y | .m line_comment = \"kk\"",
        "x: &k\n  m: 1\ny: *k\n",
        &[],
    )?;
    assert_eq!((out.as_str(), code), ("m: 1 # kk\n", 0));
    Ok(())
}
