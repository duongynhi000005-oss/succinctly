//! #5248: yq's lexer rejects a bracket suffix straight after `..` (`..[0]?` is `Error: bad expression`,
//! confirmed against yq v4.53.3), so only jq mode reads it as a postfix on the `..` term. The mode
//! decides, not the input format (ADR-0018).

use anyhow::Result;

use crate::run_yq_stdin;

#[test]
fn test_yq_rejects_a_bracket_suffix_after_recursive_descent_5248() -> Result<()> {
    for filter in ["..[0]?", "[..[]?]", r#"..["a"]?"#, "..[1:]?"] {
        let (_, exit_code) = run_yq_stdin(filter, "[[1]]\n", &[])?;
        assert_ne!(exit_code, 0, "filter: {filter}");
    }
    Ok(())
}
