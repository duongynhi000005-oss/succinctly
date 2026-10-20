//! #4185: a `key: value` pair inside parentheses in a `{...}` entry position. In yq `:` is an
//! ordinary binary operator (`CREATE_MAP`), so `{"x": 1, ("a": 2)}` is `{"x":1,"a":2}`; the
//! parenthesised pairs are flattened into the object's own entries, in order.
//!
//! Every output is captured from yq v4.53.3.

use anyhow::Result;

use crate::run_yq_stdin;

const INPUT: &str = "{\"a\":5,\"b\":6}";

/// (filter, `yq -o=json -I0` output).
const ROWS: &[(&str, &str)] = &[
    (r#"{"x": 1, ("a": 2)}"#, "{\"x\":1,\"a\":2}\n"),
    (r#"{("a": 2), "x": 1}"#, "{\"a\":2,\"x\":1}\n"),
    (r#"{("a": 2)}"#, "{\"a\":2}\n"),
    (r#"{("a": 2, "b": 3)}"#, "{\"a\":2,\"b\":3}\n"),
    (
        r#"{"x": 1, ("a": 2, "b": 3)}"#,
        "{\"x\":1,\"a\":2,\"b\":3}\n",
    ),
    (
        r#"{"x": 1, ("a": 2), ("b": 3)}"#,
        "{\"x\":1,\"a\":2,\"b\":3}\n",
    ),
    (r#"{"x": 1, (("a": 2))}"#, "{\"x\":1,\"a\":2}\n"),
    (
        r#"{"x": 1, (("a": 2), "b": 3)}"#,
        "{\"x\":1,\"a\":2,\"b\":3}\n",
    ),
    (
        r#"{("a": 2, ("b": 3, ("c": 4)))}"#,
        "{\"a\":2,\"b\":3,\"c\":4}\n",
    ),
    // The value is a full pipe, as outside parentheses.
    (r#"{"x": 1, ("a": 2 | . + 1)}"#, "{\"x\":1,\"a\":3}\n"),
    (
        r#"{"x":1, ("a": .a, "b": .b)}"#,
        "{\"x\":1,\"a\":5,\"b\":6}\n",
    ),
    // The key is an expression, as outside parentheses.
    (r#"{"x":1, (.a: .b)}"#, "{\"x\":1,\"5\":6}\n"),
    (r#"{"x": {("a": 2)}}"#, "{\"x\":{\"a\":2}}\n"),
    // A parenthesised *key* is not a pair group and keeps meaning what it did.
    (r#"{("a"): 1}"#, "{\"a\":1}\n"),
    (r#"{(("a")): 1}"#, "{\"a\":1}\n"),
    (r#"{(.a): 1, ("b": 2)}"#, "{\"5\":1,\"b\":2}\n"),
    (r"{(.a + .b): 1, ((.a) + 1: 2)}", "{\"11\":1,\"6\":2}\n"),
    (r#"{"x":1, ((.a) + 1: 2)}"#, "{\"x\":1,\"6\":2}\n"),
];

#[test]
fn test_4185_a_parenthesised_pair_group_is_flattened_into_the_object() -> Result<()> {
    for (filter, json) in ROWS {
        let (out, code) = run_yq_stdin(filter, INPUT, &["-o=json", "-I0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(&out, json, "{filter}");
    }
    Ok(())
}

/// A group holding anything but pairs, or followed by an operator, is still the parenthesised
/// expression it always was, so yq's `CollectObject` size error (a bare entry) is unchanged.
#[test]
fn test_4185_b_a_group_that_is_not_all_pairs_is_still_a_bare_entry() -> Result<()> {
    for filter in [
        r#"{"x": 1, ("a")}"#,
        r#"{"x": 1, (.a, .b)}"#,
        r#"{"x": 1, ("a", 2)}"#,
    ] {
        let (out, code) = run_yq_stdin(filter, INPUT, &["-o=json", "-I0"])?;
        assert_eq!((out.as_str(), code), ("", 1), "{filter}");
    }
    Ok(())
}

/// Shapes yq refuses stay refused (the message differs; the exit status agrees).
#[test]
fn test_4185_c_a_malformed_group_is_still_an_error() -> Result<()> {
    for filter in [
        r#"{"x": 1, ("a": )}"#,
        r#"{"x": 1, (: 2)}"#,
        r#"{"x": 1, ("a": 2),}"#,
        r#"{"x": 1, (("a": 2)}"#,
        r#"{"x": 1, ("a": 2) "y"}"#,
        r#"{"x": 1, ("a": 2) | length}"#,
        r#"{"x": 1, ("a": 1, 2)}"#,
    ] {
        let (out, code) = run_yq_stdin(filter, INPUT, &["-o=json", "-I0"])?;
        assert_eq!((out.as_str(), code), ("", 1), "{filter}");
    }
    Ok(())
}

/// The retry budget bounds a nest of failing groups, so a deep one parses (and fails) quickly
/// rather than costing `2^depth`.
#[test]
fn test_4185_d_nested_failing_groups_do_not_blow_up() -> Result<()> {
    let mut filter = String::from(".a");
    for _ in 0..40 {
        filter = format!("{{({filter}), \"x\": 1}}");
    }
    let (_, code) = run_yq_stdin(&filter, INPUT, &["-o=json", "-I0"])?;
    assert!(code == 0 || code == 1, "exit {code}");
    Ok(())
}

/// Nesting is charged against the expression depth limit, so a pathological tower of groups is a
/// parse error rather than a stack overflow.
#[test]
fn test_4185_e_a_tower_of_groups_is_refused_not_overflowed() -> Result<()> {
    let filter = format!("{{{}\"a\": 1{}}}", "(".repeat(1000), ")".repeat(1000));
    let (out, code) = run_yq_stdin(&filter, INPUT, &["-o=json", "-I0"])?;
    assert_eq!((out.as_str(), code), ("", 1));
    let ok = format!("{{{}\"a\": 1{}}}", "(".repeat(20), ")".repeat(20));
    let (out, code) = run_yq_stdin(&ok, INPUT, &["-o=json", "-I0"])?;
    assert_eq!((out.as_str(), code), ("{\"a\":1}\n", 0));
    Ok(())
}
