//! #5361: the empty string repeated a huge number of times is the empty string. The repeat loop
//! pushed it once per repetition, which never finishes for a count near `i64::MAX` (jq caps the
//! count at `INT_MAX` and answers `""` after looping that many times). A non-empty string is
//! still refused, before any loop, as it was (#1612).

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Result};

use crate::succinctly_bin;

/// `succinctly jq -nc <program>` with a wall-clock limit, so a regression to the endless loop
/// fails this test instead of stalling the suite.
fn run_limited(program: &str) -> Result<(String, i32)> {
    let mut child = Command::new(succinctly_bin())
        .args(["jq", "-nc", program])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let output = child.wait_with_output()?;
            return Ok((
                String::from_utf8(output.stdout)?,
                status.code().unwrap_or(-1),
            ));
        }
        if start.elapsed() > Duration::from_secs(30) {
            child.kill()?;
            child.wait()?;
            bail!("{program}: still running after 30 s");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn test_empty_string_repeated_a_huge_number_of_times_is_empty_5361() -> Result<()> {
    for program in [
        r#""" * 100000000000000000000"#,
        r#"100000000000000000000 * """#,
        r#""" * 1e1000"#,
        r#"1e1000 * """#,
        r#""" * 9223372036854775807"#,
        r#""" * infinite"#,
        r#""" * 3"#,
    ] {
        let (stdout, code) = run_limited(program)?;
        assert_eq!((stdout.as_str(), code), ("\"\"\n", 0), "{program}");
    }
    // The sign check still comes first.
    let (stdout, code) = run_limited(r#""" * -1"#)?;
    assert_eq!((stdout.as_str(), code), ("null\n", 0));
    // A non-empty string is refused up front instead of looping.
    let (stdout, code) = run_limited(r#""a" * 1e30"#)?;
    assert_eq!((stdout.as_str(), code), ("", 5));
    Ok(())
}
