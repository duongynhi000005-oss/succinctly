//! #1076: `@props` ends every line with a newline in `succinctly jq` too (it is a yq-flavoured
//! extension there, and real jq has no `@props`, so the pin is yq's own encoder, mirrored the way
//! `@yaml` is). The string carries the newline: `-r` prints it plus the CLI's own line break, and
//! an interpolation embeds it.

use anyhow::Result;

use crate::run_jq_stdin;

#[test]
fn test_1076_jq_mode_props_ends_every_line_with_a_newline() -> Result<()> {
    for (filter, args, want) in [
        ("@props", &["-c"][..], "\"a = 1\\nb = 2\\n\"\n"),
        ("@props", &["-r"][..], "a = 1\nb = 2\n\n"),
        ("@props | length", &["-c"][..], "12\n"),
        (r#"@props "<\(.)>""#, &["-r"][..], "<a = 1\nb = 2\n>\n"),
    ] {
        let (out, code) = run_jq_stdin(filter, r#"{"a":1,"b":2}"#, args)?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(out, want, "{filter}");
    }
    // A value with no lines is the empty string.
    let (out, _) = run_jq_stdin("@props", "{}", &["-c"])?;
    assert_eq!(out, "\"\"\n");
    Ok(())
}
