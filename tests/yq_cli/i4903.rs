//! #4903: a text argument that yields no output (an absent key, `select(false)`) is read by yq as
//! the empty string, so `split`, `test`, `match`, `capture` and `sub` still run, where jq's
//! fan-out gives no output.

use anyhow::Result;

use crate::run_yq_stdin;

/// Every row is captured from yq v4.53.3 (`yq -n -o=json -I0`). `.nan`/`.inf` are field accesses
/// on a string, so they yield nothing too (#4374). The controls at the end keep their answer: a
/// `null` separator is its text, `has` already treated the missing key as `null`, and the
/// first of several outputs is still the only one used.
#[test]
fn test_4903_an_argument_with_no_output_is_the_empty_string() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        ("\"a1b\" | split(.foo)", "[\"a\",\"1\",\"b\"]\n"),
        ("{\"x\":\"a1b\"} | .x | split(.foo)", "[\"a\",\"1\",\"b\"]\n"),
        ("{\"x\":\"a1b\"} | .x | split(.x.y)", "[\"a\",\"1\",\"b\"]\n"),
        ("{\"x\":\"a1b\"} | .x | split(select(false))", "[\"a\",\"1\",\"b\"]\n"),
        ("{\"x\":\"a1b\"} | .x | split(.nope.deeper)", "[\"a\",\"1\",\"b\"]\n"),
        ("\"a1b\" | split(.nan)", "[\"a\",\"1\",\"b\"]\n"),
        ("\"a1b\" | split(.inf)", "[\"a\",\"1\",\"b\"]\n"),
        ("\"a.nanb\" | split(.nan)", "[\"a\",\".\",\"n\",\"a\",\"n\",\"b\"]\n"),
        ("\"ab\" | split(.nan)", "[\"a\",\"b\"]\n"),
        ("\"a1b\" | split(null)", "[\"a1b\"]\n"),
        ("\"a1b\" | split(.foo; \"g\")", "[\"a\",\"1\",\"b\"]\n"),
        ("\"a1b\" | test(.foo)", "true\n"),
        ("\"a1b\" | test(.foo; \"g\")", "true\n"),
        ("\"a1b\" | test(\"a\"; .foo)", "true\n"),
        ("\"a1b\" | [match(.foo)] | length", "1\n"),
        ("\"a1b\" | match(.foo)", "{\"string\":\"\",\"offset\":0,\"length\":0,\"captures\":[]}\n"),
        ("\"a1b\" | match(.foo; \"g\")", "{\"string\":\"\",\"offset\":0,\"length\":0,\"captures\":[]}\n{\"string\":\"\",\"offset\":1,\"length\":0,\"captures\":[]}\n{\"string\":\"\",\"offset\":2,\"length\":0,\"captures\":[]}\n{\"string\":\"\",\"offset\":3,\"length\":0,\"captures\":[]}\n"),
        ("\"a1b\" | [match(\"\";\"g\")] | length", "4\n"),
        ("\"a1b\" | match(\"a\"; .foo)", "{\"string\":\"a\",\"offset\":0,\"length\":1,\"captures\":[]}\n"),
        ("\"a1b\" | capture(.foo)", "{}\n"),
        ("\"a1b\" | capture(.foo; \"g\")", "{}\n{}\n{}\n{}\n"),
        ("\"a1b\" | capture(\"(?P<x>a)\"; .foo)", "{\"x\":\"a\"}\n"),
        ("\"a1b\" | sub(.foo; \"X\")", "\"XaX1XbX\"\n"),
        ("\"a1b\" | sub(\"\";\"X\")", "\"XaX1XbX\"\n"),
        ("\"a1b\" | sub(\"a\"; select(false))", "\"1b\"\n"),
        ("\"abab\" | sub(\"a\"; select(false))", "\"bb\"\n"),
        ("\"abab\" | sub(\"a\"; select(false); \"g\")", "\"bb\"\n"),
        ("\"a1b\" | sub(\"a\";\"X\";.foo)", "\"1b\"\n"),
        ("\"a1b\" | sub(.foo;\"X\";\"g\")", "\"a1b\"\n"),
        ("\"a1b\" | sub(\"a\";.foo;\"g\")", "\"1b\"\n"),
        ("\"a1b\" | test((\"a\",\"b\"))", "true\n"),
        ("\"a1b\" | split(\",\", \";\")", "[\"a1b\"]\n"),
        ("{\"a\":1} | has(.foo)", "false\n"),
    ];
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, "null", &["-n", "-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(out, *want, "{filter}");
    }
    Ok(())
}
