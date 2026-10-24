//! Every expected answer here was captured from `/usr/bin/jq` 1.7.1 (macOS
//! arm64); the comment on a row says what in jq's bytecode it pins.

// STYLE-0004: jq object literals, not format strings.
#![allow(clippy::literal_string_with_formatting_args)]

use super::*;
use crate::jq::eval_generic::to_owned;
use crate::json::JsonIndex;

/// What jq printed: each output compactly, then the uncaught error's message,
/// if any.
#[derive(Debug, PartialEq)]
struct Answer {
    outputs: Vec<String>,
    error: Option<String>,
}

fn spike(doc: &str, filter: &str) -> Answer {
    let index = JsonIndex::build(doc.as_bytes());
    let input = to_owned::<JqSemantics, _>(&index.root(doc.as_bytes()).value()).expect("doc");
    let run = run(filter, &input).expect("parses");
    let error = match run.outcome {
        Outcome::Done => None,
        Outcome::Error(OwnedValue::String(s)) => Some(s.as_str().to_string()),
        Outcome::Error(other) => Some(other.to_json_jq_preserve()),
        other => panic!("{filter}: {other:?}"),
    };
    Answer {
        outputs: run
            .outputs
            .iter()
            .map(OwnedValue::to_json_jq_preserve)
            .collect(),
        error,
    }
}

/// Run a test body on a thread with room for the continuation-threaded
/// evaluator in a debug build (the binary runs it on a large stack too).
fn on_big_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(body)
        .expect("spawn")
        .join()
        .unwrap_or_else(|e| std::panic::resume_unwind(e));
}

fn ok(outputs: &[&str]) -> Answer {
    Answer {
        outputs: outputs.iter().map(|s| String::from(*s)).collect(),
        error: None,
    }
}

fn err(outputs: &[&str], message: &str) -> Answer {
    Answer {
        outputs: outputs.iter().map(|s| String::from(*s)).collect(),
        error: Some(message.to_string()),
    }
}

fn check(rows: &[(&str, &str, Answer)]) {
    for (doc, filter, want) in rows {
        assert_eq!(&spike(doc, filter), want, "{doc} | {filter}");
    }
}

/// The issue's live-probe table, row for row.
#[test]
fn the_issues_live_probes_match_jq_5223() {
    on_big_stack(|| {
        let ab = r#"{"a":{"b":1}}"#;
        check(&[
            // `UPDATE` is checked against the register `SOURCE` left, not the state's.
            (
                ab,
                "path(reduce (.a,.a) as $x (.; .a))",
                err(
                    &[],
                    r#"Invalid path expression near attempt to access element "a" of {"a":{"b":1}}"#,
                ),
            ),
            // The result carries the register after `INIT`.
            (ab, "path(reduce .a as $v (.; .))", ok(&["[]"])),
            // `EXTRACT` sees `UPDATE`'s register; step 2 restarts from `SOURCE`'s.
            (
                ab,
                "path(foreach (1,2) as $x (.; .a; .b))",
                err(
                    &[r#"["a","b"]"#],
                    r#"Invalid path expression near attempt to access element "a" of {"b":1}"#,
                ),
            ),
            (
                ab,
                "path(foreach .a as $v (.; .; .))",
                err(&[], r#"Invalid path expression with result {"a":{"b":1}}"#),
            ),
            // A pattern is tracked: its `INDEX` checks the bound value.
            (
                ab,
                "path(.a as {b:$v} | .)",
                err(
                    &[],
                    r#"Invalid path expression near attempt to access element "b" of {"b":1}"#,
                ),
            ),
            // `[E]`'s `E` is tracked, and the array carries the entry register.
            (
                ab,
                "path([{} | .a])",
                err(
                    &[],
                    r#"Invalid path expression near attempt to access element "a" of {}"#,
                ),
            ),
            (ab, "path(. as $x | [.a] | $x)", ok(&["[]"])),
            // A handler runs at the `try`-entry register.
            (ab, "path(. as $x | try (.a | error) catch $x)", ok(&["[]"])),
            (ab, "path(. as $x | (.a | empty) // $x)", ok(&["[]"])),
            // Arithmetic operands are subexps: the register stays at `.a`.
            (
                ab,
                "path(. as $x | .a | (1 + (.b)) | $x)",
                err(&[], r#"Invalid path expression with result {"a":{"b":1}}"#),
            ),
            // `getpath` on a non-register input is transparent; `PATH_END` checks.
            (
                ab,
                r#"path(.a | {c:2} | getpath(["c"]))"#,
                err(&[], "Invalid path expression with result 2"),
            ),
            (
                r#"{"a":"x"}"#,
                r#"path(.a | ltrimstr("x"))"#,
                err(&[], r#"Invalid path expression with result """#),
            ),
            // A jq-defined builtin's own body is tracked (`explode | map(..)`).
            (
                r#"{"a":"x"}"#,
                "path(.a | ascii_downcase)",
                err(
                    &[],
                    "Invalid path expression near attempt to iterate through [120]",
                ),
            ),
            // Equal literals parsed from different places are not identical.
            (
                r#"{"a":1,"b":1}"#,
                "path(.a as $v | .b | select(. == $v) | $v)",
                err(&[], "Invalid path expression with result 1"),
            ),
        ]);
    });
}

/// `ltrimstr` with no match returns its input in jq (`f_ltrimstr`), so the
/// path survives. The spike stubs every C builtin to a fresh result, as
/// Phase 0 specifies; Phase 1's table of builtins that can return their input
/// closes this row. Pinned so the gap is visible, not silent.
#[test]
fn a_c_builtin_that_returns_its_input_is_fresh_under_the_stub_5223() {
    on_big_stack(|| {
        check(&[(
            r#"{"a":"x"}"#,
            r#"path(.a | ltrimstr("z"))"#,
            // jq 1.7.1: ["a"]
            err(&[], r#"Invalid path expression with result "x""#),
        )]);
    });
}

/// `LOADK` hands out one `jv` per constant site: the same site evaluated
/// twice is identical, two equal constants at different sites are not.
#[test]
fn a_constant_is_one_value_per_site_5223() {
    on_big_stack(|| {
        check(&[
            (
                "null",
                r#"def f: {"a":1}; f | path(f | .a)"#,
                ok(&[r#"["a"]"#]),
            ),
            (
                "null",
                r#"{"a":1} | path({"a":1} | .a)"#,
                err(
                    &[],
                    r#"Invalid path expression near attempt to access element "a" of {"a":1}"#,
                ),
            ),
            ("null", "def f: [1]; f | path(f | .[0])", ok(&["[0]"])),
            (
                "null",
                "[1] | path([1] | .[0])",
                err(
                    &[],
                    "Invalid path expression near attempt to access element 0 of [1]",
                ),
            ),
            (
                "null",
                r#"def f: "s"; f | path(f | .[0:1])"#,
                ok(&[r#"[{"start":0,"end":1}]"#]),
            ),
        ]);
    });
}

/// `jvp_array_slice` returns a view into the same allocation: the whole
/// range is the array itself, and equal ranges of one array (a slice of a
/// slice included) are one value.
#[test]
fn an_array_slice_is_a_view_with_jqs_identity_5223() {
    on_big_stack(|| {
        check(&[
            ("[1,2]", "path(reduce (.[0:2]) as $k (.; $k))", ok(&["[]"])),
            (
                "[1,2]",
                "path(foreach (.[0:2]) as $k (.; .[0]))",
                ok(&[r#"[{"start":0,"end":2},0]"#]),
            ),
            (
                "[1,2,3]",
                "path(.[0:2] as $a | .[0:2] | $a)",
                ok(&[r#"[{"start":0,"end":2}]"#]),
            ),
            (
                "[1,2,3]",
                "path(.[1:3] | .[0:1] as $a | . as $b | $b[0:1] | $a)",
                ok(&[r#"[{"start":1,"end":3},{"start":0,"end":1}]"#]),
            ),
            (
                "[1,2,3]",
                "[.[1:], .[-2:], .[:-1], .[1.5:2.5], .[5:], .[null:2]]",
                ok(&["[[2,3],[2,3],[1,2],[2,3],[],[1,2]]"]),
            ),
        ]);
    });
}

/// `jv_identical` by kind and payload: `null`/`true`/`false` of the same kind
/// always are; a computed number never equals a literal.
#[test]
fn scalar_identity_follows_jv_identical_5223() {
    on_big_stack(|| {
        check(&[
            (r#"{"a":null}"#, "path(.a | null)", ok(&[r#"["a"]"#])),
            (r#"{"a":true}"#, "path(.a | true)", ok(&[r#"["a"]"#])),
            (
                r#"{"a":true}"#,
                "path(.a | false)",
                err(&[], "Invalid path expression with result false"),
            ),
            (
                r#"{"a":1}"#,
                "path(1 + 1 as $x | 2 | $x)",
                err(&[], "Invalid path expression with result 2"),
            ),
            (
                r#"{"a":1}"#,
                "path(1 as $x | $x)",
                err(&[], "Invalid path expression with result 1"),
            ),
            // `$v` bound by navigation is the node itself.
            (
                r#"{"a":1}"#,
                "[.[] as $v | path(.a | select(. == $v))]",
                ok(&[r#"[["a"]]"#]),
            ),
        ]);
    });
}

/// `.k?` is `INDEX_OPT`: it suppresses the get's own error but still raises a
/// `path_intact` failure; `(E)?` is `try E` and swallows it.
#[test]
fn an_opt_index_still_raises_a_path_error_and_try_does_not_5223() {
    on_big_stack(|| {
        check(&[
            (
                "null",
                "path(1|.a?)",
                err(
                    &[],
                    r#"Invalid path expression near attempt to access element "a" of 1"#,
                ),
            ),
            ("null", "[path(1|(.a)?)]", ok(&["[]"])),
            (
                r#"{"a":1}"#,
                "try path(1 | .b) catch .",
                ok(&[r#""Invalid path expression near attempt to access element \"b\" of 1""#]),
            ),
        ]);
    });
}

/// `TRY_END` wraps an error raised downstream of a body output, so the `try`
/// re-raises it; a `break` is an error a `try` catches (`GENLABEL`'s
/// `{"__jq": n}`); `?//` retries on any error after it, downstream included.
#[test]
fn try_label_and_alternatives_handle_errors_as_jq_5223() {
    on_big_stack(|| {
        check(&[
            ("null", r#"(try 1) | error("d")"#, err(&[], "d")),
            (
                "null",
                "label $f | try (break $f) catch .",
                ok(&[r#"{"__jq":0}"#]),
            ),
            (
                "null",
                r#"[[1] | . as [$a] ?// $a | if ($a|type) == "number" then error("retry") else $a end]"#,
                ok(&["[[1]]"]),
            ),
            (
                r#"{"a":{"b":1},"b":2}"#,
                "[. as [$q] ?// $q | $q]",
                ok(&[r#"[{"a":{"b":1},"b":2}]"#]),
            ),
        ]);
    });
}

/// `gen_array_matcher` emits the last element's `INDEX` first, so the second
/// `INDEX` of a two-element pattern is off the register.
#[test]
fn an_array_pattern_indexes_its_last_element_first_5223() {
    on_big_stack(|| {
        check(&[
            (
                "[1,2]",
                "path(. as [$a,$b] | .)",
                err(
                    &[],
                    "Invalid path expression near attempt to access element 0 of [1,2]",
                ),
            ),
            (
                r#"{"a":1,"b":2}"#,
                "path(. as {a:$a,b:$b} | .)",
                err(
                    &[],
                    r#"Invalid path expression near attempt to access element "b" of {"a":1,"b":2}"#,
                ),
            ),
        ]);
    });
}

/// `DUPN` moves the fold's input out of a slot `INIT`'s fork point shares, so
/// every `INIT` output after the first runs `SOURCE` on `null`.
#[test]
fn a_folds_later_init_outputs_see_a_null_source_input_5223() {
    on_big_stack(|| {
        check(&[
            (
                r#"{"a":1}"#,
                "[reduce .a as $i ((1,2); [$i,.])]",
                ok(&["[[1,1],[null,2]]"]),
            ),
            (
                r#"{"a":1}"#,
                "[foreach .a as $i ((1,2,3); [$i,.])]",
                ok(&["[[1,1],[null,2],[null,3]]"]),
            ),
            (
                r#"{"a":{"a":null}}"#,
                "path(foreach .a as $i ((.,.); .; $i))",
                err(
                    &[r#"["a"]"#],
                    r#"Invalid path expression near attempt to access element "a" of null"#,
                ),
            ),
        ]);
    });
}

/// `gen_index` evaluates the key on the node's own input, before the target;
/// 1.7.1 keeps a negative index as written; `getpath` moves the register only
/// from an identical input; `range/2` emits its start value itself first.
#[test]
fn navigation_records_jqs_own_components_5223() {
    on_big_stack(|| {
        check(&[
            (r#"{"a":{"x":1},"b":"x"}"#, "[.a[.b]]", ok(&["[1]"])),
            ("[1,2,3]", "path(.[-1])", ok(&["[-1]"])),
            (
                r#"{"a":[1,2]}"#,
                "path(.a | first, last)",
                ok(&[r#"["a",0]"#, r#"["a",-1]"#]),
            ),
            (
                r#"{"a":[1,2]}"#,
                "path(.a[1:] | .[0])",
                ok(&[r#"["a",{"start":1,"end":null},0]"#]),
            ),
            (
                r#"{"a":{"b":1}}"#,
                r#"path(.a | getpath(["b"]) | getpath([]))"#,
                ok(&[r#"["a","b"]"#]),
            ),
            (
                r#"{"a":{"b":1}}"#,
                r#"path(getpath(["x","y"]))"#,
                ok(&[r#"["x","y"]"#]),
            ),
            (
                r#"{"a":{"b":1}}"#,
                r#"path(.a | {} | getpath(["b"]))"#,
                err(&[], "Invalid path expression with result null"),
            ),
            (
                r#"{"a":{"b":1},"b":2}"#,
                "path(range(.a.b; 3))",
                err(&[r#"["a","b"]"#], "Invalid path expression with result 2"),
            ),
        ]);
    });
}

/// The write doors run `builtin.jq`'s `_modify`/`_assign`/`del` over the
/// spike's own `path()`.
#[test]
fn writes_and_jq_defined_builtins_run_from_the_prelude_5223() {
    on_big_stack(|| {
        let doc = r#"{"a":{"b":1},"b":2}"#;
        check(&[
            (doc, "del(.a)", ok(&[r#"{"b":2}"#])),
            (doc, ".a.b |= .+1", ok(&[r#"{"a":{"b":2},"b":2}"#])),
            (doc, ".a.b += 5", ok(&[r#"{"a":{"b":6},"b":2}"#])),
            (doc, ".c //= 3", ok(&[r#"{"a":{"b":1},"b":2,"c":3}"#])),
            (doc, ".a = 9", ok(&[r#"{"a":9,"b":2}"#])),
            // 1.7.1's `_modify` collects every emptied path and deletes them
            // all at the end.
            (doc, ".[] |= empty", ok(&["{}"])),
            (doc, "[paths]", ok(&[r#"[["a"],["a","b"],["b"]]"#])),
            (doc, "del(..|select(. == 1))", ok(&[r#"{"a":{},"b":2}"#])),
            (doc, "pick(.a.b)", ok(&[r#"{"a":{"b":1}}"#])),
            (doc, "[limit(2; .[])]", ok(&[r#"[{"b":1},2]"#])),
            (doc, "to_entries | from_entries", ok(&[doc])),
            ("[1,[2,[3]]]", "flatten", ok(&["[1,2,3]"])),
            ("[1,2]", r#"join(",")"#, ok(&[r#""1,2""#])),
        ]);
    });
}

/// A construct the spike does not implement ends the run as unsupported,
/// never as a jq error a `try` could swallow.
#[test]
fn an_unimplemented_construct_is_unsupported_not_an_error_5223() {
    on_big_stack(|| {
        let run = run("try input catch 1", &OwnedValue::Null).expect("parses");
        assert!(
            matches!(run.outcome, Outcome::Unsupported(_)),
            "{:?}",
            run.outcome
        );
    });
}

/// Every jq-defined builtin [`prelude_call`] maps resolves to a definition of
/// the right arity in [`PRELUDE`].
#[test]
fn every_prelude_call_resolves_5223() {
    on_big_stack(|| {
        let prelude = Prelude::new();
        let interp = Interp::new(&prelude);
        let calls = [
            "select(.)",
            "map(.)",
            "map_values(.)",
            "recurse",
            "recurse(.[]?)",
            "recurse(.[]?; .)",
            "add",
            "del(.a)",
            "to_entries",
            "from_entries",
            "with_entries(.)",
            "reverse",
            "paths",
            "paths(.)",
            "arrays",
            "objects",
            "iterables",
            "booleans",
            "numbers",
            "normals",
            "finites",
            "strings",
            "nulls",
            "values",
            "scalars",
            r#"join(",")"#,
            "flatten",
            "flatten(1)",
            "first",
            "last",
            "nth(0)",
            "isempty(.)",
            "any",
            "all",
            "any(.)",
            "all(.)",
            "any(.[]; .)",
            "all(.[]; .)",
            "in({})",
            "inside({})",
            "IN(1)",
            "IN(1; 1)",
            "ascii_downcase",
            "ascii_upcase",
            "walk(.)",
            "pick(.a)",
            "abs",
            "sort_by(.)",
            "group_by(.)",
            "unique",
            "unique_by(.)",
            "min_by(.)",
            "max_by(.)",
            "indices(1)",
            "index(1)",
            "rindex(1)",
            "debug(.)",
        ];
        for call in calls {
            let expr = parse(call).expect(call);
            let Expr::Builtin(builtin) = &expr else {
                panic!("{call} parsed as {expr:?}");
            };
            let (name, args) =
                prelude_call(builtin).unwrap_or_else(|| panic!("{call} is not mapped"));
            assert!(
                interp.prelude_env.lookup_func(name, args.len()).is_some(),
                "{call} -> {name}/{} is not in the prelude",
                args.len()
            );
        }
    });
}
