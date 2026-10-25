//! #5226: a `//` right operand whose bind source is the loop variable (`$i`, `$i.a`) in a fold UPDATE is refused as jq refuses it.
//!
//! The interpreter reads storage identity, which the `unshared-containers` holdout does not have, so these pin the
//! shipped shape only.
#![cfg(not(feature = "unshared-containers"))]

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

/// #5223 Phase 3: this issue's repro rows, with `path`, `del` and `|=`
/// wherever the row is a `path(...)`, as jq 1.7.1 answers them (stdout, the
/// first error message, the exit code). The path-mode interpreter carries
/// jq's register, so they match by construction, on both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_superseded_rows_match_jq_through_the_path_mode_interpreter_5226() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i as $q | $q))))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"del(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i as $q | $q))))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i as $q | $q)))) |= 9",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i as $q | $q)); .))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"del(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i as $q | $q)); .))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i as $q | $q)); .)) |= 9",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | reduce (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i as $q | $q))))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"del(. as $x | reduce (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i as $q | $q))))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"(. as $x | reduce (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i as $q | $q)))) |= 9",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i.a as $q | $q))))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"del(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i.a as $q | $q))))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i.a as $q | $q)))) |= 9",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i.a as $q | $q)); .))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"del(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i.a as $q | $q)); .))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i.a as $q | $q)); .)) |= 9",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | reduce (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i.a as $q | $q))))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"del(. as $x | reduce (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i.a as $q | $q))))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"(. as $x | reduce (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i.a as $q | $q)))) |= 9",
            "",
            "Invalid path expression with result 5",
            5,
        ),
    ])
}
