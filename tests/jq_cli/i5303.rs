//! #5303: `strptime` reads `%s` (seconds since the epoch) and expands it through the local zone, as
//! `localtime_r` does, so `"1425599621" | strptime("%s") | mktime` under `TZ=UTC` is the input
//! again. White space and one sign may precede the digits (`"  12"`, `"+5"` and `"-1"` read; `"- 5"`,
//! `"+-5"` and a bare sign do not), and it replaces the date and time read before it. Weekday and
//! yearday come from the date. Every answer below was captured from jq 1.7.1 under the zone named
//! in the row.

use std::process::Command;

use anyhow::Result;

use crate::{spawn_with_signal_retry, succinctly_bin};

fn run_in_zone(zone: &str, input: &str, fmt: &str, filter: &str) -> Result<(String, i32)> {
    let (output, code) = spawn_with_signal_retry(
        || {
            let mut command = Command::new(succinctly_bin());
            command
                .env("TZ", zone)
                .args(["jq", "-nc", "--arg", "i", input, "--arg", "f", fmt, filter]);
            command
        },
        None,
    )?;
    Ok((String::from_utf8(output.stdout)?, code))
}

#[test]
fn test_strptime_reads_epoch_seconds_through_the_local_zone_5303() -> Result<()> {
    for (zone, input, fmt, want) in [
        ("UTC", "1425599621", "%s", "[2015,2,5,23,53,41,4,63]\n"),
        ("UTC", "0", "%s", "[1970,0,1,0,0,0,4,0]\n"),
        ("UTC", "1425599621 x", "%s x", "[2015,2,5,23,53,41,4,63]\n"),
        ("UTC", "12 ", "%s", "[1970,0,1,0,0,12,4,0,\" \"]\n"),
        (
            "UTC",
            "2015 1425599621",
            "%Y %s",
            "[2015,2,5,23,53,41,4,63]\n",
        ),
        (
            "UTC",
            "1425599621 2015",
            "%s %Y",
            "[2015,2,5,23,53,41,4,63]\n",
        ),
        ("UTC", "1436000000", "%s", "[2015,6,4,8,53,20,6,184]\n"),
        ("UTC", "951782400", "%s", "[2000,1,29,0,0,0,2,59]\n"),
        ("UTC", "1425599621abc", "%s", ""),
        ("UTC", "1e3", "%s", ""),
        ("UTC", "1425599621", "%s|", ""),
        ("UTC", "+5", "%s", "[1970,0,1,0,0,5,4,0]\n"),
        ("UTC", "-1", "%s", "[1969,11,31,23,59,59,3,364]\n"),
        ("UTC", "  12", "%s", "[1970,0,1,0,0,12,4,0]\n"),
        ("UTC", "\t12", "%s", "[1970,0,1,0,0,12,4,0]\n"),
        ("UTC", "- 5", "%s", ""),
        ("UTC", "+-5", "%s", ""),
        ("UTC", "-", "%s", ""),
        ("UTC", "-86400", "%s", "[1969,11,31,0,0,0,3,364]\n"),
        (
            "America/New_York",
            "1425599621",
            "%s",
            "[2015,2,5,18,53,41,4,63]\n",
        ),
        ("America/New_York", "0", "%s", "[1969,11,31,19,0,0,3,364]\n"),
        (
            "America/New_York",
            "1425599621 x",
            "%s x",
            "[2015,2,5,18,53,41,4,63]\n",
        ),
        (
            "America/New_York",
            "12 ",
            "%s",
            "[1969,11,31,19,0,12,3,364,\" \"]\n",
        ),
        (
            "America/New_York",
            "2015 1425599621",
            "%Y %s",
            "[2015,2,5,18,53,41,4,63]\n",
        ),
        (
            "America/New_York",
            "1425599621 2015",
            "%s %Y",
            "[2015,2,5,18,53,41,4,63]\n",
        ),
        (
            "America/New_York",
            "1436000000",
            "%s",
            "[2015,6,4,4,53,20,6,184]\n",
        ),
        (
            "America/New_York",
            "951782400",
            "%s",
            "[2000,1,28,19,0,0,1,58]\n",
        ),
        ("America/New_York", "1425599621abc", "%s", ""),
        ("America/New_York", "1e3", "%s", ""),
        ("America/New_York", "1425599621", "%s|", ""),
        (
            "America/New_York",
            "+5",
            "%s",
            "[1969,11,31,19,0,5,3,364]\n",
        ),
        (
            "America/New_York",
            "-1",
            "%s",
            "[1969,11,31,18,59,59,3,364]\n",
        ),
        (
            "America/New_York",
            "  12",
            "%s",
            "[1969,11,31,19,0,12,3,364]\n",
        ),
        (
            "America/New_York",
            "\t12",
            "%s",
            "[1969,11,31,19,0,12,3,364]\n",
        ),
        ("America/New_York", "- 5", "%s", ""),
        ("America/New_York", "+-5", "%s", ""),
        ("America/New_York", "-", "%s", ""),
        (
            "America/New_York",
            "-86400",
            "%s",
            "[1969,11,30,19,0,0,2,363]\n",
        ),
        (
            "Australia/Sydney",
            "1425599621",
            "%s",
            "[2015,2,6,10,53,41,5,64]\n",
        ),
        ("Australia/Sydney", "0", "%s", "[1970,0,1,10,0,0,4,0]\n"),
        (
            "Australia/Sydney",
            "1425599621 x",
            "%s x",
            "[2015,2,6,10,53,41,5,64]\n",
        ),
        (
            "Australia/Sydney",
            "12 ",
            "%s",
            "[1970,0,1,10,0,12,4,0,\" \"]\n",
        ),
        (
            "Australia/Sydney",
            "2015 1425599621",
            "%Y %s",
            "[2015,2,6,10,53,41,5,64]\n",
        ),
        (
            "Australia/Sydney",
            "1425599621 2015",
            "%s %Y",
            "[2015,2,6,10,53,41,5,64]\n",
        ),
        (
            "Australia/Sydney",
            "1436000000",
            "%s",
            "[2015,6,4,18,53,20,6,184]\n",
        ),
        (
            "Australia/Sydney",
            "951782400",
            "%s",
            "[2000,1,29,11,0,0,2,59]\n",
        ),
        ("Australia/Sydney", "1425599621abc", "%s", ""),
        ("Australia/Sydney", "1e3", "%s", ""),
        ("Australia/Sydney", "1425599621", "%s|", ""),
        ("Australia/Sydney", "+5", "%s", "[1970,0,1,10,0,5,4,0]\n"),
        ("Australia/Sydney", "-1", "%s", "[1970,0,1,9,59,59,4,0]\n"),
        ("Australia/Sydney", "  12", "%s", "[1970,0,1,10,0,12,4,0]\n"),
        ("Australia/Sydney", "\t12", "%s", "[1970,0,1,10,0,12,4,0]\n"),
        ("Australia/Sydney", "- 5", "%s", ""),
        ("Australia/Sydney", "+-5", "%s", ""),
        ("Australia/Sydney", "-", "%s", ""),
        (
            "Australia/Sydney",
            "-86400",
            "%s",
            "[1969,11,31,10,0,0,3,364]\n",
        ),
    ] {
        let (stdout, code) = run_in_zone(zone, input, fmt, "$i | strptime($f)")?;
        assert_eq!(stdout, want, "{zone}: {input:?} | strptime({fmt:?})");
        assert_eq!(
            code,
            if want.is_empty() { 5 } else { 0 },
            "{zone}: {input:?} | strptime({fmt:?})"
        );
    }
    Ok(())
}

#[test]
fn test_strptime_epoch_seconds_round_trips_through_mktime_in_utc_5303() -> Result<()> {
    for secs in ["0", "1425599621", "951782400", "1436000000", "-86400"] {
        let (stdout, code) = run_in_zone("UTC", secs, "%s", "$i | strptime($f) | mktime")?;
        assert_eq!(
            (stdout.as_str(), code),
            (format!("{secs}\n").as_str(), 0),
            "{secs}"
        );
    }
    Ok(())
}
