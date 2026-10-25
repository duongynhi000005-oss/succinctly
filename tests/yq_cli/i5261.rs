//! #5261: a stage that does not read its context still runs against the empty context an absent
//! key leaves behind, in an `as` source as it does in an assignment's right side (#2540, #2588).
//! `(.zz | error("x")) as $x | 1` raises in yq, `(.zz | "s") as $x | $x` binds `"s"`, and
//! `(.zz | length) as $x | [$x]` binds nothing. `error(MSG)` is one such stage, so it also raises
//! from an `=`/`+=` right side and from an `and`/`or`/arithmetic operand. Every expected output is
//! what yq v4.53.3 prints.

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

const DOC: &str = "a: 1\nx:\n  y: 2\n";

/// (filter, expected `-o=json -I=0` output).
const VALUE_ROWS: &[(&str, &str)] = &[
    // Context-reading stages after the absent key keep binding nothing.
    ("(.zz | length) as $x | [$x]", "[]"),
    ("(.zz | tostring) as $x | [$x]", "[]"),
    ("(.zz | .y) as $x | [$x]", "[]"),
    ("(.zz | select(true)) as $x | [$x]", "[]"),
    // A context-free stage restarts the stream and the rest runs on what it emitted.
    ("(.zz | \"s\") as $x | [$x]", "[\"s\"]"),
    ("(.zz | [.]) as $x | [$x]", "[[]]"),
    ("(.zz | {\"k\": 1}) as $x | $x", "{\"k\":1}"),
    ("(.zz | 1 + 1) as $x | $x", "2"),
    ("(.zz | \"s\" | length) as $x | $x", "1"),
    ("(.x.zz | \"s\") as $x | [$x]", "[\"s\"]"),
    // The emptiness need not come from an absent key.
    ("(select(false) | \"s\") as $x | [$x]", "[\"s\"]"),
    // A source that bound something is left alone.
    ("(.a, .zz) as $x | [$x]", "[1]"),
    ("(.zz // 3) as $x | $x", "3"),
    // A constructed input takes the owned bind route.
    ("{\"a\": 1} | (.zz | \"s\") as $x | [$x]", "[\"s\"]"),
    ("{\"a\": 1} | (.zz | length) as $x | [$x]", "[]"),
    // A rewritten source that would read the position of a node it constructed is not run.
    (".x | (.zz | [.] | key) as $v | [$v]", "[]"),
    // A stage that cannot be rewritten stays empty.
    ("(.zz | length | tostring) as $x | [$x]", "[]"),
];

/// Filters that raise `x` (exit 1).
const RAISING_ROWS: &[&str] = &[
    "(.zz | error(\"x\")) as $x | 1",
    "(.zz | error(\"x\")) as $x | $x",
    "(.zz | error(\"x\")) as $x | [$x]",
    "(.x.zz | error(\"x\")) as $x | 1",
    "(.zz | length | error(\"x\")) as $x | 1",
    "(.zz | select(true) | error(\"x\")) as $x | 1",
    "(.zz | \"s\" | error(\"x\")) as $x | 1",
    "(select(false) | error(\"x\")) as $x | 1",
    ".x | (.zz | error(\"x\")) as $v | 1",
    "{\"a\": 1} | (.zz | error(\"x\")) as $x | 1",
    // A body that reads a position takes the owned-identity bind route.
    ".x | (.zz | error(\"x\")) as $v | key",
    ".x | (.zz | error(\"x\")) as $v | $v | path",
    "[(.zz | error(\"x\")) as $x | 1]",
    // A source that did not come up empty raised before this change as well.
    "(.a | error(\"x\")) as $x | 1",
    // The assignment and operand sites share the rule.
    ".a = (.x.zz | error(\"x\"))",
    ".a = (.x.zz | error(\"x\") | 1)",
    ".a += (.x.zz | error(\"x\"))",
    "(.x.zz | error(\"x\")) and true",
    "(.x.zz | error(\"x\")) or true",
    "(.x.zz | error(\"x\")) + 1",
    "1 + (.x.zz | error(\"x\"))",
];

#[test]
fn test_as_source_runs_its_context_free_stages_on_the_empty_context_5261() -> Result<()> {
    for (filter, expected) in VALUE_ROWS {
        let (output, exit_code) = run_yq_stdin(filter, DOC, &["-o=json", "-I=0"])?;
        assert_eq!(exit_code, 0, "filter: {filter}");
        assert_eq!(output.trim_end(), *expected, "filter: {filter}");
    }
    Ok(())
}

#[test]
fn test_error_after_an_absent_key_raises_in_an_as_source_5261() -> Result<()> {
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

#[test]
fn test_the_raised_message_is_evaluated_against_the_empty_context_5261() -> Result<()> {
    let (_, stderr, exit_code) = run_yq_stdin_with_stderr(
        "(.zz | error(\"x\" + \"y\")) as $x | 1",
        DOC,
        &["-o=json", "-I=0"],
    )?;
    assert_eq!(exit_code, 1);
    assert!(stderr.contains("Error: xy"), "stderr: {stderr}");
    Ok(())
}

/// The body is not part of the source: a raise there is the body's, and it keeps its scope.
#[test]
fn test_a_body_that_raises_is_not_rewritten_5261() -> Result<()> {
    let (output, stderr, exit_code) =
        run_yq_stdin_with_stderr(".zz as $x | error(\"body\")", DOC, &["-o=json", "-I=0"])?;
    assert_eq!((output.as_str(), exit_code), ("", 1));
    assert!(stderr.contains("body"), "stderr: {stderr}");
    Ok(())
}
