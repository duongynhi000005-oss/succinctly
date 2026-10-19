//! #4886: yq's `contains` pairs two scalars by tag and text (`recursiveNodeEqual`), as array
//! subtraction does, so a number keeps the spelling it was written with.

use anyhow::Result;

use crate::run_yq_stdin;

/// Every row is captured from yq v4.53.3 (`yq -n -o=json -I0`). The `false` rows are the ones
/// that used to answer `true` by value (`1.50` against `1.5`); the `true` rows are controls that
/// must keep matching, including a string's substring rule inside an array.
#[test]
fn test_4886_contains_compares_numbers_by_tag_and_text() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        ("[1.5] | contains([1.50])", "false\n"),
        ("[1.50] | contains([1.50])", "true\n"),
        ("[[1.50]] | contains([[1.5]])", "false\n"),
        ("[{\"a\":1.50}] | contains([{\"a\":1.5}])", "false\n"),
        ("{\"a\":1.50} | contains({\"a\":1.5})", "false\n"),
        ("{\"a\":1.50} | contains({\"a\":1.50})", "true\n"),
        ("1.50 | contains(1.5)", "false\n"),
        ("1.5 | contains(1.50)", "false\n"),
        ("5 | contains(5)", "true\n"),
        ("5 | contains(5.0)", "false\n"),
        ("5.0 | contains(5)", "false\n"),
        ("1e3 | contains(1000)", "false\n"),
        ("[1e3] | contains([1000])", "false\n"),
        ("[1] | contains([1.0])", "false\n"),
        ("[0.1,2] | contains([0.10])", "false\n"),
        ("[1.5] | contains([1.5, 1.50])", "false\n"),
        ("{\"a\":[1.0]} | contains({\"a\":[1]})", "false\n"),
        ("[1.5, 2] | contains([1.5])", "true\n"),
        ("[\"a1\"] | contains([\"1\"])", "true\n"),
        ("[1] | contains([\"1\"])", "false\n"),
        ("[1+0.5] | contains([1.5])", "true\n"),
        ("[1.5] | contains([1+0.5])", "true\n"),
        ("[3/2] | contains([1.5])", "true\n"),
        ("[2.0*1] | contains([2])", "false\n"),
        ("[{\"a\":1}] | contains([[1]])", "false\n"),
        ("[1] | contains([[1]])", "false\n"),
        ("{\"a\":1.5} | contains({\"b\":1.5})", "false\n"),
        ("[true] | contains([true])", "true\n"),
        ("[null] | contains([null])", "true\n"),
    ];
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, "null", &["-n", "-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(out, *want, "{filter}");
    }
    Ok(())
}
