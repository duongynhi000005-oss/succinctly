//! #4977: `succinctly jq` shares `@props` (a yq-flavoured extension there) and follows the same escaping.

use anyhow::Result;

use crate::run_jq_stdin;

/// #4977: jq has no `@props`; the extension is yq's, so the rows are yq v4.53.3's (`-p json -r '@props'`).
#[test]
fn test_4977_jq_props_escapes_as_yqs_encoder_does() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        (r#"{"a":"x\ny"}"#, "a = x\\ny"),
        (
            r#"{"a b":"  c","d:e":"f\tg"}"#,
            "a\\ b = \\  c\nd\\:e = f\\tg",
        ),
        (r#""x y""#, "x y"),
    ];
    for (input, want) in rows {
        let (out, code) = run_jq_stdin("@props", input, &["-r"])?;
        assert_eq!(code, 0, "input {input}: {out:?}");
        assert_eq!(out.trim_end_matches('\n'), *want, "input {input}");
    }
    Ok(())
}
