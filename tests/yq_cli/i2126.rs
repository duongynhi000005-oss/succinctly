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

/// #2126: the bracket spelling must change nothing but the `null` vivify. Each pair runs the same
/// `del()` spelled with brackets and with dots over a document with anchors, aliases and comments, and
/// the two must print the same bytes (the `*a` of the first row is dropped by both: its anchor is the
/// item deleted, the documented never-emit-unreadable-YAML rule). The last rows are captured from yq
/// v4.53.3: a trailing `.[]` behind an absent bracket key is still the no-op it makes of it.
#[test]
fn test_2126_bracket_spelling_keeps_presentation_and_absent_key_rules() -> Result<()> {
    let seq = "s:\n  - &a 1 # one\n  - &b 2 # two\n  - *a # three\n";
    let map = "m: &mm\n  k: v # kc\n  j: [1, 2] # jc\nn: *mm\n";
    let pairs: &[(&str, &str, &str)] = &[
        (seq, r#"del(.["s"][0])"#, "del(.s[0])"),
        (seq, r#"del(.["s"][1])"#, "del(.s[1])"),
        (seq, r#"del(.["s"][0], .["s"][1])"#, "del(.s[0], .s[1])"),
        (map, r#"del(.["m"]["k"])"#, "del(.m.k)"),
        (map, r#"del(.m["j"][0])"#, "del(.m.j[0])"),
        (map, r#"del(.["n"]["k"])"#, "del(.n.k)"),
    ];
    for (input, bracket, dotted) in pairs {
        let (b, bc) = run_yq_stdin(bracket, input, &[])?;
        let (d, dc) = run_yq_stdin(dotted, input, &[])?;
        assert_eq!((bc, dc), (0, 0), "input {input:?} {bracket} / {dotted}");
        assert_eq!(b, d, "input {input:?}: {bracket} vs {dotted}");
    }
    let rows: &[(&str, &str, &str)] = &[
        ("a: 1\n", r#"del(.["s"][])"#, "a: 1\n"),
        ("a: 1\n", r#"del(.["a"])"#, "{}\n"),
        (
            "u: null # c\nw:\n  x: null\n",
            r#"del(.["w"]["x"]["y"])"#,
            "u: null # c\nw:\n  x: {}\n",
        ),
        ("u: null\n", r#"del(.["u"][0]["z"])"#, "u:\n  - {}\n"),
        ("u: null\n", r#"del(.["u"]["z"])"#, "u: {}\n"),
    ];
    for (input, filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, input, &[])?;
        assert_eq!(code, 0, "input {input:?} filter {filter:?}: {out:?}");
        assert_eq!(out, *want, "input {input:?} filter {filter:?}");
    }
    Ok(())
}
