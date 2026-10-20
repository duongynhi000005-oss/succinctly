//! #4947: a block scalar whose `|`/`>` header sits on a line of its own below a key that carries
//! node properties after its colon (`a: &x` / `  |` / `  text`). The header was not seen as the
//! value of that key, so the scalar's text paired with the next line (`"text": "b"`) and the
//! sibling key was lost. The dash forms (`- &x` / `  |`) already worked.
//!
//! Every output is captured from yq v4.53.3.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, `yq .`, `yq -o=json -I0 .`).
const ROWS: &[(&str, &str, &str)] = &[
    (
        "a: &x\n  |\n  text\nb: 1\n",
        "a: &x |\n  text\nb: 1\n",
        "{\"a\":\"text\\n\",\"b\":1}\n",
    ),
    (
        "a: !!str\n  >\n  text\nb: 1\n",
        "a: !!str >\n  text\n\nb: 1\n",
        "{\"a\":\"text\\n\",\"b\":1}\n",
    ),
    (
        "a: &x !!str\n  |\n  text\nb: 1\n",
        "a: &x !!str |\n  text\nb: 1\n",
        "{\"a\":\"text\\n\",\"b\":1}\n",
    ),
    (
        "a: !!str &x\n  |-\n  text\n  more\nb: 1\n",
        "a: &x !!str |-\n  text\n  more\nb: 1\n",
        "{\"a\":\"text\\nmore\",\"b\":1}\n",
    ),
    (
        "a: &x\n  |\n  text\n",
        "a: &x |\n  text\n",
        "{\"a\":\"text\\n\"}\n",
    ),
    (
        "a: &x\n  |2\n    text\nb: 1\n",
        "a: &x |2\n    text\nb: 1\n",
        "{\"a\":\"  text\\n\",\"b\":1}\n",
    ),
    (
        "a: &x\n\n  |\n  text\nb: 1\n",
        "a: &x |\n  text\nb: 1\n",
        "{\"a\":\"text\\n\",\"b\":1}\n",
    ),
    (
        "a: &x\n  # c\n  |\n  text\nb: 1\n",
        "a: &x |\n  text\n# c\nb: 1\n",
        "{\"a\":\"text\\n\",\"b\":1}\n",
    ),
    (
        "k:\n  a: &x\n    |\n    text\n  b: 1\nc: 2\n",
        "k:\n  a: &x |\n    text\n  b: 1\nc: 2\n",
        "{\"k\":{\"a\":\"text\\n\",\"b\":1},\"c\":2}\n",
    ),
    (
        "- a: &x\n    |\n    text\n  b: 1\n",
        "- a: &x |\n    text\n  b: 1\n",
        "[{\"a\":\"text\\n\",\"b\":1}]\n",
    ),
    (
        "a: &x\n  |\n  text\nb: *x\n",
        "a: &x |\n  text\nb: *x\n",
        "{\"a\":\"text\\n\",\"b\":\"text\\n\"}\n",
    ),
    (
        "\"a b\": !!str\n  >\n  folded\n  text\nz: 1\n",
        "\"a b\": !!str >\n  folded text\n\nz: 1\n",
        "{\"a b\":\"folded text\\n\",\"z\":1}\n",
    ),
    (
        "a: &x\n  >\n  p1\n\n  p2\nb: 1\n",
        "a: &x >\n  p1\n\n  p2\n\nb: 1\n",
        "{\"a\":\"p1\\np2\\n\",\"b\":1}\n",
    ),
    (
        "a: &x\n  >+\n  t\n\nb: 1\n",
        "a: &x >+\n  t\n\n\nb: 1\n",
        "{\"a\":\"t\\n\\n\",\"b\":1}\n",
    ),
];

#[test]
fn test_4947_a_block_scalar_header_below_a_key_with_properties_is_its_value() -> Result<()> {
    for (input, yaml, json) in ROWS {
        let (out, code) = run_yq_stdin(".", input, &[])?;
        assert_eq!(code, 0, "{input:?}");
        assert_eq!(&out, yaml, "{input:?}");
        let (out, code) = run_yq_stdin(".", input, &["-o=json", "-I0"])?;
        assert_eq!(code, 0, "{input:?}");
        assert_eq!(&out, json, "{input:?}");
    }
    Ok(())
}

/// Controls that already worked: the dash forms, the properties on the header's own line, and a
/// header on the key's own line.
#[test]
fn test_4947_b_the_neighbouring_forms_are_unchanged() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        ("- &x\n  |\n  text\n- 2\n", "[\"text\\n\",2]\n"),
        ("- !!str\n  >\n  text\n- 2\n", "[\"text\\n\",2]\n"),
        (
            "k:\n  - &x\n    |\n    text\n  - 2\n",
            "{\"k\":[\"text\\n\",2]}\n",
        ),
        (
            "a:\n  &x |\n  text\nb: 1\n",
            "{\"a\":\"text\\n\",\"b\":1}\n",
        ),
        ("a: &x |\n  text\nb: 1\n", "{\"a\":\"text\\n\",\"b\":1}\n"),
        ("a:\n  |\n  text\nb: 1\n", "{\"a\":\"text\\n\",\"b\":1}\n"),
        // A key with properties whose value is a collection, not a block scalar.
        ("a: &x\n  - 1\n  - 2\nb: 1\n", "{\"a\":[1,2],\"b\":1}\n"),
        ("a: &x\n  b: 1\nc: 2\n", "{\"a\":{\"b\":1},\"c\":2}\n"),
    ];
    for (input, json) in rows {
        let (out, code) = run_yq_stdin(".", input, &["-o=json", "-I0"])?;
        assert_eq!(code, 0, "{input:?}");
        assert_eq!(&out, json, "{input:?}");
    }
    Ok(())
}
