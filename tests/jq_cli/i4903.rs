//! #4903: yq reads a text argument with no output as `""`; jq's fan-out still gives no output.

use anyhow::Result;

use crate::run_jq_stdin;

/// Captured from jq 1.7.1: `split(empty)`, `test(empty)`, `match(empty)` and `sub(empty; "X")` all
/// produce nothing, and a replacement with no output drops the match's text (jq's #840 rule).
#[test]
fn test_4903_jq_text_argument_with_no_output_still_yields_nothing() -> Result<()> {
    for filter in [
        "split(empty)",
        "test(empty)",
        "[match(empty)]",
        "sub(empty; \"X\")",
    ] {
        let (out, code) = run_jq_stdin(filter, "\"a1b\"", &["-c"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(
            out,
            if filter.starts_with('[') { "[]\n" } else { "" },
            "{filter}"
        );
    }
    let (out, code) = run_jq_stdin("sub(\"a\"; empty)", "\"a1b\"", &["-c"])?;
    assert_eq!(code, 0);
    assert_eq!(out, "\"a1b\"\n");
    Ok(())
}
