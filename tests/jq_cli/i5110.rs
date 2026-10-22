//! #5110: a literal stage ahead of a nested `foreach` that destructures a `$x` marker, in a `foreach` UPDATE.

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

// `. as $x | foreach . as $i (.; 1 | foreach ($x as {a:$a} | .) as $y (.; .; .); $i)` on `null`: the nested `foreach`'s
// source steps jq's register from `$x` (the register) to `$x.a`, the `1` stage moves nothing, and EXTRACT's `$i` is judged
// at `["a"]`, so jq answers `["a"]` (and `|= 9` writes `{"a":9}`). The bare nested `foreach` has been restated at the
// register its bind moved to since #4957 (`bare_bind_foreach_update_key`); behind a literal stage the UPDATE was not read as
// one, so the emission was judged at the root and refused. Only where the source cannot move the register and every `$x`
// the UPDATE reads is the fold's input: a marker bound elsewhere raises in jq, and a moving source leaves the standing to
// the stage-less reading.

/// #5110: the issue's rows, the controls that keep the reading they had (a marker that is not the register), and rows of a differential grid (document x bind site x literal stage x nested `foreach` x wrapper x source x
/// INIT x EXTRACT x context) that `main` refused or answered wrongly. Every row captured from `/usr/bin/jq` 1.7.1
/// (stdout, exit code, message), both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5110_literal_stage_ahead_of_a_bare_bind_foreach_in_a_foreach_update() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r"null",
            r"path(. as $x | foreach . as $i (.; 1 | foreach ($x as {a:$a}|.) as $y (.; .; .); $i))",
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r"null",
            r"(. as $x | foreach . as $i (.; 1 | foreach ($x as {a:$a}|.) as $y (.; .; .); $i)) |= 9",
            "{\"a\":9}\n",
            "",
            0,
        ),
        (
            r"null",
            r"del(. as $x | foreach . as $i (.; 1 | foreach ($x as {a:$a}|.) as $y (.; .; .); $i))",
            "null\n",
            "",
            0,
        ),
        (
            r"null",
            r"[path(. as $x | foreach . as $i (.; 1 | foreach (try ($x as {a:$a}|.)) as $y (.; .; .); $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r"{}",
            r#"[path(.b as $x | foreach . as $i (.; try ("s" | foreach (($x as {b:$a} | .)) as $y (.; .; .)); null))?]"#,
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"try path(.a as $x | foreach 1 as $i (.; ("s" | foreach (($x as {b:$a} | .)) as $y (.; .; .)), 1; null)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":[null]}"#,
            r#"try path(.a as $x | foreach . as $i (0; (null | foreach (try ($x as {b:$a} | .)) as $y (.; .; .)), 1; $i)) catch "c""#,
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r#"path(. as $x | foreach .[]? as $i (.; ("a" | foreach (try ($x as {a:$a} | .)) as $y (.; .; .)), 1; $i))"#,
            "[\"a\"]\n[\"b\"]\n",
            "",
            0,
        ),
        (
            r"null",
            r#"del(.a as $x | foreach 1 as $i (.; ("s" | foreach (($x as {a:$a} | .)?) as $y (.; .; .)), 1; null))"#,
            "null\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r#"try path(.a as $x | foreach (.,.) as $i (null; (0 | foreach (($x as {a:$a} | .)?) as $y (.; .; .)), 1; $i)) catch "c""#,
            "[]\n[]\n",
            "",
            0,
        ),
        (
            r"null",
            r#"[path(.a.a as $x | foreach (.,.) as $i (null; "a" | foreach (($x as {a:$a} | .)?) as $y (.; .; .); null))]"#,
            "[[\"a\"],[\"a\"]]\n",
            "",
            0,
        ),
        (
            r"null",
            r#"try path(.b as $x | foreach (1,2) as $i (.; ("a" | foreach (try ($x as {a:$a} | .)) as $y (.; .; .))?; null)) catch "c""#,
            "[\"a\"]\n[\"a\"]\n",
            "",
            0,
        ),
        (
            r"null",
            r"[path(.b as $x | foreach (1,2) as $i (.; 1.5 | foreach (($x as {a:$a} | .)) as $y (.; .; .); null))]",
            "[[\"a\"],[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"del(. as $x | foreach 1 as $i (null; true | foreach (try ($x as {b:$a} | .)) as $y (.; .; .); null))",
            "{\"a\":null}\n",
            "",
            0,
        ),
        (
            r"null",
            r"path(.a as $x | foreach . as $i (0; (true | foreach (try ($x as [$a] | .)) as $y (.; .; .))?; $i))",
            "[0]\n",
            "",
            0,
        ),
        (
            r"null",
            r"[path(.b as $x | foreach . as $i (.; (null | foreach (($x as {a:$a} | .)?) as $y (.; .; .)), 1; null))?]",
            "[[\"a\"],[]]\n",
            "",
            0,
        ),
        (
            r"null",
            r"[path(.a as $x | foreach (1,2) as $i (null; try (1.5 | foreach (($x as {a:$a} | .)?) as $y (.; .; .)); null))?]",
            "[[\"a\"],[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"[path(.b as $x | foreach .a as $i (.; (1.5 | foreach (($x as {a:$a} | .)?) as $y (.; .; .)), 1; $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"del(. as $x | foreach .a as $i ({}; ("a" | foreach (try ($x as [$a] | .)) as $y (.; .; .)), 1; $i))"#,
            "{}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach (1,2) as $i (.; (null | foreach (($x as {b:$a} | .)) as $y (.; .; .))?; .a?)) |= 9",
            "{\"a\":null,\"b\":{\"a\":9}}\n",
            "",
            0,
        ),
        (
            r"null",
            r#"path(.a as $x | foreach (.,.) as $i (.; try ("a" | foreach (($x as {b:$a} | .)) as $y (.; .; .)); null))"#,
            "[\"b\"]\n[\"b\"]\n",
            "",
            0,
        ),
        (
            r"null",
            r#"path(.b as $x | foreach . as $i ({}; ("a" | foreach (try ($x as {b:$a} | .)) as $y (.; .; .))?; null))"#,
            "[\"b\"]\n",
            "",
            0,
        ),
        (
            r"null",
            r"del(. as $x | foreach . as $i ({}; (1.5 | foreach (try ($x as {a:$a} | .)) as $y (.; .; .))?; null))",
            "null\n",
            "",
            0,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"try path(.a as $x | foreach . as $i (null; (1 | foreach (try ($x as {b:$a} | .)) as $y (.; .; .)), 1; $i)) catch "c""#,
            "[]\n",
            "",
            0,
        ),
        (
            r"{}",
            r#"try path(. as $x | foreach 1 as $i ({}; (true | foreach (($x as {a:$a} | .)) as $y (.; .; .))?; null)) catch "c""#,
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r"{}",
            r"path(.a as $x | foreach (.,.) as $i (.; (0 | foreach (try ($x as {a:$a} | .)) as $y (.; .; .)), 1; $i))",
            "[]\n[]\n",
            "",
            0,
        ),
        (
            r"null",
            r"path(.a as $x | foreach (.,.) as $i ({}; true | foreach (($x as {a:$a} | .)?) as $y (.; .; .); null))",
            "[\"a\"]\n[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r#"[path(.b as $x | foreach .[]? as $i (.; ("a" | foreach (($x as {a:$a} | .)?) as $y (.; .; .)), 1; $i))]"#,
            "[[\"a\"],[\"b\"]]\n",
            "",
            0,
        ),
        (
            r"null",
            r"path(.b as $x | foreach (1,2) as $i (0; (true | foreach (try ($x as {b:$a} | .)) as $y (.; .; .)), 1; null))",
            "[\"b\"]\n[]\n[\"b\"]\n[]\n",
            "",
            0,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"del(. as $x | foreach .a as $i (null; (1 | foreach (try ($x as {b:$a} | .)) as $y (.; .; .)), 1; $i))",
            "{\"b\":false}\n",
            "",
            0,
        ),
        (
            r"null",
            r#"try path(.b as $x | foreach (.,.) as $i (.; 1 | foreach (($x as {b:$a} | .)) as $y (.; .; .); null)) catch "c""#,
            "[\"b\"]\n[\"b\"]\n",
            "",
            0,
        ),
        (
            r"null",
            r"(.a.a as $x | foreach (1,2) as $i (null; true | foreach (try ($x as {b:$a} | .)) as $y (.; .; .); null)) |= 9",
            "{\"b\":9}\n",
            "",
            0,
        ),
        (
            r"null",
            r#"del(. as $x | foreach (.,.) as $i (0; ("s" | foreach (($x as {a:$a} | .)) as $y (.; .; .))?; null))"#,
            "null\n",
            "",
            0,
        ),
        (
            r"null",
            r#"(. as $x | foreach (1,2) as $i ({}; try ("a" | foreach (($x as {a:$a} | .)) as $y (.; .; .)); null)) |= 9"#,
            "{\"a\":9}\n",
            "",
            0,
        ),
        (
            r"null",
            r#"try path(.b as $x | foreach (1,2) as $i (null; try ("s" | foreach (($x as {a:$a} | .)?) as $y (.; .; .)); null)) catch "c""#,
            "[\"a\"]\n[\"a\"]\n",
            "",
            0,
        ),
        (
            r"null",
            r"[path(.a.a as $x | foreach . as $i ({}; (1 | foreach (try ($x as [$a] | .)) as $y (.; .; .)), 1; null))]",
            "[[0],[]]\n",
            "",
            0,
        ),
        (
            r"null",
            r"del(. as $x | foreach . as $i (null; try (0 | foreach (try ($x as {b:$a} | .)) as $y (.; .; .)); null))",
            "null\n",
            "",
            0,
        ),
        (
            r"null",
            r"del(.a.a as $x | foreach 1 as $i ({}; (0 | foreach (try ($x as {a:$a} | .)) as $y (.; .; .)), 1; null))",
            "null\n",
            "",
            0,
        ),
        (
            r#"{"a":[null]}"#,
            r#"try path(. as $x | foreach (1,2) as $i (0; try ("s" | foreach (try ($x as {b:$a} | .)) as $y (.; .; .)); null)) catch "c""#,
            "[\"b\"]\n[\"b\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"try path(. as $x | foreach . as $i (.; (1 | foreach (($x as {a:$a} | .)) as $y (.; .; .))?; null)) catch "c""#,
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r"null",
            r#"path(. as $x | foreach (.,.) as $i (null; ("a" | foreach (try ($x as {b:$a} | .)) as $y (.; .; .))?; null))"#,
            "[\"b\"]\n[\"b\"]\n",
            "",
            0,
        ),
        (
            r"null",
            r#"[path(.a.a as $x | foreach 1 as $i (null; "a" | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))]"#,
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"[path(. as $x | foreach .[]? as $i ({}; ("s" | foreach (try ($x as {b:$a} | .)) as $y (.; .; .)), 1; $i))]"#,
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r"null",
            r#"try path(.b as $x | foreach (.,.) as $i (.; (true | foreach (try ($x as {a:$a} | .)) as $y (.; .; .))?; $i)) catch "c""#,
            "[\"a\"]\n[\"a\"]\n",
            "",
            0,
        ),
        (
            r"null",
            r"path(.b as $x | foreach (1,2) as $i ({}; (true | foreach (try ($x as [$a] | .)) as $y (.; .; .)), 1; null))",
            "[0]\n[]\n[0]\n[]\n",
            "",
            0,
        ),
        (
            r#"{"a":[null]}"#,
            r#"[path(. as $x | foreach . as $i (.; try ("s" | foreach (try ($x as {b:$a} | .)) as $y (.; .; .)); null))]"#,
            "[[\"b\"]]\n",
            "",
            0,
        ),
    ])
}
