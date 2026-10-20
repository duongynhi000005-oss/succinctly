//! #4997 (item 2 of the #4099 sweep): a write that makes a document's root another kind -- a
//! comment-only file, `null`, `~` or a scalar written to a mapping or sequence, or back -- kept the
//! preamble only and dropped the comments around the document. yq keeps its head comment and, for
//! a container result, its foot. Each row is a live yq v4.53.3 capture.
//!
//! Not matched (the yq limitations): a non-first document's head, and `# c` / `!!null`.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, filter, expected output).
const ROWS: &[(&str, &str, &str)] = &[
    ("# c\n", ".a = 1", "# c\na: 1\n"),
    ("# c\n", ". = [1]", "# c\n- 1\n"),
    ("# c\n\n", ".a = 1", "# c\n\na: 1\n"),
    ("# c\n# d\n", ".a = 1", "# c\n# d\na: 1\n"),
    ("# c\nnull\n", ".a = 1", "# c\na: 1\n"),
    ("# c\n~\n", ".a = 1", "# c\na: 1\n"),
    ("# c\n5\n", ". = [1]", "# c\n- 1\n"),
    ("# c\n5 # l\n# f\n", ". = {\"a\":1}", "# c\na: 1\n# f\n"),
    ("# c\n[1]\n", ". = {\"a\":1}", "# c\na: 1\n"),
    ("# c\n{a: 1}\n", ". = [1]", "# c\n- 1\n"),
    ("# c\n{a: 1}\n# f\n", ". = [1]", "# c\n- 1\n# f\n"),
    ("# c\n[1]\n# f\n", ". = 5", "# c\n5\n"),
    ("# c\nnull\n# f\n", ".a = 1", "# c\na: 1\n# f\n"),
    ("# c\n# d\n", ".[0] = 1", "# c\n# d\n- 1\n"),
    ("# c\n\n# d\n", ".a = 1", "# c\n\n# d\na: 1\n"),
    (
        "# c\nnull\n---\n# e\nx: 1\n",
        ".a = 1",
        "# c\na: 1\n---\n# e\nx: 1\na: 1\n",
    ),
    ("x: 1\n---\n# c\n", ".a = 1", "x: 1\na: 1\n# c\n---\na: 1\n"),
    ("---\n# c\nnull\n", ".a = 1", "---\n# c\na: 1\n"),
    (
        "# top\n---\n# c\nnull\n",
        ".a = 1",
        "# top\n---\n# c\na: 1\n",
    ),
    ("# c\nnull # l\n", ".a = 1", "# c\na: 1\n"),
    ("# c\n&x null\n", ".a = 1", "# c\n&x\na: 1\n"),
];

/// A computed result that no write produced is not the document: it gains none of its comments.
#[test]
fn test_a_computed_result_does_not_take_the_roots_comments_4997() -> Result<()> {
    // A comment after the last member is that key's foot, which a construction leaves with the key
    // (#865); one inside a container (`  # f`) is a foot of a key of its own and stays.
    for (doc, filter, expected) in [
        ("d:\n  x: 2\n# f\n", "[.d]", "- x: 2\n"),
        ("d:\n  - 1\n# f\n", r#"{"k": .d}"#, "k:\n  - 1\n"),
        ("d:\n  x: 2\n  # f\n", "[.d]", "- x: 2\n  # f\n"),
    ] {
        let (stdout, code) = run_yq_stdin(filter, doc, &[])?;
        assert_eq!(
            (stdout.as_str(), code),
            (expected, 0),
            "{filter} over {doc:?}"
        );
    }
    let doc = "# c\nb: 1\nd: {x: 2}\n# f\n";
    for (filter, expected) in [
        ("[.d]", "- {x: 2}\n"),
        ("[.b, .d]", "- 1\n- {x: 2}\n"),
        (r#"{"k": .d}"#, "k: {x: 2}\n"),
    ] {
        let (stdout, code) = run_yq_stdin(filter, doc, &[])?;
        assert_eq!((stdout.as_str(), code), (expected, 0), "{filter}");
    }
    Ok(())
}

/// The shapes that still differ from yq, pinned so a fix shows up here: the head comment of a
/// non-first document is lost (yq keeps `# d`).
#[test]
fn test_known_divergence_non_first_document_head_4997() -> Result<()> {
    let (stdout, code) = run_yq_stdin(".a = 1", "# c\nnull\n---\n# d\nnull\n", &[])?;
    assert_eq!((stdout.as_str(), code), ("# c\na: 1\n---\na: 1\n", 0));
    Ok(())
}

#[test]
fn test_root_kind_change_keeps_the_comments_around_the_document_4997() -> Result<()> {
    for (doc, filter, expected) in ROWS {
        let (stdout, code) = run_yq_stdin(filter, doc, &[])?;
        assert_eq!(
            (stdout.as_str(), code),
            (*expected, 0),
            "{filter} over {doc:?}"
        );
    }
    Ok(())
}
