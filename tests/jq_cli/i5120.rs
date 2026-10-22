//! #5120: a `foreach` UPDATE or EXTRACT that destructures the register `$x` beside a computed `INIT`.

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

// `. as $x | foreach SRC as $i (INIT; try ($x as {a:$q} | .); EXTRACT)` with a computed INIT (`{}`, `0`, `null`): the
// fold's source does not move jq's register, `$x` is bound at it, so the pattern's step passes `path_intact`, UPDATE
// emits its computed state and jq's own end-of-path check refuses that (`Invalid path expression with result {}`).
// The body was resolved without the register, because `fans_out` counts every destructuring bind, so the step was read
// as raising and the `try` swallowed it: `|=`/`=`/`del` left the document and `path` printed nothing, as for a
// `reduce` UPDATE before #4901 handed it on.

/// #5120: the issue's rows, on the register `$x` bound at the root and at `.a`, and the rows of a differential grid
/// (bind site x INIT x source x UPDATE x EXTRACT x context) that `main` answered wrongly. Every row captured from
/// `/usr/bin/jq` 1.7.1 (stdout, exit code, message), both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5120_register_x_bind_in_a_foreach_body_beside_a_computed_init() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":0,"b":false}"#,
            r"(. as $x | foreach 1 as $i ({}; try ($x as {a:$q} | .); .)) |= 9",
            "",
            "Invalid path expression with result {}",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"[path(. as $x | foreach 1 as $i ({}; try ($x as {a:$q} | .); .))]",
            "",
            "Invalid path expression with result {}",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"[path(. as $x | foreach 1 as $i ({}; try ($x as {a:$q} | .); .))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"path(. as $x | foreach 1 as $i (0; try ($x as {a:$q} | .); .))",
            "",
            "Invalid path expression with result 0",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"path(. as $x | foreach 1 as $i (null; try ($x as {a:$q} | .); .))",
            "",
            "Invalid path expression with result null",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"del(. as $x | foreach 1 as $i ({}; ($x as {a:$q} | .)?; .))",
            "",
            "Invalid path expression with result {}",
            5,
        ),
        (
            r#"{"a":null}"#,
            r#"try path(.a as $x | foreach .[]? as $i ({}; try ($x as {b:$q} | .); .)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"[path(. as $x | foreach (1,2) as $i ("s"; ($x as {a:$q} | .)?))]"#,
            "",
            "Invalid path expression with result \"s\"",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"(. as $x | foreach 1 as $i ("s"; try ($x as {b:$q} | .); .)) |= 9"#,
            "",
            "Invalid path expression with result \"s\"",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"path(. as $x | foreach (1,2) as $i (0; ($x as {a:$q} | .)?))",
            "",
            "Invalid path expression with result 0",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"(. as $x | foreach (1,2) as $i ({}; try ($x as {b:$q} | .); .)) |= 9",
            "",
            "Invalid path expression with result {}",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"(. as $x | foreach (1,2) as $i ([]; ($x as {a:$q} | .)?; .)) |= 9",
            "",
            "Invalid path expression with result []",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"(. as $x | foreach 1 as $i ({}; try ($x as {b:$q} | .); .)) |= 9",
            "",
            "Invalid path expression with result {}",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"del(. as $x | foreach (1,2) as $i ("s"; try ($x as {a:$q} | .)))"#,
            "",
            "Invalid path expression with result \"s\"",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"[path(. as $x | foreach 1 as $i (null; try ($x as {b:$q} | .); .))]",
            "",
            "Invalid path expression with result null",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"try path(. as $x | foreach 1 as $i ([]; ($x as {a:$q} | .)?; $i)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"try path(. as $x | foreach (1,2) as $i ({}; try ($x as {b:$q} | .); $i)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"try path(. as $x | foreach 1 as $i ([]; ($x as {a:$q} | .)?)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"path(. as $x | foreach 1 as $i ("s"; try ($x as {a:$q} | .)))"#,
            "",
            "Invalid path expression with result \"s\"",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"(. as $x | foreach (1,2) as $i ({}; ($x as {a:$q} | .)?; .)) |= 9",
            "",
            "Invalid path expression with result {}",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"del(. as $x | foreach 1 as $i (null; try ($x as {b:$q} | .)))",
            "",
            "Invalid path expression with result null",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"try path(. as $x | foreach 1 as $i ({}; try ($x as {b:$q} | .); $i)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"path(. as $x | foreach (1,2) as $i ("s"; try ($x as {b:$q} | .)))"#,
            "",
            "Invalid path expression with result \"s\"",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"try path(. as $x | foreach (1,2) as $i (0; ($x as {a:$q} | .)?; $i)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"(. as $x | foreach (1,2) as $i (null; try ($x as {a:$q} | .))) |= 9",
            "",
            "Invalid path expression with result null",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"try path(. as $x | foreach 1 as $i ([]; try ($x as {a:$q} | .))) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"path(. as $x | foreach (1,2) as $i ([]; try ($x as {a:$q} | .)))",
            "",
            "Invalid path expression with result []",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"del(. as $x | foreach 1 as $i (0; ($x as {a:$q} | .)?; .))",
            "",
            "Invalid path expression with result 0",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"try path(. as $x | foreach 1 as $i (0; ($x as {a:$q} | .)?; .)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r"[path(. as $x | foreach 1 as $i ({}; try ($x as {b:$q} | .); .))]",
            "",
            "Invalid path expression with result {}",
            5,
        ),
        (
            r#"{"a":0,"b":false}"#,
            r#"try path(. as $x | foreach (1,2) as $i ({}; ($x as {a:$q} | .)?; .)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
    ])
}

/// #5120: rows that stay wrong, pinned to jq's answers and ignored until they are fixed: the `{}` document's `= 9`
/// (jq writes the path the bind moved the register to; the binary refuses), and the `null` markers, whose
/// `null`/boolean EXTRACT is accepted at the register the bind moved to (#5121). On `main` they were a silent no-op or
/// an empty `path` (the `{}` row's `= 9` left the document where jq writes); the binary now refuses them loudly. The
/// helper stops at the first failing row, so check all four when this is fixed.
#[test]
#[ignore = "#5121"]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5120_known_gaps_after_the_register_is_handed_on() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r"{}",
            r"(.a as $x | foreach (.a,.) as $i (.; try ($x as {a:{a:$q}} | .); null)) = 9",
            "{\"a\":{\"a\":{\"a\":9}}}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(.a as $x | foreach .a as $i ({}; try ($x as {b:$q} | .); null))",
            "[\"a\",\"b\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(.a.a as $x | foreach .a? as $i (.; try ($x as {b:$q} | .); null))]",
            "[[\"a\",\"b\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null,"b":null}"#,
            r"(.b as $x | foreach .a as $i (.; ($x as [$q] | .)?; $i)) |= 9",
            "{\"a\":[9],\"b\":null}\n",
            "",
            0,
        ),
    ])
}
