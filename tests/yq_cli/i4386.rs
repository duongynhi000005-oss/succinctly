//! #4386: `sort` and `sort_by(f)` of a mapping keep what each entry carries: its comments, its
//! flow style, its anchor or alias, its tag, and a numeric key as written. The mapping is the same
//! entries in another order, so each entry's presentation is carried to its new place; the head
//! comment of the document stays on top.

use anyhow::Result;

use crate::run_yq_stdin;

type Row = (
    &'static str,
    &'static [&'static str],
    &'static str,
    &'static str,
);

/// Every row is captured from yq v4.53.3: the document, the extra arguments, the filter and the
/// expected output. A mapping sorted at the root, behind a path (`.o | sort`) and in a second
/// document all take the route that carries the presentation; the array row is the control that
/// keeps streaming its elements.
#[test]
fn test_4386_a_sorted_mapping_keeps_its_entries_presentation() -> Result<()> {
    let rows: &[Row] = &[
        (
            "# top\nb: 2 # two\na: 1 # one\n",
            &[],
            "sort",
            "# top\na: 1 # one\nb: 2 # two\n",
        ),
        (
            "# top\nb: 2 # two\na: 1 # one\n",
            &[],
            "sort_by(.)",
            "# top\na: 1 # one\nb: 2 # two\n",
        ),
        (
            "# top\nb: 2 # two\na: 1 # one\n",
            &[],
            "sort_by(.z)",
            "# top\nb: 2 # two\na: 1 # one\n",
        ),
        (
            "# top\nb: 2 # two\na: 1 # one\n",
            &["-I=4"],
            "sort",
            "# top\na: 1 # one\nb: 2 # two\n",
        ),
        (
            "k: {z: 1}\nj: {z: 0}\n",
            &[],
            "sort",
            "k: {z: 1}\nj: {z: 0}\n",
        ),
        (
            "k: {z: 1}\nj: {z: 0}\n",
            &[],
            "sort_by(.z)",
            "j: {z: 0}\nk: {z: 1}\n",
        ),
        ("2: a\n1: b\n", &[], "sort", "2: a\n1: b\n"),
        (
            "b: &x 2\na: *x\nc: 1 # c\n",
            &[],
            "sort",
            "c: 1 # c\nb: &x 2\na: *x\n",
        ),
        (
            "b: &x 2\na: *x\nc: 1 # c\n",
            &[],
            "sort_by(.z)",
            "b: &x 2\na: *x\nc: 1 # c\n",
        ),
        (
            "o:\n  b: 2 # two\n  a: 1 # one\n  # foot\nn: 0\n",
            &[],
            ".o | sort",
            "a: 1 # one\n# foot\n\nb: 2 # two\n",
        ),
        (
            "o:\n  b: 2 # two\n  a: 1 # one\n  # foot\nn: 0\n",
            &[],
            "sort",
            "o:\n  b: 2 # two\n  a: 1 # one\n  # foot\nn: 0\n",
        ),
        (
            "b: !!str 2\n? a\n: 1\n\"c\": 3\n",
            &[],
            "sort",
            "a: 1\nb: !!str 2\n\"c\": 3\n",
        ),
        (
            "b: !!str 2\n? a\n: 1\n\"c\": 3\n",
            &[],
            "sort_by(.z)",
            "b: !!str 2\na: 1\n\"c\": 3\n",
        ),
        (
            "# top\nb: 2 # two\na: 1 # one\n---\nz: 9 # z\ny: 8 # y\n",
            &[],
            "sort",
            "# top\na: 1 # one\nb: 2 # two\n---\ny: 8 # y\nz: 9 # z\n",
        ),
        (
            "# top\nb: 2 # two\na: 1 # one\n",
            &["-P"],
            "sort",
            "# top\na: 1 # one\nb: 2 # two\n",
        ),
        (
            "# top\nb: 2 # two\na: 1 # one\n",
            &["-o=json", "-I=0"],
            "sort",
            "{\"a\":1,\"b\":2}\n",
        ),
        (
            "- 2 # two\n- 1 # one\n",
            &[],
            "sort_by(.)",
            "- 1 # one\n- 2 # two\n",
        ),
    ];
    for (doc, extra, filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, doc, extra)?;
        assert_eq!(code, 0, "{doc:?} {extra:?} {filter}");
        assert_eq!(out, *want, "{doc:?} {extra:?} {filter}");
    }
    Ok(())
}
