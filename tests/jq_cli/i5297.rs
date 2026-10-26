//! #5297: `@html` escapes `'` as `&apos;`, as jq 1.7.1 does, not `&#39;`. Captured live from the
//! pinned jq 1.7.1, through every route that reaches the formatter: the bare format, a container
//! JSON-encoded first, a format-prefixed interpolation, and `format("html")`.

use anyhow::Result;

use crate::run_jq_stdin;

#[test]
fn test_html_escapes_apostrophe_as_apos_5297() -> Result<()> {
    let input = r#""it's <a href='x'>""#;
    for (filter, want) in [
        ("@html", r#""it&apos;s &lt;a href=&apos;x&apos;&gt;""#),
        (
            "[.] | @html",
            r#""[&quot;it&apos;s &lt;a href=&apos;x&apos;&gt;&quot;]""#,
        ),
        (
            r#"@html "<p>\(.)</p>""#,
            r#""<p>it&apos;s &lt;a href=&apos;x&apos;&gt;</p>""#,
        ),
        (
            r#"format("html")"#,
            r#""it&apos;s &lt;a href=&apos;x&apos;&gt;""#,
        ),
    ] {
        let (out, code) = run_jq_stdin(filter, input, &["-c"])?;
        assert_eq!(
            (out.as_str(), code),
            (format!("{want}\n").as_str(), 0),
            "{filter}"
        );
    }
    Ok(())
}
