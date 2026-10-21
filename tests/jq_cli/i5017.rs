//! #5017: a fold source after a navigating INIT, outside every fold body, runs with jq's register where INIT left it
//! (its position and value on the frame), so a `$x` bound at INIT's own node is that register and any other marker's
//! destructure raises at its step, however the destructure is wrapped.

use crate::assert_path_rows_both_routes_3749;
use anyhow::Result;

/// #5017: on `{"a":{"b":1,"c":{"b":3}},"b":2}`, after INIT `.a`: a caught destructure of a foreign `$x` behind a stage
/// (`$x | . as P`), a `label` or a `select(true)`, or beside a comma item, empties that item as jq's `try` does
/// (`del` deleted `.a`, the comma answered `["a"]` twice); a `$x` bound at `.a` is the register, so its destructure
/// walks from it and jq's refusal at the end of the path stands (it answered `["a"]`), and `$x | .c` is
/// `["a","c"]` (it raised). Rows captured from jq 1.7.1, both routes.
#[test]
fn test_fold_source_after_a_navigating_init_runs_on_the_moved_register_5017() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"del(. as $x | foreach (try ($x | . as {b:$q} | .)) as $y (.a; .; .))",
            "{\"a\":{\"b\":1,\"c\":{\"b\":3}},\"b\":2}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"del(. as $x | foreach (label $f | try ($x as {b:$q} | .)) as $y (.a; .; .))",
            "{\"a\":{\"b\":1,\"c\":{\"b\":3}},\"b\":2}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"del(. as $x | foreach (try (select(true) | $x as {b:$q} | .)) as $y (.a; .; .))",
            "{\"a\":{\"b\":1,\"c\":{\"b\":3}},\"b\":2}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"path(. as $x | foreach ((try ($x as {b:$q} | .)), 1) as $y (.a; .; .))",
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"path(.a as $x | foreach ($x as {b:$q} | .) as $y (.a; .; .))",
            "",
            "Invalid path expression with result {\"b\":1,\"c\":{\"b\":3}}",
            5,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"path(.a as $x | foreach ($x | .c) as $y (.a; .; $y))",
            "[\"a\",\"c\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"path(. as $x | foreach (try ($x | . as {b:$q} | .)) as $y (.a; .; .))",
            "",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"(.a as $x | foreach ($x | .c) as $y (.a; .; $y)) |= 9",
            "{\"a\":{\"b\":1,\"c\":9},\"b\":2}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"del(.a as $x | foreach ($x as {b:$q} | .) as $y (.a; .; .))",
            "",
            "Invalid path expression with result {\"b\":1,\"c\":{\"b\":3}}",
            5,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"[path(. as $x | foreach (label $f | try ($x as {b:$q} | .)) as $y (.a; .; .))]",
            "[]\n",
            "",
            0,
        ),
    ])
}
