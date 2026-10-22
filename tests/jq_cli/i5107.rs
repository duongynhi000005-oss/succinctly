//! #5107: a navigation stage ahead of a bare nested \`foreach\` that destructures the register \`$x\`, in a \`foreach\` UPDATE.

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

// \`.b as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null)\`: the \`.b\` stage moves jq's
// register to \`["b"]\`, where \`$x\` is bound, so the nested \`foreach\`'s bind passes and steps it to \`["b","a"]\`; EXTRACT is
// judged there. \`#5102\` made the nested \`foreach\` empty where \`$x\` is not the register; here it is, and the emission was
// judged at \`["b"]\`, so the refusal was the resolver's guess and \`[path(..)?]\` could not catch it (\`[]\` in jq). The
// bare nested \`foreach\` is restated at the register its bind moved to since #4957, and behind a literal stage since #5110;
// behind field navigations the restatement starts at the child they reach, where every \`$x\` the UPDATE reads must equal
// that child (an absent one, or a marker bound elsewhere, keeps the reading it had).

/// #5107: the issue's rows (\`[path(..)?]\`, \`path\`, \`|= 9\`), \`del\`, the controls, and rows of a differential grid
/// (document x bind site x head navigation x nested \`foreach\` x wrapper x source x INIT x EXTRACT x context) that \`main\`
/// refused or answered wrongly. Every row captured from \`/usr/bin/jq\` 1.7.1 (stdout, exit code, message), both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5107_navigation_stage_ahead_of_a_bare_bind_foreach_in_a_foreach_update() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":null,"b":{"a":1}}"#,
            r"[path(.b as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null,"b":{"a":1}}"#,
            r"path(.b as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))",
            "",
            "Invalid path expression with result null",
            5,
        ),
        (
            r#"{"a":null,"b":{"a":1}}"#,
            r"(.b as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null)) |= 9",
            "",
            "Invalid path expression with result null",
            5,
        ),
        (
            r#"{"a":null,"b":{"a":1}}"#,
            r"del(.b as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))",
            "",
            "Invalid path expression with result null",
            5,
        ),
        (
            r#"{"a":null,"b":{"a":1}}"#,
            r"[path(.b as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); $i))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"(. as $x | foreach . as $i (.; .b | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null)) |= 9",
            "{\"a\":null}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"(.a as $x | foreach . as $i (.; .a | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null)) |= 9",
            "{\"a\":{\"a\":9}}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"path(.a as $x | foreach . as $i (.; .a | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); null))",
            "[\"a\",\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null},"b":{"a":{"a":2}}}"#,
            r#"try path(.a as $x | foreach . as $i (.; try (.a | foreach (($x as {a:$a} | .)?) as $y (.; .; .)); .a?)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":null,"b":{"a":1}}"#,
            r"path(.a as $x | foreach .a as $i (null; (.b.a | foreach (try ($x as [$a] | .)) as $y (.; .; .)), 1; $i))",
            "[\"a\",\"b\",\"a\",0]\n[\"a\"]\n",
            "",
            0,
        ),
        (
            r"null",
            r"[path(.b as $x | foreach (.,.) as $i (.; (.a | foreach (($x as {b:$a} | .)) as $y (.; .; .)), 1; null))?]",
            "[[\"a\",\"b\"],[]]\n",
            "",
            0,
        ),
        (
            r"null",
            r"[path(. as $x | foreach . as $i (.; (.b.a | foreach (($x as {a:$a} | .)?) as $y (.; .; .)), 1; null))?]",
            "[[\"b\",\"a\",\"a\"],[]]\n",
            "",
            0,
        ),
        (
            r"{}",
            r#"try path(.b as $x | foreach .a as $i (null; (.c | foreach (try ($x as {a:$a} | .)) as $y (.; .; .)), 1; $i)) catch "c""#,
            "[\"a\",\"c\",\"a\"]\n[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"b":{"a":1}}"#,
            r"[path(.b as $x | foreach . as $i (.; (.b | foreach (($x as {a:$a} | .)?) as $y (.; .; .)), 1; 1))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r"null",
            r#"try path(.b.a as $x | foreach .a as $i (.; (.b | foreach (($x as {a:$a} | .)?) as $y (.; .; .)), 1; null)) catch "c""#,
            "[\"a\",\"b\",\"a\"]\n[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null},"b":{"a":{"a":2}}}"#,
            r#"try path(.b.a as $x | foreach (.,.) as $i (.; try (.b.a | foreach (($x as {a:$a} | .)?) as $y (.; .; .)); 1)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[path(.b as $x | foreach .[]? as $i (null; (.b.a | foreach (($x as {b:$a} | .)) as $y (.; .; .)), 1; null))]",
            "[[\"a\",\"b\",\"a\",\"b\"],[\"a\"]]\n",
            "",
            0,
        ),
        (
            r"null",
            r"(.b.a as $x | foreach . as $i (null; (.a.a | foreach (try ($x as {a:$a} | .)) as $y (.; .; .)), 1; $i)) |= 9",
            "9\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null},"b":{"a":{"a":2}}}"#,
            r#"try path(.b as $x | foreach (1,2) as $i (.; (.b | foreach (($x as {a:$a} | .)?) as $y (.; .; .))?; $i)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"b":{"a":1}}"#,
            r"[path(.b.a as $x | foreach (1,2) as $i (.; .b.a | foreach (($x as {b:$a} | .)) as $y (.; .; .); null))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null},"b":{"a":{"a":2}}}"#,
            r"[path(.b.a as $x | foreach (.,.) as $i (.; (.b.a | foreach (($x as {a:$a} | .)?) as $y (.; .; .))?; null))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null},"b":{"a":{"a":2}}}"#,
            r"[path(.b as $x | foreach (1,2) as $i (.; (.b | foreach (try ($x as {b:$a} | .)) as $y (.; .; .))?; null))]",
            "[[\"b\",\"b\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null},"b":{"a":{"a":2}}}"#,
            r#"try path(.b.a as $x | foreach (1,2) as $i (.; (.b.a | foreach (($x as {a:$a} | .)) as $y (.; .; .)), 1; 1)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"[path(.b as $x | foreach . as $i (.; (.c | foreach (try ($x as [$a] | .)) as $y (.; .; .)), 1; $i))]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r"null",
            r"[path(.a as $x | foreach .a as $i (null; (.b | foreach (try ($x as {b:$a} | .)) as $y (.; .; .)), 1; null))?]",
            "[[\"a\",\"b\",\"b\"],[\"a\"]]\n",
            "",
            0,
        ),
        (
            r"null",
            r"[path(. as $x | foreach (1,2) as $i (.; (.b | foreach (($x as {a:$a} | .)?) as $y (.; .; .)), 1; null))?]",
            "[[\"b\",\"a\"],[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"try path(.b.a as $x | foreach .a as $i (null; (.b | foreach (($x as {a:$a} | .)?) as $y (.; .; .)), 1; null)) catch "c""#,
            "[\"a\",\"b\",\"a\"]\n[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"[path(.a as $x | foreach (.,.) as $i (.; (.c | foreach (($x as {a:$a} | .)?) as $y (.; .; .)), 1; $i))?]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null,"b":{"a":1}}"#,
            r#"try path(.b.a as $x | foreach 1 as $i (.; (.b.a | foreach (($x as {b:$a} | .)) as $y (.; .; .))?; 1)) catch "c""#,
            "",
            "",
            0,
        ),
        (
            r#"{"a":null,"b":{"a":1}}"#,
            r"[path(. as $x | foreach .a as $i (null; (.b | foreach (try ($x as {b:$a} | .)) as $y (.; .; .)), 1; $i))?]",
            "[[\"a\"]]\n",
            "",
            0,
        ),
        (
            r"{}",
            r"[path(.b.a as $x | foreach .a as $i (null; (.b | foreach (try ($x as [$a] | .)) as $y (.; .; .)), 1; null))?]",
            "[[\"a\",\"b\",0],[\"a\"]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null},"b":{"a":{"a":2}}}"#,
            r#"try path(.b as $x | foreach (.,.) as $i (.; .b | foreach (($x as {a:$a} | .)?) as $y (.; .; .); true)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r"null",
            r#"try path(.a.a as $x | foreach .a as $i (null; (.c | foreach (try ($x as [$a] | .)) as $y (.; .; .)), 1; $i)) catch "c""#,
            "[\"a\",\"c\",0]\n[\"a\"]\n",
            "",
            0,
        ),
        (
            r"null",
            r"(.a as $x | foreach 1 as $i (.; (.a | foreach (try ($x as {a:$a} | .)) as $y (.; .; .)), 1; null)) |= 9",
            "9\n",
            "",
            0,
        ),
        (
            r"null",
            r"path(.a.a as $x | foreach . as $i (.; (.c | foreach (($x as {a:$a} | .)?) as $y (.; .; .)), 1; null))",
            "[\"c\",\"a\"]\n[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1},"b":2}"#,
            r"[path(.a as $x | foreach (.,.) as $i (.; .a | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); 1))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null},"b":{"a":{"a":2}}}"#,
            r"[path(.b.a as $x | foreach 1 as $i (.; .b.a | foreach (try ($x as {a:$a} | .)) as $y (.; .; .); $i))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"b":{"a":1}}"#,
            r#"try path(.b.a as $x | foreach . as $i (.; (.c | foreach (try ($x as [$a] | .)) as $y (.; .; .)), 1; $i)) catch "c""#,
            "[]\n",
            "",
            0,
        ),
    ])
}
