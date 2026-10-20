//! #4959: a literal block scalar (`|`, clip chomping) that ends the input without a final line
//! break has no trailing `\n` to keep. Clip keeps one final break *if there is one*, so the value
//! is `"x\ny"` and yq prints the header as `|-`; the decoder used to add the missing break.
//!
//! Every output is captured from yq v4.53.3.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, `yq .`, `yq -o=json -I0 .`).
const ROWS: &[(&str, &str, &str)] = &[
    ("a: |\n  x\n  y", "a: |-\n  x\n  y\n", "{\"a\":\"x\\ny\"}\n"),
    (
        "a: |+\n  x\n  y",
        "a: |-\n  x\n  y\n",
        "{\"a\":\"x\\ny\"}\n",
    ),
    (
        "a: |-\n  x\n  y",
        "a: |-\n  x\n  y\n",
        "{\"a\":\"x\\ny\"}\n",
    ),
    (
        "a: |\n  x\n  y\n",
        "a: |\n  x\n  y\n",
        "{\"a\":\"x\\ny\\n\"}\n",
    ),
    (
        "a: |\n  x\n  y\n\n",
        "a: |\n  x\n  y\n",
        "{\"a\":\"x\\ny\\n\"}\n",
    ),
    (
        "a: |\n  x\n  y\nb: 1",
        "a: |\n  x\n  y\nb: 1\n",
        "{\"a\":\"x\\ny\\n\",\"b\":1}\n",
    ),
    ("- |\n  x", "- |-\n  x\n", "[\"x\"]\n"),
    ("|\n x", "x\n", "\"x\"\n"),
    ("a: |\n  x # c", "a: |-\n  x # c\n", "{\"a\":\"x # c\"}\n"),
    (
        "a: |\n  x\n\n  y",
        "a: |-\n  x\n\n  y\n",
        "{\"a\":\"x\\n\\ny\"}\n",
    ),
    (
        "a: |\n  x\r\n  y",
        "a: |-\n  x\n  y\n",
        "{\"a\":\"x\\ny\"}\n",
    ),
    (
        "a:\n  - |\n    x\n  - 2",
        "a:\n  - |\n    x\n  - 2\n",
        "{\"a\":[\"x\\n\",2]}\n",
    ),
    (
        "a: |2\n    x\n    y",
        "a: |2-\n    x\n    y\n",
        "{\"a\":\"  x\\n  y\"}\n",
    ),
    (
        "a: |\n   x\n   y",
        "a: |-\n  x\n  y\n",
        "{\"a\":\"x\\ny\"}\n",
    ),
    ("a: >\n  x\n  y", "a: >-\n  x y\n", "{\"a\":\"x y\"}\n"),
    (
        "a: |\n  x\n  y\n  # c",
        "a: |-\n  x\n  y\n  # c\n",
        "{\"a\":\"x\\ny\\n# c\"}\n",
    ),
];

#[test]
fn test_4959_a_a_literal_block_scalar_at_the_end_of_the_input_keeps_no_final_break() -> Result<()> {
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

/// The write route and `-P` print the header yq does.
#[test]
fn test_4959_b_the_write_routes_print_the_stripped_header() -> Result<()> {
    let (out, code) = run_yq_stdin(".z = 1", "a: |\n  x\n  y", &[])?;
    assert_eq!(code, 0);
    assert_eq!(out, "a: |-\n  x\n  y\nz: 1\n");
    let (out, code) = run_yq_stdin(".", "a: |\n  x\n  y", &["-P"])?;
    assert_eq!(code, 0);
    assert_eq!(out, "a: |-\n  x\n  y\n");
    Ok(())
}
