//! #4103: the slot scan resumes from the member a wide earlier scan last found (more than 32
//! members read), following the raw key/value chain, which holds nodes a merged mapping's member
//! list omits: the `<<` key, its value, an overridden duplicate. This asks whether the resumed
//! answer can differ from the full scan's for those nodes. It cannot be made to: each query reads
//! every member through a `select(true)` (the stage that leaves the node a bare cursor, so its slot
//! is found by the scan, which the remembered member then resumes), then asks `key`/`path` of the
//! `<<` value, and the answer is the one a fresh read gives and the one yq gives, for an alias, an inline mapping and a
//! sequence of aliases. An overridden duplicate is not reachable by any read (a `select` on its
//! value finds nothing in yq either).
//!
//! Every output is captured from yq v4.53.3.

use anyhow::Result;

use crate::run_yq_stdin;

/// 40 members after a `<<` entry, so the member scan is long enough to be remembered.
fn doc(merge: &str, tail: &str, head: &str) -> String {
    let mut text = String::from(head);
    text.push_str("item:\n");
    text.push_str(merge);
    for i in 0..40 {
        text.push_str(&format!("  k{i}: {i}\n"));
    }
    text.push_str(tail);
    text
}

fn alias_doc() -> String {
    doc(
        "  <<: *b\n",
        "  k5: dup\n  r: 3\n",
        "base: &b\n  p: 1\n  q: 2\n",
    )
}

fn inline_doc() -> String {
    doc("  <<: {x: 1, y: 2}\n", "  k5: dup\n  x: over\n", "")
}

fn sequence_doc() -> String {
    doc("  <<: [*a, *b]\n", "", "a: &a {x: 1}\nb: &b {y: 2}\n")
}

fn check(document: &str, rows: &[(&str, &str)]) -> Result<()> {
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, document, &["-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(&out, want, "{filter}");
    }
    Ok(())
}

#[test]
fn test_4103_alias_merge_value_after_a_wide_scan_matches_yq() -> Result<()> {
    check(
        &alias_doc(),
        &[
            (
                "[.item | (.[], .[\"<<\"]) | select(true) | path] | .[-3:]",
                "[[\"item\",\"k39\"],[\"item\",\"r\"],[\"item\",\"<<\"]]\n",
            ),
            (
                "[.item | (.[], .[\"<<\"]) | select(true) | key] | .[-3:]",
                "[\"k39\",\"r\",\"<<\"]\n",
            ),
            (
                "[.item | (.[\"<<\"], .[]) | select(true) | key] | .[0:3]",
                "[\"<<\",\"p\",\"q\"]\n",
            ),
            (
                "[.item | (.k39, .[\"<<\"], .k0) | select(true) | path]",
                "[[\"item\",\"k39\"],[\"item\",\"<<\"],[\"item\",\"k0\"]]\n",
            ),
            (
                "[.item | (.[], .[\"<<\"], .k5) | select(true) | key] | .[-4:]",
                "[\"k39\",\"r\",\"<<\",\"k5\"]\n",
            ),
        ],
    )
}

#[test]
fn test_4103_inline_merge_value_after_a_wide_scan_matches_yq() -> Result<()> {
    check(
        &inline_doc(),
        &[
            (
                "[.item | (.[], .[\"<<\"]) | select(true) | path] | .[-3:]",
                "[[\"item\",\"k38\"],[\"item\",\"k39\"],[\"item\",\"<<\"]]\n",
            ),
            (
                "[.item | (.[\"<<\"], .[]) | select(true) | key] | .[0:3]",
                "[\"<<\",\"x\",\"y\"]\n",
            ),
            (
                "[.item | (.k39, .[\"<<\"], .k0) | select(true) | path]",
                "[[\"item\",\"k39\"],[\"item\",\"<<\"],[\"item\",\"k0\"]]\n",
            ),
            (
                "[.item | (.[], .[\"<<\"], .k5) | select(true) | key] | .[-4:]",
                "[\"k38\",\"k39\",\"<<\",\"k5\"]\n",
            ),
        ],
    )
}

#[test]
fn test_4103_sequence_merge_value_after_a_wide_scan_matches_yq() -> Result<()> {
    check(
        &sequence_doc(),
        &[
            (
                "[.item | (.[], .[\"<<\"]) | select(true) | path] | .[-3:]",
                "[[\"item\",\"k38\"],[\"item\",\"k39\"],[\"item\",\"<<\"]]\n",
            ),
            (
                "[.item | (.[], .[\"<<\"]) | select(true) | key] | .[-3:]",
                "[\"k38\",\"k39\",\"<<\"]\n",
            ),
            (
                "[.item | (.k39, .[\"<<\"], .k0) | select(true) | path]",
                "[[\"item\",\"k39\"],[\"item\",\"<<\"],[\"item\",\"k0\"]]\n",
            ),
            (
                "[.item | (.[], .[\"<<\"], .k5) | select(true) | key] | .[-4:]",
                "[\"k38\",\"k39\",\"<<\",\"k5\"]\n",
            ),
        ],
    )
}
