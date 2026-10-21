//! #5062: a `: ` inside a trailing comment no longer makes a plain scalar a mapping key, which rejected
//! valid YAML as `key without value` (`- ubuntu-22.04-arm # TODO: update ...` in a GitHub workflow). Each
//! row is a live yq v4.53.3 capture; `- a#b: c` (no space before `#`) is still a key.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, extra args, filter, expected output).
const ROWS: &[(&str, &[&str], &str, &str)] = &[
    ("- a # b: c\n", &[], ".", "- a # b: c\n"),
    ("- a # b: c\n", &["-o", "json", "-I0"], ".", "[\"a\"]\n"),
    ("a # b: c\n", &["-o", "json", "-I0"], ".", "\"a\"\n"),
    ("k:\n  a # b: c\n", &["-o", "json", "-I0"], ".", "{\"k\":\"a\"}\n"),
    ("- - a # b: c\n", &["-o", "json", "-I0"], ".", "[[\"a\"]]\n"),
    ("- a\t# b: c\n", &["-o", "json", "-I0"], ".", "[\"a\"]\n"),
    ("- &x a # b: c\n", &["-o", "json", "-I0"], ".", "[\"a\"]\n"),
    ("- !t a # b: c\n", &["-o", "json", "-I0"], ".", "[\"a\"]\n"),
    ("? a # b: c\n", &["-o", "json", "-I0"], ".", "{\"a\":null}\n"),
    ("os:\n  - ubuntu-22.04-arm # TODO: update to 24.04 when https://github.com/x/y solved\n  - macos-latest\n", &[], ".", "os:\n  - ubuntu-22.04-arm # TODO: update to 24.04 when https://github.com/x/y solved\n  - macos-latest\n"),
    ("os:\n  - a # TODO: x\n", &[], ".z = 1", "os:\n  - a # TODO: x\nz: 1\n"),
    ("- a#b: c\n", &["-o", "json", "-I0"], ".", "[{\"a#b\":\"c\"}]\n"),
    ("k: v # b: c\n", &[], ".", "k: v # b: c\n"),
];

#[test]
fn test_comment_colon_after_a_plain_scalar_is_not_a_key_5062() -> Result<()> {
    for (doc, args, filter, expected) in ROWS {
        let (stdout, code) = run_yq_stdin(filter, doc, args)?;
        assert_eq!(
            (stdout.as_str(), code),
            (*expected, 0),
            "{doc:?} {args:?} {filter}"
        );
    }
    Ok(())
}
