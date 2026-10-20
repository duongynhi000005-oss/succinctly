//! #4977: `@props` escapes keys and values the way yq's properties encoder does.

use anyhow::Result;

use crate::run_yq_stdin;

/// #4977: yq writes a properties key and value through the Java `.properties` escape rules: backslash,
/// tab, newline, carriage return and form feed become `\\`, `\t`, `\n`, `\r`, `\f`; a key also escapes every
/// space and `:`; a value escapes only a leading space. Other controls, `=`, `#`, `!`, `.` and non-ASCII
/// are written as they are, and a top-level scalar is not escaped at all. Every row is captured from yq
/// v4.53.3 (`-p json -r '@props'`), JSON input so that the characters reach the encoder as such. `succinctly
/// yq` replaced a newline with a space and wrote the rest raw. The terminator is trimmed on both sides:
/// yq ends the text with a newline (#1076), which is not this issue's.
#[test]
fn test_4977_props_escapes_keys_and_values_as_yqs_encoder_does() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        ("{\"a\": \"x\\ny\"}", "a = x\\ny"),
        ("{\"a\": \"x\\ty\"}", "a = x\\ty"),
        ("{\"a\": \"x\\\\y\"}", "a = x\\\\y"),
        ("{\"a\": \"  x\"}", "a = \\  x"),
        ("{\"a b\": 1}", "a\\ b = 1"),
        ("{\"a:b\": 1}", "a\\:b = 1"),
        ("{\"a\": \"x\\n\"}", "a = x\\n"),
        ("{\"a\": \"x\\ry\"}", "a = x\\ry"),
        ("{\"a\": \"x\\fy\"}", "a = x\\fy"),
        ("{\"a\": \"x y\"}", "a = x y"),
        ("{\"a\": \"x \"}", "a = x "),
        ("{\"a\": \" \"}", "a = \\ "),
        ("{\"a\": \"x=y#z!w.v:u\"}", "a = x=y#z!w.v:u"),
        ("{\"a\": \"\\u00e9\\ud83d\\ude00\"}", "a = é😀"),
        ("{\"a\": \"x\\u0001y\"}", "a = x\u{1}y"),
        ("{\"a\": \"x\\u001by\"}", "a = x\u{1b}y"),
        ("{\"a\": \"x\\u007fy\"}", "a = x\u{7f}y"),
        ("{\"a\": \"x\\u2028y\"}", "a = x\u{2028}y"),
        ("{\"a\": \"x\\\"y'z\"}", "a = x\"y'z"),
        ("{\"a b\": \"c d\"}", "a\\ b = c d"),
        ("{\"a\": \" b\", \"c\": \"d \"}", "a = \\ b\nc = d "),
        ("{\"  a\": 1}", "\\ \\ a = 1"),
        ("{\"a  b\": 1}", "a\\ \\ b = 1"),
        ("{\"a \": 1}", "a\\  = 1"),
        ("{\"a\\tb\": 1}", "a\\tb = 1"),
        ("{\"a\\nb\": 1}", "a\\nb = 1"),
        ("{\"a\\\\b\": 1}", "a\\\\b = 1"),
        ("{\"#a\": 1}", "#a = 1"),
        ("{\"!a\": 1}", "!a = 1"),
        ("{\"a=b\": 1}", "a=b = 1"),
        ("{\"a.b\": 1}", "a.b = 1"),
        ("{\"\\u00e9\": 1}", "é = 1"),
        (
            "{\"a\": {\"b c\": {\"d:e\": \" v\\tw\"}}}",
            "a.b\\ c.d\\:e = \\ v\\tw",
        ),
        (
            "{\"a\": [{\"b c\": \"d\\te\"}, \"f g\", \" h\"]}",
            "a.0.b\\ c = d\\te\na.1 = f g\na.2 = \\ h",
        ),
        (
            "{\"a\": null, \"b\": \"\", \"c\": true, \"d\": 1.5}",
            "a = null\nb = \nc = true\nd = 1.5",
        ),
        ("[1, \"a b\", \" c\"]", "0 = 1\n1 = a b\n2 = \\ c"),
        ("\"x y\"", "x y"),
        ("\"  x\"", "  x"),
        ("\"x\\\\y\"", "x\\y"),
        ("\"\\u00e9\"", "é"),
    ];
    for (input, want) in rows {
        let (out, code) = run_yq_stdin("@props", input, &["-p=json", "-r"])?;
        assert_eq!(code, 0, "input {input}: {out:?}");
        // `-r` prints one newline after the text, and yq's text ends with one of its own.
        assert_eq!(out.trim_end_matches('\n'), *want, "input {input}");
    }
    Ok(())
}
