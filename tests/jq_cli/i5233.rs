//! #5233: no jq-mode program with a dynamic path target runs on the shipped resolver any
//! more. One that also holds a construct the path-mode interpreter does not run (the
//! succinctly extensions `key`, `file_index`, `parent`, bare `path` and the cursor-metadata
//! builtins) is refused, naming it, and a dynamic path program over a document the
//! interpreter cannot materialize whole raises the decode error. jq 1.7.1 rejects every one
//! of these programs at compile time (`key/0 is not defined`, ...), and both documents when
//! it parses them.

use anyhow::Result;

use crate::run_jq_full;

/// A dynamic path target beside an extension builtin, anywhere in the program, is refused
/// with the builtin's name and jq's runtime-error status. These rows used to reach the
/// shipped route's path-context evaluator (#2289, #2642, #2974, #3036, #3134, #3369).
#[test]
fn a_dynamic_path_target_beside_an_extension_builtin_is_refused_5233() -> Result<()> {
    for (filter, input, name) in [
        (r#".a | (try path(1) catch "x"), key"#, r#"{"a":1}"#, "key"),
        (".a[(0,1)] |= key", r#"{"a":[1,2,3]}"#, "key"),
        (r#"path(@base64 "\(key)")"#, r#"{"foo":1}"#, "key"),
        (".a[(0,1)] |= parent", r#"{"a":[1,2,3]}"#, "parent"),
        (
            ".foo | . as $x | (parent, ($x.a = 9))",
            r#"{"foo":{"a":1},"b":2}"#,
            "parent",
        ),
        (
            r#".a[] | path(@base64 "\(parent(1)|tostring)")"#,
            r#"{"a":{"foo":1}}"#,
            "parent",
        ),
        (
            ".foo | . as $x | sort | (path | empty), ($x[0] = 9)",
            r#"{"foo":[{"a":1}]}"#,
            "path",
        ),
        (
            "at_offset(11) as $x | del(.a | $x)",
            r#"{"a":1,"a":1}"#,
            "at_offset",
        ),
        (
            "file_index, path(.a as $x | .b)",
            r#"{"a":1}"#,
            "file_index",
        ),
        ("line, path(.a as $x | .b)", r#"{"a":1}"#, "line"),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-c", filter], Some(input))?;
        assert_eq!((stdout.as_str(), code), ("", 5), "`{filter}`: {stderr}");
        assert!(
            stderr.contains(&format!(
                "{name} is not supported in a program whose path expression does more than navigate"
            )),
            "`{filter}`: {stderr}"
        );
    }
    Ok(())
}

/// The same builtins beside path targets that only navigate stay on the shipped route and
/// answer as before.
#[test]
fn an_extension_builtin_beside_a_static_path_target_still_answers_5233() -> Result<()> {
    for (filter, input, want) in [
        (
            "(.a[] | select(. > 1)) |= (parent | length)",
            r#"{"a":[1,2,3]}"#,
            "{\"a\":[1,3,3]}\n",
        ),
        (".a | path(.b), key", r#"{"a":{"b":1}}"#, "[\"b\"]\n\"a\"\n"),
        ("del(.a), line", r#"{"a":1}"#, "{}\n1\n"),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-c", filter], Some(input))?;
        assert_eq!((stdout.as_str(), code), (want, 0), "`{filter}`: {stderr}");
    }
    Ok(())
}

/// A dynamic path program over a document jq rejects at parse time raises the decode error
/// instead of answering from the parts it reads: a lone surrogate, and nesting past 256
/// levels. A program with no dynamic target still reads only what it navigates (#4412).
#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq's `{k:.}`, not a format argument
fn a_dynamic_path_program_over_an_undecodable_document_raises_5233() -> Result<()> {
    let lone = r#"{"a":"\ud800","d":5}"#;
    let deep = format!(r#"{{"a":{}{}}}"#, "[".repeat(256), "]".repeat(256));
    for (filter, input, message) in [
        (
            "[.d | path(if . then . else empty end)]",
            lone,
            "invalid unicode escape sequence",
        ),
        (
            ".a as $y | {k:.} | .k.a | path($y)",
            deep.as_str(),
            "nesting depth exceeds limit of 256",
        ),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-c", filter], Some(input))?;
        assert_eq!((stdout.as_str(), code), ("", 5), "`{filter}`: {stderr}");
        assert!(stderr.contains(message), "`{filter}`: {stderr}");
    }
    let (stdout, stderr, code) = run_jq_full(&["-c", "[.d | path(.)]"], Some(lone))?;
    assert_eq!((stdout.as_str(), code), ("[[]]\n", 0), "{stderr}");
    Ok(())
}
