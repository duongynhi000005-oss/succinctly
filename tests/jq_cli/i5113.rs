//! #5113: a `foreach` UPDATE `if` whose literal `then` ran beside a fold in the `else`, on the last step of an INIT fork
//! that is not the last.

use anyhow::Result;

use crate::{assert_path_rows_both_routes_3749, run_jq_full};

// jq stores the literal the `then` arm emits as the state, and the next INIT fork replaces it with INIT's own output
// (the `foreach` re-runs INIT), so on the last step of any INIT fork the literal feeds no later UPDATE: the `if` is
// resolved as that arm, as #5104 does on the last step of the last fork. jq's `foreach` hands the source the input
// only once (`DUPN`), so a later fork's source runs on `null` and its path error is caught by the `?` around it.

/// #5113: the issue's two-value INIT row (and the same over an INIT whose count is unbounded, `(.,.) | .` and
/// `range(2) as $n | .`: only the source's bound says the step is the fork's last), and rows of `scripts/jq-foreach-update-if-arm-sweep.py` (with `null`, `true`
/// and `false` literal arms added) that `main` answered differently and now match. Captured from `/usr/bin/jq` 1.7.1,
/// both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5113_literal_then_on_the_last_step_of_an_earlier_init_fork() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"a":null}}"#,
            r"[path(. as $x | foreach .a as $i ((.,.); try (if .a then 1 else (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) end) catch 1; $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"path(. as $x | foreach . as $i ((., .a); (7, if .a then 1 elif . then 2 else foreach (try ($x as {a:$a} | .)) as $y (.; .; .) end); $i))",
            "[]\n[]\n",
            "Invalid path expression with result null",
            5,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach .a as $i ((., .a); try (if . then null else foreach (try ($x as {a:$a} | .)) as $y (.; .; .) end) catch 1; .a))",
            "[\"a\",\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"path(. as $x | foreach . as $i ((., .a); (7, if .a then true else foreach (try ($x as {a:$a} | .)) as $y (.; .; .) end); $i))",
            "[]\n[]\n",
            "Invalid path expression with result null",
            5,
        ),
        (
            r#"{"b":true}"#,
            r"(. as $x | foreach .a as $i ((., .a); try (if .b then null else foreach (try ($x as {a:$a} | .)) as $y (.; .; .) end) catch .; $i)) |= 9",
            "{\"b\":true,\"a\":9}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[path(. as $x | foreach .a as $i ((., .a); try (if .a then 1 elif . then foreach (try ($x as {a:$a} | .)) as $y (.; .; .) else 2 end) catch 1; $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[path(. as $x | foreach .a as $i ((.,.) | .; try (if .a then 1 else (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) end) catch 1; $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[path(. as $x | foreach .a as $i (range(2) as $n | .; try (if .a then 1 else (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) end) catch 1; $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
    ])
}

/// #5113 review: where the literal `then` must still not be read, so the step keeps its whole-`if` refusal: a `?//`
/// pattern (a retry runs UPDATE again on the stored literal; read there, the first row wrote `{"b":true,"a":{"a":9}}`
/// where jq raises), a step that is not its fork's last (the next step runs the fold on the stored literal; read
/// there, a `?` truncated jq's answer), and an UPDATE whose fold source is a `?//` chain (read on the first of three
/// forks, the later forks answered nothing where jq emits `["a"]`). Each row must answer as jq does or stay refused, on both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5113_literal_then_stays_unread_under_a_retry_or_before_the_forks_last_step() -> Result<()> {
    // (document, filter, jq's stdout)
    let rows: &[(&str, &str, &str)] = &[
        (
            r#"{"b":true}"#,
            r"(. as $x | foreach .a as {a:$k} ?// $k ((.,.a); (if . then true else (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) end, 7); $k)) |= 9",
            "",
        ),
        (
            r#"{"b":true}"#,
            r"[path(. as $x | foreach (.a,.) as $i ((.,.); try (if .b then null else (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) end) catch 1; .a))?]",
            "[[\"a\",\"a\"],[\"a\",\"a\"]]\n",
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach . as $i ((., null, .a); try (if . then 1 else foreach (try ($x as {a:$a} ?// [$a] | .)) as $y (.; .; .) end) catch 1; $i))",
            "[]\n[\"a\"]\n",
        ),
    ];
    for &(input, filter, jq_stdout) in rows {
        for (stdout, stderr, code) in [
            run_jq_full(&["-c", filter], Some(input))?,
            run_jq_full(&["-nc", &format!("{input} | {filter}")], None)?,
        ] {
            let refused = code == 5 && stdout.is_empty();
            let answered =
                !jq_stdout.is_empty() && stdout == jq_stdout && code == 0 && stderr.is_empty();
            assert!(
                refused || answered,
                "`{filter}` on {input}: neither jq's {jq_stdout:?} nor a refusal: {stdout:?} {stderr:?} exit {code}"
            );
        }
    }
    Ok(())
}
