//! #4461: yq rules on a slice bound that did not produce exactly one number
//! (`expected to find 1 number, got N instead`) only once the target produced a node, after any
//! error the target itself raised (#2056). The `del()` and the `key`/`parent`/`path` walks raised
//! it for a target with no node. Each row is a live yq v4.53.3 capture of
//! `yq -o=json -I=0 <filter>`.

use anyhow::Result;

use crate::run_yq_stdin_with_stderr;

const ARGV: &[&str] = &["-o", "json", "-I", "0"];

/// (document, filter, expected stdout): no error, whatever the bound count.
const QUIET: &[(&str, &str, &str)] = &[
    // `del()` of a key the document does not hold.
    (r#"{"b":1}"#, "del(.a[(0,1):2])", "{\"b\":1}\n"),
    (r#"{"b":1}"#, "del(.a.b[(0,1):2])", "{\"b\":1}\n"),
    (r#"{"b":1}"#, "del(.a[select(false):2])", "{\"b\":1}\n"),
    (r#"{"a":null}"#, "del(.a.b[(0,1):2])", "{\"a\":null}\n"),
    // An index into a mapping or a scalar, and a field of a scalar, are not nodes either.
    (
        r#"{"a":{"c":{}}}"#,
        "del(.a[0].b[(0,1):2])",
        "{\"a\":{\"c\":{}}}\n",
    ),
    (
        r#"{"a":{"c":{}}}"#,
        "del(.a[3][(0,1):2])",
        "{\"a\":{\"c\":{}}}\n",
    ),
    (r#"{"a":1}"#, "del(.a[0][(0,1):2])", "{\"a\":1}\n"),
    (r#"{"a":true}"#, "del(.a.b.c[(0,1):2])", "{\"a\":true}\n"),
    // A target that produces no node: a key of a scalar or of a `null`.
    (r#"{"a":1}"#, ".a.b[(0,1):2] | key", ""),
    (r#"{"a":1}"#, ".a.b[(0,1):2] | parent", ""),
    (r#"{"a":1}"#, ".a.b[(0,1):2] | path", ""),
    (r#"{"a":1}"#, ".a.b[select(false):2] | key", ""),
    (r#"{"a":1}"#, ".a.b[(0,1):select(false)] | key", ""),
    (r#"{"a":"s"}"#, ".a.b[(0,1):2] | key", ""),
    (r#"{"a":"s"}"#, ".a.b[(0,1):2] | parent", ""),
];

#[test]
fn test_slice_bound_count_waits_for_a_target_that_is_a_node_4461() -> Result<()> {
    for (doc, filter, expected) in QUIET {
        let (stdout, stderr, code) = run_yq_stdin_with_stderr(filter, doc, ARGV)?;
        assert_eq!(
            (stdout.as_str(), code),
            (*expected, 0),
            "{doc} {filter}: {stderr}"
        );
    }
    Ok(())
}

/// (document, filter): the count error is still raised once the target is a node -- a present
/// `null` (`del()`), an absent key (a write creates it; `key` walks it), a sequence.
const COUNT_ERROR: &[(&str, &str)] = &[
    (r#"{"a":null}"#, "del(.a[(0,1):2])"),
    (r#"{"a":[1,2,3]}"#, "del(.a[(0,1):2])"),
    // An index past the end of a sequence, an index into a `null` and the scalar itself are nodes.
    (r#"{"a":[{"b":1}]}"#, "del(.a[3][(0,1):2])"),
    (r#"{"a":null}"#, "del(.a[0][(0,1):2])"),
    (r#"{"a":1}"#, "del(.a[(0,1):2])"),
    (r#"{"b":1}"#, ".a[(0,1):2] = 5"),
    (r#"{"b":1}"#, ".a.b[(0,1):2] | key"),
    (r#"{"a":{"b":1}}"#, ".a.b[(0,1):2] | key"),
    (r#"{"a":null}"#, ".a.b[(0,1):2] | key"),
    (r#"{"a":[1,2,3]}"#, ".a[select(false):2] | key"),
];

#[test]
fn test_slice_bound_count_error_still_raised_for_a_node_4461() -> Result<()> {
    for (doc, filter) in COUNT_ERROR {
        let (stdout, stderr, code) = run_yq_stdin_with_stderr(filter, doc, ARGV)?;
        assert_eq!((stdout.as_str(), code), ("", 1), "{doc} {filter}");
        assert!(
            stderr.contains("expected to find 1 number"),
            "{doc} {filter}: {stderr}"
        );
    }
    Ok(())
}

/// The target's own error comes before the bounds' (yq: `cannot index array with 'b'`).
#[test]
fn test_slice_target_error_comes_before_the_bound_count_4461() -> Result<()> {
    for filter in [
        ".a.b[(0,1):2] | key",
        ".a.b[(0,1):2] | parent",
        ".a.b[select(false):2] | key",
        "del(.a.b[(0,1):2])",
    ] {
        let (stdout, stderr, code) = run_yq_stdin_with_stderr(filter, r#"{"a":[1,2,3]}"#, ARGV)?;
        assert_eq!((stdout.as_str(), code), ("", 1), "{filter}");
        assert!(!stderr.contains("expected to find"), "{filter}: {stderr}");
    }
    Ok(())
}
