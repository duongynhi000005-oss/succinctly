//! #4476: a value derived from a navigated node keeps the node's slot in yq (`CopyAsReplacement`),
//! so `del(.b | tostring)` deletes `.b` and `del(.arr[] | tostring)` empties `.arr`; succinctly's
//! untracked branches carry no provenance and refused with `Invalid path expression`. A pipe of
//! navigation ending in `tostring`, `length`, `not` or `@json` (stages that answer every node) now
//! deletes the navigation. The bare-root slice rows of the same issue were matched by #1325.
//!
//! Every output is captured from yq v4.53.3.

use anyhow::Result;

use crate::run_yq_stdin;

const DOC: &str =
    r#"{"a":1,"b":2,"c":3,"s":"x","arr":[1,2,3],"o":{"p":5,"q":6},"t":true,"z":null}"#;

/// (filter, `yq -o=json -I0` output over [`DOC`]).
const ROWS: &[(&str, &str)] = &[
    ("del(.b | tostring)", "{\"a\":1,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.b | length)", "{\"a\":1,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.arr | length)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.b | not)", "{\"a\":1,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.b | @json)", "{\"a\":1,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.arr | .[0] | tostring)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"arr\":[2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del((.b | tostring), .c)", "{\"a\":1,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.c, (.b | tostring))", "{\"a\":1,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.arr[] | tostring)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"arr\":[],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.o.p | tostring)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.o[] | tostring)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{},\"t\":true,\"z\":null}\n"),
    ("del(.o | length)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"t\":true,\"z\":null}\n"),
    ("del(.zz | tostring)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.b | tostring | tostring)", "{\"a\":1,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.t | length)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"z\":null}\n"),
    ("del(.z | tostring)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true}\n"),
    ("del(.z | length)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true}\n"),
    ("del((.b | tostring), 1)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(1, (.b | tostring))", "{\"a\":1,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.arr[1:2] | tostring)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n"),
    ("del(.o | not)", "{\"a\":1,\"b\":2,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"t\":true,\"z\":null}\n"),
    ("del(.b | tostring), 1", "{\"a\":1,\"c\":3,\"s\":\"x\",\"arr\":[1,2,3],\"o\":{\"p\":5,\"q\":6},\"t\":true,\"z\":null}\n1\n"),
];

#[test]
fn test_4476_a_a_derived_target_deletes_the_node_it_derives_from() -> Result<()> {
    for (filter, want) in ROWS {
        let (out, code) = run_yq_stdin(filter, DOC, &["-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(&out, want, "{filter}");
    }
    Ok(())
}

/// Arithmetic on a node and the other derivations are still refused (a type error such as
/// `.b + [1]` is an error in yq, and a rewrite would turn it into a delete), as is a derived
/// document root, which yq keeps (`del(.. | tostring)` is `{}`). Recorded in `limitations.md`.
#[test]
fn test_4476_b_the_other_derived_targets_are_still_refused() -> Result<()> {
    for filter in [
        "del(.b + 1)",
        "del(.b | . + [1])",
        "del(.arr | map(.))",
        "del(. | tostring)",
        "del(.. | tostring)",
    ] {
        let (out, code) = run_yq_stdin(filter, DOC, &["-o=json", "-I=0"])?;
        assert_eq!((out.as_str(), code), ("", 1), "{filter}");
    }
    Ok(())
}
