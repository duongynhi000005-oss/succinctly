//! #4800: a `foreach` UPDATE that is an `if` whose `then` branch holds the caught foreign-`$x` `foreach` and whose
//! `else` branch is a literal (`else null`). `FE` is `foreach (try ($x as {a:$a} | .)) as $y (.; .; .)`.

use crate::assert_path_rows_both_routes_3749;
use anyhow::Result;

/// #4800: a step whose condition is falsy runs the `else` arm alone, and the arm is resolved as a UPDATE of its own
/// (`taken_if_branch`). The `if` was still resolved whole, so `cannot_move_register` read the `foreach` in the arm that
/// did not run and withheld the tracking of the `null` the literal arm stores: with a `.a` EXTRACT that `null` is
/// jq's register by kind (`jv_identical`), and `.a` raised `Invalid path expression near attempt to access element
/// "a" of null` where jq answers `["a","a"]`. Rows captured from jq 1.7.1 (stdout, first stderr line, exit code),
/// both routes; the first eight fail on `main`, and resolving the `if` whole again fails the test.
#[test]
fn test_foreach_update_if_with_the_caught_foreach_in_then_and_a_literal_else_4800() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end; .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.a,.) as $i (.; if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end; .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach .a as $i (.; if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end; .a))",
            "[\"a\",\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"try path(. as $x | foreach .a as $i (.; if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end; .a)) catch "c""#,
            "[\"a\",\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"del(. as $x | foreach .a as $i (.; if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end; .a))",
            "{\"a\":null}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach .a as $i (.; if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end; .a)) |= 9",
            "{\"a\":{\"a\":9}}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; if .a then ((foreach (try ($x as {a:$a} | .)) as $y (.; .; .)), empty) else null end; .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) elif .b then 7 else null end; .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":true}"#,
            r"[path(. as $x | foreach .a as $i (.; if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end; .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end; $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
    ])
}
