//! #5273: growing one array, object or string in a `reduce` accumulator with `.a |= (. // C) + R`,
//! `.a = .a + R`, `.a = (.a // C) + R` or `.a += R` extends the value in its slot instead of
//! copying it on every step. These pin that the answers -- and the errors, which still come from
//! the evaluator -- are jq 1.7.1's (captured live).

use anyhow::Result;

use crate::run_jq_full;

#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_growing_one_slot_answers_as_jq_5273() -> Result<()> {
    for (filter, want) in [
        (
            "reduce range(4) as $x ({}; .a |= (. // []) + [$x])",
            r#"{"a":[0,1,2,3]}"#,
        ),
        (
            "reduce range(4) as $x ({}; .a = .a + [$x])",
            r#"{"a":[0,1,2,3]}"#,
        ),
        (
            "reduce range(4) as $x ({}; .a = (.a // []) + [$x])",
            r#"{"a":[0,1,2,3]}"#,
        ),
        (
            "reduce range(4) as $x ({}; .a += {($x|tostring): $x})",
            r#"{"a":{"0":0,"1":1,"2":2,"3":3}}"#,
        ),
        (
            "reduce range(4) as $x ({}; .a |= (. // {}) + {($x|tostring): $x})",
            r#"{"a":{"0":0,"1":1,"2":2,"3":3}}"#,
        ),
        // A falsy old value takes the default; `null` and an absent key too.
        (
            "reduce range(3) as $x ({a:false}; .a |= (. // []) + [$x])",
            r#"{"a":[0,1,2]}"#,
        ),
        (
            "reduce range(3) as $x ({a:false}; .a = (.a // []) + [$x])",
            r#"{"a":[0,1,2]}"#,
        ),
        (
            "reduce range(3) as $x ({a:null}; .a = .a + [$x])",
            r#"{"a":[0,1,2]}"#,
        ),
        (
            r#"reduce range(3) as $x ({a:"s"}; .a |= (. // "") + "t")"#,
            r#"{"a":"sttt"}"#,
        ),
        (
            r#"reduce range(3) as $x ({a:"s"}; .a = .a + "t")"#,
            r#"{"a":"sttt"}"#,
        ),
        // An index step, a longer path, and a read of a different slot.
        (
            "reduce range(3) as $x ([[]]; .[0] = .[0] + [$x])",
            "[[0,1,2]]",
        ),
        (
            "reduce range(3) as $x ([[]]; .[0] |= (. // []) + [$x])",
            "[[0,1,2]]",
        ),
        (
            "reduce range(3) as $x ({a:{b:[]}}; .a.b |= (. // []) + [$x])",
            r#"{"a":{"b":[0,1,2]}}"#,
        ),
        (
            "reduce range(3) as $x ({a:{b:[]}}; .a.b = .a.b + [$x])",
            r#"{"a":{"b":[0,1,2]}}"#,
        ),
        (
            "reduce range(3) as $x ({b:[9]}; .a = .b + [$x])",
            r#"{"b":[9],"a":[9,2]}"#,
        ),
        (
            "reduce range(3) as $x ({a:[1],b:[9]}; .a = .b + [$x])",
            r#"{"a":[9,2],"b":[9]}"#,
        ),
        // A default that is never taken, and an empty right side.
        (
            "reduce range(3) as $x ({a:[1]}; .a = (.a // 5) + [$x])",
            r#"{"a":[1,0,1,2]}"#,
        ),
        (
            "reduce range(3) as $x ({a:[]}; .a = .a + [])",
            r#"{"a":[]}"#,
        ),
        // Growing a copy leaves the value it was copied from alone.
        (
            "{a:[1,2]} as $o | ($o | .a = .a + [3]) as $n | [$o, $n]",
            r#"[{"a":[1,2]},{"a":[1,2,3]}]"#,
        ),
        (
            "{a:[1,2]} as $o | ($o | .a |= (. // []) + [3]) as $n | [$o, $n]",
            r#"[{"a":[1,2]},{"a":[1,2,3]}]"#,
        ),
        (
            "{a:[1,2]} as $o | ($o | .a += [3]) as $n | [$o, $n]",
            r#"[{"a":[1,2]},{"a":[1,2,3]}]"#,
        ),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-n", "-c", filter], None)?;
        assert_eq!((stdout.trim_end(), code), (want, 0), "`{filter}`: {stderr}");
    }
    Ok(())
}

/// A step that cannot grow its slot keeps the evaluator's own error: the old value and the right
/// side are not two containers of one kind, so nothing is taken out of the slot.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_failing_growth_keeps_the_error_5273() -> Result<()> {
    for (filter, want) in [
        (
            "reduce range(3) as $x ({a:1}; .a |= (. // []) + [$x])",
            "number (1) and array ([0]) cannot be added",
        ),
        (
            "reduce range(3) as $x ({a:1}; .a = .a + [$x])",
            "number (1) and array ([0]) cannot be added",
        ),
        (
            "reduce range(3) as $x ({a:{}}; .a += [$x])",
            "object ({}) and array ([0]) cannot be added",
        ),
        (
            "reduce range(3) as $x ([]; .a = .a + [$x])",
            r#"Cannot index array with string "a""#,
        ),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-n", "-c", filter], None)?;
        assert_eq!((stdout.as_str(), code), ("", 5), "`{filter}`: {stderr}");
        assert!(stderr.trim_end().ends_with(want), "`{filter}`: {stderr}");
    }
    Ok(())
}
