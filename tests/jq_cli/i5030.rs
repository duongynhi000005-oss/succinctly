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
