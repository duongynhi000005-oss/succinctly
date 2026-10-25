//! #2580: `JsonIndex::ib_select1_from` and `YamlIndex::ib_select1_from` share one gallop
//! (`util::gallop::select1_from`). Duplicated predicates diverge silently (#106), so this pins that
//! both call sites answer exactly like their own plain-bisection `ib_select1`, for every rank
//! and every seed word, including seeds past either end of the directory.

use succinctly::json::JsonIndex;
use succinctly::yaml::YamlIndex;

/// A JSON array of mixed widths: short numbers (dense interest bits) and a long string run (few
/// bits over many words), so the seeds drift in both directions.
fn json_doc() -> Vec<u8> {
    let mut s = String::from("[");
    for i in 0..300 {
        if i > 0 {
            s.push(',');
        }
        match i % 5 {
            0 => s.push_str(&format!("\"{}\"", "x".repeat(200))),
            1 => s.push_str(&i.to_string()),
            2 => s.push_str(&format!("{{\"k{i}\":[1,2,3]}}")),
            _ => s.push_str("true"),
        }
    }
    s.push(']');
    s.into_bytes()
}

fn yaml_doc() -> Vec<u8> {
    let mut s = String::new();
    for i in 0..300 {
        match i % 4 {
            0 => s.push_str(&format!("k{i}: \"{}\"\n", "x".repeat(200))),
            1 => s.push_str(&format!("k{i}: {i}\n")),
            2 => s.push_str(&format!("k{i}:\n  a: 1\n  b: [1, 2]\n")),
            _ => s.push_str(&format!("k{i}: true\n")),
        }
    }
    s.into_bytes()
}

/// Every rank `k` (and one past the last) against every seed in `0..=words + 2`.
fn check(
    what: &str,
    select: impl Fn(usize) -> Option<usize>,
    select_from: impl Fn(usize, usize) -> Option<usize>,
    words: usize,
) {
    let mut ranks = 0;
    while select(ranks).is_some() {
        ranks += 1;
    }
    assert!(
        ranks > 100,
        "{what}: the document is too small to exercise a gallop"
    );
    for k in 0..=ranks + 1 {
        let expected = select(k);
        for hint in 0..=words + 2 {
            assert_eq!(select_from(k, hint), expected, "{what}: k={k} hint={hint}");
        }
    }
}

#[test]
fn test_json_ib_select1_from_matches_bisection_for_every_seed_2580() {
    let doc = json_doc();
    let index = JsonIndex::build(&doc);
    check(
        "json",
        |k| index.ib_select1(k),
        |k, hint| index.ib_select1_from(k, hint),
        doc.len() / 64 + 1,
    );
}

#[test]
fn test_yaml_ib_select1_from_matches_bisection_for_every_seed_2580() {
    let doc = yaml_doc();
    let index = YamlIndex::build(&doc).expect("builds");
    check(
        "yaml",
        |k| index.ib_select1(k),
        |k, hint| index.ib_select1_from(k, hint),
        doc.len() / 64 + 1,
    );
}
