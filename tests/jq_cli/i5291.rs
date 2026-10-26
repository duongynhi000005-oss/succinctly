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
fn test_a_second_bom_is_refused_5291() -> Result<()> {
    // Only one is the parser's to skip; the next is a byte no value starts with (jq: exit 5).
    let (out, _, code) = run_jq_full(&["-c", "."], Some(&format!("{BOM}{BOM}1")))?;
    assert_eq!((out.as_str(), code), ("", 5));
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

/// jq loads `--slurpfile`, `--argjson`, `--jsonargs` and a data import through the same parser,
/// so a BOM there is skipped too -- but only at the very front of the text (a leading space
/// leaves it standing), and a BOM alone is not a JSON text for `--argjson`.
#[test]
fn test_bom_in_argument_json_and_data_files_5291() -> Result<()> {
    let dir = tempfile::TempDir::new()?;
    let arg = |name: &str| dir.path().join(name).to_string_lossy().into_owned();
    std::fs::write(arg("bom.json"), format!("{BOM}{{\"a\":1}}"))?;
    std::fs::write(arg("bom-only.json"), BOM)?;
    std::fs::write(arg("ws-bom.json"), format!(" {BOM}[1]"))?;
    std::fs::write(arg("twice.json"), format!("{BOM}{BOM}1"))?;
    std::fs::write(arg("data.json"), format!("{BOM}{{\"x\":1}}"))?;

    for (args, want, want_code) in [
        (
            vec!["-nc", "--slurpfile", "x", &arg("bom.json"), "$x"],
            "[{\"a\":1}]\n",
            0,
        ),
        (
            vec!["-nc", "--slurpfile", "x", &arg("bom-only.json"), "$x"],
            "[]\n",
            0,
        ),
        (
            vec!["-nc", "--argjson", "x", &format!("{BOM}[1]"), "$x"],
            "[1]\n",
            0,
        ),
        (
            vec![
                "-nc",
                "--jsonargs",
                "$ARGS.positional",
                &format!("{BOM}[1]"),
            ],
            "[[1]]\n",
            0,
        ),
        (
            vec![
                "-nc",
                "-L",
                &dir.path().to_string_lossy(),
                "import \"data\" as $d; $d",
            ],
            "[{\"x\":1}]\n",
            0,
        ),
        // Refused, exit 2 as in jq: a BOM alone, a doubled BOM, a BOM behind whitespace.
        (vec!["-nc", "--argjson", "x", BOM, "$x"], "", 2),
        (
            vec!["-nc", "--argjson", "x", &format!("{BOM}{BOM}[1]"), "$x"],
            "",
            2,
        ),
        (
            vec!["-nc", "--argjson", "x", &format!(" {BOM}[1]"), "$x"],
            "",
            2,
        ),
        (
            vec!["-nc", "--slurpfile", "x", &arg("ws-bom.json"), "$x"],
            "",
            2,
        ),
        (
            vec!["-nc", "--slurpfile", "x", &arg("twice.json"), "$x"],
            "",
            2,
        ),
    ] {
        let (out, err, code) = run_jq_full(&args, None)?;
        assert_eq!((out.as_str(), code), (want, want_code), "{args:?}: {err}");
    }
    Ok(())
}
