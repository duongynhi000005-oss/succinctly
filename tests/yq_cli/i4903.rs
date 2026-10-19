//! #4903: a `split`/`test`/`match`/`capture`/`sub` argument that yields no output is read as an
//! empty value in yq (the call still runs), where jq's `f(empty)` is `empty`.

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

/// Every row is captured from yq v4.53.3 (`yq -n -o=json -I0`). An absent key (`.foo` on a
/// string), `select(false)` and `.nan` (a field access on a string) all produce nothing, so the
/// argument is the empty string: `split` cuts into characters, `test` is true, `match` is one
/// empty match at offset 0.
#[test]
fn test_4903_yq_argument_with_no_output_is_the_empty_string() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        (r#""a1b" | split(.foo)"#, "[\"a\",\"1\",\"b\"]\n"),
        (r#""a1b" | split(select(false))"#, "[\"a\",\"1\",\"b\"]\n"),
        (r#""a1b" | split(.nan)"#, "[\"a\",\"1\",\"b\"]\n"),
        (r#""" | split(.foo)"#, "[]\n"),
        (r#""a1b" | test(.foo)"#, "true\n"),
        (r#""a1b" | [match(.foo)] | length"#, "1\n"),
        (
            r#""a1b" | match(.foo)"#,
            "{\"string\":\"\",\"offset\":0,\"length\":0,\"captures\":[]}\n",
        ),
        (r#""a1b" | capture(.foo)"#, "{}\n"),
        (r#""a1b" | sub(.foo; "X")"#, "\"XaX1XbX\"\n"),
        // The flags slot of the two-argument forms is blank too: no flags.
        (r#""a1b" | test("a"; .foo)"#, "true\n"),
        (r#""a1b" | [match("1"; .foo)] | length"#, "1\n"),
        (r#""a1b" | capture("(?<x>a)"; .foo)"#, "{\"x\":\"a\"}\n"),
        (r#""a1b" | test(.foo; "g")"#, "true\n"),
        (r#""a1b" | test(("a","b"); .foo)"#, "true\n"),
        (r#""a1b" | capture(.foo; "g")"#, "{}\n{}\n{}\n{}\n"),
        // sub/3 reads its pattern only, so an empty pattern replaces nothing with nothing.
        (r#""a1b" | sub(.foo; "X"; "g")"#, "\"a1b\"\n"),
        (r#""a1b" | sub(.foo; "X"; .foo)"#, "\"a1b\"\n"),
        // Controls that already matched and must keep matching: a present argument is untouched,
        // and a multi-output argument still takes its first output.
        (r#""a1b" | split(null)"#, "[\"a1b\"]\n"),
        (r#""a1b" | [split(.foo, "1")]"#, "[[\"a\",\"b\"]]\n"),
        (r#""a1b" | [test(.foo, "a")]"#, "[true]\n"),
        (r#""a1b" | has(.foo)"#, "false\n"),
    ];
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, "null", &["-n", "-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(out, *want, "{filter}");
    }
    Ok(())
}

/// The substitution is for an argument that *produced nothing*: one that raised keeps raising
/// (yq prints the error alone), and a non-string input keeps its own type error.
#[test]
fn test_4903_yq_escaping_argument_still_raises() -> Result<()> {
    for (filter, message) in [
        (r#""a1b" | split(("x", error("boom")))"#, "boom"),
        (r#""a1b" | test(error("p"); .foo)"#, "p"),
        (r#""a1b" | test(.foo; error("boom"))"#, "boom"),
        (r#""a1b" | [match(("a", error("p")))]"#, "p"),
        (
            r"1 | split(.foo)",
            "cannot split !!int, can only split strings",
        ),
    ] {
        let (out, err, code) =
            run_yq_stdin_with_stderr(filter, "null", &["-n", "-o=json", "-I=0"])?;
        assert_eq!(code, 1, "{filter}");
        assert_eq!(out, "", "{filter}");
        assert!(err.contains(message), "{filter}: {err}");
    }
    Ok(())
}
