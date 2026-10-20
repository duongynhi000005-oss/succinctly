//! #2839: a `-p json` stream whose later value is bad, and the modes that read the whole input.
//! Rows captured from yq v4.53.3 (`-p json -o=json -I=0`); the messages differ, so only the
//! values on stdout and the exit status are compared.

use anyhow::Result;
use std::io::Write;
use tempfile::NamedTempFile;

use crate::{run_yq_files, run_yq_stdin_with_stderr};

const JSON: &[&str] = &["-p=json", "-o=json", "-I=0"];

/// A stream is read value by value: yq prints the values before the bad one, then fails.
#[test]
fn test_stream_prints_values_before_a_bad_one_2839() -> Result<()> {
    // (document, filter, stdout) -- every row exits 1.
    let rows: &[(&str, &str, &str)] = &[
        ("1 2 ]", ".", "1\n2\n"),
        ("{\"a\":1} x", ".", "{\"a\":1}\n"),
        ("1 ] 3", ".", "1\n"),
        ("1 2 foo", ".", "1\n2\n"),
        // The single-value parse used to read this as a one-key mapping.
        ("1 {\"a\":", ".", "1\n"),
        ("{\"a\":1} {\"a\":", ".a", "1\n"),
        // A cut value that is not valid JSON ends the stream too.
        ("1 [1,,2]", ".", "1\n"),
        ("1 2 [1,,2] 4", ".", "1\n2\n"),
        // The filter runs on each good value.
        ("{\"a\":1}{\"a\":2} ]", ".a", "1\n2\n"),
    ];
    for &(doc, filter, expected) in rows {
        let (stdout, stderr, code) = run_yq_stdin_with_stderr(filter, doc, JSON)?;
        assert_eq!(
            (code, stdout.as_str()),
            (1, expected),
            "`{filter}` on {doc:?}: {stderr:?}"
        );
        assert!(stderr.contains("Error:"), "{doc:?}: {stderr:?}");
    }
    // Nothing is good: nothing is printed.
    for doc in ["]", "foo 1", "[1,,2]"] {
        let (stdout, _, code) = run_yq_stdin_with_stderr(".", doc, JSON)?;
        assert_eq!((code, stdout.as_str()), (1, ""), "{doc:?}");
    }
    // A filter error on a good value outranks the bad one behind it.
    let (stdout, stderr, code) = run_yq_stdin_with_stderr("error(\"boom\")", "{\"a\":1} x", JSON)?;
    assert_eq!((code, stdout.as_str()), (1, ""));
    assert!(!stderr.contains("unexpected content"), "{stderr:?}");
    Ok(())
}

/// yq stops reading at the bad value, so a later file is never read.
#[test]
fn test_stream_error_stops_later_files_2839() -> Result<()> {
    let write = |text: &[u8]| -> Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;
        file.write_all(text)?;
        Ok(file)
    };
    let (bad, good) = (write(b"1 2 ]")?, write(b"{\"a\":3}")?);
    let (stdout, _, code) = run_yq_files(".", &[bad.path(), good.path()], JSON)?;
    assert_eq!((code, stdout.as_str()), (1, "1\n2\n"));
    let (stdout, _, code) = run_yq_files(".", &[good.path(), bad.path()], JSON)?;
    assert_eq!((code, stdout.as_str()), (1, "{\"a\":3}\n1\n2\n"));
    Ok(())
}

/// `--eval-all` and `--slurp` read every value of a stream, and fail before printing when one
/// is bad; they used to read the first value and drop the rest without a word.
#[test]
fn test_whole_input_modes_read_the_stream_2839() -> Result<()> {
    let ok: &[(&str, &str, &[&str], &str)] = &[
        ("1 2", "[.]", &["--eval-all"], "[1,2]\n"),
        ("1 2", ".", &["--eval-all"], "1\n2\n"),
        (
            "{\"a\":1}{\"b\":2}",
            "[.]",
            &["--eval-all"],
            "[{\"a\":1},{\"b\":2}]\n",
        ),
        (
            "{\"a\":1}{\"b\":2}",
            "[.] | .[0] * .[1]",
            &["--eval-all"],
            "{\"a\":1,\"b\":2}\n",
        ),
        ("1 2", ".", &["--slurp"], "[1,2]\n"),
        ("[3]", "[.]", &["--eval-all"], "[[3]]\n"),
    ];
    for &(doc, filter, flags, expected) in ok {
        let args: Vec<&str> = JSON.iter().chain(flags).copied().collect();
        let (stdout, stderr, code) = run_yq_stdin_with_stderr(filter, doc, &args)?;
        assert_eq!(
            (code, stdout.as_str()),
            (0, expected),
            "`{filter}` {flags:?} on {doc:?}: {stderr:?}"
        );
    }
    // A bad value anywhere fails the run with nothing printed -- `{"a":1} x` used to answer `1`.
    for flag in ["--eval-all", "--slurp"] {
        for doc in ["1 2 ]", "{\"a\":1} x", "1 {\"a\":", "1 2 foo"] {
            let args: Vec<&str> = JSON.iter().chain(&[flag]).copied().collect();
            let (stdout, _, code) = run_yq_stdin_with_stderr("[.]", doc, &args)?;
            assert_eq!((code, stdout.as_str()), (1, ""), "{flag} on {doc:?}");
        }
    }
    Ok(())
}

/// `--inplace` rewrites every value of a stream, and leaves the file alone when one is bad.
#[test]
fn test_inplace_reads_the_stream_2839() -> Result<()> {
    let run = |text: &[u8]| -> Result<(String, i32)> {
        let mut file = NamedTempFile::new()?;
        file.write_all(text)?;
        let (_, _, code) = run_yq_files(
            ".a += 1",
            &[file.path()],
            &["-p=json", "-o=json", "-I=0", "--inplace"],
        )?;
        Ok((std::fs::read_to_string(file.path())?, code))
    };
    let (after, code) = run(b"{\"a\":1}{\"a\":2}")?;
    assert_eq!((after.as_str(), code), ("{\"a\":2}\n{\"a\":3}\n", 0));
    let (after, code) = run(b"{\"a\":1} x")?;
    assert_eq!((after.as_str(), code), ("{\"a\":1} x", 1));
    Ok(())
}
