//! #5302: `@sh`, `@csv`, `@tsv` and `@html` render U+0000 as the two characters `\0`, as jq 1.7.1
//! does. Expected output captured live from the pinned jq 1.7.1; `-r` prints it as it is written.

use anyhow::Result;

use crate::run_jq_stdin;

const INPUT: &str = r#""a\u0000b""#;

#[test]
fn test_formats_render_nul_as_backslash_zero_5302() -> Result<()> {
    for (filter, want) in [
        ("@sh", "'a\\0b'"),
        ("[.] | @csv", "\"a\\0b\""),
        ("[.] | @tsv", "a\\0b"),
        ("@html", "a\\0b"),
        ("[., .] | @sh", "'a\\0b' 'a\\0b'"),
        (r#"@sh "x\(.)""#, "x'a\\0b'"),
        (r#"@html "<p>\(.)</p>""#, "<p>a\\0b</p>"),
    ] {
        let (out, code) = run_jq_stdin(filter, INPUT, &["-r"])?;
        assert_eq!(
            (out.as_str(), code),
            (format!("{want}\n").as_str(), 0),
            "{filter}"
        );
    }
    Ok(())
}

#[test]
fn test_formats_leave_other_escapes_alone_5302() -> Result<()> {
    // The NUL rewrite composes with each format's own escaping.
    for (filter, input, want) in [
        ("[.] | @csv", r#""a\u0000\"b""#, "\"a\\0\"\"b\""),
        ("[.] | @tsv", r#""a\u0000\tb""#, "a\\0\\tb"),
        ("@html", r#""<\u0000>""#, "&lt;\\0&gt;"),
        ("@sh", r#""'\u0000""#, "''\\''\\0'"),
        // Formats that already agreed with jq are unchanged.
        ("@uri", INPUT, "a%00b"),
    ] {
        let (out, code) = run_jq_stdin(filter, input, &["-r"])?;
        assert_eq!(
            (out.as_str(), code),
            (format!("{want}\n").as_str(), 0),
            "{filter}"
        );
    }
    Ok(())
}
