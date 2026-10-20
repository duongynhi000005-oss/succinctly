//! #4099: an input with no bytes at all is one empty document in yq, not an error: `printf '' |
//! yq '.a'` is `null`, `.a = 1` is `a: 1`, `length` is `0`. `succinctly yq` stopped with
//! `YAML parse error: empty input` (exit 1). Each row is a live yq v4.53.3 capture over empty
//! stdin; an empty file, and `-` for stdin, answer the same.
//!
//! yq skips an empty file among several and reads the others; that is not done here, so a run
//! with more than one input is not covered.

use anyhow::Result;
use tempfile::NamedTempFile;

use crate::{run_yq_files, run_yq_stdin};

/// (filter, expected stdout).
const ROWS: &[(&str, &str)] = &[
    (".", "\n"),
    (".a", "null\n"),
    (".a = 1", "a: 1\n"),
    ("length", "0\n"),
    ("type", "!!null\n"),
    (". == null", "true\n"),
    ("document_index", "0\n"),
    ("tag", "!!null\n"),
    ("(.a, .b)", "null\nnull\n"),
];

#[test]
fn test_empty_stdin_is_one_empty_document_4099() -> Result<()> {
    for (filter, expected) in ROWS {
        let (stdout, code) = run_yq_stdin(filter, "", &[])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{filter}");
    }
    let (stdout, code) = run_yq_stdin(".a", "", &["-o", "json", "-I", "0"])?;
    assert_eq!((stdout.as_str(), code), ("null\n", 0));
    let (stdout, code) = run_yq_stdin(".a", "", &["-p", "json"])?;
    assert_eq!((stdout.as_str(), code), ("null\n", 0));
    Ok(())
}

#[test]
fn test_empty_file_is_one_empty_document_4099() -> Result<()> {
    let empty = NamedTempFile::new()?;
    for (filter, expected) in ROWS {
        let (stdout, stderr, code) = run_yq_files(filter, &[empty.path()], &[])?;
        assert_eq!(
            (stdout.as_str(), code),
            (*expected, 0),
            "{filter}: {stderr}"
        );
    }
    Ok(())
}
