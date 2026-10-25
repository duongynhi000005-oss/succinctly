//! #2789: `key | tostring` and `key | @format` over a string-spelled key hand the key's text to the
//! owned `tostring`/`@format` bypass (#2543) instead of re-materializing the key node through its
//! cursor. The bypass is a speed-only route, so every row here is the output the cursor route gave
//! (and, for the rows marked, real yq v4.53.3 gives).

use anyhow::Result;

use crate::run_yq_stdin;

const KEYS: &str =
    "plain: 1\n\"dq\": 2\nYes: 3\nname with space: 4\n\"\\u00e9\\u20ac\": 5\n\"\": 6\n";

fn lines(filter: &str, input: &str) -> Result<Vec<String>> {
    let (out, code) = run_yq_stdin(filter, input, &["-r"])?;
    assert_eq!(code, 0, "{filter}");
    Ok(out.lines().map(str::to_owned).collect())
}

/// Live yq v4.53.3 capture for every row.
#[test]
fn test_key_tostring_and_formats_over_string_keys_2789() -> Result<()> {
    let rows: &[(&str, &[&str])] = &[
        (
            ".[] | key | tostring",
            &[
                "plain",
                "dq",
                "Yes",
                "name with space",
                "\u{e9}\u{20ac}",
                "",
            ],
        ),
        (
            ".[] | key | @json",
            &[
                "\"plain\"",
                "\"dq\"",
                "\"Yes\"",
                "\"name with space\"",
                "\"\u{e9}\u{20ac}\"",
                "\"\"",
            ],
        ),
        (
            ".[] | key | @base64",
            &[
                "cGxhaW4=",
                "ZHE=",
                "WWVz",
                "bmFtZSB3aXRoIHNwYWNl",
                "w6nigqw=",
                "",
            ],
        ),
        (
            ".[] | key | @sh",
            &[
                "'plain'",
                "'dq'",
                "'Yes'",
                "'name with space'",
                "'\u{e9}\u{20ac}'",
                "''",
            ],
        ),
    ];
    for (filter, want) in rows {
        assert_eq!(&lines(filter, KEYS)?, want, "{filter}");
    }
    Ok(())
}

/// A key spelled like a null, bool or number keeps the cursor route, because `as_str` is the
/// scalar's text and not its type: `~` is a null and `01` an integer. This pins the *route
/// guard* (`key_spelling_may_retype`), not real yq's answer for these keys, which differs
/// (`~` and `01` come back verbatim there); that gap is not this issue's.
#[test]
fn test_key_tostring_keeps_the_typed_key_route_2789() -> Result<()> {
    let input = "~: 1\n01: 2\ntrue: 3\nnull: 4\n";
    assert_eq!(
        lines(".[] | key | tostring", input)?,
        ["null", "1", "true", "null"]
    );
    Ok(())
}

/// `keys[]` reaches the same arm through the lazy-keys producer, which does not check a key's
/// explicit tag: a `!!null`/`!!bool` key must still come back typed (this is the cursor route's
/// answer, unchanged; real yq prints the raw text for `tostring`, a gap that is not this issue's),
/// while a plain string key and a quoted number-spelled one stay strings.
#[test]
fn test_keys_iteration_tostring_keeps_tagged_keys_typed_2789() -> Result<()> {
    let input = "plain: 1\n\"12\": 2\n!!null abc: 3\n!!bool xyz: 4\n";
    assert_eq!(
        lines("keys[] | tostring", input)?,
        ["plain", "12", "null", "false"]
    );
    assert_eq!(
        lines("keys[] | @json", input)?,
        ["\"plain\"", "\"12\"", "null", "false"]
    );
    Ok(())
}
