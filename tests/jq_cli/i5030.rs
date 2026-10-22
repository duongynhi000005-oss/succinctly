//! #5030: a fold source on a known register (after a navigating INIT) whose foreign `$x` destructure sits inside a nested
//! fold under a `try`/`?`.

use crate::assert_path_rows_both_routes_3749;
use anyhow::Result;

/// #5030: on `{"a":{"b":1,"c":{"b":3}},"b":2}` after INIT `.a`, `try reduce ($x as {b:$q} | .) as $w (0; .)` as a
/// `foreach` source: the inner destructure raises, jq's `try` catches it and the source is empty, so `path` emits
/// nothing and `del`/`|=` leave the document. The marker sat under the `try` and the source was read by value, so
/// `del` deleted `.a`. A handler that emits (`catch 5`) and a `try` around a comma that emits first answer as before.
/// Rows captured from jq 1.7.1, both routes.
#[test]
fn test_fold_source_try_around_a_nested_fold_on_a_known_register_5030() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"del(. as $x | foreach (try reduce ($x as {b:$q} | .) as $w (0; .)) as $y (.a; .; .))",
            "{\"a\":{\"b\":1,\"c\":{\"b\":3}},\"b\":2}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"path(. as $x | foreach (try reduce ($x as {b:$q} | .) as $w (0; .)) as $y (.a; .; .))",
            "",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"[path(. as $x | foreach ((reduce ($x as {b:$q} | .) as $w (0; .))?) as $y (.a; .; .))]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"(. as $x | foreach (try reduce ($x as {b:$q} | .) as $w (0; .)) as $y (.a; .; .)) |= 9",
            "{\"a\":{\"b\":1,\"c\":{\"b\":3}},\"b\":2}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"path(. as $x | foreach (try reduce ($x as {b:$q} | .) as $w (0; .) catch 5) as $y (.a; .; .))",
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"path(. as $x | foreach (try (1, ($x as {b:$q} | .))) as $y (.a; .; .))",
            "[\"a\"]\n",
            "",
            0,
        ),
    ])
}

/// #5030: only the forms that skip a raise without emitting are read through (a handler-less `try`, `?`,
/// `catch empty`): a handler that can emit is kept, so a foreign destructure in the handler itself (`try error("x")
/// catch reduce ($x as {b:$q} | .) ..`) still raises as jq does, and a `catch empty` around the nested fold leaves the
/// source empty. Rows captured from jq 1.7.1, both routes.
#[test]
fn test_fold_source_handler_destructure_on_a_known_register_still_raises_5030() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r#"del(. as $x | foreach (try error("x") catch reduce ($x as {b:$q} | .) as $w (0; .)) as $y (.a; .; .))"#,
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"a\":{\"b\":1,\"c\":{\"b\":3}},\"...",
            5,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r#"(. as $x | foreach (try error("x") catch reduce ($x as {b:$q} | .) as $w (0; .)) as $y (.a; .; .)) |= 9"#,
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"a\":{\"b\":1,\"c\":{\"b\":3}},\"...",
            5,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r#"path(. as $x | reduce (try error("x") catch ($x as {b:$q} | .)) as $y (.a; .))"#,
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"a\":{\"b\":1,\"c\":{\"b\":3}},\"...",
            5,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"del(. as $x | foreach (try reduce ($x as {b:$q} | .) as $w (0; .) catch empty) as $y (.a; .; .))",
            "{\"a\":{\"b\":1,\"c\":{\"b\":3}},\"b\":2}\n",
            "",
            0,
        ),
    ])
}

/// #5030: a `$x` the fold reads as jq's register, piped into a destructure (`$x | . as P`, `$x | (. as P | ..)`,
/// `$x | . | . as P`), destructures the register as `$x as P` does. After INIT `.a` with `.a as $x`, jq's register is
/// `$x`'s node, the pattern step moves it, and the path ends off it (`with result`), where the source was read by
/// value: `del` deleted `.a` and `path` answered `["a"]`. The same holds on a `null` INIT behind a computed stage and under
/// `?`/`try`. A `reduce` source (which backtracks), a register at the root and a bare `$x | .` answer as before.
/// Rows captured from jq 1.7.1, both routes.
#[test]
fn test_fold_source_register_marker_piped_into_a_destructure_5030() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"del(.a as $x | foreach ($x | . as {b:$q} | .) as $y (.a; .; .))",
            "",
            "Invalid path expression with result {\"b\":1,\"c\":{\"b\":3}}",
            5,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"path(.a as $x | foreach ($x | . as {b:$q} | $q) as $y (.a; .; .))",
            "",
            "Invalid path expression with result {\"b\":1,\"c\":{\"b\":3}}",
            5,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"(.a as $x | foreach ($x | (. as {b:$q} | .)) as $y (.a; .; .)) |= 9",
            "",
            "Invalid path expression with result {\"b\":1,\"c\":{\"b\":3}}",
            5,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"del(.a as $x | .a | foreach ($x | . | . as {b:$q,a:$r} | .) as $y (.; .))",
            "",
            "Invalid path expression near attempt to access element \"a\" of {\"b\":1}",
            5,
        ),
        (
            r#"{"a":{"b":{"c":3}},"b":{"c":4}}"#,
            r"[path(. as $x | null | foreach ($x | . as {c:$q} | $q) as $y (null; $y))?]",
            "[[\"c\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"[path(. as $x | null | foreach (($x | . as {a:$q} | $q)?) as $y (null; $y))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | 1 | foreach ($x | . | . as {c:$q} | 1) as $y (null; .b?)) = 9",
            "{\"a\":null,\"c\":{\"b\":9}}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":null},"b":null}"#,
            r#"try path(.a as $x | foreach (try ($x | (. as {c:$q} | .))) as $y ((.a,.b); .)) catch "C""#,
            "\"C\"\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"path(.a as $x | reduce ($x | . as {b:$q} | .) as $y (.a; .))",
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"del(.a as $x | foreach ($x | . as {b:$q} | .) as $y (.; .; .))",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"b\":1,\"c\":{\"b\":3}}",
            5,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"path(.a as $x | foreach ($x | .) as $y (.a; .; .))",
            "[\"a\"]\n",
            "",
            0,
        ),
    ])
}

/// #5030 (review): the walks through a `reduce` that decide a fold body's loud refusal do not read a listed marker in
/// a comma, an `if` branch or a `limit` as the register on every output. Inside another fold's UPDATE, `($x, $x) | .
/// as [$q]` raises jq's own catchable error and `try` catches it; reading the marker as the register there refused
/// with an error no `try` catches. Rows captured from jq 1.7.1, both routes.
#[test]
fn test_fold_body_marker_in_a_comma_or_branch_is_not_the_register_5030() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":1}"#,
            r#"try path(. as $x | foreach 1 as $i (.; .; foreach (($x, $x) | . as [$q] | .) as $y (.; 1; .))) catch "C""#,
            "\"C\"\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(try (.a.b as $x | foreach 1 as $i (.; foreach (($x, $x) | . as {a:$q} | 1) as $y (null; .; empty); .)))]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":[1],"b":[2]}"#,
            r#"try del(. as $x | reduce .a as $i (.; foreach (limit(1; $x) | . as $w | $w as {b:$q} | .) as $y (null; .b?; empty))) catch "D""#,
            "\"D\"\n",
            "",
            0,
        ),
        (
            r#"{"a":1}"#,
            r#"try path(. as $x | foreach 1 as $i (.; .; foreach ((if true then $x else $x end) | . as [$q] | .) as $y (.; 1; .))) catch "C""#,
            "\"C\"\n",
            "",
            0,
        ),
    ])
}

/// #5030: a fold source outside every fold body, after a stage that navigated to a missing key, so that jq's register is
/// a `null` (`.a as $x | .nope | reduce ($x as {b:$b} | .) ..`). A `$x` holding a container is not that `null`, and its
/// destructure raises at the pattern step. The routing to the resolver read a `null` ambient as never holding a foreign
/// marker, so the source was read by value: `path` answered `["nope"]`, `= 9` wrote a `nope` key and `del` left the
/// document. A `null` `$x` (identical to the register by kind), a `foreach` source and a non-`null` register answer as
/// before. Rows captured from jq 1.7.1, both routes.
#[test]
fn test_fold_source_foreign_destructure_on_a_null_register_after_a_stage_5030() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1},"c":{"b":2}}"#,
            r"path(.a as $x | .nope | reduce ($x as {b:$b} | .) as $y (.; .))",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"b\":1}",
            5,
        ),
        (
            r#"{"a":{"b":1},"c":{"b":2}}"#,
            r"path(.a as $x | .nope | reduce ($x | . as {b:$b} | .) as $y (.; .))",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"b\":1}",
            5,
        ),
        (
            r#"{"a":{"b":1},"c":{"b":2}}"#,
            r"(. as $x | .nope | reduce (($x as {b:$q} | $q), 1) as $y (.a | .c; .b?)) = 9",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"a\":{\"b\":1},\"c\":{\"b\":2}}",
            5,
        ),
        (
            r#"{"a":{"b":1},"b":null}"#,
            r"(.a as $x | .nope | reduce ($x | . | . as {b:$q} | $q) as $y (.a.b; .)) |= 9",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"b\":1}",
            5,
        ),
        (
            r#"{"a":{"b":1},"c":{"b":2}}"#,
            r"del(. as $x | .nope | reduce (foreach ($x | . | . as {b:{c:$q}} | empty) as $w (0; .; .)) as $y (.a; .))",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"a\":{\"b\":1},\"c\":{\"b\":2}}",
            5,
        ),
        (
            r#"{"a":{"b":1},"c":{"b":2}}"#,
            r#"try path(.a as $x | .nope | reduce ($x | . as {a:$q} | .) as $y (null; .)) catch "C""#,
            "\"C\"\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | .nope | reduce (first($x as {c:$q} | 1)) as $y (.b; .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"c":{"b":2}}"#,
            r"path(.nope as $x | .zz | reduce ($x as {b:$b} | .) as $y (.; .))",
            "[\"zz\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"c":{"b":2}}"#,
            r"path(.a as $x | .nope | foreach ($x as {b:$b} | .) as $y (.; .; .))",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"b\":1}",
            5,
        ),
        (
            r#"{"a":{"b":1},"c":{"b":2}}"#,
            r"path(.a as $x | .c | reduce ($x as {b:$b} | .) as $y (.; .))",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"b\":1}",
            5,
        ),
    ])
}

// #5030, the `reduce` spelling of a `null` `$x` below INIT's node: every row below is captured from jq 1.7.1, both routes.

/// The issue's rows: `$x` is `null` (bound at a missing key), INIT moved the register onto `.a`, a container.
#[test]
fn test_reduce_source_destructures_a_null_x_below_inits_register_5030() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{},"b":{}}"#,
            r"path(.a.c as $x | reduce ($x as {b:$q} | .) as $y (.a; .))",
            "",
            "Invalid path expression near attempt to access element \"b\"",
            5,
        ),
        (
            r#"{"a":{},"b":{}}"#,
            r"[path(.a.c as $x | reduce ($x as {b:$q} | .) as $y (.a; .))?]",
            "[]\n",
            "",
            0,
        ),
    ])
}

/// Rows from a 12,000-row differential grid (bind site x INIT x source wrapper x fold kind x context x document) that `main` answered or wrote and jq raises, empties, or answers differently: a `try`/`?` around the destructure catches the raise, a bare one does not.
#[test]
fn test_reduce_source_null_x_below_init_register_in_the_grid_5030() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":null},"b":1}"#,
            r"[path(.a.b as $x | reduce (($x as {b:$q} | .)?) as $y (.a; .c?))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":null},"b":1}"#,
            r#"try path(.a.b as $x | reduce (try ($x | . as {b:$q} | .)) as $y (.b; $y)) catch "C""#,
            "[\"b\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"(.a.c.d as $x | reduce (try ($x | . as {b:$q} | .)) as $y (.b; null)) = 9",
            "{\"a\":{\"b\":1,\"c\":{\"b\":3}},\"b\":9}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":null},"b":1}"#,
            r#"try path(.a.b as $x | reduce (($x as {b:$q} | .)?) as $y (.a; $y)) catch "C""#,
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{},"b":{}}"#,
            r"path(.a.c as $x | reduce ($x as [$q] | .) as $y (.a; .))",
            "",
            "Invalid path expression near attempt to access element 0",
            5,
        ),
        (
            r#"{"a":{"b":1},"c":{"b":2}}"#,
            r"del(.a.c as $x | reduce (try ($x | . as {b:$q} | .)) as $y (.a.b; null))",
            "{\"a\":{},\"c\":{\"b\":2}}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"c":null},"b":{"b":4}}"#,
            r"[path(.a.b as $x | reduce (try ($x | . as {b:$q} | .)) as $y (.b; .b))?]",
            "[[\"b\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"c":{"b":2}}"#,
            r"(.a.c.d as $x | reduce ($x as {b:$q} | .) as $y (.a.b; .)) = 9",
            "",
            "Invalid path expression near attempt to access element \"b\"",
            5,
        ),
        (
            r#"{"a":{"c":null},"b":{"b":4}}"#,
            r"(.a.c as $x | reduce (($x as {b:$q} | .)?) as $y (.a; .b)) |= 9",
            "{\"a\":9,\"b\":{\"b\":4}}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"path(.a.c.d as $x | reduce (($x as {b:$q} | .)?) as $y (.a; .b))",
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":null},"b":1}"#,
            r#"try path(.nope as $x | reduce ($x as {b:$q} | $q) as $y (.b; .)) catch "C""#,
            "\"C\"\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":null},"b":1}"#,
            r"del(.a.b as $x | reduce ($x | . as {b:$q} | .) as $y (.a; .))",
            "",
            "Invalid path expression near attempt to access element \"b\"",
            5,
        ),
        (
            r#"{"a":{"c":null},"b":{"b":4}}"#,
            r"path(.nope as $x | reduce (try ($x as {b:$q} | .)) as $y (.b; $y))",
            "[\"b\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"del(.nope as $x | reduce (($x as {b:$q} | .)?) as $y (.a.b; .b))",
            "{\"a\":{\"c\":{\"b\":3}},\"b\":2}\n",
            "",
            0,
        ),
    ])
}

/// Guards: a `$x` that is the register (a container at INIT's node, or a `null`/`false` where INIT left the register on one) is still answered.
#[test]
fn test_reduce_source_x_that_is_the_register_keeps_its_answer_5030() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1}}"#,
            r"path(.a as $x | reduce ($x as {b:$q} | .) as $y (.a; .))",
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"[path(.a as $x | reduce ($x as {b:$q} | .) as $y (.a; .))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":1}"#,
            r"path(.nope as $x | reduce ($x as {b:$q} | .) as $y (.nope; .))",
            "[\"nope\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(.a as $x | reduce ($x as {b:$q} | .) as $y (.a; .))",
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"del(.a as $x | reduce ($x | . as {b:$q} | .) as $y (.a; .))",
            "{}\n",
            "",
            0,
        ),
    ])
}
