//! #4965: `strptime` fails on text left over after the format and appends what follows white space.

use anyhow::Result;

use crate::assert_path_rows_both_routes_3749;

// jq's `strptime` is `end = strptime(input, fmt, &tm); if (end == NULL || (*end != '\0' && !isspace(*end)))` error,
// then `if (*end != '\0')` the remainder is appended to the broken-down time as a ninth element. The binary
// accepted any leftover text (`"2020-01-01x" | strptime("%Y-%m-%d")` answered a date) and never appended one.
// Every row captured from `/usr/bin/jq`
// 1.7.1 (stdout, error message, exit code), both routes.

/// #4965: leftover text, leftover white space and what follows it, the callers that read the ninth element's array
/// (`mktime`, `todate`, `length`), `?`/`try`, and the rows that matched before.
#[test]
fn test_4965_strptime_leftover_input() -> Result<()> {
    assert_path_rows_both_routes_3749(&[
        (
            "null",
            r#""2020-01-01x" | strptime("%Y-%m-%d")"#,
            "",
            r#"date "2020-01-01x" does not match format "%Y-%m-%d""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01xyz" | strptime("%Y-%m-%d")"#,
            "",
            r#"date "2020-01-01xyz" does not match format "%Y-%m-%d""#,
            5,
        ),
        (
            "null",
            r#""abc" | strptime("a")"#,
            "",
            r#"date "abc" does not match format "a""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01 x" | strptime("%Y-%m-%d")"#,
            "[2020,0,1,0,0,0,3,0,\" x\"]\n",
            r"",
            0,
        ),
        (
            "null",
            r#""2020-01-01 " | strptime("%Y-%m-%d")"#,
            "[2020,0,1,0,0,0,3,0,\" \"]\n",
            r"",
            0,
        ),
        (
            "null",
            r#""2020-01-01  " | strptime("%Y-%m-%d")"#,
            "[2020,0,1,0,0,0,3,0,\"  \"]\n",
            r"",
            0,
        ),
        (
            "null",
            r#""2020-01-01\ttail" | strptime("%Y-%m-%d")"#,
            "[2020,0,1,0,0,0,3,0,\"\\ttail\"]\n",
            r"",
            0,
        ),
        (
            "null",
            r#""2020-01-01\n" | strptime("%Y-%m-%d")"#,
            "[2020,0,1,0,0,0,3,0,\"\\n\"]\n",
            r"",
            0,
        ),
        (
            "null",
            r#""2020-01-01 trailing words" | strptime("%F") | length"#,
            "9\n",
            r"",
            0,
        ),
        (
            "null",
            r#""2020-01-01 tail" | strptime("%Y-%m-%d") | .[8]"#,
            "\" tail\"\n",
            r"",
            0,
        ),
        (
            "null",
            r#""2020-01-01 tail" | strptime("%Y-%m-%d") | mktime"#,
            "1577836800\n",
            r"",
            0,
        ),
        (
            "null",
            r#""2020-01-01 tail" | strptime("%Y-%m-%d") | todate"#,
            "\"2020-01-01T00:00:00Z\"\n",
            r"",
            0,
        ),
        (
            "null",
            r#""2020-01-01x" | strptime("%Y-%m-%d")?"#,
            "",
            r"",
            0,
        ),
        (
            "null",
            r#"[.[]?, ("2020-01-01x" | try strptime("%Y-%m-%d") catch "no")]"#,
            "[\"no\"]\n",
            r"",
            0,
        ),
        (
            "null",
            r#""2020-01-01T10:00:00Z" | strptime("%Y-%m-%dT%H:%M:%SZ")"#,
            "[2020,0,1,10,0,0,3,0]\n",
            r"",
            0,
        ),
        (
            "null",
            r#""2020-01-01T10:00:00Z" | strptime("%Y-%m-%dT%H:%M:%S")"#,
            "",
            r#"date "2020-01-01T10:00:00Z" does not match format "%Y-%m-%dT%H:%M:%S""#,
            5,
        ),
        (
            "null",
            r#""2020-01-01" | strptime("%Y-%m-%d")"#,
            "[2020,0,1,0,0,0,3,0]\n",
            r"",
            0,
        ),
        (
            "null",
            r#""2020-01-01" | strptime("%Y-%m-%d ")"#,
            "[2020,0,1,0,0,0,3,0]\n",
            r"",
            0,
        ),
        (
            "null",
            r#""Mon, 02 Jan 2006" | strptime("%a, %d %b %Y")"#,
            "[2006,0,2,0,0,0,1,1]\n",
            r"",
            0,
        ),
        (
            "null",
            r#""12/31/20x" | strptime("%D")"#,
            "",
            r#"date "12/31/20x" does not match format "%D""#,
            5,
        ),
    ])
}
