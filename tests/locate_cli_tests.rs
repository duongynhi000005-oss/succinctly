//! End-to-end CLI coverage for `jq-locate` / `yq-locate` `--line/--column`.
//!
//! The line/column -> byte offset path had no integration coverage at all
//! before #228 — only help-text snapshots mentioned these commands — so the
//! `LineIndex` migration would have been invisible to the test suite at the
//! CLI boundary. These spawn the built binary directly (the
//! `corpus_stats_cli_tests.rs` pattern) rather than going through `cargo run`.
//!
//! The working directory for integration tests is the crate root, so the seed
//! corpus paths resolve as-is.
#![cfg(feature = "cli")]

use std::process::Command;

use anyhow::Result;

#[path = "common/cargo_run_exit.rs"]
mod cargo_run_exit;
use cargo_run_exit::exit_code_or_signal_death;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_succinctly")
}

fn run(args: &[&str]) -> Result<(String, String, i32)> {
    let out = Command::new(bin())
        .args(args)
        .output()
        .expect("spawn succinctly");
    let code = exit_code_or_signal_death(out.status, &out.stderr)?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Ok((stdout, stderr, code))
}

const JSON_FIXTURE: &str = "tests/data/bench-corpus/seed/json/charts/bullet-data.json";
const YAML_FIXTURE: &str = "tests/data/bench-corpus/seed/yaml/k8s/nginx-deployment.yml";

#[test]
fn jq_locate_resolves_line_and_column() -> Result<()> {
    let (stdout, stderr, code) = run(&["jq-locate", JSON_FIXTURE, "--line", "3", "--column", "5"])?;

    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, ".[1].title");
    Ok(())
}

#[test]
fn jq_locate_line_column_agrees_with_offset() -> Result<()> {
    // Line 3 column 5 of the fixture is byte 122. The two paths share nothing:
    // --offset skips the line index entirely, so agreement is a real check
    // that the line index maps positions the way the file is actually laid out.
    let (by_position, _, _) = run(&["jq-locate", JSON_FIXTURE, "--line", "3", "--column", "5"])?;
    let (by_offset, _, _) = run(&["jq-locate", JSON_FIXTURE, "--offset", "122"])?;

    assert_eq!(by_position, by_offset);
    assert!(!by_position.is_empty(), "{by_position:?}");
    Ok(())
}

#[test]
fn jq_locate_rejects_out_of_range_position() -> Result<()> {
    let (_, stderr, code) = run(&["jq-locate", JSON_FIXTURE, "--line", "9999", "--column", "1"])?;

    assert_ne!(code, 0, "out-of-range position must fail");
    assert!(
        stderr.contains("Invalid position"),
        "expected an invalid-position error, got: {stderr}"
    );
    Ok(())
}

#[test]
fn jq_locate_rejects_zero_column() -> Result<()> {
    let (_, stderr, code) = run(&["jq-locate", JSON_FIXTURE, "--line", "1", "--column", "0"])?;

    assert_ne!(code, 0, "column 0 must fail");
    assert!(
        stderr.contains("Invalid position"),
        "expected an invalid-position error, got: {stderr}"
    );
    Ok(())
}

#[test]
fn yq_locate_resolves_line_and_column() -> Result<()> {
    // Line 5 of the manifest is "  labels:", so column 3 is the key itself.
    let (stdout, stderr, code) = run(&["yq-locate", YAML_FIXTURE, "--line", "5", "--column", "3"])?;

    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, ".[0].metadata.labels");
    Ok(())
}

#[test]
fn yq_locate_rejects_out_of_range_position() -> Result<()> {
    let (_, stderr, code) = run(&["yq-locate", YAML_FIXTURE, "--line", "9999", "--column", "1"])?;

    assert_ne!(code, 0, "out-of-range position must fail");
    assert!(
        stderr.contains("Invalid position"),
        "expected an invalid-position error, got: {stderr}"
    );
    Ok(())
}

/// A leading-dot number's own `byte_range` must span just the literal
/// itself, not run off to the next structural byte -- `text_range()`
/// (`src/json/light.rs`) previously had no leading-dot case at all
/// (unlike `value()`, which classified the cursor as `Number` correctly),
/// so this returned `None` and `locate_offset_detailed`'s own fallback
/// substituted a byte range spanning to end-of-file. Found by code
/// review of #1171 before merge.
#[test]
fn jq_locate_leading_dot_number_byte_range_is_exact() -> Result<()> {
    let mut fixture = tempfile::NamedTempFile::new().expect("create temp fixture");
    std::io::Write::write_all(&mut fixture, b"[.5, 6, 7]").expect("write fixture");
    let path = fixture.path().to_str().expect("utf8 path");

    let (stdout, stderr, code) = run(&["jq-locate", path, "--offset", "1", "--format", "json"])?;

    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("\"byte_range\": [\n    1,\n    3\n  ]"),
        "expected byte_range [1, 3], got: {stdout}"
    );
    Ok(())
}

/// #1627: `yq`/`yq --validate`/`at_offset` all reject an encoding-invalid
/// document at the input boundary (#1391/#1242), but `yq-locate` -- a
/// diagnostic tool for "what's at byte N", most useful on exactly the file a
/// parser just rejected -- deliberately keeps answering instead, warning to
/// stderr rather than silently disagreeing with the rest of the yq surface.
#[test]
fn yq_locate_warns_but_still_answers_on_invalid_utf8() -> Result<()> {
    let mut fixture = tempfile::NamedTempFile::new().expect("create temp fixture");
    std::io::Write::write_all(&mut fixture, b"a: 1\nb: \"x\xe4y\"\n").expect("write fixture");
    let path = fixture.path().to_str().expect("utf8 path");

    let (stdout, stderr, code) = run(&["yq-locate", path, "--offset", "9"])?;

    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout, ".[0].b");
    assert!(
        stderr.contains("not valid UTF-8"),
        "expected a UTF-8 warning on stderr, got: {stderr}"
    );
    Ok(())
}

/// #1759 (found reviewing #1627/PR #1758): the warn-and-keep-answering fix
/// first covered only an invalid byte inside a *value*. One inside a *mapping
/// key* made `path_to_bp` give up, so the key itself, every value under it
/// and every descendant failed with a generic "Could not locate" error. The
/// key is now read lossily (U+FFFD), which forces the bracket form.
#[test]
fn yq_locate_answers_when_invalid_utf8_is_in_a_key() -> Result<()> {
    let mut fixture = tempfile::NamedTempFile::new().expect("create temp fixture");
    std::io::Write::write_all(&mut fixture, b"a: 1\n\"b\xe4c\": 2\n").expect("write fixture");
    let path = fixture.path().to_str().expect("utf8 path");

    // The key (7), its value (12), and the same key by line/column.
    for args in [
        &["--offset", "7"][..],
        &["--offset", "12"][..],
        &["--line", "2", "--column", "4"][..],
    ] {
        let mut argv = vec!["yq-locate", path];
        argv.extend_from_slice(args);
        let (stdout, stderr, code) = run(&argv)?;
        assert_eq!(code, 0, "{args:?}: {stderr}");
        assert_eq!(stdout, ".[0][\"b\u{fffd}c\"]", "{args:?}");
        assert!(stderr.contains("not valid UTF-8"), "{args:?}: {stderr}");
    }
    Ok(())
}

/// #1759: the bad key is an *ancestor* of the requested offset, and is
/// single-quoted (its own quote pair is not part of the key text).
#[test]
fn yq_locate_answers_below_a_key_with_invalid_utf8() -> Result<()> {
    for (doc, offset, expected) in [
        (
            &b"a: 1\n\"b\xe4c\":\n  d: 5\n"[..],
            "17",
            ".[0][\"b\u{fffd}c\"].d",
        ),
        (
            &b"a: 1\n'b\xe4c':\n  d: 5\n"[..],
            "17",
            ".[0][\"b\u{fffd}c\"].d",
        ),
        (
            &b"a: 1\nb\xe4c:\n  d: 5\n"[..],
            "15",
            ".[0][\"b\u{fffd}c\"].d",
        ),
    ] {
        let mut fixture = tempfile::NamedTempFile::new().expect("create temp fixture");
        std::io::Write::write_all(&mut fixture, doc).expect("write fixture");
        let path = fixture.path().to_str().expect("utf8 path");
        let (stdout, stderr, code) = run(&["yq-locate", path, "--offset", offset])?;
        assert_eq!(code, 0, "{doc:?}: {stderr}");
        assert_eq!(stdout, expected, "{doc:?}");
    }
    Ok(())
}

/// #1759: `jq-locate` had the identical hole in `src/json/locate.rs`.
#[test]
fn jq_locate_answers_when_invalid_utf8_is_in_a_key() -> Result<()> {
    let mut fixture = tempfile::NamedTempFile::new().expect("create temp fixture");
    std::io::Write::write_all(&mut fixture, b"{\"a\":1,\"b\xe4c\":{\"d\":5}}")
        .expect("write fixture");
    let path = fixture.path().to_str().expect("utf8 path");

    for (offset, expected) in [("9", ".[\"b\u{fffd}c\"]"), ("16", ".[\"b\u{fffd}c\"].d")] {
        let (stdout, stderr, code) = run(&["jq-locate", path, "--offset", offset])?;
        assert_eq!(code, 0, "offset {offset}: {stderr}");
        assert_eq!(stdout, expected, "offset {offset}");
    }
    Ok(())
}

/// #5313: a leading UTF-8 byte-order mark is not part of the document. `succinctly jq` drops it
/// before indexing (#5291), so `at_offset(n)` counts from the first byte after it; `jq-locate`
/// does the same, so a BOM-prefixed file answers exactly as the same file without the BOM, in
/// both tools.
#[test]
fn jq_locate_and_at_offset_agree_on_a_bom_prefixed_file() -> Result<()> {
    let doc = br#"{"name": "Alice", "age": 30}"#;
    let mut with_bom = b"\xef\xbb\xbf".to_vec();
    with_bom.extend_from_slice(doc);
    let mut bom_file = tempfile::NamedTempFile::new().expect("create temp fixture");
    std::io::Write::write_all(&mut bom_file, &with_bom).expect("write fixture");
    let mut plain_file = tempfile::NamedTempFile::new().expect("create temp fixture");
    std::io::Write::write_all(&mut plain_file, doc).expect("write fixture");
    let bom = bom_file.path().to_str().expect("utf8 path");
    let plain = plain_file.path().to_str().expect("utf8 path");

    for offset in ["0", "1", "9", "12", "27"] {
        let (located, stderr, code) = run(&["jq-locate", bom, "--offset", offset])?;
        assert_eq!(code, 0, "offset {offset}: {stderr}");
        let (plain_located, _, _) = run(&["jq-locate", plain, "--offset", offset])?;
        assert_eq!(located, plain_located, "offset {offset}");

        // `at_offset` lands on the node it lands on in the same file without the mark.
        let (by_offset, stderr, code) = run(&["jq", "-c", &format!("at_offset({offset})"), bom])?;
        assert_eq!(code, 0, "at_offset({offset}): {stderr}");
        let (plain_by_offset, _, _) = run(&["jq", "-c", &format!("at_offset({offset})"), plain])?;
        assert_eq!(by_offset, plain_by_offset, "offset {offset}");
    }

    // Line 1 starts after the BOM too: column 1 is the `{`, column 10 is inside `"Alice"`.
    let (root, _, _) = run(&["jq-locate", bom, "--line", "1", "--column", "1"])?;
    assert_eq!(root, ".");
    let (alice, _, _) = run(&["jq-locate", bom, "--line", "1", "--column", "10"])?;
    assert_eq!(alice, ".name");

    // The byte range counts from the first byte after the mark, and `bom_length` says how many
    // bytes that is; a file without a mark has no such field.
    let (json, _, _) = run(&["jq-locate", bom, "--offset", "12", "--format", "json"])?;
    let json: serde_json::Value = serde_json::from_str(&json)?;
    assert_eq!(json["byte_range"], serde_json::json!([9, 16]));
    assert_eq!(json["bom_length"], serde_json::json!(3));
    let (json, _, _) = run(&["jq-locate", plain, "--offset", "9", "--format", "json"])?;
    let json: serde_json::Value = serde_json::from_str(&json)?;
    assert_eq!(json["byte_range"], serde_json::json!([9, 16]));
    assert!(json.get("bom_length").is_none(), "{json}");

    // Later lines are not shifted, and a file that is only the mark has no document to locate.
    let mut multi = tempfile::NamedTempFile::new().expect("create temp fixture");
    std::io::Write::write_all(&mut multi, b"\xef\xbb\xbf{\n  \"k\": [10, 20]\n}\n")
        .expect("write fixture");
    let multi = multi.path().to_str().expect("utf8 path");
    let (second, _, _) = run(&["jq-locate", multi, "--line", "2", "--column", "3"])?;
    assert_eq!(second, ".k");
    let (element, _, _) = run(&["jq-locate", multi, "--line", "2", "--column", "10"])?;
    assert_eq!(element, ".k[0]");
    let mut only_bom = tempfile::NamedTempFile::new().expect("create temp fixture");
    std::io::Write::write_all(&mut only_bom, b"\xef\xbb\xbf").expect("write fixture");
    let only_bom = only_bom.path().to_str().expect("utf8 path");
    let (_, stderr, code) = run(&["jq-locate", only_bom, "--offset", "0"])?;
    assert_ne!(code, 0);
    assert!(
        stderr.contains("file size: 3 bytes; offsets count from after the 3-byte byte-order mark, so the document is 0 bytes"),
        "{stderr}"
    );

    // The first byte past the document is out of bounds, and the message says the mark was skipped.
    let (_, stderr, code) = run(&["jq-locate", bom, "--offset", "28"])?;
    assert_ne!(code, 0);
    assert!(
        stderr.contains("file size: 31 bytes; offsets count from after the 3-byte byte-order mark, so the document is 28 bytes"),
        "{stderr}"
    );
    Ok(())
}

/// #5356: `yq-locate` skips a leading UTF-8 byte-order mark as `jq-locate` does (#5313), because
/// `succinctly yq` drops it before indexing: a BOM-prefixed file answers as the same file without
/// it, and the `json` format says how many bytes were skipped.
#[test]
fn yq_locate_and_at_offset_agree_on_a_bom_prefixed_file() -> Result<()> {
    let doc = b"a: 1\nb:\n  c: 2\n";
    let mut bom_file = tempfile::NamedTempFile::new().expect("create temp fixture");
    std::io::Write::write_all(&mut bom_file, b"\xef\xbb\xbf").expect("write fixture");
    std::io::Write::write_all(&mut bom_file, doc).expect("write fixture");
    let mut plain_file = tempfile::NamedTempFile::new().expect("create temp fixture");
    std::io::Write::write_all(&mut plain_file, doc).expect("write fixture");
    let bom = bom_file.path().to_str().expect("utf8 path");
    let plain = plain_file.path().to_str().expect("utf8 path");

    for offset in ["0", "3", "5", "11", "14"] {
        let (located, stderr, code) = run(&["yq-locate", bom, "--offset", offset])?;
        assert_eq!(code, 0, "offset {offset}: {stderr}");
        let (plain_located, _, _) = run(&["yq-locate", plain, "--offset", offset])?;
        assert_eq!(located, plain_located, "offset {offset}");
        let (by_offset, stderr, code) = run(&[
            "yq",
            "-o=json",
            "-I=0",
            &format!("at_offset({offset})"),
            bom,
        ])?;
        assert_eq!(code, 0, "at_offset({offset}): {stderr}");
        let (plain_by_offset, _, _) = run(&[
            "yq",
            "-o=json",
            "-I=0",
            &format!("at_offset({offset})"),
            plain,
        ])?;
        assert_eq!(by_offset, plain_by_offset, "offset {offset}");
    }
    let (root, _, _) = run(&["yq-locate", bom, "--line", "1", "--column", "1"])?;
    let (plain_root, _, _) = run(&["yq-locate", plain, "--line", "1", "--column", "1"])?;
    assert_eq!(root, plain_root);

    let (json, _, _) = run(&["yq-locate", bom, "--offset", "14", "--format", "json"])?;
    let json: serde_json::Value = serde_json::from_str(&json)?;
    assert_eq!(json["bom_length"], serde_json::json!(3));
    let (json, _, _) = run(&["yq-locate", plain, "--offset", "14", "--format", "json"])?;
    let json: serde_json::Value = serde_json::from_str(&json)?;
    assert!(json.get("bom_length").is_none(), "{json}");

    let (_, stderr, code) = run(&["yq-locate", bom, "--offset", "15"])?;
    assert_ne!(code, 0);
    assert!(
        stderr.contains("file size: 18 bytes; offsets count from after the 3-byte byte-order mark, so the document is 15 bytes"),
        "{stderr}"
    );
    Ok(())
}
