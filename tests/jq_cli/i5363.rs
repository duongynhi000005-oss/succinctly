//! #5363: `capture` and `scan` emit `null` for non-participating capture groups,
//! matching jq's behavior instead of dropping the group entirely.

use anyhow::Result;

use crate::run_jq_full;

fn run(filter: &str) -> Result<(String, String, i32)> {
    run_jq_full(&["-nc", filter], None)
}

#[test]
fn test_capture_and_scan_non_participating_groups_5363() -> Result<()> {
    for (filter, want) in [
        // Leading optional group omitted in match
        (
            r#""b" | capture("(?<x>a)?(?<y>b)")"#,
            "{\"x\":null,\"y\":\"b\"}\n",
        ),
        // Trailing optional group omitted in match
        (
            r#""a" | capture("(?<x>a)(?<y>b)?")"#,
            "{\"x\":\"a\",\"y\":null}\n",
        ),
        // Alternative branches in named captures
        (
            r#""c" | capture("(?<a>a)|(?<b>b)|(?<c>c)")"#,
            "{\"a\":null,\"b\":null,\"c\":\"c\"}\n",
        ),
        // capture with "g" flag generator
        (
            r#""a c" | [capture("(?<a>a)|(?<b>b)|(?<c>c)";"g")]"#,
            "[{\"a\":\"a\",\"b\":null,\"c\":null},{\"a\":null,\"b\":null,\"c\":\"c\"}]\n",
        ),
        // scan with trailing optional group
        (
            r#""a" | [scan("(a)(b)?(c)?")]"#,
            "[[\"a\",null,null]]\n",
        ),
        // scan with alternative groups
        (
            r#""b" | [scan("(a)|(b)")]"#,
            "[[null,\"b\"]]\n",
        ),
        // scan with multiple matches
        (
            r#""a b" | [scan("(a)|(b)")]"#,
            "[[\"a\",null],[null,\"b\"]]\n",
        ),
    ] {
        let (out, err, code) = run(filter)?;
        assert_eq!((out.as_str(), code), (want, 0), "{filter}: {err}");
    }
    Ok(())
}
