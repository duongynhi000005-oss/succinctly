//! #5102: a navigation stage ahead of a caught foreign-`$x` `foreach` in a `foreach` UPDATE. The converse, where `$x` is
//! the register the stage navigated to, is #5107's.

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

// `FE = foreach (try ($x as {a:$a} | .)) as $y (.; .; .)`, `Q = . as $x | foreach . as $i (.; U; null)`. A stage ahead
// of FE that navigates (`.b | FE`) moves jq's register off the root `$x` is bound at, so FE's destructure of `$x` raises
// its path error, the `try` swallows it, and FE's source is empty: the UPDATE emits nothing, the fold has no output, and
// `path(Q)` is empty, `Q |= 9` leaves the document. The binary read FE as the sole UPDATE only, so with a stage
// ahead of it the source was driven by value: `["b"]` came back as a path (a silent write of `9` at `.b` under `|=`).

/// #5102: the issue's rows on `{"a":null}` (a head stage that lands on `null`): `.b`, `.a`, `.c.d`, `.[0]?`, FE
/// piped into `.`, FE behind a `try` and an `if`, in `|=`, `[path(..)?]`, `path` and `del`; the `[$a]` pattern and the
/// `?` spelling; and the heads that land on a container, which used to be refused loudly. Every row captured from
/// `/usr/bin/jq` 1.7.1 (stdout, exit code), on both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5102_navigation_stage_ahead_of_a_caught_foreign_foreach_in_a_foreach_update() -> Result<()>
{
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null)) |= 9",
            "{\"a\":null}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))",
            "",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach . as $i (.; .a | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null)) |= 9",
            "{\"a\":null}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach . as $i (.; .a | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach . as $i (.; .a | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))",
            "",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach . as $i (.; .c.d | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null)) |= 9",
            "{\"a\":null}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach . as $i (.; .c.d | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach . as $i (.; .c.d | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))",
            "",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach . as $i (.; .[0]? | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null)) |= 9",
            "{\"a\":null}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach . as $i (.; .[0]? | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach . as $i (.; .[0]? | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))",
            "",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .) | .; null)) |= 9",
            "{\"a\":null}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .) | .; null))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .) | .; null))",
            "",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach . as $i (.; try (.b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) catch 1; null)) |= 9",
            "{\"a\":null}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach . as $i (.; try (.b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) catch 1; null))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach . as $i (.; try (.b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) catch 1; null))",
            "",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach . as $i (.; if . then (.b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else 3 end; null)) |= 9",
            "{\"a\":null}\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(. as $x | foreach . as $i (.; if . then (.b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else 3 end; null))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"path(. as $x | foreach . as $i (.; if . then (.b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) else 3 end; null))",
            "",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"del(. as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))",
            "{\"a\":null}\n",
            "",
            0,
        ),
        (
            r#"{"a":[1]}"#,
            r"(. as $x | foreach . as $i (.; .b | foreach (($x as [$a] | .)?) as $y (.; .; .); null)) |= 9",
            "{\"a\":[1]}\n",
            "",
            0,
        ),
        (
            r#"{"a":[1]}"#,
            r"[path(. as $x | foreach . as $i (.; .b | foreach (($x as [$a] | .)?) as $y (.; .; .); null))]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null,"b":{"c":1}}"#,
            r"[path(. as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null,"b":{"c":1}}"#,
            r"(. as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null)) |= 9",
            "{\"a\":null,\"b\":{\"c\":1}}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"z":1},"b":[1]}"#,
            r"[path(. as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"z":1},"b":[1]}"#,
            r"(. as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null)) |= 9",
            "{\"a\":{\"z\":1},\"b\":[1]}\n",
            "",
            0,
        ),
    ])
}
