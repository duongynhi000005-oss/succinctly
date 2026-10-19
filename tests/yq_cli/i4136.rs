//! #4136: yq's array subtraction compares numbers by tag and text.

use anyhow::Result;

use crate::{run_jq_stdin, run_yq_stdin};

/// #4136: yq's `subtractArray` pairs two elements with `recursiveNodeEqual`, which
/// compares a scalar by tag and text, so a number keeps the spelling it was written
/// with: `[1.50] - [1.5]` removes nothing. Every row is captured from yq v4.53.3.
#[test]
fn test_4136_array_subtraction_compares_numbers_by_spelling() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        // Same value, different spelling: kept.
        ("[1.50] - [1.5]", "[1.50]"),
        ("[1.5] - [1.50]", "[1.5]"),
        ("[1.0] - [1.00]", "[1.0]"),
        ("[1e3] - [1E3]", "[1e3]"),
        ("[-0.0] - [0.0]", "[-0.0]"),
        ("[1.50] - [1.5e0]", "[1.50]"),
        ("[1.5, 1.50, 2.0] - [1.5]", "[1.50,2.0]"),
        ("[1.5, 1.50, 2.0] - [1.50]", "[1.5,2.0]"),
        // A computed number is spelled the way yq formats it, not as a literal.
        ("[5.0] - [2.5 * 2]", "[5.0]"),
        ("[2.5 * 2] - [5.0]", "[5.0]"),
        // The same spelling is removed, as before.
        ("[1.50] - [1.50]", "[]"),
        ("[1.5] - [3/2]", "[]"),
        ("[3/2] - [1.5]", "[]"),
        ("[0.1 + 0.2] - [0.1 + 0.2]", "[]"),
        ("[.nan] - [.nan]", "[]"),
        // Int and float never match, and neither does a string and a number.
        ("[1.0] - [1]", "[1.0]"),
        ("[1] - [1.0]", "[1]"),
        ("[\"1.5\"] - [1.5]", "[\"1.5\"]"),
        // The rule holds at every depth.
        ("[[1.50]] - [[1.5]]", "[[1.50]]"),
        ("[[1.50]] - [[1.50]]", "[]"),
        ("[{\"a\":1.50}] - [{\"a\":1.5}]", "[{\"a\":1.50}]"),
        ("[{\"a\":1.50}] - [{\"a\":1.50}]", "[]"),
        // Containers still compare structurally: an object ignores key order, an array does not.
        ("[{\"a\":1,\"b\":2}] - [{\"b\":2,\"a\":1}]", "[]"),
        ("[[1,2]] - [[2,1]]", "[[1,2]]"),
        ("[null] - [null]", "[]"),
        ("[true] - [\"true\"]", "[true]"),
    ];
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, "null", &["-o=json", "-I=0"])?;
        assert_eq!(code, 0, "filter {filter:?}: {out:?}");
        assert_eq!(out.trim(), *want, "filter {filter:?}");
    }
    Ok(())
}

/// #4136: the same rule over spellings read from a YAML document; a JSON-sourced number has none.
#[test]
fn test_4136_array_subtraction_reads_the_document_spelling() -> Result<()> {
    let doc = "a: 1.50\nb: 1.5\nc: 1.0000000000000000000001\n";
    for (filter, want) in [
        ("[.a] - [.b]", "[1.50]"),
        ("[.a,.b] - [.b]", "[1.50]"),
        ("[.a] - [1.5]", "[1.50]"),
        ("[.a] - [.a * 1]", "[1.50]"),
        ("[.c] - [.c | . * 1]", "[1.0000000000000000000001]"),
        ("[.a] - [.a]", "[]"),
        ("[.a] - [1.50]", "[]"),
    ] {
        let (out, code) = run_yq_stdin(filter, doc, &["-o=json", "-I=0"])?;
        assert_eq!(code, 0, "filter {filter:?}: {out:?}");
        assert_eq!(out.trim(), want, "filter {filter:?}");
    }

    // yq's JSON decoder normalises a number (`1.50` reads as `1.5`), so the document
    // spelling does not survive there and the two are equal.
    let json = r#"{"a":1.50,"b":1.5,"c":[1.50,1.5]}"#;
    for (filter, want) in [
        ("[.a] - [.b]", "[]"),
        (".c - [.b]", "[]"),
        ("[.a] - [1.50]", "[1.5]"),
    ] {
        let (out, code) = run_yq_stdin(filter, json, &["-p=json", "-o=json", "-I=0"])?;
        assert_eq!(code, 0, "filter {filter:?}: {out:?}");
        assert_eq!(out.trim(), want, "filter {filter:?}");
    }
    Ok(())
}

/// #4136: jq's `-` still compares by value, so the spelling rule is yq-only.
#[test]
fn test_4136_jq_array_subtraction_still_compares_numbers_by_value() -> Result<()> {
    for (filter, want) in [
        ("[1.50] - [1.5]", "[]"),
        ("[1.0] - [1]", "[]"),
        ("[1e3] - [1000]", "[]"),
    ] {
        let (out, code) = run_jq_stdin(filter, "null", &["-c"])?;
        assert_eq!(code, 0, "filter {filter:?}: {out:?}");
        assert_eq!(out.trim(), want, "filter {filter:?}");
    }
    Ok(())
}
