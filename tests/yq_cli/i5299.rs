//! #5299 makes a `.` and a digit a number literal in `succinctly jq` only. yq's lexer reads it as a
//! field name (`.5` is the key `5`, `.a.1` an index), so `succinctly yq` keeps that. Captured from
//! yq v4.53.3.

use anyhow::Result;

use crate::run_yq_stdin;

#[test]
fn test_leading_dot_digit_stays_a_key_in_yq_5299() -> Result<()> {
    let input = "{\"5\": five, \"a\": [10, 20]}\n";
    for (filter, want) in [(".5", "five\n"), (".5 + 1", "five1\n"), (".a.1", "20\n")] {
        let (out, code) = run_yq_stdin(filter, input, &[])?;
        assert_eq!((out.as_str(), code), (want, 0), "{filter}");
    }
    Ok(())
}
