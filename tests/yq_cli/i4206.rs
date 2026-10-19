//! #4206: yq's `to_json`, `to_json(N)`, `to_yaml` and `to_yaml(N)`.

use anyhow::Result;

use crate::run_yq_stdin;

/// #4206: `to_json` is the value as JSON text with two spaces of indent and a trailing line break,
/// `to_json(N)` takes the indent (`0` is compact with no line break). `to_yaml` is the `@yaml` encoder and `to_yaml(N)` its
/// indent, which yq reads as `0` -> four spaces, `1` -> two and otherwise as written. Every row is
/// captured from yq v4.53.3 (`-n -o=json -I=0`, so a result string prints as one JSON line).
#[test]
fn test_4206_to_json_and_to_yaml() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        ("{\"a\":[1,{\"b\":2}]}|to_json", "\"{\\n  \\\"a\\\": [\\n    1,\\n    {\\n      \\\"b\\\": 2\\n    }\\n  ]\\n}\\n\"\n"),
        ("{\"a\":[1,{\"b\":2}]}|to_json(0)", "\"{\\\"a\\\":[1,{\\\"b\\\":2}]}\"\n"),
        ("{\"a\":[1,{\"b\":2}]}|to_json(1)", "\"{\\n \\\"a\\\": [\\n  1,\\n  {\\n   \\\"b\\\": 2\\n  }\\n ]\\n}\\n\"\n"),
        ("{\"a\":[1,{\"b\":2}]}|to_json(4)", "\"{\\n    \\\"a\\\": [\\n        1,\\n        {\\n            \\\"b\\\": 2\\n        }\\n    ]\\n}\\n\"\n"),
        ("{\"a\":[1,{\"b\":2}]}|to_yaml", "\"a:\\n  - 1\\n  - b: 2\\n\"\n"),
        ("{\"a\":[1,{\"b\":2}]}|to_yaml(0)", "\"a:\\n    - 1\\n    - b: 2\\n\"\n"),
        ("{\"a\":[1,{\"b\":2}]}|to_yaml(1)", "\"a:\\n  - 1\\n  - b: 2\\n\"\n"),
        ("{\"a\":[1,{\"b\":2}]}|to_yaml(3)", "\"a:\\n   - 1\\n   - b: 2\\n\"\n"),
        ("{\"a\":[1,{\"b\":2}]}|to_yaml(8)", "\"a:\\n        - 1\\n        - b: 2\\n\"\n"),
        ("(1e10*2)|to_json", "\"2e+10\\n\"\n"),
        ("[1e10*2]|to_json(0)", "\"[2e+10]\"\n"),
        ("(0.5+0.5)|to_json", "\"1.0\\n\"\n"),
        ("[]|to_json", "\"[]\\n\"\n"),
        ("{}|to_json", "\"{}\\n\"\n"),
        ("[]|to_yaml", "\"[]\\n\"\n"),
        ("null|to_json", "\"null\\n\"\n"),
        ("null|to_yaml", "\"null\\n\"\n"),
        ("\"x\"|to_json", "\"\\\"x\\\"\\n\"\n"),
        ("1|to_json", "\"1\\n\"\n"),
        ("1.50|to_json", "\"1.50\\n\"\n"),
        ("1.50|to_yaml", "\"1.50\\n\"\n"),
        ("[1.0, 2e3]|to_json", "\"[\\n  1.0,\\n  2e3\\n]\\n\"\n"),
        ("true|to_json", "\"true\\n\"\n"),
        ("[1,2]|to_json|length", "13\n"),
        ("[1,2]|to_json|fromjson", "[1,2]\n"),
        ("[1,2]|to_json(0)|from_json", "[1,2]\n"),
        ("{\"a\":1}|[to_json, to_yaml]", "[\"{\\n  \\\"a\\\": 1\\n}\\n\",\"a: 1\\n\"]\n"),
        ("[{\"x\":1},{\"y\":[]}]|to_yaml", "\"- x: 1\\n- y: []\\n\"\n"),
        ("{\"k\":\"a b\"}|to_json(2)|length", "17\n"),
    ];
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, "null", &["-n", "-o=json", "-I=0"])?;
        assert_eq!(code, 0, "filter {filter:?}: {out:?}");
        assert_eq!(out, *want, "filter {filter:?}");
    }
    Ok(())
}

/// #4206: the indent is a non-negative integer literal and nothing else: yq's parser rejects
/// `to_json(.a)`, `to_json(2+2)`, `to_yaml(-1)`, a quoted or fractional number, and a space before
/// the parenthesis (`to_json (2)` is `to_json` followed by a stray `(2)`).
#[test]
fn test_4206_to_json_indent_must_be_a_literal() -> Result<()> {
    for filter in [
        "{\"a\":1}|to_json(.a)",
        "{\"a\":1}|to_json(-1)",
        "{\"a\":1}|to_json(2+2)",
        "{\"a\":1}|to_yaml(1.5)",
        "{\"a\":1}|to_json(\"x\")",
        "{\"a\":1} | to_json (2)",
        "{\"a\":1} | to_json( 2 )",
        "{\"a\":1} | to_json(0x2)",
    ] {
        let (out, code) = run_yq_stdin(filter, "null", &["-n", "-o=json", "-I=0"])?;
        assert_ne!(code, 0, "filter {filter:?} should be rejected: {out:?}");
    }
    Ok(())
}

/// #4206: an indent past the cap is refused up front (yq would try to build it): `to_json(4000000000)`
/// must be a parse error, not an allocation.
#[test]
fn test_4206_to_json_indent_is_capped() -> Result<()> {
    for filter in [
        "to_json(100001)",
        "to_yaml(4000000000)",
        "to_json(99999999999999999999999)",
    ] {
        let (out, code) = run_yq_stdin(filter, "null", &["-n", "-o=json", "-I=0"])?;
        assert_ne!(code, 0, "filter {filter:?} should be refused: {out:?}");
    }
    // The cap itself still works.
    let (out, code) = run_yq_stdin("[1] | to_json(100000) | length", "null", &["-n"])?;
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "100006");
    Ok(())
}
