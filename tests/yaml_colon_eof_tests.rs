//! A colon at absolute end-of-input ends a plain scalar the same way one
//! followed by white space does (#434).
//!
//! `parse_unquoted_value_with_indent_impl`'s colon-terminator check
//! (`src/yaml/parser.rs`) matched `Some(b' ' | b'\t' | b'\n' | b'\r')` but had
//! no `None` arm, so a colon as the very last byte of the document (no
//! trailing newline) was absorbed as scalar content instead of ending the
//! value. `find_scalar_end` (`src/yaml/light.rs`), the cursor's independent
//! re-derivation of a scalar's extent used by `at_offset`/`syq-locate`,
//! already had an explicit EOF check (added for #370's `a:\t1` case) - so
//! `syq` printed `abc:` for `key: abc:` while `syq-locate` on that same node
//! reported a byte range covering only `abc`, the same eval/locate
//! divergence shape as #370, just reached via absolute EOF instead of a
//! missing tab.
//!
//! Since #4463 the document is rejected: a value's scalar must be the last thing on
//! its line, and a strict parser (`yq`, PyYAML) refuses `key: abc:` outright
//! ("mapping values are not allowed in this context"), so the scanner stopping at the
//! colon now surfaces as that error rather than as a silently dropped tail. The two
//! derivations of a value's extent still agree where the document parses (the
//! controls below), which is what #434 protected.
//!
//! Inputs are byte literals so no editor or formatter can quietly add a
//! trailing newline that hides the EOF condition under test.
//!
//! Run with: `cargo test --test yaml_colon_eof_tests`

use succinctly::yaml::{locate_offset_detailed, YamlError, YamlIndex, YamlValue};

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

/// Renders a byte string with any non-ASCII-printable bytes visible in
/// assertion output.
struct Text<'a>(&'a [u8]);

impl core::fmt::Debug for Text<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", String::from_utf8_lossy(self.0))
    }
}

#[track_caller]
fn assert_json(yaml: &[u8], expected: &str, why: &str) {
    let actual =
        to_json(yaml).unwrap_or_else(|e| panic!("{:?} failed to parse: {e} — {why}", Text(yaml)));
    assert_eq!(actual, expected, "input {:?} — {why}", Text(yaml));
}

/// The eval half: a colon at absolute EOF ends the value, which #4463 turns into
/// yq's error instead of the dropped tail it used to be.
#[test]
fn a_colon_at_absolute_eof_ends_the_value() {
    let err = to_json(b"key: abc:").expect_err("yq rejects `key: abc:`");
    assert!(
        err.to_string()
            .contains("mapping values are not allowed in this context"),
        "{err}"
    );
    assert_json(
        b"key: abc",
        r#"{"key":"abc"}"#,
        "the control: no trailing colon at all",
    );
    assert_json(
        b"key: http://example.com",
        r#"{"key":"http://example.com"}"#,
        "a colon NOT at EOF, and not followed by white space, stays content",
    );
}

/// The `syq-locate`/`at_offset` half: for a document that parses, the byte range
/// of the value is what `syq` prints for the same node. (`key: abc:` no longer
/// builds an index at all, #4463.)
#[test]
fn the_located_byte_range_agrees_with_the_printed_value() {
    assert!(YamlIndex::build(b"key: abc:").is_err());
    let yaml: &[u8] = b"key: abc";
    let index = YamlIndex::build(yaml).expect("parses");
    let found = locate_offset_detailed(&index, yaml, 5)
        .unwrap_or_else(|| panic!("offset 5 of {:?} located nothing", Text(yaml)));
    let (start, end) = found.byte_range;
    assert_eq!(
        &yaml[start..end],
        b"abc",
        "offset 5 of {:?}: range {:?} covers {:?}",
        Text(yaml),
        found.byte_range,
        Text(&yaml[start..end])
    );
}
