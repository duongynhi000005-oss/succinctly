//! #5285: a fold whose UPDATE is a pipe of assignments (`.l += [$x.id] | .n += 1`) is applied to
//! the moved state one stage after another. These pin that the answers and the errors are jq 1.7.1's
//! (captured live), including a later stage the in-place step declines (a padding or negative index,
//! a fan-out, a failing `+`), which runs on the evaluator against the state so far, and that a value
//! another binding still holds is not written through.

use anyhow::Result;

use crate::run_jq_full;

const RECORDS: &str = r#"[{"id":1,"name":"a","score":5,"tags":["x"],"s":"p"},{"id":2,"name":"b","score":0,"tags":[],"s":"q"},{"id":3,"name":"a","score":7,"tags":["y","z"],"s":"r"},{"id":4,"name":"c","score":null,"tags":null,"s":"s"},{"id":5,"name":"d","score":9,"tags":["w"],"s":"t"}]"#;

#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_pipe_of_assignments_folds_answer_as_jq_5285() -> Result<()> {
    for (filter, want) in [
        (
            "reduce .[] as $x ({n:0,l:[]}; .l += [$x.id] | .n += 1)",
            r#"{"n":5,"l":[1,2,3,4,5]}"#,
        ),
        (
            "reduce .[] as $x ({n:0,l:[]}; .n += 1 | .l += [$x.id])",
            r#"{"n":5,"l":[1,2,3,4,5]}"#,
        ),
        (
            "reduce .[] as $x ({n:0,l:[]}; .l += [$x.id] | .n += 1 | .s = (.n * 2))",
            r#"{"n":5,"l":[1,2,3,4,5],"s":10}"#,
        ),
        (
            "reduce .[] as $x ({i:0,a:[]}; .i += 1 | .a += [.i])",
            r#"{"i":5,"a":[1,2,3,4,5]}"#,
        ),
        (
            "reduce .[] as $x ({i:0,a:[]}; .a += [.i] | .i += 1)",
            r#"{"i":5,"a":[0,1,2,3,4]}"#,
        ),
        (
            "reduce .[] as $x ({n:0,o:{}}; .n += 1 | .o += {($x.name): .n})",
            r#"{"n":5,"o":{"a":3,"b":2,"c":4,"d":5}}"#,
        ),
        (
            r#"reduce .[] as $x ({n:0,s:""}; .s += $x.name | .n += 1)"#,
            r#"{"n":5,"s":"abacd"}"#,
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .n |= . * 2 | .n //= 7)",
            r#"{"n":62}"#,
        ),
        (
            "reduce .[] as $x ({n:null}; .n //= $x.id | .n //= 99)",
            r#"{"n":1}"#,
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .k = ($x.name | ascii_upcase))",
            r#"{"n":5,"k":"D"}"#,
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .[$x.name] += 1)",
            r#"{"n":5,"a":2,"b":1,"c":1,"d":1}"#,
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .a.b.c = $x.id | .a.b.d = 2 | .a.e = 3)",
            r#"{"n":5,"a":{"b":{"c":5,"d":2},"e":3}}"#,
        ),
        (
            "reduce .[] as $x (null; .n += 1 | .l += [$x.id])",
            r#"{"n":5,"l":[1,2,3,4,5]}"#,
        ),
        (
            "reduce .[] as $x ([]; .[length] = $x.id | .[length] = 0)",
            "[1,0,2,0,3,0,4,0,5,0]",
        ),
        (
            "reduce .[] as $x (0; . += $x.id | . += 1)",
            "20",
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .k = $x.tags)",
            r#"{"n":5,"k":["w"]}"#,
        ),
        (
            "reduce .[] as $x ({n:0,l:[]}; if $x.id > 2 then .n += 1 | .l += [$x.id] else . end)",
            r#"{"n":3,"l":[3,4,5]}"#,
        ),
        (
            "reduce .[] as $x ({n:0,l:[]}; (.n += 1 | .l += [$x.id]) // 5)",
            r#"{"n":5,"l":[1,2,3,4,5]}"#,
        ),
        (
            "reduce .[] as $x ({n:0,l:[]}; .n += 1 | .l += [$x.id] | .l |= map(. + 1))",
            r#"{"n":5,"l":[6,6,6,6,6]}"#,
        ),
        (
            "foreach .[] as $x ({n:0,l:[]}; .l += [$x.id] | .n += 1; [.n, (.l|length)])",
            "[1,1]
[2,2]
[3,3]
[4,4]
[5,5]",
        ),
        (
            "{i:0,a:[]} | until(.i >= 5; .i += 1 | .a += [.i])",
            r#"{"i":5,"a":[1,2,3,4,5]}"#,
        ),
        (
            "{i:0,a:[]} | [while(.i < 3; .i += 1 | .a += [.i])]",
            r#"[{"i":0,"a":[]},{"i":1,"a":[1]},{"i":2,"a":[1,2]}]"#,
        ),
        (
            "reduce .[] as $x ([0]; .[0] += 1 | .[3] = $x.id)",
            "[5,null,null,5]",
        ),
        (
            "reduce .[] as $x ([0]; .[0] += 1 | .[-1] = $x.id)",
            "[5]",
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .m[0] = $x.id | .m[2] = 1)",
            r#"{"n":5,"m":[5,null,1]}"#,
        ),
        (
            "reduce .[] as $x ({n:0,m:null}; .m.k = 1 | .m[0] = 1)?",
            "",
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .k = $x.tags)",
            r#"{"n":5,"k":["w"]}"#,
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .k = ($x.tags // [] | .[]))",
            r#"{"n":1,"k":"w"}"#,
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | to_entries | from_entries)",
            r#"{"n":5}"#,
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .k = ($x.tags | .[]?))",
            r#"{"n":1,"k":"w"}"#,
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .k = ($x.name | ascii_upcase | [., .]))",
            r#"{"n":5,"k":["D","D"]}"#,
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .k //= (1 | .,2))",
            r#"{"n":5,"k":2}"#,
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .a = (1,2))",
            r#"{"n":5,"a":2}"#,
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .a |= empty)",
            r#"{"n":5}"#,
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | (.a, .b) = $x.id)",
            r#"{"n":5,"a":5,"b":5}"#,
        ),
        (
            ". as $orig | reduce .[] as $x ({n:0,l:[]}; .n += 1 | .l += [$x.id]) | [., ($orig|length)]",
            r#"[{"n":5,"l":[1,2,3,4,5]},5]"#,
        ),
        (
            "{n:0,l:[]} as $s | reduce .[] as $x ($s; .l += [$x.id] | .n += 1) | [., $s]",
            r#"[{"n":5,"l":[1,2,3,4,5]},{"n":0,"l":[]}]"#,
        ),
        (
            "[.[0]] as $s | reduce .[] as $x ({n:0,l:$s}; .l += [$x.id] | .n += 1) | [., $s]",
            r#"[{"n":5,"l":[{"id":1,"name":"a","score":5,"tags":["x"],"s":"p"},1,2,3,4,5]},[{"id":1,"name":"a","score":5,"tags":["x"],"s":"p"}]]"#,
        ),
        (
            "reduce .[] as $x ({n:0,l:[]}; try (.n += 1 | .l += 1) catch .)",
            r#""Cannot index string with string \"n\"""#,
        ),
        (
            "(reduce .[] as $x ({n:0,l:[]}; .n += 1 | .l += 1))?",
            "",
        ),
    ] {
        let (stdout, stderr, code) =
            run_jq_full(&["-n", "-c", &format!("{RECORDS} | {filter}")], None)?;
        assert_eq!((stdout.trim_end(), code), (want, 0), "`{filter}`: {stderr}");
    }
    Ok(())
}

/// A stage that fails keeps the evaluator's own error, whichever stage it is, and the state the
/// earlier stages wrote is dropped with the fold.
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object patterns read like format arguments
fn test_failing_pipe_of_assignments_keeps_the_error_5285() -> Result<()> {
    for (filter, want) in [
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .a[] = 1)",
            "Cannot iterate over null (null)",
        ),
        (
            "reduce .[] as $x ({n:0,l:[]}; .n += 1 | .l += 1)",
            "array ([]) and number (1) cannot be added",
        ),
        (
            "reduce .[] as $x ({n:0,l:5}; .n += 1 | .l += [1])",
            "number (5) and array ([1]) cannot be added",
        ),
        (
            r#"reduce .[] as $x ({n:0,l:"a"}; .n += 1 | .l += $x.id)"#,
            r#"string ("a") and number (1) cannot be added"#,
        ),
        (
            r#"reduce .[] as $x ({n:0}; .n += "a" | .a = 1)"#,
            r#"number (0) and string ("a") cannot be added"#,
        ),
        (
            r#"reduce .[] as $x ({n:0}; .a = 1 | .n += "a")"#,
            r#"number (0) and string ("a") cannot be added"#,
        ),
        (
            r#"reduce .[] as $x ({n:0}; .n += 1 | .a = error("boom"))"#,
            "boom",
        ),
        (
            "reduce .[] as $x ({n:1}; .n += 1 | .n.a = 2)",
            r#"Cannot index number with string "a""#,
        ),
        (
            "reduce .[] as $x ({n:1}; .n += 1 | .[0] = 2)",
            "Cannot index object with number",
        ),
        (
            r#"reduce .[] as $x ([1]; .[0] += 1 | .["a"] = 2)"#,
            r#"Cannot index array with string "a""#,
        ),
        (
            "reduce .[] as $x ([1]; .[0] += 1 | .[null] = 2)",
            "Cannot index array with null",
        ),
        (
            "reduce .[] as $x ({n:0}; .n += 1 | .k = ($x.tags | .[]))",
            "Cannot iterate over null (null)",
        ),
        (
            r#"{i:0,a:[]} | until(.i >= 3; .i += 1 | .a += "x")"#,
            r#"array ([]) and string ("x") cannot be added"#,
        ),
    ] {
        let (stdout, stderr, code) =
            run_jq_full(&["-n", "-c", &format!("{RECORDS} | {filter}")], None)?;
        assert_eq!((stdout.as_str(), code), ("", 5), "`{filter}`: {stderr}");
        assert!(stderr.trim_end().ends_with(want), "`{filter}`: {stderr}");
    }
    Ok(())
}
