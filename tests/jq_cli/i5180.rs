//! #5180: a destructure whose source navigates two or more steps from a `$x` marker, or iterates it, onto the node jq's
//! register stands on.

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

// jq's `path_intact` compares the source's `jv` with the register by pointer. `$x.a.c` is the child handle of `$x`'s node,
// so it is the register after a stage `.a.c`, and the destructure's first step passes. The resolver took one step from the
// marker (#4827) but a head of two or more steps was evaluated by value, which bridges the second step through a fresh
// copy: the pointer test failed, the first step was refused, and a `try` swallowed the refusal, so `del`/`|=` left the
// document where jq writes (and `path` raised where jq answers). The head is now also walked straight off the marker's node.

/// #5180: the issue's rows, `del` and `path`, both routes. Every row captured from `/usr/bin/jq` 1.7.1.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5180_multi_step_navigation_of_a_marker_onto_the_register_node() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"c":{"d":{"b":1}}}}"#,
            r"del(. as $x | .a.c | try ($x.a.c as {d:$q} | $q))",
            "{\"a\":{\"c\":{}}}\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1}]}"#,
            r"del(. as $x | .a[0] | try ($x.a[0] as {b:$q} | $q))",
            "{\"a\":[{}]}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"c":{"d":{"b":1}}}}"#,
            r"(. as $x | .a.c | try ($x.a.c as {d:$q} | $q)) |= 9",
            "{\"a\":{\"c\":{\"d\":9}}}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":{"c":1}}}"#,
            r"del(. as $x | foreach (try ($x.a.b as {c:$q} | .)) as $y (.a.b; .; .))",
            "",
            "Invalid path expression with result {\"c\":1}",
            5,
        ),
        (
            r#"{"a":{"c":{"d":{"b":1}}}}"#,
            r"path(.a as $x | .a.c.d | $x.c.d as {b:$q} | $q)",
            "[\"a\",\"c\",\"d\",\"b\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#,
            r"path(. as $x | .a.c | $x | .a.c as {b:$q} | $q)",
            "[\"a\",\"c\",\"b\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":[{"b":1},{"b":2}]}"#,
            r"[path(. as $x | .a[1] | $x.a[-1] as {b:$q} | $q)]",
            "[[\"a\",1,\"b\"]]\n",
            "",
            0,
        ),
    ])
}

/// #5180: the `[]` head, which yields several nodes of which only one is the register.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5180_iterating_a_marker_places_the_register_among_its_outputs() -> Result<()> {
    assert_path_rows_both_routes_3749(&[(
        r#"{"a":{"b":1,"c":{"b":3}},"b":{"b":2}}"#,
        r"del(. as $x | .a | try ($x[] as {b:$q} | $q))",
        "{\"a\":{\"c\":{\"b\":3}},\"b\":{\"b\":2}}\n",
        "",
        0,
    )])
}

/// #5180: a head that reaches another node than the register's still refuses, or is caught, as before.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5180_a_head_reaching_another_node_is_still_refused() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"c":{"d":{"b":1}}}}"#,
            r"path(. as $x | .a.c | $x.a.b as {b:$q} | $q)",
            "",
            "Invalid path expression near attempt to access element \"b\" of null",
            5,
        ),
        (
            r#"{"a":{"c":{"d":{"b":1}}}}"#,
            r"path(. as $x | .a | $x.a.c as {d:$q} | $q)",
            "",
            "Invalid path expression near attempt to access element \"d\" of",
            5,
        ),
        (
            r#"{"a":{"c":{"d":{"b":1}}}}"#,
            r"path(. as $x | .a.c | try ($x.a.d as {b:$q} | $q))",
            "",
            "",
            0,
        ),
    ])
}
