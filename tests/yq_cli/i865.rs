//! #865: `{...}` and `[...]` built from nodes of the document kept none of their presentation --
//! quote style, flow style, trailing comment, anchor -- where yq keeps each node's own. A computed
//! entry (`.a + "x"`) takes what its left operand had, as in yq, and a literal takes nothing.
//! Each row is a live yq v4.53.3 capture over the document below.

use anyhow::Result;

use crate::run_yq_stdin;

const DOC: &str = "a: 'single' # ca\nb: \"double\" # cb\nc: [1, 2]\nd: {x: 1}\ne: &an 5\n";

/// (filter, expected output).
const ROWS: &[(&str, &str)] = &[
    (
        r#"{"a": .a, "b": .b}"#,
        "a: 'single' # ca\nb: \"double\" # cb\n",
    ),
    ("[.a, .b]", "- 'single' # ca\n- \"double\" # cb\n"),
    (
        r#"{"a": (.a + "x"), "b": .b}"#,
        "a: 'singlex' # ca\nb: \"double\" # cb\n",
    ),
    (r#"{"a": {"b": .b}}"#, "a:\n  b: \"double\" # cb\n"),
    ("[[.a]]", "- - 'single' # ca\n"),
    (r#"{"k": .c}"#, "k: [1, 2]\n"),
    (".c | [.]", "- [1, 2]\n"),
    (
        r#"{"x": .d, "y": [.e, .e]}"#,
        "x: {x: 1}\ny:\n  - &an 5\n  - &an 5\n",
    ),
    (r#"{"a": .a, "n": 5}"#, "a: 'single' # ca\nn: 5\n"),
    (r#"[.a, 5, "s"]"#, "- 'single' # ca\n- 5\n- s\n"),
    (r#"{"a": .zz}"#, "a: null\n"),
    (
        r#"{"m": (.a | length), "n": .b}"#,
        "m: 6\nn: \"double\" # cb\n",
    ),
];

#[test]
fn test_construction_keeps_the_presentation_of_the_nodes_it_reads_865() -> Result<()> {
    for (filter, expected) in ROWS {
        let (stdout, code) = run_yq_stdin(filter, DOC, &[])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{filter}");
    }
    Ok(())
}

/// (document, filter, expected output): where yq's own placement is not the obvious one.
const QUIRKS: &[(&str, &str, &str)] = &[
    // A standalone comment above a node is the key's (or the document's), so a construction
    // leaves it behind and keeps the node's foot.
    (
        "# head\na: 1 # one\n\n# mid\nb:\n  - x # ix\n  - y\n  # foot\n",
        r#"{"a": .a, "b": .b}"#,
        "a: 1 # one\nb:\n  - x # ix\n  - y\n  # foot\n",
    ),
    (
        "# head\na: 1 # one\n\n# mid\nb:\n  - x # ix\n  - y\n  # foot\n",
        r#"{"x": .}"#,
        "x:\n  a: 1 # one\n  # mid\n  b:\n    - x # ix\n    - y\n    # foot\n",
    ),
    // A non-empty flow mapping is a block once it is not the first entry of a constructed
    // mapping, and so is each mapping nested in it by mappings; a sequence keeps its style and
    // what it holds, and an array keeps every element's.
    (
        "a: 1 # one\nc: {k: v}\nq: [1, 2]\n",
        r#"{"d": .d, "c": .c}"#,
        "d: null\nc:\n  k: v\n",
    ),
    (
        "a: 1 # one\nc: {k: v}\nq: [1, 2]\n",
        r#"{"c": .c, "d": .d}"#,
        "c: {k: v}\nd: null\n",
    ),
    (
        "a: 1 # one\nc: {k: v}\nq: [1, 2]\n",
        r#"{"d": .a, "c": .c}"#,
        "d: 1 # one\nc:\n  k: v\n",
    ),
    (
        "a: 1\nc: {k: {z: 1}, w: [1]}\nd:\n  e: {f: 1}\n",
        r#"{"a": .a, "c": .c}"#,
        "a: 1\nc:\n  k:\n    z: 1\n  w: [1]\n",
    ),
    (
        "a: 1\nc: {k: {z: 1}, w: [1]}\nd:\n  e: {f: 1}\n",
        r#"{"a": .a, "d": .d}"#,
        "a: 1\nd:\n  e:\n    f: 1\n",
    ),
    (
        "a: 1\nc: {k: {z: 1}, w: [1]}\nd:\n  e: {f: 1}\n",
        "[.a, .c]",
        "- 1\n- {k: {z: 1}, w: [1]}\n",
    ), // The foot of a scalar's key stays behind with it, a comment trailing a flow mapping that
    // becomes a block is dropped, other arithmetic keeps the left operand's presentation, a null
    // left operand takes the right's, and the comment above a sequence element goes with it into
    // an array and below the entry in a mapping.
    (
        "a:\n  b: x\n  # inner foot\nc: 1\n",
        r#"{"x": .a.b}"#,
        "x: x\n",
    ),
    (
        "z: 1\na: {p: 1, q: 2} # c\n",
        r#"{"p": .z, "y": .a}"#,
        "p: 1\ny:\n  p: 1\n  q: 2\n",
    ),
    ("z: 5 # c\n", "[(.z - 1)]", "- 4 # c\n"),
    ("a: x # c\n", r#"{"x": (.n + .a)}"#, "x: x # c\n"),
    (
        "# head\n- 'a' # t\n# mid\n- b\n",
        "[.[0], .[1]]",
        "- 'a' # t\n# mid\n- b\n",
    ),
    (
        "# head\n- 'a' # t\n# mid\n- b\n",
        r#"{"k": .[1]}"#,
        "k: b\n# mid\n",
    ),
];

#[test]
fn test_construction_follows_yqs_placement_865() -> Result<()> {
    for (doc, filter, expected) in QUIRKS {
        let (stdout, code) = run_yq_stdin(filter, doc, &[])?;
        assert_eq!(
            (stdout.as_str(), code),
            (*expected, 0),
            "{filter} over {doc:?}"
        );
    }
    Ok(())
}

/// What is not replayed keeps the plain route: several outputs, a repeated key, a computed key.
/// The values are right on every route; only the presentation is left out.
#[test]
fn test_construction_outside_the_replay_prints_plain_865() -> Result<()> {
    for (filter, expected) in [
        (r#"{"q": (.a, .b)}"#, "q: single\nq: double\n"),
        (r#"{"a": .a, "a": .b}"#, "a: double\n"),
        ("[.[]] | length", "5\n"),
        // A construction a later stage reshapes is not followed through it.
        (r#"{"a": .a} | .b = 1"#, "a: single\nb: 1\n"),
    ] {
        let (stdout, code) = run_yq_stdin(filter, DOC, &[])?;
        assert_eq!((stdout.as_str(), code), (expected, 0), "{filter}");
    }
    Ok(())
}

/// `-P` drops the styles but keeps the comments, and `-o json` has no presentation to keep.
#[test]
fn test_construction_presentation_follows_pretty_print_and_json_865() -> Result<()> {
    let (stdout, _) = run_yq_stdin(r#"{"a": .a, "b": .b}"#, DOC, &["-P"])?;
    assert_eq!(stdout, "a: single # ca\nb: double # cb\n");
    let (stdout, _) = run_yq_stdin(r#"{"a": .a, "b": .b}"#, DOC, &["-o", "json", "-I", "0"])?;
    assert_eq!(stdout, "{\"a\":\"single\",\"b\":\"double\"}\n");
    Ok(())
}
