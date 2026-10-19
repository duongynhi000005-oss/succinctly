//! #4903: the yq-only "an argument with no output is the empty string" rule leaves jq's `f(empty)`
//! alone -- jq runs the call once per argument output, so no output is no call.

use anyhow::Result;

use crate::run_jq_stdin;

/// Captured from jq 1.7.1: each of these prints nothing and exits 0.
#[test]
fn test_4903_jq_argument_with_no_output_is_still_no_call() -> Result<()> {
    for filter in [
        "split(empty)",
        "test(empty)",
        "[match(empty)]",
        "capture(empty)",
        "sub(empty; \"X\")",
        "test(\"a\"; empty)",
    ] {
        let (out, code) = run_jq_stdin(filter, "\"a1b\"", &["-c"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(
            out,
            if filter.starts_with("[match") {
                "[]\n"
            } else {
                ""
            },
            "{filter}"
        );
    }
    Ok(())
}
