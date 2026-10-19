//! #4899: yq's `contains` looks inside an array on its left for a right-hand side that is not an
//! array, at any depth, where an object on the left never yields a scalar.

use anyhow::Result;

use crate::run_yq_stdin;

/// Every row is captured from yq v4.53.3 (`yq -n -o=json -I0`). The nested-array and
/// object-holding-an-array rows used to answer `false`; the rest are controls that must keep
/// their answer (an object element yields no scalar, an array on the right still pairs
/// element by element, and a number keeps its spelling, #4886).
#[test]
fn test_4899_contains_looks_inside_a_nested_array() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        (
            "[[1]] | contains([1])",
            "true
",
        ),
        (
            "[[1,2]] | contains([1])",
            "true
",
        ),
        (
            "[[1,2]] | contains([1,2])",
            "true
",
        ),
        (
            "[[1,2]] | contains([[1]])",
            "true
",
        ),
        (
            "[[[1]]] | contains([1])",
            "true
",
        ),
        (
            "[[1],[2]] | contains([2])",
            "true
",
        ),
        (
            "[[1],[2]] | contains([1,2])",
            "true
",
        ),
        (
            "[[1]] | contains([1,1])",
            "true
",
        ),
        (
            "[[\"ab\"]] | contains([\"a\"])",
            "true
",
        ),
        (
            "[[[\"ab\"]]] | contains([\"a\"])",
            "true
",
        ),
        (
            "[[[\"ab\"]]] | contains([[\"a\"]])",
            "true
",
        ),
        (
            "[[true]] | contains([true])",
            "true
",
        ),
        (
            "[[null]] | contains([null])",
            "true
",
        ),
        (
            "[[{\"a\":1}]] | contains([{\"a\":1}])",
            "true
",
        ),
        (
            "[{\"a\":[1]}] | contains([{\"a\":1}])",
            "true
",
        ),
        (
            "{\"a\":[1]} | contains({\"a\":1})",
            "true
",
        ),
        (
            "{\"a\":[1,2]} | contains({\"a\":2})",
            "true
",
        ),
        (
            "{\"a\":[{\"b\":1}]} | contains({\"a\":{\"b\":1}})",
            "true
",
        ),
        (
            "{\"a\":[[1]]} | contains({\"a\":1})",
            "true
",
        ),
        (
            "{\"a\":{\"b\":[1]}} | contains({\"a\":{\"b\":1}})",
            "true
",
        ),
        (
            "[[1,\"a\"]] | contains([\"a\"])",
            "true
",
        ),
        (
            "[1,[2]] | contains([2])",
            "true
",
        ),
        (
            "[[1,2],3] | contains([1])",
            "true
",
        ),
        (
            "{\"a\":[1.50]} | contains({\"a\":1.5})",
            "false
",
        ),
        (
            "[[1.50]] | contains([1.5])",
            "false
",
        ),
        (
            "[[1.0]] | contains([1])",
            "false
",
        ),
        (
            "[[]] | contains([1])",
            "false
",
        ),
        (
            "[[1]] | contains([])",
            "true
",
        ),
        (
            "[[1]] | contains([[]])",
            "true
",
        ),
        (
            "[{\"a\":1}] | contains([1])",
            "false
",
        ),
        (
            "[{\"a\":[1]}] | contains([1])",
            "false
",
        ),
        (
            "[[1]] | contains([\"1\"])",
            "false
",
        ),
        (
            "[[\"1\"]] | contains([1])",
            "false
",
        ),
        (
            "[[null]] | contains([null, 1])",
            "false
",
        ),
        (
            "[[1]] | contains([1, 2])",
            "false
",
        ),
        (
            "[1] | contains([[1]])",
            "false
",
        ),
        (
            "[[1]] | contains([{\"a\":1}])",
            "false
",
        ),
        (
            "{\"a\":{\"b\":1}} | contains({\"a\":1})",
            "false
",
        ),
        (
            "{\"a\":1} | contains({\"a\":[1]})",
            "false
",
        ),
        ("[[[[[[1]]]]]] | contains([1])", "true\n"),
        ("[[[[[[1]]]]]] | contains([2])", "false\n"),
        (
            "{\"x\":[[{\"a\":1}]]} | contains({\"x\":{\"a\":1}})",
            "true\n",
        ),
        ("[[[{\"a\":[2]}]]] | contains([{\"a\":2}])", "true\n"),
    ];
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, "null", &["-n", "-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(out, *want, "{filter}");
    }
    Ok(())
}
