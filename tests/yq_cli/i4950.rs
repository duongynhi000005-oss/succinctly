//! #4950: a key written in YAML whose value is over 128 bytes (or holds a line break) is not a
//! simple key to go-yaml's emitter, so yq writes it as `? key` with the `:` on a line of its own
//! -- on every route, with the spelling the key had in the source (`-P` clears it). #3587 did this
//! for a key decoded from JSON only.
//!
//! Every output is captured from yq v4.53.3. `<Kn>` stands for `n` repeated `k`s and `<S130>` for
//! 130 bytes of `a b a b ...`, expanded here so the rows stay readable.

use anyhow::Result;

use crate::run_yq_stdin;

fn expand(text: &str) -> String {
    let mut out = text.replace("<S130>", &"a b ".repeat(33));
    for n in [125, 128, 129, 130, 140] {
        out = out.replace(&format!("<K{n}>"), &"k".repeat(n));
    }
    out
}

/// (filter, extra args, document, expected output).
const ROWS: &[(&str, &[&str], &str, &str)] = &[
    (".", &[], "<K129>: 1\n", "? <K129>\n: 1\n"),
    (".", &["-P"], "<K129>: 1\n", "? <K129>\n: 1\n"),
    (".z = 1", &[], "<K129>: 1\n", "? <K129>\n: 1\nz: 1\n"),
    (".", &[], "<K128>: 1\n", "<K128>: 1\n"),
    (".", &["-P"], "<K128>: 1\n", "<K128>: 1\n"),
    (".", &[], "<K130>:\n  - a\n", "? <K130>\n: - a\n"),
    (".", &["-P"], "<K130>:\n  - a\n", "? <K130>\n: - a\n"),
    (".", &[], "\"<K129>\": 1\n", "? \"<K129>\"\n: 1\n"),
    (".", &["-P"], "\"<K129>\": 1\n", "? <K129>\n: 1\n"),
    (".", &[], "'<K129>': 1\n", "? '<K129>'\n: 1\n"),
    (".", &["-P"], "'<K129>': 1\n", "? <K129>\n: 1\n"),
    (".", &[], "<S130>: 1\n", "? a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b a b\n: 1\n"),
    (".", &[], "m:\n  <K140>:\n    q: 1\n", "m:\n  ? <K140>\n  : q: 1\n"),
    (".", &["-P"], "m:\n  <K140>:\n    q: 1\n", "m:\n  ? <K140>\n  : q: 1\n"),
    (".", &[], "<K140>:\n  a: 1\n", "? <K140>\n: a: 1\n"),
    (".", &[], "<K140>: &a\n  a: 1\n", "? <K140>\n: &a\n  a: 1\n"),
    (".", &[], "<K140>: 1 # c\n", "? <K140>\n: 1 # c\n"),
    (".", &[], "# h\n<K140>: 1\n", "# h\n? <K140>\n: 1\n"),
    (".", &[], "<K140>: 1\nb: 2\n", "? <K140>\n: 1\nb: 2\n"),
    (".", &["-P"], "<K140>: 1\nb: 2\n", "? <K140>\n: 1\nb: 2\n"),
    (".", &[], "- <K140>: 1\n  b: 2\n", "- ? <K140>\n  : 1\n  b: 2\n"),
    (".", &["-P"], "- <K140>: 1\n  b: 2\n", "- ? <K140>\n  : 1\n  b: 2\n"),
    (".", &[], "&x <K125>: 1\n", "&x <K125>: 1\n"),
    (".", &[], "&x <K140>: 1\n", "? &x <K140>\n: 1\n"),
];

#[test]
fn test_4950_a_a_yaml_key_over_128_bytes_is_an_explicit_key() -> Result<()> {
    for (filter, args, input, expected) in ROWS {
        let (out, code) = run_yq_stdin(filter, &expand(input), args)?;
        assert_eq!(code, 0, "{filter} {args:?} {input:?}");
        assert_eq!(out, expand(expected), "{filter} {args:?} {input:?}");
    }
    Ok(())
}
