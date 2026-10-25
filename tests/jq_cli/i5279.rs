//! #5279: a fold whose UPDATE is an `if` (each branch a step of its own), a chain of `+`
//! (`. + $x.name + ","`) or an append to a field (`.a += [$x]`) is applied to the moved state in
//! place. These pin that the answers and the errors are jq 1.7.1's (captured live), including that a
//! value another binding still holds is not written through.

use anyhow::Result;

use crate::run_jq_full;

const RECORDS: &str = r#"[{"id":1,"name":"a","score":5,"tags":["x"],"s":"p"},{"id":2,"name":"b","score":0,"tags":[],"s":"q"},{"id":3,"name":"a","score":7,"tags":["y","z"],"s":"r"},{"id":4,"name":"c","score":null,"tags":null,"s":"s"},{"id":5,"name":"d","score":9,"tags":["w"],"s":"t"}]"#;

#[test]
fn test_if_chain_and_field_append_folds_answer_as_jq_5279() -> Result<()> {
    for (filter, want) in [
        (
            "reduce .[] as $x ([]; if $x.id % 2 == 0 then .[:-1] else . + [$x.id] end)",
            "[5]",
        ),
        (
            "reduce .[] as $x ([]; if $x.id % 2 == 0 then .[1:] else . + [$x.id] end)",
            "[5]",
        ),
        (
            "reduce .[] as $x ([]; if $x.score then . + [$x.id] else . end)",
            "[1,2,3,5]",
        ),
        (
            "reduce .[] as $x ([]; if $x.score then . + [$x.id] end)",
            "[1,2,3,5]",
        ),
        (
            "reduce .[] as $x ({}; if $x.score > 5 then .big += 1 elif $x.score then .small += 1 else .none += 1 end)",
            r#"{"small":2,"big":2,"none":1}"#,
        ),
        (
            "reduce .[] as $x ({}; if $x.score == null then .[$x.name] = 0 else .[$x.name] += $x.score end)",
            r#"{"a":12,"b":0,"c":0,"d":9}"#,
        ),
        (
            "reduce .[] as $x ([]; if length > 2 then .[1:] else . + [$x.id] end)",
            "[2,3,5]",
        ),
        (
            "reduce .[] as $x ([]; if length < 2 then . + [$x.id] else .[:-1] end)",
            "[1]",
        ),
        (
            r#"reduce .[] as $x (""; if $x.id > 2 then . + $x.name else . + "-" end)"#,
            r#""--acd""#,
        ),
        (
            "reduce .[] as $x (0; if $x.score then . + $x.score else . end)",
            "21",
        ),
        (
            r#"reduce .[] as $x ([]; if $x.name == "a" then . + [1] else empty end)"#,
            "null",
        ),
        (
            "reduce .[] as $x ([]; if ($x.tags | length) > 1 then . + $x.tags else . end)",
            r#"["y","z"]"#,
        ),
        (
            "reduce .[] as $x ([]; if (.[0] // 0) > 3 then .[1:] else . + [$x.id] end)",
            "[1,2,3,4,5]",
        ),
        (
            "[foreach .[] as $x ([]; if $x.id % 2 == 0 then .[:-1] else . + [$x.id] end; .)]",
            "[[1],[],[3],[],[5]]",
        ),
        (
            "[. | until(length < 2; if length > 3 then .[1:] else .[:-1] end)]",
            r#"[[{"id":3,"name":"a","score":7,"tags":["y","z"],"s":"r"}]]"#,
        ),
        (
            r#"reduce .[] as $x (""; . + $x.name + ",")"#,
            r#""a,b,a,c,d,""#,
        ),
        (
            r#"reduce .[] as $x (""; . + "<" + $x.name + ">" + $x.s)"#,
            r#""<a>p<b>q<a>r<c>s<d>t""#,
        ),
        (
            "reduce .[] as $x (0; . + $x.id - 1)",
            "10",
        ),
        (
            "reduce .[] as $x (0; . + $x.id * 2 - 1)",
            "25",
        ),
        (
            "reduce .[] as $x ([]; . + [$x.id] + [$x.id])",
            "[1,1,2,2,3,3,4,4,5,5]",
        ),
        (
            "reduce .[] as $x ({}; . + {a: 1} + {b: $x.id})",
            r#"{"a":1,"b":5}"#,
        ),
        (
            "reduce .[] as $x (null; . + [$x.id] + [1])",
            "[1,1,2,1,3,1,4,1,5,1]",
        ),
        (
            "reduce .[] as $x (0; . - $x.id - 1)",
            "-20",
        ),
        (
            "reduce .[] as $x (100; . / 2 / 2)",
            "0.09765625",
        ),
        (
            "reduce .[] as $x (1; . * 2 + 1)",
            "63",
        ),
        (
            "reduce .[] as $x ({a:[]}; .a += [$x.id])",
            r#"{"a":[1,2,3,4,5]}"#,
        ),
        (
            "reduce .[] as $x ({a:{}}; .a += {($x.name): $x.id})",
            r#"{"a":{"a":3,"b":2,"c":4,"d":5}}"#,
        ),
        (
            r#"reduce .[] as $x ({a:""}; .a += $x.name)"#,
            r#"{"a":"abacd"}"#,
        ),
        (
            "reduce .[] as $x ({a:[]}; .a |= . + [$x.id])",
            r#"{"a":[1,2,3,4,5]}"#,
        ),
        (
            "reduce .[] as $x ({}; .a += [$x.id])",
            r#"{"a":[1,2,3,4,5]}"#,
        ),
        (
            r#"reduce .[] as $x ({"a":null}; .a += [$x.id])"#,
            r#"{"a":[1,2,3,4,5]}"#,
        ),
        (
            "reduce .[] as $x ({a:[]}; .a += [$x.id] | .n += 1)",
            r#"{"a":[1,2,3,4,5],"n":5}"#,
        ),
        (
            "reduce .[] as $x ([[]]; .[0] += [$x.id])",
            "[[1,2,3,4,5]]",
        ),
        (
            "reduce .[] as $x ([[],[]]; .[$x.id % 2] += [$x.id])",
            "[[2,4],[1,3,5]]",
        ),
        (
            "reduce .[] as $x ([[]]; .[1] += [$x.id])",
            "[[],[1,2,3,4,5]]",
        ),
        (
            "reduce .[] as $x ({a:[1]}; .a += .a)",
            r#"{"a":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]}"#,
        ),
        (
            "reduce .[] as $x ({a:[1]}; .a += [length])",
            r#"{"a":[1,1,1,1,1,1]}"#,
        ),
        (
            ". as $d | reduce .[] as $x ({a:$d}; .a += [$x])",
            r#"{"a":[{"id":1,"name":"a","score":5,"tags":["x"],"s":"p"},{"id":2,"name":"b","score":0,"tags":[],"s":"q"},{"id":3,"name":"a","score":7,"tags":["y","z"],"s":"r"},{"id":4,"name":"c","score":null,"tags":null,"s":"s"},{"id":5,"name":"d","score":9,"tags":["w"],"s":"t"},{"id":1,"name":"a","score":5,"tags":["x"],"s":"p"},{"id":2,"name":"b","score":0,"tags":[],"s":"q"},{"id":3,"name":"a","score":7,"tags":["y","z"],"s":"r"},{"id":4,"name":"c","score":null,"tags":null,"s":"s"},{"id":5,"name":"d","score":9,"tags":["w"],"s":"t"}]}"#,
        ),
        (
            "{a:[1]} as $s | reduce range(3) as $i ($s; .a += [$i]) | [., $s]",
            r#"[{"a":[1,0,1,2]},{"a":[1]}]"#,
        ),
        (
            "[1] as $l | reduce range(3) as $i ({a:$l}; .a += [$i]) | [., $l]",
            r#"[{"a":[1,0,1,2]},[1]]"#,
        ),
        (
            "{a:[1]} | . as $o | reduce range(3) as $i (.; .a += [$i]) | [., $o]",
            r#"[{"a":[1,0,1,2]},{"a":[1]}]"#,
        ),
        (
            "reduce .[] as $x ({a:[]}; .a += [$x.id]) | .a | length",
            "5",
        ),
        (
            "[foreach .[] as $x ({a:[]}; .a += [$x.id]; .a | length)]",
            "[1,2,3,4,5]",
        ),
        (
            "{i:0,a:[]} | until(.i >= 5; .i += 1 | .a += [.i])",
            r#"{"i":5,"a":[1,2,3,4,5]}"#,
        ),
        (
            "{a:[]} | until(.a | length >= 5; .a += [1])",
            r#"{"a":[1,1,1,1,1]}"#,
        ),
        (
            r#"{a:""} | [while(.a | length < 4; .a += "x")]"#,
            r#"[{"a":""},{"a":"x"},{"a":"xx"},{"a":"xxx"}]"#,
        ),
    ] {
        let (stdout, stderr, code) =
            run_jq_full(&["-n", "-c", &format!("{RECORDS} | {filter}")], None)?;
        assert_eq!((stdout.trim_end(), code), (want, 0), "`{filter}`: {stderr}");
    }
    Ok(())
}

/// A step that fails keeps the evaluator's own error, and nothing is written.
#[test]
fn test_failing_if_chain_and_field_append_keep_the_error_5279() -> Result<()> {
    for (filter, want) in [
        (
            r#"reduce .[] as $x ([]; if error("x") then . else . end)"#,
            "x",
        ),
        (
            r#"reduce .[] as $x ({}; if $x.id == 3 then error("boom") else .[$x.name] = 1 end)"#,
            "boom",
        ),
        (
            r#"reduce .[] as $x (""; . + $x.name + 1)"#,
            r#"string ("a") and number (1) cannot be added"#,
        ),
        (
            r#"reduce .[] as $x (""; . + $x.score + "x")"#,
            r#"string ("") and number (5) cannot be added"#,
        ),
        (
            "reduce .[] as $x ({a:[]}; .a += $x.name)",
            r#"array ([]) and string ("a") cannot be added"#,
        ),
        (
            r#"reduce .[] as $x ({"a":1}; .a += [1])"#,
            "number (1) and array ([1]) cannot be added",
        ),
    ] {
        let (stdout, stderr, code) =
            run_jq_full(&["-n", "-c", &format!("{RECORDS} | {filter}")], None)?;
        assert_eq!((stdout.as_str(), code), ("", 5), "`{filter}`: {stderr}");
        assert!(stderr.trim_end().ends_with(want), "`{filter}`: {stderr}");
    }
    Ok(())
}
