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
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; if .b then 1 elif .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end; .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
    ])
}

/// #4800: the arm is resolved alone behind a wrapper too (`resolved_if_arm`): a comma whose other items emit nothing and
/// a `label` scope without a `break` ahead of the item do not change what the `if` emits, and a handler-less `try`/`?`
/// around a state that took a literal `else` arm drops a handler that could catch nothing. jq answers
/// `[["a","a"]]` for the `.a` EXTRACT rows, and those fail on the base (`[]`; an `elif` chain and a `(.a,.)` source
/// included). The `$i` EXTRACT rows, the `| .` row and the `{"a":true}` row already matched on the base: they pin that
/// the arm-only resolve does not move them. Resolving the wrapped `if` whole again fails the test.
#[test]
fn test_foreach_update_wrapped_if_with_a_literal_else_4800() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; try (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end); .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end)?; .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end, empty); .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; label $l | if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end; .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end) | .; .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; label $l | (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end, break $l); .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; try (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) elif .b then 7 else null end); .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; try (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end); $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end)?; $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end, empty); $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; label $l | if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end; $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.a,.) as $i (.; try (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end); .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":true}"#,
            r"[path(. as $x | foreach .a as $i (.; try (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end); .a))?]",
            "[]\n",
            "",
            0,
        ),
    ])
}

/// #4800: the wrappers `resolved_if_arm` does not read keep the whole-`if` resolve: a `try` with a handler and a comma
/// with an item that emits, with a `$i` EXTRACT, answer `[]` where jq answers `[["a"]]` (`[["a"],["a"]]` for the
/// comma). This pins the answer as it stands (a fix flips it), as the sibling tests pin their refusals.
#[test]
fn test_foreach_update_wrapped_if_with_a_literal_else_stays_differing_4800() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; try (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end) catch 1; $i))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach .a as $i (.; (if .a then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else null end, 7); $i))?]",
            "[]\n",
            "",
            0,
        ),
    ])
}

/// #4800: a condition the walk cannot decide on the state (`.a?`, a comma, `range(2)`, a bind) names no arm
/// ([`taken_if_branch`] answers the `if` itself), so the wrapped `if` is resolved whole and the pipe's `if` stage is
/// read by `alternate_update_shape`, which is the one route that still reaches the `return None` of its `Pipe` arm
/// (the literal-arm rows above are resolved as the arm and never get there). Where jq answers `[[]]` or `[[],[]]`
/// these answer `[]`; the answers are pinned as they stand (a fix flips them), and the rows keep the arm covered.
#[test]
fn test_foreach_update_wrapped_if_with_an_undecidable_condition_stays_differing_4800() -> Result<()>
{
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":true,"b":2}"#,
            r"[path(foreach . as $i (.; if .a? then 7 else .b end | .; $i))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":true,"b":2}"#,
            r"[path(foreach . as $i (.; if .a as $x | $x then 7 else .b end | .; $i))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":true,"b":2}"#,
            r"[path(foreach . as $i (.; (if (.a,.b) then 7 else .b end) | select(true); $i))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":false,"b":2}"#,
            r"[path(foreach . as $i (.; if range(2) then 7 else .b end | .; $i))?]",
            "[]\n",
            "",
            0,
        ),
    ])
}
