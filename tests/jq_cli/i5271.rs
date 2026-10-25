//! #5271: `reduce` with a state-reading array operand (`(. // []) + [$x.id]`), `setpath(P; V)` and
//! `. += X` write the accumulator in place instead of copying it on every step. These pin that the
//! answers -- and the errors, which still come from the evaluator -- are jq 1.7.1's (captured live).

use anyhow::Result;

use crate::run_jq_full;

const RECORDS: &str = r#"[{"id":1,"name":"a","tags":["x"],"s":"p"},{"id":2,"name":"b","tags":[],"s":"q"},{"id":3,"name":"a","tags":["y","z"],"s":"r"},{"id":4,"name":"c","tags":null,"s":"s"}]"#;

#[test]
fn test_array_operand_setpath_and_self_append_answer_as_jq_5271() -> Result<()> {
    for (filter, want) in [
        (
            "reduce .[] as $x ({}; .[$x.name] |= (. // []) + [$x.id])",
            r#"{"a":[1,3],"b":[2],"c":[4]}"#,
        ),
        (
            "reduce .[] as $x ({}; .[$x.name] |= (. // []) + $x.tags)",
            r#"{"a":["x","y","z"],"b":[],"c":[]}"#,
        ),
        (
            "reduce .[] as $x ({}; .[$x.name] |= (. // []) + [$x.tags, $x.id])",
            r#"{"a":[["x"],1,["y","z"],3],"b":[[],2],"c":[null,4]}"#,
        ),
        (
            "reduce .[] as $x ({}; .[$x.name] |= [.] + [$x.id])",
            r#"{"a":[[null,1],3],"b":[null,2],"c":[null,4]}"#,
        ),
        (
            "reduce .[] as $x ({}; setpath([$x.id|tostring]; 1))",
            r#"{"1":1,"2":1,"3":1,"4":1}"#,
        ),
        (
            r#"reduce .[] as $x ({}; setpath(["a", $x.id|tostring]; $x.name))"#,
            r#"{"a":{"1":"a","2":"b","3":"a","4":"c"}}"#,
        ),
        (
            "reduce .[] as $x ({}; setpath([$x.name]; (getpath([$x.name]) // 0) + 1))",
            r#"{"a":2,"b":1,"c":1}"#,
        ),
        (
            "reduce .[] as $x ([]; setpath([length]; $x.id))",
            "[1,2,3,4]",
        ),
        ("reduce .[] as $x ({}; setpath([]; $x.id))", "4"),
        ("reduce .[] as $x ([]; . += [$x.id])", "[1,2,3,4]"),
        ("reduce .[] as $x ([]; . += $x.tags)", r#"["x","y","z"]"#),
        (
            "reduce .[] as $x ({}; . += {($x.name): $x.id})",
            r#"{"a":3,"b":2,"c":4}"#,
        ),
        (r#"reduce .[] as $x (""; . += $x.s)"#, r#""pqrs""#),
        ("reduce .[] as $x (null; . += [$x.id])", "[1,2,3,4]"),
        ("reduce .[] as $x ([]; . += [length])", "[0,1,2,3]"),
        ("reduce .[] as $x ([[]]; .[0] += [$x.id])", "[[1,2,3,4]]"),
        (
            "[foreach .[] as $x ({}; setpath([$x.id|tostring]; 1); length)]",
            "[1,2,3,4]",
        ),
    ] {
        let (stdout, stderr, code) =
            run_jq_full(&["-n", "-c", &format!("{RECORDS} | {filter}")], None)?;
        assert_eq!((stdout.trim_end(), code), (want, 0), "`{filter}`: {stderr}");
    }
    Ok(())
}

/// A step that fails keeps the evaluator's own error: the owned step declines (or, for a `setpath`
/// whose write raises, reports the same message) and nothing is written.
#[test]
fn test_failing_setpath_and_self_append_keep_the_error_5271() -> Result<()> {
    for (filter, want) in [
        (
            r#"reduce .[] as $x ({"a":1}; setpath(["a","b"]; 1))"#,
            r#"Cannot index number with string "b""#,
        ),
        (
            r#"reduce .[] as $x ({}; setpath("a"; 1))"#,
            "Path must be specified as an array",
        ),
        (
            "reduce .[] as $x ([]; . += $x.s)",
            r#"array ([]) and string ("p") cannot be added"#,
        ),
        (
            "reduce .[] as $x ({}; . += [1])",
            "object ({}) and array ([1]) cannot be added",
        ),
        (
            r#"reduce .[] as $x ("a"; . += 1)"#,
            r#"string ("a") and number (1) cannot be added"#,
        ),
    ] {
        let (stdout, stderr, code) =
            run_jq_full(&["-n", "-c", &format!("{RECORDS} | {filter}")], None)?;
        assert_eq!((stdout.as_str(), code), ("", 5), "`{filter}`: {stderr}");
        assert!(stderr.trim_end().ends_with(want), "`{filter}`: {stderr}");
    }
    Ok(())
}
