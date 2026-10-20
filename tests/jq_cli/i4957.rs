//! #4957: an `if` on a literal truthy condition around a caught foreign-`$x` `foreach` in a `foreach` UPDATE over a
//! moving source.

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

// `if true then FE else . end` runs `FE` and nothing else, so jq answers it as it does the bare `FE`. The binary read
// the `if` as one that may leave the state `null` (the `truthy` check of `if . then FE else . end`), so the
// foreach's EXTRACT `.a` raised jq's index error as a guess that no `?` or `try` catches, and `[path(..)?]` exited 5
// where jq prints `[["a","a"]]`.

/// #4957: `true`, `1` and `"s"` as the condition, with and without an `else .`, and nested in a taken `then`; a falsy
/// literal and a literal `else` keep the reading they had. Every row captured from `/usr/bin/jq` 1.7.1 (stdout, first stderr line up to the
/// container, exit code), both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_4957_literal_truthy_if_around_a_caught_foreign_foreach() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))",
            "[\"a\",\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"try path(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a)) catch "C""#,
            "[\"a\",\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"del(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))",
            "{\"a\":null}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a)) |= 5",
            "{\"a\":{\"a\":5}}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; $i))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; $i)) |= 5",
            "",
            r#"Invalid path expression with result {"a":null}"#,
            5,
        ),
        (
            r#"{"a":true}"#,
            r"[path(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":1}}"#,
            r"[path(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.a,.) as $i (.; if 1 then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[del(. as $x | foreach (.a,.) as $i (.; if 1 then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))?]",
            "[{\"a\":null}]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"[path(. as $x | foreach (.a,.) as $i (.; if "s" then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))?]"#,
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) end; .a))",
            "[\"a\",\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[del(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) end; .a))?]",
            "[{\"a\":null}]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; $y)) else . end; .a))?]",
            "[[\"a\",\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; $y)) else . end; $i))",
            "",
            r#"Invalid path expression with result {"a":null}"#,
            5,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.a,.) as $i (.; if false then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach (.a,.) as $i (.; if false then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))",
            "",
            r#"Invalid path expression near attempt to access element "a""#,
            5,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.a,.) as $i (.; if true then (if . then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end) else . end; .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.a,.) as $i (.; if true then (if . then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end) else . end; $i))?]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.a,.) as $i (.; if true then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) elif . then 1 else . end; $i))?]",
            "[]\n",
            "",
            0,
        ),
    ])
}

// Rows 2 and 3 of the issue: a bare caught-bind `foreach` (`FE`) as the UPDATE of a `foreach` whose source leaves jq's
// register on the root at the step (`1,2`, `.`, `..`, `.,.a`), or whose caught bind is of `.`. The accumulator is the
// register there, so `$x` (bound at the root) is it: the pattern's step passes and moves the register, `FE` emits the
// state, and the EXTRACT's `.a` raises jq's own index error -- which the enclosing `?` catches. The binary refused
// instead, with a guess no `?` or `try` catches (exit 5).

/// #4957: the root-leaving sources against the `if . then FE else . end`, bare, comma, pipe and `try` spellings of the
/// UPDATE, in `path`, `del`, `|=` and `=` position, with the `$i` and `.` EXTRACTs. Every row captured from
/// `/usr/bin/jq` 1.7.1 (stdout, first stderr line up to the container, exit code), both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_4957_bare_caught_bind_foreach_update_on_a_root_register() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.,.a) as $i (.; if . then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (1,2) as $i (.; if . then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (..) as $i (.; if . then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.) as $i (.; if . then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":1}"#,
            r"[path(. as $x | foreach (1,2) as $i (.; if . then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach (1,2) as $i (.; if . then (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else . end; .a))",
            "",
            r#"Invalid path expression near attempt to access element "a""#,
            5,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach (.,.a) as $i (.; foreach (try ($x as {a:$a} | .)) as $y (.; .; .); .a))",
            "",
            r#"Invalid path expression near attempt to access element "a""#,
            5,
        ),
        (
            r#"{"a":null}"#,
            r#"try path(. as $x | foreach (.,.a) as $i (.; foreach (try ($x as {a:$a} | .)) as $y (.; .; .); .a)) catch "C""#,
            "\"C\"\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.,.a) as $i (.; foreach (try ($x as {a:$a} | .)) as $y (.; .; .); .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach (.,.a) as $i (.; foreach (try ($x as {a:$a} | .)) as $y (.; .; .); .))",
            "",
            r#"Invalid path expression with result {"a":null}"#,
            5,
        ),
        (
            r#"{"a":{"b":2}}"#,
            r"[del(. as $x | foreach (1,2) as $i (.; foreach (try ($x as {a:$a} | .)) as $y (.; .; .); .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":2}}"#,
            r"[(. as $x | foreach (1,2) as $i (.; foreach (try ($x as {a:$a} | .)) as $y (.; .; .); .a)) |= 5]",
            "",
            r#"Invalid path expression near attempt to access element "a""#,
            5,
        ),
        (
            r"[1]",
            r"[path(. as $x | foreach (1,2) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)); .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r"[1]",
            r"path(. as $x | foreach (1,2) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)); .))",
            "",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (1,2) as $i (.; foreach (try (. as {a:$a} | .)) as $y (.; .; .); .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":1}"#,
            r"[path(. as $x | foreach (.,.a) as $i (.; foreach (try (. as {a:$a} | .)) as $y (.; .; .); .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":1}"#,
            r"[del(. as $x | foreach (.a,.) as $i (.; foreach (try (. as {a:$a} | .) catch .) as $y (.; .; .); $i))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":1}"#,
            r"del(. as $x | foreach (.a,.) as $i (.; foreach (try (. as {a:$a} | .) catch .) as $y (.; .; .); $i))",
            "",
            r#"Invalid path expression with result {"a":1}"#,
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"[del(. as $x | foreach (.a,.) as $i (.; foreach (try (. as {a:$a} | .) catch .) as $y (.; .; .); $i))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (1,2) as $i (.; (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)), 7; .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (.,.a) as $i (.; foreach (try ($x as {a:$a} | .)) as $y (.; .; .) | .; .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (1,2) as $i (.; foreach (try ($x as [$a] | .)) as $y (.; .; .); .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (..) as $i (.; foreach ($x as {a:$a} | .) as $y (.; .; .); .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[path(. as $x | foreach (1,2) as $i (.; foreach ($x as {a:$a} | .) as $y (.; .; .); .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach (1,2) as $i (.; foreach (try ($x as {a:$a} | .)) as $y (.; .; .); $i))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach (1,2) as $i (.; foreach (try ($x as {a:$a} | .)) as $y (.; .; .); .a)) |= 5",
            "",
            r#"Invalid path expression near attempt to access element "a""#,
            5,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach (1,2) as $i (.; foreach (try ($x as {a:$a} | .)) as $y (.; .; .); .a)) = 5",
            "",
            r#"Invalid path expression near attempt to access element "a""#,
            5,
        ),
    ])
}
