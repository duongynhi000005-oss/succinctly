//! #4983: yq iterates a mapping with a sequence merge in a different order, and with a different value for a key two
//! sources share, than it renders it.

use anyhow::Result;

use crate::run_yq_stdin;

/// #4983: with `a: {x: 1, z: 9}` and `b: {y: 2, z: 8}`, yq renders `<<: [*a, *b]` as `{"y":2,"z":9,"x":1,"k":0}` (a
/// sequence folded in reverse) but iterates it (`.[]`, `.[] | key`, `map(.)`) in listed order, each source's field
/// upserting in place: keys `x, z, y, k`, values `1, 8, 2, 0`. An own key written after the merge overrides the
/// merged value in place and one written before it is overridden by it. The single-source forms and the rendering
/// agree both ways. Every row is captured from yq v4.53.3 (`-o=json -I0`) over 13 merge shapes.
#[test]
fn test_4983_iteration_and_rendering_of_a_merged_mapping_follow_yq() -> Result<()> {
    let rows: &[(&str, &str, &str)] = &[
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  k: 0\n", "[.item|.[]|key]", "[\"x\",\"z\",\"k\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  k: 0\n", "[.item|.[]]", "[1,9,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  k: 0\n", ".item", "{\"x\":1,\"z\":9,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  k: 0\n", "[.item|.[]|[key,.]]", "[[\"x\",1],[\"z\",9],[\"k\",0]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  k: 0\n", ".item|map(.)", "[1,9,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  k: 0\n  <<: *a\n", "[.item|.[]|key]", "[\"k\",\"x\",\"z\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  k: 0\n  <<: *a\n", "[.item|.[]]", "[0,1,9]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  k: 0\n  <<: *a\n", ".item", "{\"k\":0,\"x\":1,\"z\":9}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  k: 0\n  <<: *a\n", "[.item|.[]|[key,.]]", "[[\"k\",0],[\"x\",1],[\"z\",9]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  k: 0\n  <<: *a\n", ".item|map(.)", "[0,1,9]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  z: 5\n  k: 0\n", "[.item|.[]|key]", "[\"x\",\"z\",\"k\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  z: 5\n  k: 0\n", "[.item|.[]]", "[1,5,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  z: 5\n  k: 0\n", ".item", "{\"x\":1,\"z\":5,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  z: 5\n  k: 0\n", "[.item|.[]|[key,.]]", "[[\"x\",1],[\"z\",5],[\"k\",0]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  z: 5\n  k: 0\n", ".item|map(.)", "[1,5,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  z: 5\n  <<: *a\n  k: 0\n", "[.item|.[]|key]", "[\"z\",\"x\",\"k\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  z: 5\n  <<: *a\n  k: 0\n", "[.item|.[]]", "[9,1,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  z: 5\n  <<: *a\n  k: 0\n", ".item", "{\"z\":9,\"x\":1,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  z: 5\n  <<: *a\n  k: 0\n", "[.item|.[]|[key,.]]", "[[\"z\",9],[\"x\",1],[\"k\",0]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  z: 5\n  <<: *a\n  k: 0\n", ".item|map(.)", "[9,1,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a]\n  k: 0\n", "[.item|.[]|key]", "[\"x\",\"z\",\"k\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a]\n  k: 0\n", "[.item|.[]]", "[1,9,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a]\n  k: 0\n", ".item", "{\"x\":1,\"z\":9,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a]\n  k: 0\n", "[.item|.[]|[key,.]]", "[[\"x\",1],[\"z\",9],[\"k\",0]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a]\n  k: 0\n", ".item|map(.)", "[1,9,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b]\n  k: 0\n", "[.item|.[]|key]", "[\"x\",\"z\",\"y\",\"k\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b]\n  k: 0\n", "[.item|.[]]", "[1,8,2,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b]\n  k: 0\n", ".item", "{\"y\":2,\"z\":9,\"x\":1,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b]\n  k: 0\n", "[.item|.[]|[key,.]]", "[[\"x\",1],[\"z\",8],[\"y\",2],[\"k\",0]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b]\n  k: 0\n", ".item|map(.)", "[1,8,2,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*b, *a]\n  k: 0\n", "[.item|.[]|key]", "[\"y\",\"z\",\"x\",\"k\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*b, *a]\n  k: 0\n", "[.item|.[]]", "[2,9,1,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*b, *a]\n  k: 0\n", ".item", "{\"x\":1,\"z\":8,\"y\":2,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*b, *a]\n  k: 0\n", "[.item|.[]|[key,.]]", "[[\"y\",2],[\"z\",9],[\"x\",1],[\"k\",0]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*b, *a]\n  k: 0\n", ".item|map(.)", "[2,9,1,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b, *c]\n  k: 0\n", "[.item|.[]|key]", "[\"x\",\"z\",\"y\",\"w\",\"k\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b, *c]\n  k: 0\n", "[.item|.[]]", "[7,8,2,3,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b, *c]\n  k: 0\n", ".item", "{\"w\":3,\"x\":1,\"y\":2,\"z\":9,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b, *c]\n  k: 0\n", "[.item|.[]|[key,.]]", "[[\"x\",7],[\"z\",8],[\"y\",2],[\"w\",3],[\"k\",0]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b, *c]\n  k: 0\n", ".item|map(.)", "[7,8,2,3,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b]\n  z: 5\n  k: 0\n", "[.item|.[]|key]", "[\"x\",\"z\",\"y\",\"k\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b]\n  z: 5\n  k: 0\n", "[.item|.[]]", "[1,5,2,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b]\n  z: 5\n  k: 0\n", ".item", "{\"y\":2,\"z\":5,\"x\":1,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b]\n  z: 5\n  k: 0\n", "[.item|.[]|[key,.]]", "[[\"x\",1],[\"z\",5],[\"y\",2],[\"k\",0]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b]\n  z: 5\n  k: 0\n", ".item|map(.)", "[1,5,2,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  z: 5\n  <<: [*a, *b]\n  k: 0\n", "[.item|.[]|key]", "[\"z\",\"x\",\"y\",\"k\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  z: 5\n  <<: [*a, *b]\n  k: 0\n", "[.item|.[]]", "[8,1,2,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  z: 5\n  <<: [*a, *b]\n  k: 0\n", ".item", "{\"z\":9,\"y\":2,\"x\":1,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  z: 5\n  <<: [*a, *b]\n  k: 0\n", "[.item|.[]|[key,.]]", "[[\"z\",8],[\"x\",1],[\"y\",2],[\"k\",0]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  z: 5\n  <<: [*a, *b]\n  k: 0\n", ".item|map(.)", "[8,1,2,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  k: 0\n  <<: [*a, *b]\n", "[.item|.[]|key]", "[\"k\",\"x\",\"z\",\"y\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  k: 0\n  <<: [*a, *b]\n", "[.item|.[]]", "[0,1,8,2]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  k: 0\n  <<: [*a, *b]\n", ".item", "{\"k\":0,\"y\":2,\"z\":9,\"x\":1}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  k: 0\n  <<: [*a, *b]\n", "[.item|.[]|[key,.]]", "[[\"k\",0],[\"x\",1],[\"z\",8],[\"y\",2]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  k: 0\n  <<: [*a, *b]\n", ".item|map(.)", "[0,1,8,2]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  k: 0\n  <<: *b\n", "[.item|.[]|key]", "[\"x\",\"z\",\"k\",\"y\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  k: 0\n  <<: *b\n", "[.item|.[]]", "[1,8,0,2]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  k: 0\n  <<: *b\n", ".item", "{\"x\":1,\"z\":8,\"k\":0,\"y\":2}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  k: 0\n  <<: *b\n", "[.item|.[]|[key,.]]", "[[\"x\",1],[\"z\",8],[\"k\",0],[\"y\",2]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: *a\n  k: 0\n  <<: *b\n", ".item|map(.)", "[1,8,0,2]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*c, *a, *b]\n  k: 0\n", "[.item|.[]|key]", "[\"w\",\"x\",\"z\",\"y\",\"k\"]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*c, *a, *b]\n  k: 0\n", "[.item|.[]]", "[3,1,8,2,0]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*c, *a, *b]\n  k: 0\n", ".item", "{\"y\":2,\"z\":9,\"x\":7,\"w\":3,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*c, *a, *b]\n  k: 0\n", "[.item|.[]|[key,.]]", "[[\"w\",3],[\"x\",1],[\"z\",8],[\"y\",2],[\"k\",0]]"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*c, *a, *b]\n  k: 0\n", ".item|map(.)", "[3,1,8,2,0]"),
    ];
    for (input, filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, input, &["-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter} on {input:?}: {out:?}");
        assert_eq!(out.trim_end(), *want, "{filter} on {input:?}");
    }
    Ok(())
}

/// #4983 (review): the form that rebuilds the mapping keeps reading the rendering's order, not the
/// iteration's: `with_entries(.)` (yq renders the rebuilt mapping).
/// Rows where `succinctly yq` already differed from yq for another reason (`to_entries` lists the raw `<<`,
/// #4954) are not pinned. Captured from yq v4.53.3.
#[test]
fn test_4983_rebuilding_forms_keep_the_rendering_order() -> Result<()> {
    let rows: &[(&str, &str, &str)] = &[
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*a, *b]\n  k: 0\n", ".item|with_entries(.)", "{\"y\":2,\"z\":9,\"x\":1,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*b, *a]\n  k: 0\n", ".item|with_entries(.)", "{\"x\":1,\"z\":8,\"y\":2,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  <<: [*c, *a, *b]\n  k: 0\n", ".item|with_entries(.)", "{\"y\":2,\"z\":9,\"x\":7,\"w\":3,\"k\":0}"),
        ("a: &a {x: 1, z: 9}\nb: &b {y: 2, z: 8}\nc: &c {w: 3, x: 7}\nitem:\n  k: 0\n  <<: [*a, *b]\n", ".item|with_entries(.)", "{\"k\":0,\"y\":2,\"z\":9,\"x\":1}"),
    ];
    for (input, filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, input, &["-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter} on {input:?}: {out:?}");
        assert_eq!(out.trim_end(), *want, "{filter} on {input:?}");
    }
    Ok(())
}

/// #4983 (review): a write that carries the comment tree (`. + {}`) builds it from the same rendering view as
/// the value, so a mapping with a sequence merge keeps every comment and the anchors (it lost all of them when the
/// value was rendered and the tree iterated).
#[test]
fn test_4983_comment_tree_matches_the_rendered_value_for_a_sequence_merge() -> Result<()> {
    let input = "a: &a {x: 1, z: 9} # ca\nb: &b {y: 2, z: 8}\nitem:\n  <<: [*a, *b]\n  k: 0 # c3\nother: 1 # co\n";
    let (out, code) = run_yq_stdin(". + {}", input, &[])?;
    assert_eq!(code, 0, "{out:?}");
    for kept in ["# ca", "# c3", "# co", "&a {x: 1, z: 9}"] {
        assert!(out.contains(kept), "{kept:?} lost: {out}");
    }
    Ok(())
}
