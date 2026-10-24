//! #5223 Phase 0: `succinctly dev jq-path-spike`, the dev-only entry to the register-carrying
//! path-mode spike (`src/jq/path_spike.rs`). The spike's own semantics are pinned by its unit
//! tests; these pin the entry's contract with the three-way harness
//! (`scripts/jq-path-spike-3way.py`): jq's input handling and exit codes, and the
//! `path-spike: unsupported`/`path-spike: stubbed` lines it classifies rows by. Expected
//! stdout and exit codes are jq 1.7.1's for the same argv.
//!
//! The interpreter reads storage identity, which the `unshared-containers` holdout does not
//! have, so these pin the shipped shape only.
#![cfg(not(feature = "unshared-containers"))]

use std::process::Command;

use anyhow::Result;

use crate::{spawn_with_signal_retry, succinctly_bin};

/// Run the dev entry with `args` (options, then the filter), returning stdout, stderr and the
/// exit code.
fn spike(args: &[&str], stdin: Option<&str>) -> Result<(String, String, i32)> {
    let (output, code) = spawn_with_signal_retry(
        || {
            let mut command = Command::new(succinctly_bin());
            command.args(["dev", "jq-path-spike"]).args(args);
            command
        },
        stdin.map(str::as_bytes),
    )?;
    Ok((
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?,
        code,
    ))
}

/// Every input value runs in turn, the exit status is the last input's, and a malformed value
/// ends the read with jq's status 5 after the values before it.
#[test]
fn the_spike_entry_reads_inputs_as_jq_5223() -> Result<()> {
    for (input, filter, stdout, code) in [
        ("1 2", ". + 1", "2\n3\n", 0),
        (r#""x" 3"#, ". + 1", "4\n", 0),
        (r#"3 "x""#, ". + 1", "4\n", 5),
        ("{", ".", "", 5),
        ("1 [1,", ".", "1\n", 5),
        (" \n", ".", "", 0),
        (r#"{"a":[1,2]}"#, "path(.a[])", "[\"a\",0]\n[\"a\",1]\n", 0),
    ] {
        let (out, _, rc) = spike(&["-c", "--", filter], Some(input))?;
        assert_eq!((out.as_str(), rc), (stdout, code), "{input} | {filter}");
    }
    Ok(())
}

/// `-r`, `-n`, a file argument, an error payload that is not a string, and `halt`.
#[test]
fn the_spike_entry_takes_jqs_options_and_reports_its_errors_5223() -> Result<()> {
    let (out, _, rc) = spike(&["-r", ".a"], Some(r#"{"a":"x"}"#))?;
    assert_eq!((out.as_str(), rc), ("x\n", 0));
    let (out, _, rc) = spike(&["-n", "-c", "[., 1]"], None)?;
    assert_eq!((out.as_str(), rc), ("[null,1]\n", 0));
    let (out, err, rc) = spike(&["-n", "error({})"], None)?;
    assert_eq!((out.as_str(), rc), ("", 5));
    assert!(err.contains("(not a string): {}"), "{err}");
    let (out, err, rc) = spike(&["-n", r#"error("boom")"#], None)?;
    assert_eq!((out.as_str(), rc), ("", 5));
    assert!(err.contains("jq: error (at <unknown>): boom"), "{err}");
    let (out, _, rc) = spike(&["-n", "1, halt, 2"], None)?;
    assert_eq!((out.as_str(), rc), ("1\n", 0));

    let mut file = tempfile::NamedTempFile::new()?;
    std::io::Write::write_all(&mut file, br#"{"a":1}"#)?;
    let path = file.path().to_str().expect("utf-8 temp path");
    let (out, _, rc) = spike(&["-c", "path(.a)", path], None)?;
    assert_eq!((out.as_str(), rc), ("[\"a\"]\n", 0));
    let (_, _, rc) = spike(&[".", "/nonexistent/5223.json"], None)?;
    assert_eq!(rc, 2);
    Ok(())
}

/// The lines the three-way harness classifies by: a declined construct exits 64 with
/// `path-spike: unsupported`, a compile error exits 3 as jq does, and a run that answered a
/// builtin through the fresh stub names it.
#[test]
fn the_spike_entry_marks_declined_and_stubbed_runs_5223() -> Result<()> {
    let (_, err, rc) = spike(&["-n", "input"], None)?;
    assert_eq!(rc, 64);
    assert!(err.contains("path-spike: unsupported: "), "{err}");
    let (_, _, rc) = spike(&["-n", "$a"], None)?;
    assert_eq!(rc, 3);
    let (out, err, rc) = spike(&["-c", r#"[scan("x")]"#], Some(r#""x""#))?;
    assert_eq!((out.as_str(), rc), ("[\"x\"]\n", 0));
    assert!(err.contains("path-spike: stubbed: Scan"), "{err}");
    // A C builtin that hands back its input keeps the path, as in jq 1.7.1.
    let (out, err, rc) = spike(&["-c", r#"path(.a | ltrimstr("z"))"#], Some(r#"{"a":"x"}"#))?;
    assert_eq!((out.as_str(), rc), ("[\"a\"]\n", 0));
    assert!(!err.contains("stubbed"), "{err}");
    Ok(())
}
