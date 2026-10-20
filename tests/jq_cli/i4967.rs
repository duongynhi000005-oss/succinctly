//! #4967: `fromdate`/`fromdateiso8601` parse exactly `%Y-%m-%dT%H:%M:%SZ`, as jq's definition does.

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

// jq defines `fromdateiso8601` as `strptime("%Y-%m-%dT%H:%M:%SZ") | mktime` and `fromdate` as `fromdateiso8601`. The
// binary parsed a lenient ISO 8601 grammar of its own instead (a date alone, no `Z`, fractional seconds, an offset,
// lower-case `t`/`z`, leading white space), so jq's `does not match format` error was an answer here, and the dates
// `mktime` refuses (before 1900, a result of `-1`) were numbers. Every row captured from `/usr/bin/jq` 1.7.1
// (stdout, error message, exit code), both routes. yq's `to_unix`, which shares the lenient parser, is unchanged.

/// #4967: the accepted shape, trailing white space and text, the forms that were accepted and are now errors, the
/// dates `mktime` refuses, `?`/`try`, and the round trip through `todate`.
#[test]
fn test_4967_fromdate_parses_exactly_the_iso_format() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            "null",
            r#""2020-01-01T00:00:00Z" | fromdate"#,
            "1577836800\n",
            "",
            0,
        ),
        (
            "null",
            r#""2020-01-01" | fromdate"#,
            "",
            r#"date "2020-01-01" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01T00:00:00" | fromdate"#,
            "",
            r#"date "2020-01-01T00:00:00" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01T00:00:00.5Z" | fromdate"#,
            "",
            r#"date "2020-01-01T00:00:00.5Z" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01T00:00:00+01:00" | fromdate"#,
            "",
            r#"date "2020-01-01T00:00:00+01:00" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01 00:00:00Z" | fromdate"#,
            "",
            r#"date "2020-01-01 00:00:00Z" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01t00:00:00z" | fromdate"#,
            "",
            r#"date "2020-01-01t00:00:00z" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""  2020-01-01T00:00:00Z" | fromdate"#,
            "",
            r#"date "  2020-01-01T00:00:00Z" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01T00:00:00Z " | fromdate"#,
            "1577836800\n",
            "",
            0,
        ),
        (
            "null",
            r#""2020-01-01T00:00:00Z x" | fromdate"#,
            "1577836800\n",
            "",
            0,
        ),
        (
            "null",
            r#""2020-01-01T00:00:00Zx" | fromdate"#,
            "",
            r#"date "2020-01-01T00:00:00Zx" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01T00:00Z" | fromdate"#,
            "",
            r#"date "2020-01-01T00:00Z" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""1969-12-31T23:59:59Z" | fromdate"#,
            "",
            r"invalid gmtime representation",
            5,
        ),
        (
            "null",
            r#""1900-01-01T00:00:00Z" | fromdate"#,
            "-2208988800\n",
            "",
            0,
        ),
        (
            "null",
            r#""2038-01-19T03:14:08Z" | fromdate"#,
            "2147483648\n",
            "",
            0,
        ),
        (
            "null",
            r#""2020-13-01T00:00:00Z" | fromdate"#,
            "",
            r#"date "2020-13-01T00:00:00Z" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""2020-02-30T00:00:00Z" | fromdate"#,
            "1583020800\n",
            "",
            0,
        ),
        (
            "null",
            r#""2020-01-01T24:00:00Z" | fromdate"#,
            "",
            r#"date "2020-01-01T24:00:00Z" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01T23:60:00Z" | fromdate"#,
            "",
            r#"date "2020-01-01T23:60:00Z" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""" | fromdate"#,
            "",
            r#"date "" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""abc" | fromdate"#,
            "",
            r#"date "abc" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01" | fromdateiso8601"#,
            "",
            r#"date "2020-01-01" does not match format "%Y-%m-%dT%H:%M:%SZ""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01T00:00:00Z" | fromdateiso8601"#,
            "1577836800\n",
            "",
            0,
        ),
        ("null", r#""2020-01-01" | fromdate?"#, "", "", 0),
        (
            "null",
            r#""2020-01-01" | try fromdate catch "E""#,
            "\"E\"\n",
            "",
            0,
        ),
        (
            "null",
            r#""2020-01-01T00:00:00Z" | fromdate | todate"#,
            "\"2020-01-01T00:00:00Z\"\n",
            "",
            0,
        ),
        (
            "null",
            r#""2020-01-01T00:00:00Z" | fromdate | todate | fromdate"#,
            "1577836800\n",
            "",
            0,
        ),
        (
            "null",
            r"5 | fromdate",
            "",
            r"strptime/1 requires string inputs and arguments",
            5,
        ),
    ])
}
