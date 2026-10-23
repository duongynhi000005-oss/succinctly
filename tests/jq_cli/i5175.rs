//! #5175: a `//` over a bind of an outer `$x` in a `reduce` or two-argument `foreach` UPDATE whose right operand holds a
//! bind of its own.

use anyhow::Result;

use crate::{assert_path_rows_both_routes_3749, run_jq_stdin};

// #5167 read the right operand of the `//` only when it held no bind. An operand with one (`.a as $q | 1`,
// `$x as {a:$a} | .`) kept the resolver's refusal, a guess that a `try`/`?` around the `//` swallowed, so the UPDATE
// emitted nothing and `path`/`del`/`|=` answered (or wrote) where jq raises at the emission of the left operand's
// truthy output. jq runs the right operand only after the left one backtracked, from the register the `//` was entered
// with, and runs a bind's source inside a subexpression that tracks no path, so the operand is read as the left one is: a
// chain of binds that each run once, whose source is `.`, an outer `$x` or a navigation of `.`.

/// #5175: the issue's row 2 (a navigation-source bind in the right operand), for both folds, and the `|=` write that ran
/// where jq raises. Every row captured from `/usr/bin/jq` 1.7.1, both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5175_alternative_with_a_navigation_source_bind_in_the_right_operand() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | foreach (1,2) as $i (false; try (($x as {a:$a} | 5) // .a as $q | 1)))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | reduce (1,2) as $i (false; try (($x as {a:$a} | 5) // .a as $q | 1)))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r"null",
            r"(. as $x | foreach (.a,.) as $i (5; try (($x as {a:$a} | .) // (.a as $q | $q)))) |= 9",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r"null",
            r"(. as $x | reduce (.a,.) as $i (5; try (($x as {a:$a} | .) // (.a as $q | $q)))) |= 9",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r"null",
            r"path(. as $x | reduce (.,.a) as $i (null; try (($x as [$a] | 5) // (.a as $q | $q))))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | foreach (.,.a) as $i (null; try (($x as {a:{b:$a}} | .) // .a as $q | 1)))",
            "",
            "Invalid path expression with result 1",
            5,
        ),
    ])
}

/// #5175: a right operand that destructures the outer `$x` again, which jq runs from the register the `//` was entered with.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5175_alternative_with_a_destructuring_bind_in_the_right_operand() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1}}"#,
            r"del(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | .) // ($x as {a:$a} | .))))",
            "",
            "Invalid path expression with result null",
            5,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"del(. as $x | reduce (1,2) as $i (null; try (($x as {a:$a} | .) // ($x as {a:$a} | .))))",
            "",
            "Invalid path expression with result null",
            5,
        ),
        // a falsy left output hands the register back, so the operand's own bind runs from it and the fold ends clean
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | foreach (1,2) as $i (.; try (($x as {a:$a} | false) // .a as $q | .)))",
            "[]\n[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | reduce (1,2) as $i (.; try (($x as {a:$a} | false) // .a as $q | .)))",
            "[]\n",
            "",
            0,
        ),
    ])
}

/// #5175: the three-argument `foreach`, whose EXTRACT is the emission, reads the same right operand. Rows captured from
/// `/usr/bin/jq` 1.7.1, both routes.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5175_three_argument_foreach_with_a_bind_in_the_right_operand() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | foreach (1,2) as $i (false; try (($x as {a:$a} | 5) // .a as $q | 1); .))",
            "",
            "Invalid path expression with result 5",
            5,
        ),
        (
            r"null",
            r"(. as $x | foreach (.a,.) as $i (5; try (($x as {a:$a} | .) // (.a as $q | $q)); $i)) |= 9",
            "{\"a\":9}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":1}}"#,
            r"path(. as $x | foreach (1,2) as $i (.; try (($x as {a:$a} | false) // .a as $q | .); $i))",
            "",
            "Invalid path expression with result 1",
            5,
        ),
    ])
}

/// #5175: a right operand with a bind behind a left operand that fans out stays refused. The bare reading answered
/// `["a"]` once where jq answers it twice (the #5170 under-count), and `del`/`=` write through such a count.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_5175_left_operand_that_fans_out_is_not_read_with_a_bind_in_the_right_operand() -> Result<()>
{
    let (stdout, _code) = run_jq_stdin(
        r"path(. as $x | foreach . as $i (5; ($x as {a:$a} | (.,.)) // ($x as {a:$a} | .); $x))",
        "null",
        &["-c"],
    )?;
    assert_ne!(
        stdout, "[\"a\"]\n",
        "answered one path where jq answers two"
    );
    Ok(())
}
