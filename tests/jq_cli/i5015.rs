//! #5015: an EXTRACT that navigates, or a `first`/`limit` over a `null`, after the bare caught-bind `foreach` UPDATE of
//! #4957.

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

// After the inner bind passes, jq's register sits on the bound child and the EXTRACT runs from there: `.a?` on a `null`
// state navigates (a `null` state is the `null` register by kind), `(.. | select(. == null))` emits the state, and
// `first(null)` is the register's node. #5014 judged only an EXTRACT that cannot move the register that way, so these
// kept the refusal a `try` does not catch, or answered a path-less `"C"` where jq names a path.

/// #5015: navigating, `//`, `first(..)`, `not`-after-navigation and comparison EXTRACTs, the bare, `try` and `?` UPDATE
/// spellings, the `$x`, `.` and array-pattern binds, sources that leave the register on the root or move it, in
/// `path`, `[path(..)?]`, `try .. catch`, `del`, `=` and `|=`. Every row captured from `/usr/bin/jq` 1.7.1 (stdout,
/// first stderr line up to the container, exit code), both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5015_navigating_extract_after_a_bare_caught_bind_foreach_update() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r"[1]",
            r"[del(. as $x | foreach ($x) as $i (.; foreach ($x as {a:$a} | .) as $y (.; .; .); first(.a?, null)))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":1,"b":[2]}}"#,
            r"path(. as $x | foreach (.,.a) as $i (.; try (foreach (try ($x as {b:$a} | .)) as $y (.; .; .)); null, .a?))",
            "[\"b\"]\n",
            r#"Invalid path expression near attempt to access element "a""#,
            5,
        ),
        (
            r"{}",
            r"[path(. as $x | foreach (1,2) as $i (0; foreach (($x as {a:$a} | .)?) as $y (.; .; .); try (.a | not) catch null))]",
            "[[\"a\"],[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"b":null}"#,
            r#"try path(. as $x | foreach (..) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; .)); (.a? | not))) catch "C""#,
            "\"C\"\n",
            "",
            0,
        ),
        (
            r"[]",
            r"(. as $x | foreach (.[]?,.) as $i (.; (foreach (try ($x as [$a] | .)) as $y (.; .; .))?; first(null))) |= 5",
            "[5]\n",
            "",
            0,
        ),
        (
            r"[null]",
            r"[limit(2; path(. as $x | foreach (first(.,.a)) as $i (.; foreach (try ($x as [$a] | .)) as $y (.; .; .); first(null))))]",
            "[[0]]\n",
            "",
            0,
        ),
        (
            r"null",
            r"first(path(. as $x | foreach (.) as $i (0; (foreach ($x as {a:$a} | .) as $y (.; .; .))?; null, .a?)))",
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r"[false]",
            r#"try first(path(. as $x | foreach ($x) as $i (.; foreach ($x as {a:$a} | .) as $y (.; .; .); (.a? | not)))) catch "C""#,
            "\"C\"\n",
            "",
            0,
        ),
        (
            r"true",
            r"[del(. as $x | foreach (.) as $i (.; try (foreach ($x as {a:$a} | .) as $y (.; .; .)); (.. | select(. == null))))?]",
            "[true]\n",
            "",
            0,
        ),
        (
            r"[[1],[2]]",
            r"path(. as $x | foreach (.) as $i (.; foreach (try ($x as [$a] | .)) as $y (.; .; .); (.a | not)?))",
            "",
            "",
            0,
        ),
        (
            r"false",
            r"[limit(2; path(. as $x | foreach ((1,.)) as $i (.; (foreach ($x as {a:$a} | .) as $y (.; .; .))?; (.a?, null))))]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":1}"#,
            r"[path(. as $x | foreach (.,.,.a) as $i (.; foreach (try (. as {a:$a} | .)) as $y (.; .; .); (.a? | not)))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r"true",
            r"(. as $x | foreach ((.,1)) as $i (true; try (foreach ($x as {a:$a} | .) as $y (.; .; .)); null, .a?)) |= 5",
            "true\n",
            "",
            0,
        ),
        (
            r"[]",
            r"first(path(. as $x | foreach ($x) as $i (.; foreach (try ($x as [$a] | .)) as $y (.; .; .); (.a | not)?)))",
            "",
            "",
            0,
        ),
        (
            r"null",
            r"path(. as $x | foreach (1,2) as $i (0; foreach ($x as {a:$a} | .) as $y (.; .; .); first(null)))",
            "[\"a\"]\n[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"b":null}"#,
            r#"try del(. as $x | foreach ($x) as $i (.; (foreach ($x as {a:$a} | .) as $y (.; .; .))?; (.a // null))) catch "C""#,
            "\"C\"\n",
            "",
            0,
        ),
        (
            r"{}",
            r"[path(. as $x | foreach (null) as $i (.; try (foreach (try ($x as {b:$a} | .)) as $y (.; .; .)); null, .a?))?]",
            "[[\"b\"]]\n",
            "",
            0,
        ),
        (
            r"[[1],[2]]",
            r"[del(. as $x | foreach ($x,.a) as $i (.; foreach (try ($x as [$a] | .)) as $y (.; .; .); (.a?, 1) | . == null))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r"[false]",
            r"(. as $x | foreach (first(.,.a)) as $i (.; try (foreach ($x as {a:$a} | .) as $y (.; .; .)); (.a?, 1) | not)) |= 5",
            "[false]\n",
            "",
            0,
        ),
        (
            r"[1]",
            r"first(path(. as $x | foreach ((1,.)) as $i (.; try (foreach ($x as {a:$a} | .) as $y (.; .; .)); (.a.a? // false))))",
            "",
            "",
            0,
        ),
        (
            r"[null]",
            r"[limit(2; path(. as $x | foreach ([]) as $i (null; try (foreach (try ($x as [$a] | .)) as $y (.; .; .)); null, .a?)))]",
            "[[0],[0,\"a\"]]\n",
            "",
            0,
        ),
        (
            r"false",
            r"[del(. as $x | foreach ($x) as $i (.; foreach ($x as {a:$a} | .) as $y (.; .; .); (.[0]?, true)))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":[1,2]}"#,
            r#"try del(. as $x | foreach (.,.,.a) as $i (.; ((foreach (try (. as {a:$a} | .)) as $y (.; .; .))); (.a? // false))) catch "C""#,
            "\"C\"\n",
            "",
            0,
        ),
        (
            r"[1]",
            r"[path(. as $x | foreach (true) as $i (.; foreach ($x as {a:$a} | .) as $y (.; .; .); first(null)))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r"{}",
            r"(. as $x | foreach ((.,1)) as $i (.; foreach (try (. as {a:$a} | .)) as $y (.; .; .); (.a | not)?)) = 5",
            "{}\n",
            "",
            0,
        ),
        (
            r#"{"b":null}"#,
            r"path(. as $x | foreach ($x,.a) as $i (true; foreach ($x as {a:$a} | .) as $y (.; .; .); null, .a?))",
            "[\"a\"]\n",
            r#"Invalid path expression near attempt to access element "a""#,
            5,
        ),
        (
            r"[]",
            r#"try path(. as $x | foreach ($x) as $i (.; try (foreach (try ($x as [$a] | .)) as $y (.; .; .)); (.a?, null))) catch "C""#,
            "\"C\"\n",
            "",
            0,
        ),
        (
            r"[null]",
            r"[path(. as $x | foreach (..) as $i (.; try (foreach (try ($x as [$a] | .)) as $y (.; .; .)); (.a | not)?))]",
            "[]\n",
            "",
            0,
        ),
        (
            r"[false]",
            r"[del(. as $x | foreach (.,.,.a) as $i (.; try (foreach ($x as {a:$a} | .) as $y (.; .; .)); (.a?, null)))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r"[[1],[2]]",
            r"(. as $x | foreach (..) as $i (.; (foreach ($x as {a:$a} | .) as $y (.; .; .))?; .a?, null)) = 5",
            "[[1],[2]]\n",
            "",
            0,
        ),
        (
            r"null",
            r"[path(. as $x | foreach ((1,.)) as $i (true; foreach ($x as {a:$a} | .) as $y (.; .; .); first(null)))]",
            "[[\"a\"],[\"a\"]]\n",
            "",
            0,
        ),
        (
            r"{}",
            r"first(path(. as $x | foreach ((1,.)) as $i (null; (foreach (try ($x as {a:$a} | .)) as $y (.; .; .))?; .a?, null)))",
            "[\"a\",\"a\"]\n",
            "",
            0,
        ),
    ])
}
