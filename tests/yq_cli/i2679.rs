//! #2679: a metadata write behind a static navigation (`.a | .b line_comment = "hi"`) is applied
//! on the navigated node, which keeps its own presentation, as real yq does. Every expected output
//! is what yq v4.53.3 prints.

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
            "x: &k\n  m: 1\n",
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
        // A comment after a sequence item's `-` belongs to the item, which the write keeps.
        (
            ".[0] | .b line_comment = \"x\"",
            "- # c\n  b: 1\n- z\n",
            "# c\nb: 1 # x\n",
        ),
        (
            ".[0] | .b style = \"double\"",
            "- # c\n  b: 1 # old\n- z\n",
            "# c\nb: \"1\" # old\n",
        ),
        (
            ".[0] | .[0] line_comment = \"x\"",
            "- # c\n  - 1\n  - 2\n",
            "# c\n- 1 # x\n- 2\n",
        ),
        // A scalar keeps its source spelling.
        (".a | . style = \"flow\"", "a: ~\n", "~\n"),
        (".a | . line_comment = \"x\"", "a: ~ # n\n", "~\n"),
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

/// A write whose text is read from the node it is on (`key`, `line`, `path`, a comment) is
/// answered from the node's place in the document, which the navigated route has not got, so it
/// is refused rather than printed with the wrong text (yq prints `b: 1 # a` for the first).
#[test]
fn test_write_reading_the_nodes_own_context_behind_navigation_is_refused_2679() -> Result<()> {
    for filter in [
        ".a | .b line_comment = (key)",
        ".a | .b line_comment = (path | join(\".\"))",
        ".a | .b line_comment = (line | tostring)",
        ".a | .b line_comment = (parent | tag)",
        ".a | .b line_comment = (.b | line_comment)",
    ] {
        let (out, err, code) = run_yq_stdin_with_stderr(filter, "a:\n  b: 1\nz: 2\n", &[])?;
        assert_eq!((out.as_str(), code), ("", 1), "{filter}: {err}");
        assert!(err.contains("#2679"), "{filter}: {err}");
    }
    Ok(())
}

/// `tag` is the one metadata slot that changes JSON output, so it applies behind a navigation
/// there too.
#[test]
fn test_tag_write_behind_navigation_applies_to_json_output_2679() -> Result<()> {
    let (out, code) = run_yq_stdin(
        ".a | .b tag = \"!!str\"",
        "a:\n  b: 1\n",
        &["-o", "json", "-I0"],
    )?;
    assert_eq!((out.trim(), code), (r#"{"b":"1"}"#, 0));
    let (out, code) = run_yq_stdin(".a | .b tag = \"!!str\"", "a:\n  b: 1\n", &[])?;
    assert_eq!((out.as_str(), code), ("b: \"1\"\n", 0));
    Ok(())
}

/// JSON output has no comments to place, so the write changes nothing there; and a document that
/// holds an alias is refused, since yq prints an alias whose anchor lies outside the navigated
/// node as `*k` (`limitations.md`).
#[test]
fn test_write_behind_navigation_json_and_alias_rows_2679() -> Result<()> {
    let (out, code) = run_yq_stdin(
        ".a | .b line_comment = \"hi\"",
        "a:\n  b: 1\n",
        &["-o", "json", "-I0"],
    )?;
    assert_eq!((out.trim(), code), (r#"{"b":1}"#, 0));

    for filter in [
        ".y | .m line_comment = \"kk\"",
        ".x | .m line_comment = \"kk\"",
    ] {
        let (out, err, code) = run_yq_stdin_with_stderr(filter, "x: &k\n  m: 1\ny: *k\n", &[])?;
        assert_eq!((out.as_str(), code), ("", 1), "{filter}: {err}");
        assert!(err.contains("#2679"), "{filter}: {err}");
    }
    Ok(())
}
