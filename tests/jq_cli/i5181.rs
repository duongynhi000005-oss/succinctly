//! #5181: a plain `as $q` bind, a `?//` chain and a head wrapped in `first(..)`/`limit(N; ..)` over a navigation of a `$x`
//! marker onto the node jq's register stands on.

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

// #4827 and #5180 let a destructure whose source navigates from a `$x` marker pass jq's register check when the node it
// reaches shares storage with the register. Four neighbouring spellings reach the same node and were still refused at the
// step: a `?//` chain (the head was only read for a single pattern), a head wrapped in `first(..)`/`limit(N; ..)`, and a
// plain `as $q` bind (bound as a plain value, with no mark to say it is the register). jq passes all of them.

const DOC: &str = r#"{"a":{"b":1,"c":{"b":3}},"b":2}"#;

/// #5181: a plain `as $q` bind of a marker navigation onto the register, used as the path and navigated from.
#[test]
fn test_5181_plain_bind_of_a_marker_navigation_onto_the_register() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            DOC,
            r"path(. as $x | .a | $x.a as $q | $q)",
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            DOC,
            r"path(. as $x | .a | $x.a as $q | $q | .b)",
            "[\"a\",\"b\"]\n",
            "",
            0,
        ),
        (
            DOC,
            r"del(. as $x | .a | try ($x.a as $q | $q))",
            "{\"b\":2}\n",
            "",
            0,
        ),
        (
            DOC,
            r"path(. as $x | .a.c | $x.a.c as $q | $q)",
            "[\"a\",\"c\"]\n",
            "",
            0,
        ),
        // another node than the register's still refuses
        (
            DOC,
            r"path(. as $x | .a | $x.c as $q | $q)",
            "",
            "Invalid path expression with result null",
            5,
        ),
        (
            DOC,
            r"path(. as $x | .a.c | $x.a as $q | $q.c)",
            "",
            "Invalid path expression near attempt to access element \"c\"",
            5,
        ),
    ])
}

/// #5181: a `?//` chain over a marker navigation, which passes the first step and so never retries unless the body fails.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5181_alternation_over_a_marker_navigation_onto_the_register() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            DOC,
            r"path(. as $x | .a | $x.a as {b:$q} ?// $q | $q)",
            "[\"a\",\"b\"]\n",
            "",
            0,
        ),
        (
            DOC,
            r"path(. as $x | .a | $x | .a as {b:$q} ?// $q | $q)",
            "[\"a\",\"b\"]\n",
            "",
            0,
        ),
        (
            DOC,
            r"path(. as $x | .a | $x | .a as [$q] ?// {b:$q} | $q)",
            "[\"a\",\"b\"]\n",
            "",
            0,
        ),
        (
            DOC,
            r"del(. as $x | .a | try ($x.a as {b:$q} ?// $q | .b))",
            "{\"a\":{\"c\":{\"b\":3}},\"b\":2}\n",
            "",
            0,
        ),
        // a head that is not the register: the first step raises and the chain retries onto the bare alternative, as in jq
        (
            DOC,
            r"path(. as $x | .a | $x | .b as {b:$q} ?// $q | $q)",
            "",
            "Invalid path expression with result 2",
            5,
        ),
    ])
}

/// #5181: heads wrapped in `first(..)` and `limit(N; ..)` over the marker's `.`.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5181_wrapped_head_over_a_marker_navigation_onto_the_register() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            DOC,
            r"path(. as $x | .a | $x | first(.a) as {b:$q} | $q)",
            "[\"a\",\"b\"]\n",
            "",
            0,
        ),
        (
            DOC,
            r"path(. as $x | .a | $x | (.a | limit(1; .)) as {b:$q} | $q)",
            "[\"a\",\"b\"]\n",
            "",
            0,
        ),
        (
            DOC,
            r"path(. as $x | .a | $x | limit(2; .a) as {b:$q} | $q)",
            "[\"a\",\"b\"]\n",
            "",
            0,
        ),
        (
            DOC,
            r"del(. as $x | .a | $x | try (first(.a) as {b:$q} | $q))",
            "{\"a\":{\"c\":{\"b\":3}},\"b\":2}\n",
            "",
            0,
        ),
    ])
}
