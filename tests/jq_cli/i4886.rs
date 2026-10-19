//! #4886: the yq-only tag-and-text rule for `contains` leaves jq's by-value comparison alone.

use anyhow::Result;

use crate::run_jq_stdin;

/// jq 1.7.1 compares numbers by value, so `1.50` is contained in `[1.5]` (captured from jq 1.7.1).
#[test]
fn test_4886_jq_contains_still_compares_numbers_by_value() -> Result<()> {
    for (filter, input) in [
        ("contains([1.50])", "[1.5]"),
        ("contains({\"a\":1.5})", "{\"a\":1.50}"),
        ("contains(1.5)", "1.50"),
    ] {
        let (out, code) = run_jq_stdin(filter, input, &["-c"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(out, "true\n", "{filter}");
    }
    Ok(())
}
