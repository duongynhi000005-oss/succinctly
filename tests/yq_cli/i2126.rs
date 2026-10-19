//! #2126: yq reads `.["k0"]`, `.[0]` and `.[("k0","k1")]` through its array-index traversal, which
//! turns a `null` it indexes into the empty container the key implies; the dotted `.k0` leaves
//! the `null` alone.

use anyhow::Result;

use crate::run_yq_stdin;

/// #2126: `null | del(.["k0"])` is `{}` in yq (and `del(.k0)` is `null`), `del(.[0])` and
/// `del(.[(0,1)])` are `[]`, and the vivified container is whatever the bracket named first:
/// `{"a":null} | del(.a["k0"])` is `{"a":{}}`. Every row is captured from yq v4.53.3 (`-p=json
/// -o=json -I=0`). A non-null target (`5`, an object that has the key, an object that lacks it)
/// is untouched by the pre-pass.
#[test]
fn test_2126_del_vivifies_a_null_behind_a_bracket_key() -> Result<()> {
    let rows: &[(&str, &str, &str)] = &[
        ("null", "del(.k0)", "null\n"),
        ("null", "del(.\"k0\")", "null\n"),
        ("null", "del(.[\"k0\"])", "{}\n"),
        ("null", "del(.[(\"k0\")])", "{}\n"),
        ("null", "del(.[0])", "[]\n"),
        ("null", "del(.[(0,1)])", "[]\n"),
        ("null", "del(.[(\"k0\",\"k1\")])", "{}\n"),
        ("null", "del(.[\"k0\"].x)", "{}\n"),
        ("null", "del(.[\"k0\"][0])", "{}\n"),
        ("null", "del(.[\"k0\"]?)", "{}\n"),
        ("null", "del(.k0.x)", "null\n"),
        ("null", "del(.a.k0)", "null\n"),
        ("null", "del(.[\"k0\"], .z)", "{}\n"),
        ("{\"a\":null}", "del(.a[\"k0\"])", "{\"a\":{}}\n"),
        ("{\"a\":null}", "del(.a.[\"k0\"])", "{\"a\":{}}\n"),
        ("{\"a\":null}", "del(.a[\"k0\"].x)", "{\"a\":{}}\n"),
        ("{\"a\":null}", "del(.a[0])", "{\"a\":[]}\n"),
        ("{\"a\":null}", "del(.a.k0)", "{\"a\":null}\n"),
        ("{\"a\":null}", "del(.b[\"k0\"])", "{\"a\":null}\n"),
        ("{\"k0\":1,\"z\":2}", "del(.[\"k0\"])", "{\"z\":2}\n"),
        ("{\"k0\":1,\"z\":2}", "del(.[\"z\"])", "{\"k0\":1}\n"),
        ("{\"k0\":1,\"z\":2}", "del(.[\"k0\"], .[\"z\"])", "{}\n"),
        (
            "{\"a\":{\"b\":null}}",
            "del(.[\"a\"][\"b\"][\"c\"])",
            "{\"a\":{\"b\":{}}}\n",
        ),
        (
            "{\"a\":{\"b\":null}}",
            "del(.a[\"b\"][0])",
            "{\"a\":{\"b\":[]}}\n",
        ),
        ("{\"a\":[1,null]}", "del(.a[1][\"k\"])", "{\"a\":[1,{}]}\n"),
        ("5", "del(.[\"k0\"])", "5\n"),
        ("\"s\"", "del(.[\"k0\"])", "\"s\"\n"),
    ];
    for (input, filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, input, &["-p=json", "-o=json", "-I=0"])?;
        assert_eq!(code, 0, "input {input:?} filter {filter:?}: {out:?}");
        assert_eq!(out, *want, "input {input:?} filter {filter:?}");
    }
    Ok(())
}
