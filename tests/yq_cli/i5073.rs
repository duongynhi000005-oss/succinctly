//! #5073: a block scalar whose first content line is a `#` line indented deeper than its parent takes that line's
//! indent, not the indent of the deeper line after it: the scalar ended early and its remaining lines were read as
//! structure (`a: |-` / `  # h` / `    deep` / `  k` / `b: 1` lost `b: 1`). Each row is a live yq v4.53.3 capture
//! (`-o json -I0`).

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, `-o json -I0` output).
const ROWS: &[(&str, &str)] = &[
    (
        "- a: |\n   # x\n     y\n   z\n- q\n",
        "[{\"a\":\"# x\\n  y\\nz\\n\"},\"q\"]\n",
    ),
    (
        "- - |\n    # x\n      y\n    z\n",
        "[[\"# x\\n  y\\nz\\n\"]]\n",
    ),
    (
        "- a: |\n    # x\n      y\n    z\n- q\n",
        "[{\"a\":\"# x\\n  y\\nz\\n\"},\"q\"]\n",
    ),
    (
        "a: |-\n  # h\n    deep\n  k\nb: 1\n",
        "{\"a\":\"# h\\n  deep\\nk\",\"b\":1}\n",
    ),
    ("- |\n  # h\n    deep\n  x\n", "[\"# h\\n  deep\\nx\\n\"]\n"),
    ("a: |\n  # h\n  deep\n", "{\"a\":\"# h\\ndeep\\n\"}\n"),
    (
        "a: |\n  # h\n    deep\nb: 1\n",
        "{\"a\":\"# h\\n  deep\\n\",\"b\":1}\n",
    ),
    (
        "- >\n  #\n    x\n  y\n- z\n",
        "[\"#\\n  x\\ny\\n\",\"z\"]\n",
    ),
    (
        "a: |\n  x\n  # h\nb: 1\n",
        "{\"a\":\"x\\n# h\\n\",\"b\":1}\n",
    ),
    (
        "a:\n  b: |\n    # h\n      deep\n    k\n  c: 1\n",
        "{\"a\":{\"b\":\"# h\\n  deep\\nk\\n\",\"c\":1}}\n",
    ),
    (
        "a: |\n #\n    #x\n      - i\n# h\nb: 1\n",
        "{\"a\":\"#\\n   #x\\n     - i\\n\",\"b\":1}\n",
    ),
    (
        "- |\n  #x\n  # h\n   k\n- z\n",
        "[\"#x\\n# h\\n k\\n\",\"z\"]\n",
    ),
    ("- >\n\n   # c\n    deep\n", "[\"\\n# c\\n deep\\n\"]\n"),
    (
        "- >+\n #x\n  deep\n deep\n   # h\n- z\n",
        "[\"#x\\n deep\\ndeep\\n  # h\\n\",\"z\"]\n",
    ),
    (
        "- >+\n #x\n      - i\n    - i\n x: y\n---\nq: 1\n",
        "[\"#x\\n     - i\\n   - i\\nx: y\\n\"]\n{\"q\":1}\n",
    ),
    (
        "- >+\n\n # c\n     # r\n  deep\n",
        "[\"\\n# c\\n    # r\\n deep\\n\"]\n",
    ),
];

#[test]
fn test_block_scalar_first_content_line_hash_sets_the_indent_5073() -> Result<()> {
    for (doc, expected) in ROWS {
        let (stdout, code) = run_yq_stdin(".", doc, &["-o", "json", "-I0"])?;
        assert_eq!((stdout.as_str(), code), (*expected, 0), "{doc:?}");
    }
    Ok(())
}
