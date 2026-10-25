//! #5243: a path target whose index key only computes a value is static, so it stays on the
//! streaming walkers instead of the path-mode interpreter (which copied a `reduce`'s accumulator
//! on every step, quadratic in the input).

use anyhow::Result;

use crate::run_jq_full;

/// A value-only index key, on both routes (stdin and `-n` with `input`), as jq 1.7.1 answers
/// (captured live). jq runs the key as a subexpression on the target's *input*, so `.a[.k]` reads
/// the root's `.k` and `path(.b[length - 1])` takes the root's length.
#[test]
fn test_value_only_index_key_answers_as_jq_5243() -> Result<()> {
    let records = r#"[{"name":"a","score":1},{"name":"b","score":2},{"name":"a","score":3}]"#;
    for (input, filter, want) in [
        (
            records,
            "reduce .[] as $r ({}; .[$r.name] = $r.score)",
            r#"{"a":3,"b":2}"#,
        ),
        (
            records,
            "reduce .[] as $r ({}; .[$r.name] += $r.score)",
            r#"{"a":4,"b":2}"#,
        ),
        (
            r#"{"a":{"x":1,"y":2},"k":"x"}"#,
            ".a[.k] |= . + 10",
            r#"{"a":{"x":11,"y":2},"k":"x"}"#,
        ),
        (
            r#"{"a":[1,2,3],"i":1}"#,
            "del(.a[.i, .i + 1])",
            r#"{"a":[1],"i":1}"#,
        ),
        (r#"{"b":[10,20,30]}"#, "path(.b[length - 1])", r#"["b",0]"#),
        // A key that names `nan` stays on the interpreter (#5245).
        ("[1,2]", "path(.[nan])", "[null]"),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-c", filter], Some(input))?;
        assert_eq!((stdout.trim_end(), code), (want, 0), "`{filter}`: {stderr}");
        let (stdout, stderr, code) =
            run_jq_full(&["-n", "-c", &format!("input | {filter}")], Some(input))?;
        assert_eq!((stdout.trim_end(), code), (want, 0), "-n `{filter}`: {stderr}");
    }
    Ok(())
}
