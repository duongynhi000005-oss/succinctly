//! #5233: a jq program with a dynamic path target runs on the path-mode interpreter, so the shipped
//! path resolver is reached by yq alone (and by a jq static target, which has no handler or
//! `any`/`all` probe to run). These pin two of its arms that only yq now drives, with the
//! `--jq-extensions` the jq-only builtins need: a `break` caught by a `try` hands its handler a
//! `null` payload, and an `any`/`all` condition that raises ends the path drive with that error.
//! Neither has a yq equivalent (yq's lexer rejects `label`, `any(g; c)` and `all(g; c)`), so the rows
//! pin succinctly's own behaviour.

use anyhow::Result;

use crate::run_yq_stdin_with_stderr;

const DOC: &str = "a: [1, 2]\nb: 3\n";

/// `try (.a, break $out) catch .b` catches the break and resolves `.b` against its untracked
/// `null` payload, which is no path.
#[test]
fn a_break_caught_in_path_position_hands_its_handler_a_null_payload_5233() -> Result<()> {
    let args = ["--jq-extensions", "-o=json", "-I=0"];
    let (stdout, stderr, code) = run_yq_stdin_with_stderr(
        "label $out | [path(try (.a, break $out) catch .b)]",
        DOC,
        &args,
    )?;
    assert_eq!((stdout.as_str(), code), ("", 1), "{stderr}");
    assert!(
        stderr.contains(r#"Invalid path expression near attempt to access element "b" of null"#),
        "{stderr}"
    );
    // A handler that emits nothing leaves the paths resolved before the break.
    let (stdout, stderr, code) = run_yq_stdin_with_stderr(
        "label $out | [path(try (.a, break $out) catch empty)]",
        DOC,
        &args,
    )?;
    assert_eq!((stdout.as_str(), code), ("[[\"a\"]]\n", 0), "{stderr}");
    Ok(())
}

/// A condition that raises on an element ends the `any`/`all` path drive with its error.
#[test]
fn an_any_all_condition_that_raises_ends_the_path_drive_5233() -> Result<()> {
    for filter in [
        r#"[path(.a | any(.[]; error("x")))]"#,
        r#"[path(.a | all(.[]; error("x")))]"#,
        r#"del(.a | any(.[]; error("x")))"#,
    ] {
        let (stdout, stderr, code) =
            run_yq_stdin_with_stderr(filter, DOC, &["--jq-extensions", "-o=json", "-I=0"])?;
        assert_eq!((stdout.as_str(), code), ("", 1), "`{filter}`: {stderr}");
        assert!(stderr.contains("Error: x"), "`{filter}`: {stderr}");
    }
    Ok(())
}
