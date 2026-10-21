//! #5013: a fold whose INIT navigates moves jq's register before its SOURCE runs, so a `$x` frozen where the fold
//! entered is no longer the register, and a destructure of it in SOURCE raises at its first step.

use crate::assert_path_rows_both_routes_3749;
use anyhow::Result;

/// #5013: `foreach`/`reduce` SOURCE `$x as {b:$q} | .` after INIT `.a`: jq raises `near attempt to access element "b"`
/// at the pattern's step. The by-value drive answered `["a"]`, so `del` deleted `.a` and `|= 9`/`= 9` wrote `.a`
/// where jq leaves the document and errors. Rows captured from jq 1.7.1 (stdout, first stderr line, exit code), both
/// routes.
#[test]
fn test_fold_source_foreign_destructure_after_a_navigating_init_raises_5013() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"path(. as $x | foreach ($x as {b:$q} | .) as $y (.a; .; .))",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"a\":{\"b\":1},\"b\":2}",
            5,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"del(. as $x | foreach ($x as {b:$q} | .) as $y (.a; .; .))",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"a\":{\"b\":1},\"b\":2}",
            5,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"(. as $x | foreach ($x as {b:$q} | .) as $y (.a; .; .)) |= 9",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"a\":{\"b\":1},\"b\":2}",
            5,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"(. as $x | reduce ($x as {b:$q} | .) as $y (.a; .)) = 9",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"a\":{\"b\":1},\"b\":2}",
            5,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"path(. as $x | reduce ($x as {b:$q} | .) as $y (.a; .))",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"a\":{\"b\":1},\"b\":2}",
            5,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"del(. as $x | foreach ($x as {b:$q} | .) as $y (.a.b; .; .))",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"a\":{\"b\":1},\"b\":2}",
            5,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"[path(. as $x | foreach ($x as {b:$q} | .) as $y (.a; .; .))?]",
            "[]\n",
            "",
            0,
        ),
    ])
}

/// #5013: a `try`/`?` around that destructure catches the raise and leaves the source empty, so a `foreach` emits
/// nothing and a `reduce` answers its INIT (`del` deleted `.a` where jq leaves the document); a handler that emits
/// supplies the element instead. Rows captured from jq 1.7.1, both routes.
#[test]
fn test_fold_source_caught_foreign_destructure_after_a_navigating_init_empties_the_source_5013(
) -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"del(. as $x | foreach (try ($x as {b:$q} | .)) as $y (.a; .; .))",
            "{\"a\":{\"b\":1},\"b\":2}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"path(. as $x | foreach (try ($x as {b:$q} | .)) as $y (.a; .; .))",
            "",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"[path(. as $x | foreach (($x as {b:$q} | .)?) as $y (.a; .; .))]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"path(. as $x | foreach (try ($x as {b:$q} | .) catch 5) as $y (.a; .; .))",
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"path(. as $x | reduce (try ($x as {b:$q} | .)) as $y (.a; .))",
            "[\"a\"]\n",
            "",
            0,
        ),
    ])
}

/// #5013: unchanged: an INIT that does not move the register (`.`, `0`), a SOURCE that navigates rather than
/// destructures (it already raised), and a `$x` bound at INIT's own node. Rows captured from jq 1.7.1, both routes.
#[test]
fn test_fold_source_destructure_controls_5013() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"path(. as $x | foreach ($x as {b:$q} | .) as $y (.; .; .))",
            "",
            "Invalid path expression with result {\"a\":{\"b\":1},\"b\":2}",
            5,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"path(foreach (.b) as $y (.a; .; .))",
            "",
            "Invalid path expression near attempt to access element \"b\" of {\"a\":{\"b\":1},\"b\":2}",
            5,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"path(. as $x | foreach ($x as {b:$q} | .) as $y (0; .; .))",
            "",
            "Invalid path expression with result 0",
            5,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"path(.a as $x | foreach (.) as $y (.a; .; $x))",
            "[\"a\"]\n",
            "",
            0,
        ),
    ])
}

/// #5013: a `?//` chain is left to the by-value route: jq names the last alternative's failing step (`element 0`),
/// where the resolver names the first's. Rows captured from jq 1.7.1, both routes.
#[test]
fn test_fold_source_alternative_chain_after_a_navigating_init_keeps_jqs_message_5013() -> Result<()>
{
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"a":1}}"#,
            r"(. as $x | foreach ($x as {a:$q} ?// [$q] | 1) as $y ((.a,.b); .b?)) = 9",
            "",
            "Invalid path expression near attempt to access element 0 of {\"a\":{\"a\":1}}",
            5,
        ),
        (
            r#"{"a":{"b":null},"b":null}"#,
            r"((. as $x | foreach ($x as {a:$q} ?// [$q] | 1) as $y ((.a,.b); .b?))) |= 9",
            "",
            "Invalid path expression near attempt to access element 0 of {\"a\":{\"b\":null},\"b\":null}",
            5,
        ),
    ])
}
