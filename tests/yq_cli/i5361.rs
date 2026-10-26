//! #5361: the empty string repeated a huge number of times is the empty string in yq mode too: the
//! cap there compares the byte total, which is 0 for it, so the repeat loop ran once per
//! repetition. yq v4.53.3 answers an empty line at once, in either operand order; a negative
//! count is still refused first.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Result};

/// `succinctly yq -n <filter>` with a wall-clock limit, so a regression to the endless loop fails
/// this test instead of stalling the suite.
fn run_limited(filter: &str) -> Result<(String, String, i32)> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_succinctly"))
        .args(["yq", "-n", filter])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let output = child.wait_with_output()?;
            return Ok((
                String::from_utf8(output.stdout)?,
                String::from_utf8(output.stderr)?,
                status.code().unwrap_or(-1),
            ));
        }
        if start.elapsed() > Duration::from_secs(30) {
            child.kill()?;
            child.wait()?;
            bail!("{filter}: still running after 30 s");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn test_empty_string_repeated_a_huge_number_of_times_is_empty_5361() -> Result<()> {
    for filter in [
        r#""" * 9223372036854775807"#,
        r#"9223372036854775807 * """#,
        r#""" * 3"#,
        r#""" * 0"#,
        r#"5 * """#,
    ] {
        let (stdout, stderr, code) = run_limited(filter)?;
        assert_eq!((stdout.as_str(), code), ("\n", 0), "{filter}: {stderr}");
    }
    // The sign check still comes first.
    let (stdout, stderr, code) = run_limited(r#""" * -1"#)?;
    assert_eq!((stdout.as_str(), code), ("", 1), "{stderr}");
    assert!(
        stderr.contains("cannot repeat string by a negative number"),
        "{stderr}"
    );
    Ok(())
}
