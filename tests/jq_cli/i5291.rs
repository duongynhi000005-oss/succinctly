//! #5291: jq skips a UTF-8 byte-order mark at the start of its input; `succinctly jq` answered
//! `Invalid JSON text`. Expected output captured live against jq 1.7.1 (`/usr/bin/jq`).

use anyhow::Result;

use crate::run_jq_full;

const BOM: &str = "\u{feff}";

#[test]
fn test_bom_prefixed_stdin_parses_as_jq_does_5291() -> Result<()> {
    let doc = format!("{BOM}{{\"a\":1}}");
    for (args, want) in [
        (vec!["-c", "."], "{\"a\":1}\n"),
        (vec!["-c", "-s", "."], "[{\"a\":1}]\n"),
        (vec!["-c", "--stream", "."], "[[\"a\"],1]\n[[\"a\"]]\n"),
        (vec!["-c", "-n", "[inputs]"], "[{\"a\":1}]\n"),
        (vec!["-c", "-n", "input"], "{\"a\":1}\n"),
        // `--validate` is succinctly's own strict mode; the BOM is no more a fault there.
        (vec!["-c", "--validate", "."], "{\"a\":1}\n"),
    ] {
        let (out, err, code) = run_jq_full(&args, Some(&doc))?;
        assert_eq!((out.as_str(), code), (want, 0), "{args:?}: {err}");
    }
    Ok(())
}

#[test]
fn test_bom_only_input_is_empty_5291() -> Result<()> {
    let (out, _, code) = run_jq_full(&["-c", "."], Some(BOM))?;
    assert_eq!((out.as_str(), code), ("", 0));
    let (out, _, code) = run_jq_full(&["-c", "-s", "."], Some(BOM))?;
    assert_eq!((out.as_str(), code), ("[]\n", 0));
    Ok(())
}

#[test]
fn test_raw_input_keeps_the_bom_5291() -> Result<()> {
    // `-R` never parses, so the BOM is an ordinary character of the line (jq 1.7.1 agrees).
    let (out, _, code) = run_jq_full(&["-R", "-c", "length"], Some(&format!("{BOM}ab\n")))?;
    assert_eq!((out.as_str(), code), ("3\n", 0));
    Ok(())
}

#[test]
fn test_bom_prefixed_files_5291() -> Result<()> {
    let dir = tempfile::TempDir::new()?;
    let path = |name: &str| dir.path().join(name);
    let arg = |name: &str| path(name).to_string_lossy().into_owned();
    std::fs::write(path("bom.json"), format!("{BOM}{{\"a\":1}}"))?;
    std::fs::write(path("empty.json"), "")?;
    std::fs::write(path("plain.json"), "{\"b\":3}")?;
    std::fs::write(path("bom2.json"), format!("{BOM}[2]"))?;

    for (files, extra, want) in [
        (vec!["bom.json"], vec![], "{\"a\":1}\n"),
        // The stream's front is the first file that has bytes.
        (vec!["empty.json", "bom.json"], vec![], "{\"a\":1}\n"),
        (
            vec!["bom.json", "plain.json"],
            vec![],
            "{\"a\":1}\n{\"b\":3}\n",
        ),
        (
            vec!["bom.json", "plain.json"],
            vec!["-s"],
            "[{\"a\":1},{\"b\":3}]\n",
        ),
        (vec!["bom.json"], vec!["--validate"], "{\"a\":1}\n"),
        (
            vec!["bom.json"],
            vec!["--stream"],
            "[[\"a\"],1]\n[[\"a\"]]\n",
        ),
    ] {
        let mut args: Vec<String> = vec!["-c".into()];
        args.extend(extra.iter().map(|&a| String::from(a)));
        args.push(".".into());
        args.extend(files.iter().map(|f| arg(f)));
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let (out, err, code) = run_jq_full(&refs, None)?;
        assert_eq!(
            (out.as_str(), code),
            (want, 0),
            "{files:?} {extra:?}: {err}"
        );
    }

    // Only the front of the stream: a BOM at the front of a *later* file is a byte inside it,
    // and jq refuses it (exit 5) after printing the first file's value.
    let (out, _, code) = run_jq_full(&["-c", ".", &arg("plain.json"), &arg("bom2.json")], None)?;
    assert_eq!((out.as_str(), code), ("{\"b\":3}\n", 5));
    Ok(())
}
