//! #2513: a write expands a merge key (`<<: *base`), and the expansion copied the merge source's
//! nested `&anchor` declarations next to the originals, so every later `*y` bound to the copy.
//! The merged-in fields now carry no declaration of their own. (yq itself keeps `<<: *base`
//! as written; that expansion is recorded in the yq limitations.)

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, filter, expected output).
const ROWS: &[(&str, &str, &str)] = &[
    // The issue's repro, with a later reference to the anchor.
    (
        "default: &d\n  a: &y 1\nitem:\n  <<: *d\n  b: 2\nz: *y\n",
        ".item.b = 3",
        "default: &d\n  a: &y 1\nitem:\n  a: 1\n  b: 3\nz: *y\n",
    ),
    // A container under the source's anchor, and an anchor inside it.
    (
        "default: &d\n  a: &y {k: &w 1}\nitem:\n  <<: *d\n  b: 2\nz: *y\nq: *w\n",
        ".z = 5",
        "default: &d\n  a: &y {k: &w 1}\nitem:\n  a: {k: 1}\n  b: 2\nz: 5\nq: *w\n",
    ),
    // The merging mapping keeps its own anchor.
    (
        "default: &d\n  a: &y 1\nitem: &i\n  <<: *d\n  b: 2\nj: *i\n",
        ".z = 5",
        "default: &d\n  a: &y 1\nitem: &i\n  a: 1\n  b: 2\nj: *i\nz: 5\n",
    ),
    // An inline source is written nowhere else, so it keeps its declaration.
    (
        "item:\n  <<: {a: &y 1}\n  b: 2\nz: *y\n",
        ".item.b = 3",
        "item:\n  a: &y 1\n  b: 3\nz: *y\n",
    ),
];

#[test]
fn test_merged_in_fields_do_not_redeclare_the_sources_anchors_2513() -> Result<()> {
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
