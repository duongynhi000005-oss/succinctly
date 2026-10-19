//! #4922: yq's `del()` vivifies a `null` along multi-valued steps and computed keys.

use anyhow::Result;

use crate::run_yq_stdin;

/// #4922: yq evaluates `del()`'s argument against a tree it grows: a bracket key, a position or an
/// iteration on a `null` turns it into the container the step implies (`{}` for a name, `[]` for a
/// position or an absent key), and every output of a multi-valued step continues the rest of the
/// target. `#2126` covered a plain chain of field, position and literal-bracket steps; these rows
/// are the multi-valued and computed ones it left: `.[]` over arrays and objects, `..`, a comma
/// ahead of a bracket, `.[(0,1)]`, a computed key evaluated against the whole document, and a
/// short array padded out to the position a later bracket vivifies. Every row captured from yq
/// v4.53.3 (`-p json -o=json -I=0`).
#[test]
fn test_4922_del_vivifies_null_along_multi_valued_steps_and_computed_keys() -> Result<()> {
    let rows: &[(&str, &str, &str)] = &[
        (r#"{"a":null}"#, r"del(.a[0])", r#"{"a":[]}"#),
        (r#"{"a":null}"#, r"del(.a[(0,1)])", r#"{"a":[]}"#),
        (
            r#"{"a":null}"#,
            r#"del(.a[2]["k"])"#,
            r#"{"a":[null,null,{}]}"#,
        ),
        (r#"{"a":null}"#, r#"del(.a[0]["k"]["z"])"#, r#"{"a":[{}]}"#),
        (r#"{"a":null}"#, r#"del(.a[("x","y")]["k"])"#, r#"{"a":{}}"#),
        (
            r#"{"a":null}"#,
            r#"del(.a[(0,1)]["k"])"#,
            r#"{"a":[{},{}]}"#,
        ),
        (r#"{"a":null}"#, r#"del(.a[("x",0)]["k"])"#, r#"{"a":{}}"#),
        (
            r#"{"a":null,"n":"x"}"#,
            r"del(.a[.n])",
            r#"{"a":{},"n":"x"}"#,
        ),
        (r#"{"a":null,"n":0}"#, r"del(.a[.n])", r#"{"a":[],"n":0}"#),
        (
            r#"{"a":null,"n":0}"#,
            r#"del(.a[.n]["k"])"#,
            r#"{"a":[{}],"n":0}"#,
        ),
        (
            r#"{"a":null,"n":["x","y"]}"#,
            r"del(.a[.n[]])",
            r#"{"a":{},"n":["x","y"]}"#,
        ),
        (
            r#"{"a":null,"n":"x"}"#,
            r#"del(.a[.n]["k"])"#,
            r#"{"a":{},"n":"x"}"#,
        ),
        (
            r#"{"a":[null,{"b":null}]}"#,
            r#"del(.a[] | .["b"]["c"])"#,
            r#"{"a":[{},{"b":{}}]}"#,
        ),
        (
            r#"{"x":[null,null]}"#,
            r#"del(.x[]["a"])"#,
            r#"{"x":[{},{}]}"#,
        ),
        (r#"{"x":[null,null]}"#, r"del(.x[][0])", r#"{"x":[[],[]]}"#),
        (
            r#"{"x":{"p":null,"q":null}}"#,
            r#"del(.x[]["a"])"#,
            r#"{"x":{"p":{},"q":{}}}"#,
        ),
        (
            r#"{"x":{"p":null,"q":null}}"#,
            r#"del(.x[] | .["a"])"#,
            r#"{"x":{"p":{},"q":{}}}"#,
        ),
        (r#"{"x":null}"#, r#"del(.x[]["a"])"#, r#"{"x":[]}"#),
        (r#"{"x":5}"#, r#"del(.x[]["a"])"#, r#"{"x":5}"#),
        (
            r#"{"x":[[null],[null]]}"#,
            r#"del(.x[][0]["k"])"#,
            r#"{"x":[[{}],[{}]]}"#,
        ),
        (
            r#"{"x":[[null],[null]]}"#,
            r#"del(.x[][]["k"])"#,
            r#"{"x":[[{}],[{}]]}"#,
        ),
        (r#"{"a":null}"#, r#"del(.. | .["k"])"#, r#"{"a":{}}"#),
        (r"[null,null]", r#"del(.[]["k"])"#, r"[{},{}]"),
        (r"[null,null]", r#"del(.[0]["k"], .[1]["j"])"#, r"[{},{}]"),
        (r#"{"a":null}"#, r#"del(.a["k"], .a[0])"#, r#"{"a":{}}"#),
        (
            r#"{"a":null,"b":null}"#,
            r#"del((.a,.b)["k"])"#,
            r#"{"a":{},"b":{}}"#,
        ),
        (
            r#"{"a":null,"b":null}"#,
            r#"del((.a,.b) | .["k"])"#,
            r#"{"a":{},"b":{}}"#,
        ),
        (
            r#"{"a":null}"#,
            r#"del(.a | .["k"] | .["z"])"#,
            r#"{"a":{}}"#,
        ),
        (r#"{"a":[1,null]}"#, r#"del(.a[1]["k"])"#, r#"{"a":[1,{}]}"#),
        (
            r#"{"a":[1,null]}"#,
            r#"del(.a[-1]["k"])"#,
            r#"{"a":[1,{}]}"#,
        ),
        (
            r#"{"a":[1,null]}"#,
            r#"del(.a[5]["k"])"#,
            r#"{"a":[1,null,null,null,null,{}]}"#,
        ),
        (r#"{"a":null}"#, r#"del(.a["k"][0])"#, r#"{"a":{}}"#),
        (
            r#"{"a":{"k":null}}"#,
            r#"del(.a["k"][0])"#,
            r#"{"a":{"k":[]}}"#,
        ),
        (r#"{"a":{"k":null}}"#, r"del(.a.k[0])", r#"{"a":{"k":[]}}"#),
        (
            r#"{"a":{"k":null}}"#,
            r#"del(.a.k["z"])"#,
            r#"{"a":{"k":{}}}"#,
        ),
        (
            r#"{"a":[null,null]}"#,
            r#"del(.a[0:2][0]["k"])"#,
            r#"{"a":[null,null]}"#,
        ),
        (
            r#"{"a":null}"#,
            r#"del(.a[3]["k"])"#,
            r#"{"a":[null,null,null,{}]}"#,
        ),
        (r#"{"a":null}"#, r"del(.a[2])", r#"{"a":[null,null]}"#),
        (r#"{"a":null}"#, r"del(.a[])", r#"{"a":[]}"#),
        (
            r#"{"a":{"p":null}}"#,
            r#"del(.a[] | .["z"][0])"#,
            r#"{"a":{"p":{}}}"#,
        ),
        (
            r#"{"a":[{"p":null},{"p":[null]}]}"#,
            r#"del(.a[].p[0]["k"])"#,
            r#"{"a":[{"p":[{}]},{"p":[{}]}]}"#,
        ),
        (
            r#"{"a":null,"b":null}"#,
            r#"del(.a["k"], .b[0]["z"])"#,
            r#"{"a":{},"b":[{}]}"#,
        ),
        (r#"{"a":null}"#, r#"del((.a, .a)["k"])"#, r#"{"a":{}}"#),
        (
            r#"{"a":null,"n":1}"#,
            r#"del(.a[.n]["k"])"#,
            r#"{"a":[null,{}],"n":1}"#,
        ),
        (
            r#"{"a":[[1],null]}"#,
            r#"del(.a[1][0]["z"])"#,
            r#"{"a":[[1],[{}]]}"#,
        ),
        (
            r#"{"a":[[1],null]}"#,
            r#"del(.a[][0]["z"])"#,
            r#"{"a":[[1],[{}]]}"#,
        ),
        (
            r#"{"a":{"b":{"c":null}}}"#,
            r#"del(.. | .["x"])"#,
            r#"{"a":{"b":{"c":{}}}}"#,
        ),
        (
            r#"{"a":{"b":{"c":null}}}"#,
            r#"del(.. | .c["x"])"#,
            r#"{"a":{"b":{"c":{}}}}"#,
        ),
        (r#"{"a":"s"}"#, r#"del(.a[]["k"])"#, r#"{"a":"s"}"#),
        (r#"{"a":1}"#, r#"del(.a["k"])"#, r#"{"a":1}"#),
        (
            r#"{"a":null}"#,
            r"del(.. | .[(0,1)].x)",
            r#"{"a":[null,null]}"#,
        ),
        (
            r#"{"a":{"n":0,"q":null},"n":"x"}"#,
            r#"del(.a | .q[.n]["k"])"#,
            r#"{"a":{"n":0,"q":[{}]},"n":"x"}"#,
        ),
        (
            r#"{"a":null,"n":"x"}"#,
            r"del(.n | .[.n])",
            r#"{"a":null,"n":"x"}"#,
        ),
    ];
    for (doc, filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, doc, &["-p", "json", "-o=json", "-I=0"])?;
        assert_eq!(code, 0, "filter {filter:?} on {doc}: {out:?}");
        assert_eq!(out.trim(), *want, "filter {filter:?} on {doc}");
    }
    Ok(())
}
