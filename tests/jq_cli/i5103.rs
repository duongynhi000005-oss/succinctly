//! #5103: a register spelled through a slice that spans the whole of a non-empty array (`.a[0:]`), with `$x` bound at
//! its container, or the other way round.

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

// jq's array slice is a view (`jvp_array_slice` moves the offset and size over the shared buffer) and `jv_identical`
// compares pointer, offset and size, so `.a[0:]` of a non-empty array *is* `.a`: `$x` bound at one is the register
// standing at the other, its destructure passes, and jq's later answer or error follows. The slice of an empty array
// is a fresh `[]`, so there `$x` is provably another node and its refusal is jq's catchable one. #5095 read every
// slice as possibly the register and refused uncatchably; every row below exited 5.

/// #5103: the issue's rows and their `first(.a,.b)`/`(.a,.b)` binds, on a non-empty array (`$x` is the register) and an
/// empty one (`$x` is not), under `try`, `?` and bare. Every row captured from `/usr/bin/jq` 1.7.1, both routes.
#[test]
fn test_5103_register_through_a_whole_slice_with_x_at_its_container() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r#"try path(.a as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"[path(.a as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r#"try path(.a[0:] as $x | .a | foreach ($x as [$q] | .) as $y (.; .a?; 1)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"[path(.a[0:] as $x | .a | foreach ($x as [$q] | .) as $y (.; .a?; 1))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r#"try path(first(.a,.b) as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"[path(first(.a,.b) as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r#"try path((.a,.b) as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"[path((.a,.b) as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"path(.a as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1))",
            "",
            "Invalid path expression near attempt to access element \"a\" of [{\"b\":1},{\"b\":1}]",
            5,
        ),
        (
            r#"{"a":[],"b":[]}"#,
            r#"try path(.a as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":[],"b":[]}"#,
            r"[path(.a as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":[],"b":[]}"#,
            r#"try path(.a[0:] as $x | .a | foreach ($x as [$q] | .) as $y (.; .a?; 1)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":[],"b":[]}"#,
            r"[path(.a[0:] as $x | .a | foreach ($x as [$q] | .) as $y (.; .a?; 1))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":[],"b":[]}"#,
            r#"try path(first(.a,.b) as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":[],"b":[]}"#,
            r"[path(first(.a,.b) as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":[],"b":[]}"#,
            r#"try path((.a,.b) as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1)) catch "c""#,
            "\"c\"\n",
            "",
            0,
        ),
        (
            r#"{"a":[],"b":[]}"#,
            r"[path((.a,.b) as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1))?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":[],"b":[]}"#,
            r"path(.a as $x | .a[0:] | foreach ($x as [$q] | .) as $y (.; .a?; 1))",
            "",
            "Invalid path expression near attempt to access element 0 of []",
            5,
        ),
    ])
}

/// #5103: the slice-spelled register answers in jq's own spelling (`["a",{"start":0,"end":null}]`), a `del` through it
/// deletes the slice, an assignment raises jq's slice error, and an equal-valued sibling bound as `$x` (`(.a,.b)`) is
/// provably another node, so its refusal is caught.
#[test]
fn test_5103_whole_slice_names_the_node_it_slices() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"path(.a as $x | .a[0:] | $x)",
            "[\"a\",{\"start\":0,\"end\":null}]\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"path(.a[0:] as $x | .a | $x)",
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"path(.a as $x | .a[0:] | $x | .[0])",
            "[\"a\",{\"start\":0,\"end\":null},0]\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"path(.a as $x | .a[-9:] | $x)",
            "[\"a\",{\"start\":-9,\"end\":null}]\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"path(.a[1:] as $x | .a[1:][0:] | $x)",
            "[\"a\",{\"start\":1,\"end\":null},{\"start\":0,\"end\":null}]\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"del(.a as $x | .a[0:] | $x)",
            "{\"a\":[]}\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"(.a as $x | .a[0:] | $x) |= 9",
            "",
            "A slice of an array can only be assigned another array",
            5,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"path(.a as $x | .a[0:] | ($x as [$q] | .) | .[1])",
            "",
            "Invalid path expression near attempt to access element 1 of [{\"b\":1},{\"b\":1}]",
            5,
        ),
        (
            r#"{"a":[[1]],"b":[[1]]}"#,
            r#"try path((.a,.b) as $x | .a | .[0:] | $x as [$q] | $q) catch "C""#,
            "[\"a\",{\"start\":0,\"end\":null},0]\n\"C\"\n",
            "",
            0,
        ),
        (
            r#"{"a":[1],"b":[1]}"#,
            r#"try path((.a,.b) as $x | .a[-9:] | reduce ($x as [$q] | .) as $y (.; .)) catch "C""#,
            "[\"a\",{\"start\":-9,\"end\":null}]\n\"C\"\n",
            "",
            0,
        ),
    ])
}

/// #5103: a slice that does not span the whole array (`[1:]`, `[0:0]`), the slice of an empty array and a string slice
/// are not the node they slice, so jq refuses (or `?` empties) and so does this.
#[test]
fn test_5103_partial_empty_and_string_slices_stay_other_nodes() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"path(.a as $x | .a[1:] | $x)",
            "",
            "Invalid path expression with result [{\"b\":1},{\"b\":1}]",
            5,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"path(.a as $x | .a[0:][1:] | $x)",
            "",
            "Invalid path expression with result [{\"b\":1},{\"b\":1}]",
            5,
        ),
        (
            r#"{"a":[{"b":1},{"b":1}]}"#,
            r"[path(.a as $x | .a[0:0] | $x)?]",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":[]}"#,
            r"path(.a as $x | .a[0:] | $x)",
            "",
            "Invalid path expression with result []",
            5,
        ),
        (
            r#"{"a":[]}"#,
            r"path(.a[0:] as $x | .a[0:] | $x)",
            "",
            "Invalid path expression with result []",
            5,
        ),
        (
            r#"{"a":"ab"}"#,
            r"path(.a as $x | .a[0:] | $x)",
            "",
            "Invalid path expression with result \"ab\"",
            5,
        ),
    ])
}
