//! #4957: a caught foreign-`$x` `foreach` as, or inside, the UPDATE of a `foreach`: an `if` on a literal truthy condition
//! around it over a moving source, and a bare one over a source that leaves the register on the root.

use anyhow::Result;

use crate::{assert_path_rows_both_routes_3749, run_jq_full};

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

/// #4957: the spellings of the `foreach` that is read (a `$name` or `.` bound, an object or array pattern, a
/// `catch .` handler, a `?` around the bind) and of the UPDATE around it (`try`, a comma, a pipe, an `if`), each on a
/// row the rest of the file's rows do not reach. Every row captured from `/usr/bin/jq` 1.7.1, both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_4957_bare_caught_bind_foreach_update_spellings() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r"[1]",
            r"[del(. as $x | foreach (.) as $i (.; (foreach (($x as {b:$a} | .)) as $y (.; .; .)), .; .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":true}"#,
            r"[del(. as $x | foreach (1,2) as $i (.; (foreach ((. as {a:$a} | .)) as $y (.; .; .)), .; .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r"[1]",
            r"[del(. as $x | foreach (.) as $i (.; (foreach (($x as [$a] | .)) as $y (.; .; .)) | 5; .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r"[1]",
            r"[del(. as $x | foreach (..) as $i (.; (foreach (($x as [$a] | .)) as $y (.; .; .)) | 5; $i))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r"[1]",
            r"del(. as $x | foreach (.) as $i (.; try (foreach ($x as {a:$a} | .) as $y (.; .; .)); .))",
            "[1]\n",
            "",
            0,
        ),
        (
            r"1",
            r"del(. as $x | foreach (.) as $i (.; try (foreach ($x as {a:$a} | .) as $y (.; .; .)); .))",
            "1\n",
            "",
            0,
        ),
        (
            r#"{"a":1}"#,
            r"[path(. as $x | foreach (.) as $i (.; foreach (try (. as {a:$a} | .) catch .) as $y (.; .; .); .a))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":2}}"#,
            r"[path(. as $x | foreach (..) as $i (.; foreach (try ($x as {a:$a} | .) catch .) as $y (.; .; .); .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":false,"b":null}"#,
            r"[path(. as $x | foreach ($x) as $i (.; foreach (($x as {b:$a} | .)?) as $y (.; .; .); .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[del(. as $x | foreach (.) as $i (.; foreach (($x as {a:$a} | .)?) as $y (.; .; .); .[0]))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":2}}"#,
            r"[path(. as $x | foreach ((.,.)) as $i (.; foreach ((. as [$a] | .)) as $y (.; .; .); .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[del(. as $x | foreach (.) as $i (.; foreach (try (. as {a:$a} | .)) as $y (.; .; .); .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[del(. as $x | foreach (.) as $i (.; foreach ($x as {a:$a} | .) as $y (.; .; .); .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":1}"#,
            r"[del(. as $x | foreach (.) as $i (.; foreach ($x as {a:$a} | .) as $y (.; .; .); .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r"[1]",
            r"[del(. as $x | foreach (.) as $i (.; foreach (try ($x as [$a] | .)) as $y (.; .; .); .))?]",
            "[]\n",
            "",
            0,
        ),
    ])
}

/// #4957: shapes the read does not take -- a pattern of two or more variables (its absent key lands on a `null` the
/// resolver places differently from jq's), a body other than `.`, a `?//` chain, a nested pattern, a `select` stage, a source that computes --
/// refuse as they did, loudly, or answer as jq does. Reading them as the bare one-variable bind flipped these rows to
/// a caught raise that printed `[]`/`"C"` where jq prints a path, or to an answer where jq refuses (11 and 6 of 40,000
/// and 30,000 random rows). Each row is `(input, filter, jq 1.7.1's stdout, jq's exit code)`.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_4957_shapes_outside_the_read_never_answer_where_jq_differs() -> Result<()> {
    let rows: &[(&str, &str, &str, i32)] = &[
        (
            r"[1]",
            r"[del(. as $x | foreach ((1,.)) as $i (.; (foreach (($x as [$a,$b,$c] | .)?) as $y (.; .; .)) | .; .[0]))?]",
            "[[1]]\n",
            0,
        ),
        (
            r"[1]",
            r#"try path(. as $x | foreach ((.,.)) as $i (.; (foreach (try ($i as [$a,$b,$c] | .)) as $y (.; .; .)) | .; .)) catch "C""#,
            "",
            0,
        ),
        (
            r"[[1],[2]]",
            r"[del(. as $x | foreach (1,2) as $i (.; try (foreach (try ($x as [$a,$b] | .)) as $y (.; .; .)); $i))?]",
            "[[[1],[2]]]\n",
            0,
        ),
        (
            r#"{"a":false,"b":null}"#,
            r#"try path(. as $x | foreach ($x,.a) as $i (.; (foreach (try ($x as {a:$a,b:$b} | .)) as $y (.; .; .)), .; $i)) catch "C""#,
            "[]\n[\"a\"]\n",
            0,
        ),
        (
            r#"{"a":true}"#,
            r"[del(. as $x | foreach (first(.,.a)) as $i (.; (foreach (try ($x as {a:$a,b:$b} | .)) as $y (.; .; .)), 7; $i))?]",
            "[null]\n",
            0,
        ),
        (
            r#"{"a":{"b":2}}"#,
            r#"try path(. as $x | foreach (1,2) as $i (.; foreach (try ($x as {a:$a,b:$b} | .)) as $y (.; .; .); .a)) catch "C""#,
            "",
            0,
        ),
        (
            r"[1]",
            r"[path(. as $x | foreach (.) as $i (.; foreach (first(try ($x as {a:$a} ?// [$a] | $i))) as $y (0; ($i), $i; .); $i))]",
            "",
            5,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach ((.,.)) as $i (.; foreach (select(.) | try (. as {a:{b:$a}} | .)) as $y (0; try (5); $i); $i))",
            "",
            5,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"path(. as $x | foreach (null) as $i (.; foreach (($x as {a:$a} ?// [$a] | .)) as $y ($x; (.a?), .; .a?); .))",
            "",
            5,
        ),
        (
            r#"{"a":{"b":2}}"#,
            r"[path(. as $x | foreach (..) as $i (.; (try (.a? // .)), foreach (select($i) | ($x as {a:{b:$a}} | $x)) as $y (.; empty; $i); .))]",
            "",
            5,
        ),
        (
            r"[1]",
            r"path(. as $x | foreach (..) as $i (.; if true then (foreach (try (($i as {a:$a} ?// [$a] | .))) as $y ($x; .a?; .)) else empty end; .a))",
            "",
            5,
        ),
        (
            r#"{"a":[1,2]}"#,
            r"[path(. as $x | foreach (..) as $i (.; foreach (select($x) | try ($i as {a:$a} ?// [$a] | $x)) as $y (.; try ($i); .a?); $i))]",
            "",
            5,
        ),
        (
            r#"{"a":true}"#,
            r"[del(. as $x | foreach (1,2) as $i (.; try (foreach (($x as {a:$a} | $a)) as $y (null; (., 1); $y)); .))?]",
            "[{}]\n",
            0,
        ),
        (
            r#"{"a":[1,2]}"#,
            r"[path(. as $x | foreach (1,2) as $i (.; try (foreach (try ($x as {b:$a} | $a)) as $y (.; $y; .)); .))?]",
            "[[\"b\"],[\"b\"]]\n",
            0,
        ),
    ];
    for (input, filter, jq_stdout, jq_code) in rows {
        let (stdout, _stderr, code) = run_jq_full(&["-c", filter], Some(input))?;
        let same_as_jq = stdout == *jq_stdout && code == *jq_code;
        // jq answers and the binary refuses (a refusal prints nothing), or jq refuses and the binary refuses too.
        let refuses = code != 0 && (*jq_code != 0 || stdout.is_empty());
        assert!(
            same_as_jq || refuses,
            "{filter} on {input}: jq {jq_stdout:?} (exit {jq_code}), binary {stdout:?} (exit {code})"
        );
    }
    Ok(())
}
