//! #5233: a jq-mode program with a dynamic path target runs on the path-mode interpreter or is
//! refused; the shipped resolver no longer sees one.

use anyhow::Result;

use crate::run_jq_full;

/// A dynamic path target beside a succinctly extension the interpreter declines is refused,
/// with an error that names the extension. jq 1.7.1 rejects every one of these programs at
/// compile time (`key/0 is not defined`, ...). Most rows were pinned to the shipped route's
/// answers before #5233 (#2289, #2642, #2974, #3036, #3134, #3369).
#[test]
fn test_dynamic_path_target_beside_an_extension_is_refused_5233() -> Result<()> {
    for (input, filter, what) in [
        (r#"{"a":[1,2,3]}"#, ".a[(0,1)] |= key", "`key`"),
        (r#"{"a":[1,2,3]}"#, ".a[(0,1)] |= parent", "`parent`"),
        (r#"{"foo":1}"#, r#"path(@base64 "\(key)")"#, "`key`"),
        (
            r#"{"a":{"foo":1}}"#,
            r#".a[] | path(@base64 "\(parent(1)|tostring)")"#,
            "`parent`",
        ),
        (
            r#"{"a":1}"#,
            r#".a | (try path(1) catch "x"), key"#,
            "`key`",
        ),
        (
            r#"{"foo":{"a":1},"b":2}"#,
            ".foo | . as $x | (parent, ($x.a = 9))",
            "`parent`",
        ),
        (
            r#"{"a":1}"#,
            ". as $x | objects | ((path | empty), ($x.a = 9))",
            "`path` with no argument",
        ),
        (
            r#"{"a":1,"a":1}"#,
            "at_offset(5) as $x | del(.a | $x)",
            "`at_offset`",
        ),
        (r#"{"a":1}"#, "del(.a as $x | $x) | line", "`line`"),
        (
            r#"{"a":1}"#,
            "(.a as $x | .a) |= document_index",
            "`document_index`",
        ),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-c", filter], Some(input))?;
        assert_eq!(
            (stdout.as_str(), code),
            ("", 5),
            "`{filter}`: stderr={stderr:?}"
        );
        assert!(
            stderr.contains(&format!(
                "{what} cannot be used in a program whose path expression is more than navigation"
            )),
            "`{filter}`: stderr={stderr:?}"
        );
    }
    Ok(())
}

/// The same extensions beside only static path targets (navigation, and value filters over
/// it) stay on the shipped route and answer as before.
#[test]
fn test_extension_beside_a_static_path_target_still_answers_5233() -> Result<()> {
    for (input, filter, want) in [
        (
            r#"{"a":[1,2,3]}"#,
            "(.a[] | select(. > 1)) |= (parent | length)",
            "{\"a\":[1,3,3]}\n",
        ),
        (r#"{"a":[1,2,3]}"#, ".a[] |= key", "{\"a\":[0,1,2]}\n"),
        (r#"{"a":1}"#, "[path(.a)], line", "[[\"a\"]]\n1\n"),
        (
            r#"{"a":1,"b":2}"#,
            "[del(.a), at_offset(0)]",
            "[{\"b\":2},{\"a\":1,\"b\":2}]\n",
        ),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-c", filter], Some(input))?;
        assert_eq!(
            (stdout.as_str(), code),
            (want, 0),
            "`{filter}`: stderr={stderr:?}"
        );
    }
    Ok(())
}

/// A document the interpreter cannot materialize whole raises the decode error for a program
/// with a dynamic target, where the shipped route used to read only what the program did. jq
/// 1.7.1 rejects both documents when it parses them (`Invalid \uXXXX\uXXXX surrogate pair
/// escape`; `Exceeds depth limit for parsing`); without a dynamic target they still answer.
#[test]
fn test_undecodable_document_raises_for_a_dynamic_path_target_5233() -> Result<()> {
    let lone = r#"{"a":"\ud800","d":5}"#;
    let deep = format!(r#"{{"a":{}{},"d":5}}"#, "[".repeat(256), "]".repeat(256));
    for (input, error) in [
        (lone, "invalid unicode escape"),
        (deep.as_str(), "nesting depth"),
    ] {
        let (stdout, stderr, code) = run_jq_full(
            &["-c", "[.d | path(if . then . else empty end)]"],
            Some(input),
        )?;
        assert_eq!((stdout.as_str(), code), ("", 5), "stderr={stderr:?}");
        assert!(stderr.contains(error), "stderr={stderr:?}");
        let (stdout, stderr, code) = run_jq_full(&["-c", "[.d | path(.)]"], Some(input))?;
        assert_eq!((stdout.as_str(), code), ("[[]]\n", 0), "stderr={stderr:?}");
    }
    Ok(())
}

/// The rows of the library tests that pinned the shipped resolver's jq-mode answers for a
/// dynamic path target, as jq 1.7.1 answers them (stdout and exit status, captured live), run
/// through the CLI. Since #5223 the CLI answers them on the path-mode interpreter; since #5233
/// nothing else does, and the resolver code those tests pinned is retired. A row whose answer
/// the resolver got wrong (`of null` where jq says `of {"__jq":0}`, ...) holds jq's answer.
#[test]
fn test_retired_resolver_rows_match_jq_through_the_interpreter_5233() -> Result<()> {
    const ROWS: &str = include_str!("../data/jq-path-mode/retired-resolver-rows-5233.jsonl");
    let mut checked = 0;
    for line in ROWS.lines().filter(|l| !l.trim().is_empty()) {
        let row: serde_json::Value = serde_json::from_str(line)?;
        let field = |name: &str| row[name].as_str().expect(name).to_string();
        let (input, filter, want) = (field("input"), field("filter"), field("stdout"));
        let status = row["status"].as_i64().expect("status");
        let (stdout, stderr, code) = run_jq_full(&["-c", &filter], Some(&input))?;
        assert_eq!(
            (stdout.as_str(), i64::from(code)),
            (want.as_str(), status),
            "{} `{filter}` on {input}: stderr={stderr:?}",
            field("source")
        );
        checked += 1;
    }
    assert_eq!(checked, 258);
    Ok(())
}
