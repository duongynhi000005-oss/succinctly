//! #5278: an `as` in a path position or the target of a write retries a source that bound
//! nothing the way the value routes do (#5261): a stage that does not read its context still runs
//! against the empty context an absent key leaves. `del((.zz | error("x")) as $x | .a)` raises,
//! and `del((.zz | "a") as $k | .[$k])` deletes `.a` because `$k` is `"a"`. Every expected output
//! is what yq v4.53.3 prints.

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

const DOC: &str = "a:\n  b: 1\nx: 5\nl: [1, 2]\n";

/// (filter, expected `-o=json -I=0` output).
const VALUE_ROWS: &[(&str, &str)] = &[
    // The retried source binds a constant that the body reads in path position.
    ("del((.zz | \"a\") as $k | .[$k])", "{\"x\":5,\"l\":[1,2]}"),
    (
        "((.zz | \"a\") as $k | .[$k]) = 9",
        "{\"a\":9,\"x\":5,\"l\":[1,2]}",
    ),
    (
        "((.zz | \"a\") as $k | .[$k]) |= 9",
        "{\"a\":9,\"x\":5,\"l\":[1,2]}",
    ),
    (
        "del((.zz | 1) as $i | .l[$i])",
        "{\"a\":{\"b\":1},\"x\":5,\"l\":[1]}",
    ),
    // A context-reading stage still binds nothing, so the body runs unbound.
    ("del((.zz | length) as $x | .a)", "{\"x\":5,\"l\":[1,2]}"),
    (
        "((.zz | \"s\" | select(false)) as $x | .a) = 5",
        "{\"a\":5,\"x\":5,\"l\":[1,2]}",
    ),
];

/// Filters that raise `x` (exit 1) and write nothing.
const RAISING_ROWS: &[&str] = &[
    "del((.zz | error(\"x\")) as $x | .a)",
    "((.zz | error(\"x\")) as $x | .a) = 5",
    "((.zz | error(\"x\")) as $x | .a) |= 5",
    "((.zz | error(\"x\")) as $x | .a) += 5",
    "((.zz | error(\"x\")) as $x | .a) = (.x)",
    "del((.zz | \"s\" | error(\"x\")) as $x | .a)",
    "del((select(false) | error(\"x\")) as $x | .a)",
    "del(.l[] as $i | (.zz | error(\"x\")) as $x | .a)",
    "del((.zz | error(\"x\")) as $x | .a) | .x",
];

#[test]
fn test_a_path_position_as_source_runs_its_context_free_stages_5278() -> Result<()> {
    for (filter, expected) in VALUE_ROWS {
        let (output, exit_code) = run_yq_stdin(filter, DOC, &["-o=json", "-I=0"])?;
        assert_eq!(exit_code, 0, "filter: {filter}");
        assert_eq!(output.trim_end(), *expected, "filter: {filter}");
    }
    Ok(())
}

#[test]
fn test_error_after_an_absent_key_raises_in_a_path_position_as_source_5278() -> Result<()> {
    for filter in RAISING_ROWS {
        let (output, stderr, exit_code) =
            run_yq_stdin_with_stderr(filter, DOC, &["-o=json", "-I=0"])?;
        assert_eq!(exit_code, 1, "filter: {filter}");
        assert_eq!(output, "", "filter: {filter}");
        assert!(
            stderr.contains("Error: x"),
            "filter: {filter}, stderr: {stderr}"
        );
    }
    Ok(())
}
