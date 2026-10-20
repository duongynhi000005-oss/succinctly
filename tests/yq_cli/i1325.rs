//! #1325: a comma-grouped `del()` stops at a bare slice of the root, as yq does. yq builds a slice
//! result as a replacement of the node it slices, so a slice of the root has no parent;
//! `deleteChildOperator` walks the matches from the last to the first and returns at the first one
//! without a parent. The branches after it in the group are deleted, it and the branches before it
//! are not, hence the order sensitivity (`del(.a[0:1], .[2:3])` deletes nothing, `del(.[2:3],
//! .a[0:1])` deletes `.a`). A slice further down (`.a[0:1]`) has a parent and a tail after the run
//! (`.[2:3][0]`) is inert without stopping anything.
//!
//! Every output is captured from yq v4.53.3.

use anyhow::Result;

use crate::run_yq_stdin;

const OBJECT: &str = r#"{"a":{"x":1,"y":2},"b":6,"c":9,"d":[1,2,3]}"#;
const ARRAY: &str = "[[1,2,3],[4,5,6],[7,8,9],[10,11,12]]";

/// (filter, `yq -o=json -I0` output over [`OBJECT`]).
const OBJECT_ROWS: &[(&str, &str)] = &[
    (
        "del(.a[0:1], .[2:3])",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"c\":9,\"d\":[1,2,3]}\n",
    ),
    ("del(.[2:3], .a[0:1])", "{\"b\":6,\"c\":9,\"d\":[1,2,3]}\n"),
    (
        "del(.b, .[2:3])",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"c\":9,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.a[0:1], .[2:3], .b)",
        "{\"a\":{\"x\":1,\"y\":2},\"c\":9,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.b, .[2:3][0:1])",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"c\":9,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.b, .[2:3][0:1], .c)",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.b, .[2:3][0])",
        "{\"a\":{\"x\":1,\"y\":2},\"c\":9,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.b, .[2:3]?)",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"c\":9,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.b, (.[2:3]))",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"c\":9,\"d\":[1,2,3]}\n",
    ),
    (
        "del((.b, .[2:3]), .c)",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.b, (.[2:3], .c))",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.[2:3], .b, .[1:2], .c)",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.d[0:1], .[1:2], .c)",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.[2:3] | .[0:1], .b)",
        "{\"a\":{\"x\":1,\"y\":2},\"c\":9,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.b, .[2:3] | .[0:1])",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"c\":9,\"d\":[1,2,3]}\n",
    ),
    ("del(.c, .d[1:2])", "{\"a\":{\"x\":1,\"y\":2},\"b\":6}\n"),
    (
        "del(.[0:2], .c)",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.b, .[:])",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"c\":9,\"d\":[1,2,3]}\n",
    ),
    (
        "del(.b, .[1:])",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"c\":9,\"d\":[1,2,3]}\n",
    ),
    (
        "del((.b, .a[0:1]), .[2:3])",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6,\"c\":9,\"d\":[1,2,3]}\n",
    ),
    ("del(.[2:3], (.b, .a[0:1]))", "{\"c\":9,\"d\":[1,2,3]}\n"),
    (
        "del((.b, (.[2:3], .c)), .d[0:1])",
        "{\"a\":{\"x\":1,\"y\":2},\"b\":6}\n",
    ),
];

/// (filter, `yq -o=json -I0` output over [`ARRAY`]).
const ARRAY_ROWS: &[(&str, &str)] = &[
    (
        "del(.[0][0:1], .[1:2][0:1])",
        "[[1,2,3],[4,5,6],[7,8,9],[10,11,12]]\n",
    ),
    (
        "del(.[3], .[2:3])",
        "[[1,2,3],[4,5,6],[7,8,9],[10,11,12]]\n",
    ),
    ("del(.[2:3], .[3])", "[[1,2,3],[4,5,6],[7,8,9]]\n"),
    ("del(.[2:3])", "[[1,2,3],[4,5,6],[7,8,9],[10,11,12]]\n"),
    ("del(.[0:2])", "[[1,2,3],[4,5,6],[7,8,9],[10,11,12]]\n"),
    (
        "del(.[0][0:1], .[3], .[1:2])",
        "[[1,2,3],[4,5,6],[7,8,9],[10,11,12]]\n",
    ),
    (
        "del(.[0][0:1], .[2:3])",
        "[[1,2,3],[4,5,6],[7,8,9],[10,11,12]]\n",
    ),
    ("del(.[2:3], .[0][0:1])", "[[4,5,6],[7,8,9],[10,11,12]]\n"),
    ("del(.[0], .[1:2], .[2])", "[[1,2,3],[4,5,6],[10,11,12]]\n"),
    ("del(.[1:2], .[0], .[2])", "[[4,5,6],[10,11,12]]\n"),
];

const NESTED: &str = r#"{"m":null,"x":[1],"z":2,"o":{"p":{"q":1}}}"#;

/// Resolving a path creates (an index past the end extends an array, an index or `.[]` on `null`
/// makes an array), and yq resolves every path before it stops. A branch before the stop that
/// only selects existing nodes, or stops at a missing key, is dropped whole; one that would create
/// keeps the group's earlier behaviour, so these rows are the ones that agree with yq.
/// (filter, `yq -o=json -I0` output over [`NESTED`]).
const NESTED_ROWS: &[(&str, &str)] = &[
    (
        "del(.m[], .[2:3])",
        "{\"m\":[],\"x\":[1],\"z\":2,\"o\":{\"p\":{\"q\":1}}}\n",
    ),
    (
        "del(.x[-1], .[2:3])",
        "{\"m\":null,\"x\":[1],\"z\":2,\"o\":{\"p\":{\"q\":1}}}\n",
    ),
    (
        "del(.m.n, .[2:3])",
        "{\"m\":null,\"x\":[1],\"z\":2,\"o\":{\"p\":{\"q\":1}}}\n",
    ),
    (
        "del(.x[0], .z, .[2:3], .o)",
        "{\"m\":null,\"x\":[1],\"z\":2}\n",
    ),
    (
        "del(.x[], .[2:3])",
        "{\"m\":null,\"x\":[1],\"z\":2,\"o\":{\"p\":{\"q\":1}}}\n",
    ),
    (
        "del(.o[], .[2:3])",
        "{\"m\":null,\"x\":[1],\"z\":2,\"o\":{\"p\":{\"q\":1}}}\n",
    ),
    (
        "del(.missing[], .[2:3])",
        "{\"m\":null,\"x\":[1],\"z\":2,\"o\":{\"p\":{\"q\":1}}}\n",
    ),
    (
        "del(.missing.y, .[2:3])",
        "{\"m\":null,\"x\":[1],\"z\":2,\"o\":{\"p\":{\"q\":1}}}\n",
    ),
    (
        "del(.[2:3], .x[0])",
        "{\"m\":null,\"x\":[],\"z\":2,\"o\":{\"p\":{\"q\":1}}}\n",
    ),
    (
        "del(.[2:3], .x[-1])",
        "{\"m\":null,\"x\":[],\"z\":2,\"o\":{\"p\":{\"q\":1}}}\n",
    ),
];

#[test]
fn test_1325_d_branches_before_the_stop_that_create_nothing() -> Result<()> {
    for (filter, want) in NESTED_ROWS {
        let (out, code) = run_yq_stdin(filter, NESTED, &["-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(&out, want, "{filter}");
    }
    // An index below the start raises before the stop is reached.
    let (out, code) = run_yq_stdin("del(.x[-5], .[2:3])", NESTED, &["-o=json", "-I=0"])?;
    assert_eq!((out.as_str(), code), ("", 1));
    Ok(())
}

#[test]
fn test_1325_a_a_bare_root_slice_stops_the_delete_over_an_object() -> Result<()> {
    for (filter, want) in OBJECT_ROWS {
        let (out, code) = run_yq_stdin(filter, OBJECT, &["-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(&out, want, "{filter}");
    }
    Ok(())
}

#[test]
fn test_1325_b_a_bare_root_slice_stops_the_delete_over_an_array() -> Result<()> {
    for (filter, want) in ARRAY_ROWS {
        let (out, code) = run_yq_stdin(filter, ARRAY, &["-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(&out, want, "{filter}");
    }
    Ok(())
}

/// yq resolves every path before it deletes any, so a branch that raises still raises when a bare
/// root slice stands after it (the message differs; the exit status agrees).
#[test]
fn test_1325_c_an_error_branch_before_the_stop_still_raises() -> Result<()> {
    for filter in ["del(.a[0:2].x, .[2:3])", "del(.[2:3], .a[0:2].x)"] {
        let (out, code) = run_yq_stdin(filter, r#"{"a":[1,2,3],"b":6}"#, &["-o=json", "-I=0"])?;
        assert_eq!((out.as_str(), code), ("", 1), "{filter}");
    }
    Ok(())
}
