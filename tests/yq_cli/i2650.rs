//! #2650: a bare `as` binds from the nodes its source matches, and a key the source does not
//! find matches none. `.zz as $x | $x` is no output in yq (the body runs once with `$x` unbound,
//! #4139) although the plain read `.zz` is `null`; the same holds on the lazy, eager, owned-identity
//! and path-position bind routes. Every expected output is what yq v4.53.3 prints.
//!
//! Only the source is read-only: the body keeps the scope it had.

use anyhow::Result;

use crate::run_yq_stdin;

const DOC: &str = "a:\n  b: 1\nc:\n  b: 1\n";

/// (filter, expected `-o=json -I=0` output).
const ROWS: &[(&str, &str)] = &[
    (".zz as $x | $x", ""),
    ("[.zz as $x | $x]", "[]"),
    (".zz as $x | [$x]", "[]"),
    (".zz as $x | $x | length", ""),
    ("[.zz as $x | $x, 5]", "[5]"),
    ("(.zz as $x | $x) // \"d\"", "\"d\""),
    // The unbound body still runs once.
    (".zz as $x | 1", "1"),
    (".zz.y as $x | 5", "5"),
    // A present key still binds, `null` included.
    (".a.b as $x | $x", "1"),
    ("(.a, .zz) as $x | [$x]", "[{\"b\":1}]"),
    // Through a leading stage, and read by a position builtin.
    (".c | (.a as $x | $x)", ""),
    (".c | (.a as $x | $x) | path", ""),
    (".c | .a as $x | $x | key", ""),
    (".c | .a as $x | $x | parent", ""),
    (".c | .b as $x | $x | path", "[\"c\",\"b\"]"),
    (".c | .a | path", "[\"c\",\"a\"]"),
    // The body is not read-only: a write through an unbound variable's body still creates.
    (
        ".zz as $x | .zz = 3",
        "{\"a\":{\"b\":1},\"c\":{\"b\":1},\"zz\":3}",
    ),
    (".zz as $x | .c.b |= 3", "{\"a\":{\"b\":1},\"c\":{\"b\":3}}"),
    (".a as $x | .zz | [$x.b]", "[1]"),
];

#[test]
fn test_a_variable_bound_from_an_absent_key_has_no_candidates_2650() -> Result<()> {
    for (filter, expected) in ROWS {
        let (output, exit_code) = run_yq_stdin(filter, DOC, &["-o=json", "-I=0"])?;
        assert_eq!(exit_code, 0, "filter: {filter}");
        assert_eq!(output.trim_end(), *expected, "filter: {filter}");
    }
    Ok(())
}
