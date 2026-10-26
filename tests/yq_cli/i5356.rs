//! #5356: a UTF-8 byte-order mark at the front of a YAML stream is not data. yq reads past it, per
//! file, so `.a` finds the first key; succinctly kept it in the key, so `.a` was `null`, `--- a`
//! after a mark was a parse error and `-i` wrote the key back quoted as `"﻿a"`. Every row is
//! a live yq v4.53.3 capture. A mark that is not at the front of the stream (`mid`) stays data, and
//! a JSON document with one is refused by yq (`invalid character 'ï'`) as it is here.

use std::io::Write;

use anyhow::Result;
use tempfile::NamedTempFile;

use crate::{run_yq_files, run_yq_stdin};

const JSON: &[&str] = &["-o=json", "-I=0"];

fn args<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    JSON.iter().chain(extra).copied().collect()
}

#[test]
fn test_bom_at_the_front_is_skipped_on_stdin_5356() -> Result<()> {
    for (filter, input, want) in [
        ("keys", "\u{feff}a: 1\nb: 2\n", "[\"a\",\"b\"]\n"),
        (".a", "\u{feff}a: 1\nb: 2\n", "1\n"),
        (".", "\u{feff}a: 1\nb: 2\n", "{\"a\":1,\"b\":2}\n"),
        // After the mark: a comment first, or an explicit document start with a second document.
        (".a", "\u{feff}# c\na: 1\n", "1\n"),
        (
            ".",
            "\u{feff}---\na: 1\n---\nb: 2\n",
            "{\"a\":1}\n{\"b\":2}\n",
        ),
        // A mark that is not the stream's first bytes is part of the key it sits in.
        ("keys", "x: 1\n\u{feff}y: 2\n", "[\"x\",\"\u{feff}y\"]\n"),
    ] {
        let (stdout, code) = run_yq_stdin(filter, input, JSON)?;
        assert_eq!((stdout.as_str(), code), (want, 0), "{filter} on {input:?}");
    }
    // A file that is only the mark is an empty file: `null` as JSON, a blank line as YAML.
    let (stdout, code) = run_yq_stdin(".", "\u{feff}", JSON)?;
    assert_eq!((stdout.as_str(), code), ("null\n", 0));
    let (stdout, code) = run_yq_stdin(".", "\u{feff}", &[])?;
    assert_eq!((stdout.as_str(), code), ("\n", 0));
    // The default YAML output too.
    let (stdout, code) = run_yq_stdin(".", "\u{feff}# c\na: 1\n", &[])?;
    assert_eq!((stdout.as_str(), code), ("# c\na: 1\n", 0));
    Ok(())
}

#[test]
fn test_each_file_skips_its_own_bom_5356() -> Result<()> {
    let mut plain = NamedTempFile::new()?;
    write!(plain, "a: 1\nb: 2\n")?;
    let mut bom = NamedTempFile::new()?;
    write!(bom, "\u{feff}a: 1\nb: 2\n")?;
    let mut bom_docs = NamedTempFile::new()?;
    write!(bom_docs, "\u{feff}---\na: 1\n---\nb: 2\n")?;

    // The second file's mark is skipped as the first's is.
    let (stdout, stderr, code) = run_yq_files("keys", &[plain.path(), bom.path()], JSON)?;
    assert_eq!(
        (stdout.as_str(), code),
        ("[\"a\",\"b\"]\n[\"a\",\"b\"]\n", 0),
        "{stderr}"
    );
    let (stdout, stderr, code) =
        run_yq_files("keys", &[bom.path(), plain.path()], &args(&["--eval-all"]))?;
    assert_eq!(
        (stdout.as_str(), code),
        ("[\"a\",\"b\"]\n[\"a\",\"b\"]\n", 0),
        "{stderr}"
    );
    let (stdout, stderr, code) =
        run_yq_files("[.a]", &[bom.path(), plain.path()], &args(&["--eval-all"]))?;
    assert_eq!((stdout.as_str(), code), ("[1,1]\n", 0), "{stderr}");
    // A mark before an explicit `---`: both documents are read.
    let (stdout, stderr, code) = run_yq_files(".", &[bom_docs.path()], JSON)?;
    assert_eq!(
        (stdout.as_str(), code),
        ("{\"a\":1}\n{\"b\":2}\n", 0),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn test_inplace_rewrites_without_the_bom_5356() -> Result<()> {
    let mut file = NamedTempFile::new()?;
    write!(file, "\u{feff}a: 1\nb: 2\n")?;
    let (_, stderr, code) = run_yq_files(".c = 3", &[file.path()], &["-i"])?;
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(std::fs::read(file.path())?, b"a: 1\nb: 2\nc: 3\n");
    Ok(())
}

#[test]
fn test_json_input_keeps_refusing_a_bom_5356() -> Result<()> {
    let (_, code) = run_yq_stdin(".", "\u{feff}{\"a\":1}", &["-p=json", "-o=json"])?;
    assert_ne!(code, 0);
    Ok(())
}
