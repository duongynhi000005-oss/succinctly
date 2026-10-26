//! #5309: jq mode's regex builtins now read their input before their pattern and flags, but yq's
//! own order and wording were not part of that change and are left as they were. Real yq refuses
//! a non-string input with `cannot match with !!null, can only match strings. Hint: ...`, which
//! this does not reproduce; these rows pin only that the call is still refused (exit 1), so the
//! yq-mode path that jq mode no longer reaches stays exercised without enshrining the wording.

use anyhow::Result;

use crate::run_yq_stdin_with_stderr;

#[test]
fn test_regex_builtins_still_refuse_a_non_string_input_in_yq_mode_5309() -> Result<()> {
    for filter in [
        r#"match("a")"#,
        r#"capture("(?<x>a)")"#,
        r#"test("a")"#,
        r#"sub("a"; "b")"#,
    ] {
        for input in ["~\n", "1\n", "[1]\n"] {
            let (stdout, stderr, code) = run_yq_stdin_with_stderr(filter, input, &[])?;
            assert_eq!(
                (stdout.as_str(), code),
                ("", 1),
                "{filter} on {input:?}: {stderr}"
            );
        }
    }
    Ok(())
}
