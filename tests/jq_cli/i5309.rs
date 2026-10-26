//! #5309: the regex builtins read their input before their pattern and flags, `nth` words a
//! non-number index the way jq's `if $n < 0 ... limit($n + 1; ...)` does, and `delpaths` names the
//! first non-array in value order. The messages themselves are pinned over both evaluators by the
//! probes in `tests/data/jq-error-probes.tsv`; these pin what a probe cannot hold: the catchable and
//! optional forms, multi-output arguments, arguments that produce nothing, and path mode. Every
//! expected value was captured live against jq 1.7.1 (`/usr/bin/jq`).

use anyhow::Result;

use crate::run_jq_full;

fn run(filter: &str) -> Result<(String, String, i32)> {
    run_jq_full(&["-nc", filter], None)
}

const NOT_A_STRING: &str = "null (null) cannot be matched, as it is not a string";

#[test]
fn test_the_input_error_is_catchable_and_optional_5309() -> Result<()> {
    for (filter, want) in [
        (
            "null | try match(1;2) catch .",
            format!("\"{NOT_A_STRING}\"\n"),
        ),
        // A multi-output pattern or flags still reports the input once.
        (
            "null | try match((1,2);3) catch .",
            format!("\"{NOT_A_STRING}\"\n"),
        ),
        (
            "[null | try match(\"a\";(1,2)) catch .]",
            format!("[\"{NOT_A_STRING}\"]\n"),
        ),
        (
            "[null | try match((1,\"a\");(2,3)) catch .]",
            format!("[\"{NOT_A_STRING}\"]\n"),
        ),
        (
            "try (null | gsub((1,2);\"x\";\"g\")) catch .",
            format!("\"{NOT_A_STRING}\"\n"),
        ),
        ("null | [match(1;2)?]", "[]\n".to_string()),
        (
            "null | [try test(1;2) catch \"x\"]",
            "[\"x\"]\n".to_string(),
        ),
        // An argument that produces nothing never reaches the call that reads the input.
        ("null | [match(empty;2)?]", "[]\n".to_string()),
        ("null | try match(empty;2) catch .", String::new()),
        ("null | [scan(empty;\"g\")]", "[]\n".to_string()),
        ("null | try scan(1;empty) catch .", String::new()),
    ] {
        let (out, err, code) = run(filter)?;
        assert_eq!((out.as_str(), code), (want.as_str(), 0), "{filter}: {err}");
    }
    Ok(())
}

#[test]
fn test_nth_and_delpaths_in_path_and_try_positions_5309() -> Result<()> {
    for (filter, want, want_code) in [
        (
            "[\"a\"] | try nth(\"a\";.[]) catch .",
            "\"string (\\\"a\\\") and number (1) cannot be added\"\n",
            0,
        ),
        (
            "null | try delpaths([[0],{}]) catch .",
            "\"Path must be specified as array, not object\"\n",
            0,
        ),
        // The kind named is the lowest in value order, whatever order the entries are written in.
        (
            "[[1,\"a\",null],[null,\"a\",1],[\"a\",1,true]] | map(. as $p | try delpaths($p) catch .)",
            "[\"Path must be specified as array, not null\",\"Path must be specified as array, not null\",\"Path must be specified as array, not boolean\"]\n",
            0,
        ),
    ] {
        let (out, err, code) = run(filter)?;
        assert_eq!((out.as_str(), code), (want, want_code), "{filter}: {err}");
    }
    Ok(())
}

/// `nth` in a path position follows the same index rules; the holdout build refuses a path
/// expression that does more than navigate, so it cannot run these programs.
#[test]
#[cfg_attr(feature = "unshared-containers", ignore = "holdout refusal (#5246)")]
fn test_nth_in_a_path_position_5309() -> Result<()> {
    for (filter, want, want_code) in [
        ("[path(null|nth(1;.))]", "[]\n", 0),
        // A negative-branch index (`null` sorts below every number) raises, even in a path.
        ("[path(nth(null;1))]", "", 5),
    ] {
        let (out, err, code) = run(filter)?;
        assert_eq!((out.as_str(), code), (want, want_code), "{filter}: {err}");
    }
    Ok(())
}
