//! #5004: a `catch` handler's output in a `foreach` UPDATE over a moving source, when the UPDATE is a `try` around the
//! caught foreign-`$x` `foreach` of #4800 (`FE'` = `foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))`, whose
//! EXTRACT raises).

use crate::{assert_path_rows_both_routes_3749, run_jq_full};
use anyhow::Result;

/// #5004: at the step whose element is the root, `$x` is jq's register, the pattern's step moves it onto `.a` and the
/// nested `foreach`'s EXTRACT raises. The `try` backtracks to its fork point, which restores the register to where it
/// entered (the root element), and the handler's output is emitted there, so the `$i` EXTRACT (the root) is
/// `jv_identical` to it and the path is `[]`. The handler was seeded with no register (the fold's frame carries none),
/// so `$i` was refused as a guess: `path`, `|=` and `del` raised `with result <doc>` and `[path(..)?]` dropped jq's
/// `[]`. For a `null` landing and a container landing, an emitting handler of either kind. Rows captured from jq 1.7.1
/// (stdout, first stderr line, exit code), both routes.
#[test]
fn test_foreach_update_try_handler_output_sits_at_the_try_entry_5004() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":null}"#,
            r#"[path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch (.); $i))?]"#,
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch (.); $i))"#,
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch (.); $i)) |= 9"#,
            "9\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"del(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch (.); $i))"#,
            "null\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"[path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch 7; $i))?]"#,
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch 7; $i))"#,
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch 7; $i)) |= 9"#,
            "9\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r#"del(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch 7; $i))"#,
            "null\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r#"[path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch (.); $i))?]"#,
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r#"path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch (.); $i))"#,
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r#"(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch (.); $i)) |= 9"#,
            "9\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r#"del(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch (.); $i))"#,
            "null\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r#"[path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch 7; $i))?]"#,
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r#"path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch 7; $i))"#,
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r#"(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch 7; $i)) |= 9"#,
            "9\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r#"del(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error("x"))) catch 7; $i))"#,
            "null\n",
            "",
            0,
        ),
    ])
}

/// #5004: the `try` may sit behind a value head or a `select(true)` stage (`null | (try (FE) catch 7)`), whose pipe
/// hands it on copied and carries the register on its frame: the handler is still seeded with the register the step
/// entered its UPDATE with, and its statement is read back. Rows captured from jq 1.7.1, both routes.
#[test]
fn test_foreach_update_try_handler_behind_a_value_head_5004() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":[1]}"#,
            r#"del(. as $x | foreach (.[]?) as $i (.; not | (try (foreach ($x as {a:$a} | .) as $y (0; .a?; error("x"))) catch $x); $i))"#,
            "{}\n",
            "",
            0,
        ),
        (
            r#"{"a":[null]}"#,
            r"(. as $x | foreach (.a,.) as $i (null; null | (try (foreach ($x as {a:$a} | .) as $y (null; empty; error($x))) catch $__loc__); $i)) = 9",
            "{\"a\":9}\n",
            "",
            0,
        ),
        (
            r#"{"a":[null]}"#,
            r#"[del(. as $x | foreach (.) as $i (.; not | (try (foreach (try ($x as {a:$a} | .)) as $y (.; .a?; (., error("x")))) catch 7); $i | .a))?]"#,
            "[{}]\n",
            "",
            0,
        ),
        (
            r#"{"a":[null]}"#,
            r#"[path(. as $x | foreach ((.a,.),.) as $i (0; not | (try (foreach (try ($x as {a:$a} | .)) as $y (0; $y; error("x"))) catch true); $x))?]"#,
            "[[],[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":[null]}"#,
            r"[path(. as $x | foreach (.,.a) as $i (0; null | (try (foreach (($x as {a:$a} | .)?) as $y (.; .a?; empty)) catch 7); $i))?]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":true}"#,
            r"[del(. as $x | foreach ((.,.)) as $i (null; not | (try (foreach (try ($x as {a:$a} | .)) as $y (.; .; error({}))) catch null); $i))?]",
            "[null]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":1}}"#,
            r"(. as $x | foreach (.a,.) as $i (.; (try (foreach (try ($x as {b:$a} | .)) as $y (.; .; error({}))) catch 7) | .; $i | .a)) = 9",
            "{\"a\":9}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":1}}"#,
            r#"try path(. as $x | foreach (..) as $i (null; select(true) | (try (foreach (($x as {a:$a} | .)?) as $y (.; .a?; .)) catch 7); $i)) catch "C""#,
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r#"try path(. as $x | foreach ($x) as $i (.; null | (try (foreach (try ($x as {a:$a} | .)) as $y (.; .a?; error(.))) catch $__loc__); $x)) catch "C""#,
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":true}}"#,
            r"(. as $x | foreach (.a) as $i (.; not | (try (foreach ($x as {a:$a} | .) as $y (null; empty; error({}))) catch (.)); $i)) = 9",
            "{\"a\":9}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":true}}"#,
            r#"try path(. as $x | foreach ((.,.)) as $i (.; not | (try (foreach ($x as {a:$a} | .) as $y (.; .; error)) catch "h"); $i | .a)) catch "C""#,
            "[\"a\"]\n[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":{"a":null}}}"#,
            r"path(. as $x | foreach (.a,.) as $i (.; (try (foreach (try ($x as {b:$a} | .)) as $y (.; $y; error(.))) catch true) | .; $i))",
            "[]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":2}}"#,
            r"(. as $x | foreach (.) as $i (0; null | (try (foreach (try ($x as {b:$a} | .)) as $y (null; $y; error({}))) catch true); $i | .a)) = 9",
            "{\"a\":9}\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":2}}"#,
            r"del(. as $x | foreach ((.,.)) as $i (null; (try (foreach (try ($x as {a:$a} | .)) as $y (0; .; error($x))) catch null) | .; $x))",
            "null\n",
            "",
            0,
        ),
        (
            r#"{"a":{"b":2}}"#,
            r#"try path(. as $x | foreach (.a,.) as $i (.; select(true) | (try (foreach (try ($x as {b:$a} | .)) as $y (null; $y; error(.))) catch length); $i | .a)) catch "C""#,
            "[\"a\"]\n",
            "",
            0,
        ),
        (
            r#"{"b":null}"#,
            r"[del(. as $x | foreach (.) as $i (.; null | (try (foreach (try ($x as {a:$a} | .)) as $y (.; .a?; error(null))) catch (empty, 7)); $i | .a))?]",
            "[{\"b\":null}]\n",
            "",
            0,
        ),
    ])
}

/// #5004: where the resolver does not model whether the nested `foreach` raises at all, its handler is not seeded and
/// the refusal stays, with the wording it had (jq answers these). A two-entry pattern (`{a:$a,b:$b}`: jq's second step
/// is emptied by the source's own `try`, so jq's handler never runs where this one would), an outer `?//` chain (whose
/// first alternative jq retries on the body's raise), a nested `foreach` whose INIT navigates (an earlier step's raise
/// the resolver does not see), and a source that is a comma of caught destructures (jq's `try`s empty each item):
/// seeding the handler there answered or wrote what jq does not (`= 9` wrote `{"a":9}`, `del` wrote `{"a":{}}`).
/// Loud, not wrong.
#[test]
fn test_foreach_update_try_handler_where_the_nested_foreach_is_not_modelled_stays_refused_5004(
) -> Result<()> {
    for (input, filter, message) in [
        (
            r#"{"a":[1]}"#,
            r"(. as $x | foreach (.) as $i (0; try (foreach (try ($x as {a:$a,b:$b} | .)) as $y (.; .; .a)) catch $x; $i | .a)) |= 9",
            "Invalid path expression near attempt to access element \"a\" of {\"a\":[1]}",
        ),
        (
            r#"{"a":[1]}"#,
            r"[path(. as $x | foreach (.,.a) as $i (0; try (foreach (try ($x as {a:$a,b:$b} | .)) as $y (.; $y; error($x))) catch $x; .a?))]",
            "Invalid path expression near attempt to access element \"a\" of {\"a\":[1]}",
        ),
        (
            r#"{"a":false}"#,
            r"(. as $x | foreach (1,2) as $i (0; (try (foreach (try ($x as {a:$a,b:$b} | .)) as $y (.; .; error({}))) catch null)?; $x)) = 9",
            "Invalid path expression with result {\"a\":false}",
        ),
        (
            r#"{"a":{"a":{"a":null}}}"#,
            r"path(. as $x | foreach (.a,.) as $i (0; try (foreach (try ($x as {a:$a,b:$b} | .)) as $y (null; .; error($x))) catch (.); $i))",
            "Invalid path expression with result {\"a\":{\"a\":{\"a\":null}}}",
        ),
        (
            r#"{"a":1}"#,
            r"(. as $x | foreach (.,.a) as $i (0; first(try (foreach (try ($x as {a:$a,b:$b} | .)) as $y (0; .a?; error(.))) catch $x); .)) |= 9",
            "Invalid path expression with result {\"a\":1}",
        ),
        (
            r#"{"a":1}"#,
            r"path(. as $x | (foreach ($x) as $i ?// [$i] (0; try (foreach (try ($x as {a:$a,b:$b} | .)) as $y (.; .; error)) catch 7; $i) | .))",
            "Cannot index object with number",
        ),
        (
            r#"{"a":true}"#,
            r"(. as $x | foreach ((.a,.),.) as $i (.; try (foreach (try ($x as {a:$a,b:$b} | .)) as $y (.; $y; error({}))) catch (., 1); $i)) = 9",
            "Invalid path expression with result {\"a\":true}",
        ),
        (
            r#"{"a":{"a":1}}"#,
            r"del(. as $x | foreach ((.a,.),.) as $i (.; try (foreach (try ($x as {a:$a,b:$b} | .)) as $y (null; $y; error({}))) catch null; $i))",
            "Invalid path expression with result {\"a\":{\"a\":1}}",
        ),
        (
            r#"{"a":{"a":true}}"#,
            r"(. as $x | foreach (.,.a) as $i (.; (try (foreach ($x as {a:$a} | .) as $y (.a?; empty; error($x))) catch null)?; $i)) = 9",
            "Invalid path expression with result {\"a\":true}",
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[path(. as $x | foreach (.a) as {a:$q} ?// $i (.; try (foreach ($x as {a:$a} | .) as $y (.; $y; $y)) catch null; $i))]",
            "Invalid path expression with result {\"a\":null}",
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"del(. as $x | foreach (.a) as $i (.; null | (try (foreach (try ($x as {a:$a} | .), try ($x as {a:$a} | .)) as $y (null; .; error($x))) catch null); $i | .a?))",
            "Invalid path expression near attempt to access element \"a\" of {\"a\":null}",
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"(. as $x | foreach (.a) as $i (.; null | (try (foreach (try ($x as {a:$a} | .), try ($x as {a:$a} | .)) as $y (null; .; error($x))) catch null); $i | .a?)) |= 9",
            "Invalid path expression near attempt to access element \"a\" of {\"a\":null}",
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[path(. as $x | foreach (.a) as $i (.; null | (try (foreach (try ($x as {a:$a} | .), try ($x as {a:$a} | .)) as $y (null; .; error($x))) catch null); $i | .a?))]",
            "Invalid path expression near attempt to access element \"a\" of {\"a\":null}",
        ),
    ] {
        for (stdout, stderr, code) in [
            run_jq_full(&["-c", filter], Some(input))?,
            run_jq_full(&["-nc", &format!("{input} | {filter}")], None)?,
        ] {
            assert_eq!(
                (stdout.as_str(), code),
                ("", 5),
                "{input} | {filter}: stderr {stderr:?}"
            );
            assert!(stderr.contains(message), "{input} | {filter}: {stderr:?}");
        }
    }
    Ok(())
}

/// #5004: a step that destructures (`as {a:$i}`) is not read: its walk moves the register in ways the step does not
/// report, and #4978's refusal of a container landing behind a handler that emits stands there. Reading it answered
/// `{"a":[null]}` for jq's error from `|= 9` and changed two messages. Rows captured from jq 1.7.1, both routes.
#[test]
fn test_foreach_update_try_handler_behind_a_destructuring_step_is_not_read_5004() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":[1]}"#,
            r#"(. as $x | (foreach ((.,.)) as {a:$i} (null; try (foreach ($x as {a:$a} | .) as $y (.; .; null)) catch error("y"); $i) | .)) = 9"#,
            "",
            "y",
            5,
        ),
        (
            r#"{"a":[null]}"#,
            r"(. as $x | foreach ((.,.)) as {a:$i} (null; try (foreach ($x as {a:$a} | .) as $y (.; empty; $y)) catch (., 1); .a?)) |= 9",
            "",
            "Invalid path expression near attempt to access element \"a\" of \"Invalid path expression n...",
            5,
        ),
        (
            r#"{"a":[null]}"#,
            r"[path(. as $x | foreach ($x) as {a:$i} (null; try (foreach ($x as {a:$a} | .) as $y (null; .; null)) catch (.a?); 7))]",
            "",
            "Invalid path expression near attempt to access element \"a\" of \"Invalid path expression n...",
            5,
        ),
    ])
}

/// #5004: a `break` to a label outside the `foreach` is caught by the `try` too (its handler gets jq's label object),
/// and jq runs that handler from the same restored register: the `$i` EXTRACT is the root, `[]`. Rows captured from
/// jq 1.7.1, both routes.
#[test]
fn test_foreach_update_try_handler_of_a_caught_break_sits_at_the_try_entry_5004() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            r#"{"a":null}"#,
            r"[label $out | path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch (.); $i))]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[label $out | path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch (.); $i))?]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"label $out | del(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch (.); $i))",
            "null\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[label $out | path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch 7; $i))]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[label $out | path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch 7; $i))?]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"label $out | del(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch 7; $i))",
            "null\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[label $out | path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch $i; $i))]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"[label $out | path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch $i; $i))?]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":null}"#,
            r"label $out | del(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch $i; $i))",
            "null\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[label $out | path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch (.); $i))]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[label $out | path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch (.); $i))?]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"label $out | del(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch (.); $i))",
            "null\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[label $out | path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch 7; $i))]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[label $out | path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch 7; $i))?]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"label $out | del(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch 7; $i))",
            "null\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[label $out | path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch $i; $i))]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"[label $out | path(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch $i; $i))?]",
            "[[]]\n",
            "",
            0,
        ),
        (
            r#"{"a":{"a":null}}"#,
            r"label $out | del(. as $x | foreach (.a,.) as $i (.; try (foreach (try ($x as {a:$a} | .)) as $y (.; .; break $out)) catch $i; $i))",
            "null\n",
            "",
            0,
        ),
    ])
}
