//! A tab before `#` starts a comment in a plain key, the same as a space (#410).
//!
//! YAML's `s-b-comment ::= ( s-separate-in-line c-nb-comment-text? )? b-comment`
//! requires `s-separate-in-line` before the `#` that opens a comment, and
//! `s-white` is a space *or a tab*. `parse_unquoted_key`'s comment guard
//! (`src/yaml/parser.rs`) tested only for a preceding space, so `a\t# c: d` loaded
//! the comment text into the key as `{"a\t# c":"d"}`. Until #5062 both spellings were
//! then rejected as `KeyWithoutValue`, because the mapping-entry lookahead read the `: `
//! inside the comment as a key's colon; yq v4.53.3 reads them as the plain scalar `a`,
//! and so does succinctly now.
//!
//! This is the comment-indicator sibling of `tests/yaml_tab_separation_tests.rs`
//! (#370), which fixed the same "space but not tab" shape for the `:` value
//! indicator thirty lines away in the same function. A `#` that is *not* preceded
//! by whitespace stays part of the key either way — `a#b: value` is unaffected,
//! per `nb-ns-plain-in-line`.
//!
//! Inputs are byte literals so that no editor, `.gitattributes` or clippy lint can
//! quietly launder a tab into spaces.
//!
//! Run with: `cargo test --test yaml_tab_comment_tests`

use succinctly::yaml::{YamlError, YamlIndex, YamlValue};

/// Render as `succinctly yq -o json '.'` does, one JSON value per document.
/// Mirrors `yaml_to_json_documents` in `tests/yaml_test_suite.rs`.
fn to_json(yaml: &[u8]) -> Result<String, YamlError> {
    let index = YamlIndex::build(yaml)?;
    let root = index.root(yaml);
    Ok(match root.value() {
        YamlValue::Sequence(mut elements) => {
            let mut docs = Vec::new();
            while let Some((cursor, rest)) = elements.uncons_cursor() {
                docs.push(cursor.to_json());
                elements = rest;
            }
            docs.join(" ")
        }
        _ => root.to_json_document(),
    })
}

/// Renders a byte string with its tabs visible in assertion output.
struct Text<'a>(&'a [u8]);

impl core::fmt::Debug for Text<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", String::from_utf8_lossy(self.0))
    }
}

/// A document where a tab before `#` starts a comment, so the line is the plain scalar
/// before it (never a key holding the comment text), with yq v4.53.3's JSON for it and
/// the reason it's here.
struct CommentRow {
    yaml: &'static [u8],
    json: &'static str,
    why: &'static str,
}

const COMMENTED: &[CommentRow] = &[
    CommentRow {
        yaml: b"a\t# c: d\n",
        json: r#""a""#,
        why: "the #410 repro: loaded as {\"a\\t# c\":\"d\"} before the fix",
    },
    CommentRow {
        yaml: b"a # c: d\n",
        json: r#""a""#,
        why: "the space form",
    },
    CommentRow {
        yaml: b"a\t\t# c: d\n",
        json: r#""a""#,
        why: "a run of tabs: the guard checks the byte immediately before #, not a scan",
    },
    CommentRow {
        yaml: b"- a\t# c: d\n",
        json: r#"["a"]"#,
        why: "a sequence item, the compact-entry path",
    },
];

#[track_caller]
fn assert_scalar_before_the_comment(yaml: &[u8], json: &str, why: &str) {
    let got = to_json(yaml).unwrap_or_else(|err| panic!("{:?} — {why}: {err:?}", Text(yaml)));
    assert_eq!(got, json, "{:?} — {why}", Text(yaml));
}

#[test]
fn a_tab_before_hash_starts_a_comment_in_a_plain_key() {
    for row in COMMENTED {
        assert_scalar_before_the_comment(row.yaml, row.json, row.why);
    }
}

/// The other side of the rule: a `#` *not* preceded by whitespace is ordinary key
/// content, tab or otherwise, and must still parse.
#[test]
fn a_hash_not_preceded_by_whitespace_stays_part_of_the_key() {
    assert_eq!(
        to_json(b"a#b: value\n").expect("a#b is a valid key"),
        r#"{"a#b":"value"}"#,
        "# immediately after a key character is content, not a comment start"
    );
}

/// Value-side control: the comment guard for values (`parser.rs:1160`) already
/// handled tab correctly and must stay untouched by this fix.
#[test]
fn a_tab_before_hash_already_started_a_comment_in_a_value() {
    assert_eq!(
        to_json(b"name: Alice\t# comment\n").expect("trailing tab-comment is valid"),
        r#"{"name":"Alice"}"#,
        "the value-side guard was already tab-aware before #410"
    );
}

/// The property behind the table: whichever `s-white` byte spells the separation
/// before `#`, the document reads the same way. Stated as an invariance so a
/// future change has to break the *rule*, not just one pinned error case.
#[test]
fn spelling_the_separation_with_a_space_gives_the_same_answer() {
    for row in COMMENTED {
        let spaces: Vec<u8> = row
            .yaml
            .iter()
            .map(|&b| if b == b'\t' { b' ' } else { b })
            .collect();
        assert_scalar_before_the_comment(&spaces, row.json, row.why);
    }
}
