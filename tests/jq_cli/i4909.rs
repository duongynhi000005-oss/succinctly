//! #4909: the yq-only `sub` replacement rules (subject as `.`, `$1` expansion) do not reach jq.

use anyhow::Result;

use crate::run_jq_stdin;

/// Captured from jq 1.7.1: a replacement reads the captures object (`.n`), and `$1` is plain text.
#[test]
fn test_4909_jq_sub_replacement_still_reads_the_captures_object() -> Result<()> {
    for (filter, want) in [
        ("sub(\"(?<n>a)\"; \"[\" + .n + \"]\")", "\"[a]1b\"\n"),
        ("sub(\"(?<n>a)\"; .n)", "\"a1b\"\n"),
        ("sub(\"a\"; \"$1\")", "\"$11b\"\n"),
        ("sub(\"(a)\"; \"[$1]\")", "\"[$1]1b\"\n"),
    ] {
        let (out, code) = run_jq_stdin(filter, "\"a1b\"", &["-c"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(out, want, "{filter}");
    }
    // jq still fans out over every replacement output (`"X1b"` and `"Y1b"`), and `gsub` keeps the
    // captures object.
    let (out, code) = run_jq_stdin("sub(\"a\"; \"X\", \"Y\")", "\"a1b\"", &["-c"])?;
    assert_eq!(code, 0);
    assert_eq!(out, "\"X1b\"\n\"Y1b\"\n");
    let (out, code) = run_jq_stdin("gsub(\"(?<n>a)\"; .n + \"!\")", "\"a1b\"", &["-c"])?;
    assert_eq!(code, 0);
    assert_eq!(out, "\"a!1b\"\n");
    Ok(())
}
