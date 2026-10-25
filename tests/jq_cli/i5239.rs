//! #5239: a `foreach` or `reduce` in a dynamic path target, beside a builtin the path-mode interpreter declines (it reads the cursor
//! or the evaluator's path context, which a materialized document does not have), is refused. It stayed on the shipped resolver,
//! which answered the shapes of #5225, #5226 and #5228 with a refusal for a program jq answers and, for #5226's, a write that
//! lands where jq exits 5.
//!
//! The interpreter reads storage identity, which the `unshared-containers` holdout does not have, so these pin the shipped shape
//! only.
#![cfg(not(feature = "unshared-containers"))]

use anyhow::Result;

use crate::run_jq_full;

/// The builtins `path_mode::cursor_dependent_builtin` names, each as an expression that yields exactly one value.
const BUILTINS: &[(&str, &str)] = &[
    ("at_offset(0)", "at_offset"),
    ("at_position(1; 1)", "at_position"),
    ("line", "line"),
    ("column", "column"),
    ("document_index", "document_index"),
    ("anchor", "anchor"),
    ("style", "style"),
    ("line_comment", "line_comment"),
    ("head_comment", "head_comment"),
    ("foot_comment", "foot_comment"),
    ("first(key, 0)", "key"),
    ("file_index", "file_index"),
    ("path", "path"),
    ("first(parent, 0)", "parent"),
];

/// One target from each of the three issues' files, with a `path`, `del` and `|=` spelling of each.
const TARGETS: &[&str] = &[
    // #5226: the loop variable as a bind source of the `//` right operand, under a `try`.
    r"(. as $x | foreach (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i as $q | $q))))",
    r"(. as $x | reduce (1,2) as $i (null; try (($x as {a:$a} | 5) // ($i.a as $q | $q))))",
    // #5225: an outer INIT on the node `$x` is bound to.
    r"(. as $x | foreach (.a,.) as $i (.; ((foreach (try ($x as {a:$a} | .)) as $y (.; .; .)) // .) | .; $i))",
    // #5228: the explicit pipe spelling of a caught fold.
    r"(.a as $x | foreach 1 as $i (.a; foreach (try ($x as {b:$q} | $q)) as $y (.; $y; .); .))",
];

/// Every spelling of every target, beside every builtin, on both the stdin route and the `-n` literal-document route: refused with
/// the builtin's name, nothing on stdout, exit 5 -- never a write that lands.
#[test]
fn test_fold_in_a_path_target_beside_a_declined_builtin_is_refused_5239() -> Result<()> {
    let input = r#"{"a":{"b":1}}"#;
    for (builtin, name) in BUILTINS {
        for target in TARGETS {
            for spelling in [
                format!("path{target}"),
                format!("del{target}"),
                format!("{target} |= 9"),
            ] {
                let filter = format!("(try {builtin} catch 0) as $z | {spelling}");
                // The `-n` route reads its document from the program text.
                let literal = format!("{input} | {filter}");
                for (args, document) in [
                    (vec!["-c", filter.as_str()], Some(input)),
                    (vec!["-nc", literal.as_str()], None),
                ] {
                    let (stdout, stderr, code) = run_jq_full(&args, document)?;
                    assert_eq!(stdout, "", "`{filter}` ({args:?})");
                    assert_eq!(code, 5, "`{filter}` ({args:?}): {stderr}");
                    assert!(
                        stderr.contains(&format!(
                            "{name} cannot be combined with a foreach or reduce in a path target"
                        )),
                        "`{filter}` ({args:?}): {stderr}"
                    );
                }
            }
        }
    }
    Ok(())
}

/// The repro: `del` and `|=` printed the document unchanged and `path` printed nothing, all exit 0, where jq exits 5 with
/// `Invalid path expression with result 5`. Without the builtin the interpreter gives jq's answer, so the refusal is the
/// only difference.
#[test]
fn test_5226_repro_no_longer_exits_zero_with_a_declined_builtin_5239() -> Result<()> {
    let target = TARGETS[0];
    let input = r#"{"a":{"b":1}}"#;
    for spelling in [
        format!("path{target}"),
        format!("del{target}"),
        format!("{target} |= 9"),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-c", &spelling], Some(input))?;
        assert_eq!(
            (stdout.as_str(), code),
            ("", 5),
            "control `{spelling}`: {stderr}"
        );
        assert!(
            stderr.contains("Invalid path expression with result 5"),
            "control `{spelling}`: {stderr}"
        );

        let filter = format!("(try at_offset(0) catch 0) as $z | {spelling}");
        let (stdout, stderr, code) = run_jq_full(&["-c", &filter], Some(input))?;
        assert_eq!((stdout.as_str(), code), ("", 5), "`{filter}`: {stderr}");
        assert!(
            stderr.contains("cannot be combined with a foreach or reduce in a path target"),
            "`{filter}`: {stderr}"
        );
    }
    Ok(())
}

/// What stays on the shipped resolver: a declined builtin beside a dynamic target with no fold in it, beside a fold that is not
/// in a target, and beside a fold whose target is static. The refusal is scoped to where the resolver was shown wrong.
#[test]
fn test_declined_builtin_beside_other_dynamic_targets_stays_shipped_5239() -> Result<()> {
    for (input, filter, want) in [
        // A bind in the target, no fold.
        (
            r#"{"a":{"b":1}}"#,
            r"(try line catch 0) as $z | path(.a | . as $x | .b)",
            "[\"a\",\"b\"]\n",
        ),
        // A fold outside every path target.
        (
            r#"{"a":{"b":1}}"#,
            r"(reduce (1,2) as $i (0; . + $i)) as $s | (try line catch 0) as $z | path(.a | . as $x | .b)",
            "[\"a\",\"b\"]\n",
        ),
        // A comma-valued index: `key` reads the path, and the write lands.
        (r#"{"a":[1,2,3]}"#, r".a[(0,1)] |= key", "{\"a\":[0,1,3]}\n"),
        // A static target holding no fold at all.
        (
            r#"{"a":{"b":1}}"#,
            r"(try at_offset(0) catch 0) as $z | path(.a.b)",
            "[\"a\",\"b\"]\n",
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
