//! #4899: yq's array-in-array `contains` rule does not reach `succinctly jq`.

use anyhow::Result;

use crate::run_jq_stdin;

/// jq 1.7.1 compares an array element with a right-hand side of another kind as unequal, so none
/// of yq's nested-array matches apply (captured from jq 1.7.1).
#[test]
fn test_4899_jq_contains_does_not_look_inside_a_nested_array() -> Result<()> {
    for (filter, input) in [
        ("contains([1])", "[[1]]"),
        ("contains({\"a\":1})", "{\"a\":[1]}"),
        ("contains([\"a\"])", "[[[\"ab\"]]]"),
    ] {
        let (out, code) = run_jq_stdin(filter, input, &["-c"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(out, "false\n", "{filter}");
    }
    Ok(())
}
