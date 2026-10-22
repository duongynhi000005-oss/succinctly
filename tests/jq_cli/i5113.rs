//! #5113: a `foreach` UPDATE `if` whose literal `then` ran beside a fold in the `else`, on the last step of an INIT fork
//! that is not the last.

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

// jq stores the literal the `then` arm emits as the state, and the next INIT fork replaces it with INIT's own output
// (the `foreach` re-runs INIT), so on the last step of any INIT fork the literal feeds no later UPDATE: the `if` is
// resolved as that arm, as #5104 does on the last step of the last fork. jq's `foreach` hands the source the input
// only once (`DUPN`), so a later fork's source runs on `null` and its path error is caught by the `?` around it.

/// #5113: the issue's two-value INIT row, and rows of `scripts/jq-foreach-update-if-arm-sweep.py` (with `null`, `true`
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
            r#"{"a":false,"b":null}"#,
            r"(. as $x | foreach . as $i ((., .a); try (if . then false else foreach (try ($x as {a:$a} ?// [$a] | .)) as $y (.; .; .) end) catch .; $i)) |= 9",
            "9\n",
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
            r#"{"a":[true]}"#,
            r"[path(. as $x | foreach .a as $i ((., .a); try (if .a then true else foreach (try ($x as {a:$a} ?// [$a] | .)) as $y (.; .; .) end) catch .; $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach .a as $i ((., .a); try (if . then null else foreach (try ($x as {a:$a} | .)) as $y (.; .; .) end) catch 1; .a))",
            "[\"a\",\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"b":true}"#,
            r"path(. as $x | foreach . as $i ((., .a); try (if .b then false else foreach (try ($x as {a:$a} ?// [$a] | .)) as $y (.; .; .) end) catch .; $i))",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":false,"b":null}"#,
            r"path(. as $x | foreach .a as $i ((., .a); try (if . then false else foreach (try ($x as {a:$a} ?// [$a] | .)) as $y (.; .; .) end) catch .; $i))",
            "[\"a\"]\n",
            "Invalid path expression near attempt to access element \"a\" of null",
            5,
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
            r#"{"b":true}"#,
            r"[path(. as $x | foreach .a as $i ((., .a); (if . then null else foreach (try ($x as {a:$a} ?// [$a] | .)) as $y (.; .; .) end, 7); .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
    ])
}
