//! #1076: `@props` ends every line with a newline, the last included, as yq's properties encoder
//! writes it. `{"a":1} | @props` is the string `"a = 1\n"`, which the CLI then prints with its own
//! line break (`a = 1`, a blank line). A value with no lines is the empty string.
//!
//! Every output is captured from yq v4.53.3.

use anyhow::Result;

use crate::run_yq_stdin;

/// (filter, `yq -n` output).
const ROWS: &[(&str, &str)] = &[
    (r#"{"a":1} | @props"#, "a = 1\n\n"),
    (r#"{"a":{"b":[1,2]}} | @props"#, "a.b.0 = 1\na.b.1 = 2\n\n"),
    (r"[1,2] | @props", "0 = 1\n1 = 2\n\n"),
    (r#"[{"a":1}] | @props"#, "0.a = 1\n\n"),
    (r#""x" | @props"#, "x\n\n"),
    (r"1 | @props", "1\n\n"),
    (r"null | @props", "null\n\n"),
    (r"{} | @props", "\n"),
    (r"[] | @props", "\n"),
    (r#"{"v": 1e10*2} | @props"#, "v = 2e+10\n\n"),
    (r#"{"v": 4/2} | @props"#, "v = 2\n\n"),
    (
        r#"{"a":"x y","b":null,"c":true} | @props"#,
        "a = x y\nb = null\nc = true\n\n",
    ),
];

#[test]
fn test_1076_a_props_ends_every_line_with_a_newline() -> Result<()> {
    for (filter, want) in ROWS {
        let (out, code) = run_yq_stdin(filter, "", &["-n"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(&out, want, "{filter}");
    }
    Ok(())
}

/// The newline is part of the string: it counts in `length`, and `-o json` and `@json` escape it.
#[test]
fn test_1076_b_the_newline_is_part_of_the_string() -> Result<()> {
    let (out, _) = run_yq_stdin(r#"{"a":1} | @props | length"#, "", &["-n"])?;
    assert_eq!(out, "6\n");
    let (out, _) = run_yq_stdin(r#"{"a":1} | @props"#, "", &["-n", "-o=json"])?;
    assert_eq!(out, "\"a = 1\\n\"\n");
    let (out, _) = run_yq_stdin(r#"{"a":1} | @props | @json"#, "", &["-n", "-o=json"])?;
    assert_eq!(out, "\"\\\"a = 1\\\\n\\\"\"\n");
    Ok(())
}
