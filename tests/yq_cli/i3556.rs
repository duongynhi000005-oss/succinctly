//! #3556: `keys`, `length` and `has` list a mapping's `<<` merge key as the ordinary key it is
//! spelled as, where a value read (`.p`, `.[]`, the document itself) goes through the merge.
//!
//! Every expected output is captured from yq v4.53.3 (`-o=json -I=0`, default flags, stdout only:
//! yq also prints a `--yaml-fix-merge-anchor-to-spec is false` warning on stderr). One test per
//! merge shape, so a regression names the shape.

use anyhow::Result;

use crate::run_yq_stdin;

fn check(input: &str, rows: &[(&str, &str)]) -> Result<()> {
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, input, &["-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(out, *want, "{filter}");
    }
    Ok(())
}

/// One source.
#[test]
fn test_3556_one() -> Result<()> {
    check(
        "base: &base\n  p: 1\n  q: 2\nitem:\n  <<: *base\n  r: 3\n",
        &[
            (".item | keys", "[\"<<\",\"r\"]\n"),
            (".item | length", "2\n"),
            (".item | has(\"p\")", "false\n"),
            (".item | has(\"<<\")", "true\n"),
            (".item | has(\"r\")", "true\n"),
            (".item | (keys | length) == length", "true\n"),
            (".item | [keys[]] | length", "2\n"),
            (".item | select(has(\"p\")) | length", ""),
        ],
    )
}

/// A sequence of sources.
#[test]
fn test_3556_sequence() -> Result<()> {
    check(
        "a: &a {x: 1, p: a}\nb: &b {x: 2, q: b}\nitem: {<<: [*a, *b], r: 3}\n",
        &[
            (".item | keys", "[\"<<\",\"r\"]\n"),
            (".item | length", "2\n"),
            (".item | has(\"p\")", "false\n"),
            (".item | has(\"<<\")", "true\n"),
            (".item | has(\"r\")", "true\n"),
            (".item | (keys | length) == length", "true\n"),
            (".item | [keys[]] | length", "2\n"),
            (".item | select(has(\"p\")) | length", ""),
        ],
    )
}

/// An own key written around the merge key, overriding a merged one.
#[test]
fn test_3556_override() -> Result<()> {
    check(
        "base: &base {p: 1, q: 2}\nitem:\n  r: 3\n  <<: *base\n  p: 9\n",
        &[
            (".item | keys", "[\"r\",\"<<\",\"p\"]\n"),
            (".item | length", "3\n"),
            (".item | has(\"p\")", "true\n"),
            (".item | has(\"<<\")", "true\n"),
            (".item | has(\"r\")", "true\n"),
            (".item | (keys | length) == length", "true\n"),
            (".item | [keys[]] | length", "3\n"),
            (".item | select(has(\"p\")) | length", "3\n"),
        ],
    )
}

/// An inline mapping as the source.
#[test]
fn test_3556_inline() -> Result<()> {
    check(
        "item:\n  <<: {p: 1, q: 2}\n  r: 3\n",
        &[
            (".item | keys", "[\"<<\",\"r\"]\n"),
            (".item | length", "2\n"),
            (".item | has(\"p\")", "false\n"),
            (".item | has(\"<<\")", "true\n"),
            (".item | has(\"r\")", "true\n"),
            (".item | (keys | length) == length", "true\n"),
            (".item | [keys[]] | length", "2\n"),
            (".item | select(has(\"p\")) | length", ""),
        ],
    )
}

/// A source that merges another.
#[test]
fn test_3556_nested() -> Result<()> {
    check(
        "b0: &b0 {z: 0}\nbase: &base\n  <<: *b0\n  p: 1\nitem:\n  <<: *base\n  r: 3\n",
        &[
            (".item | keys", "[\"<<\",\"r\"]\n"),
            (".item | length", "2\n"),
            (".item | has(\"p\")", "false\n"),
            (".item | has(\"<<\")", "true\n"),
            (".item | has(\"r\")", "true\n"),
            (".item | (keys | length) == length", "true\n"),
            (".item | [keys[]] | length", "2\n"),
            (".item | select(has(\"p\")) | length", ""),
        ],
    )
}

/// A merge key on the root mapping.
#[test]
fn test_3556_root() -> Result<()> {
    check(
        "x: &x {p: 1}\n<<: *x\nr: 2\n",
        &[
            ("keys", "[\"x\",\"<<\",\"r\"]\n"),
            ("length", "3\n"),
            ("has(\"p\")", "false\n"),
            ("has(\"<<\")", "true\n"),
        ],
    )
}

/// `has` with a non-string key takes its own arm (the key's text is compared, #2800).
#[test]
fn test_3556_has_with_a_number_key() -> Result<()> {
    check(
        "base: &base {7: y}\nitem: {<<: *base, 1: z}\n",
        &[
            (".item | has(7)", "false\n"),
            (".item | has(1)", "true\n"),
            (".item | has(\"7\")", "false\n"),
            (".item | keys", "[\"<<\",1]\n"),
            (".item | length", "2\n"),
        ],
    )
}
