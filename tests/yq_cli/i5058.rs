//! #5058: go-yaml writes a `,` before the `}` of a non-empty flow mapping that owns a foot comment
//! (`- {a: 1, b: 2,}` with `# c` under it), on the read and write routes. An empty mapping, a flow
//! sequence, a mapping whose comment heads the next item and the output's root do not get one. Each
//! row is a live yq v4.53.3 capture.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, extra args, filter, expected output).
const ROWS: &[(&str, &[&str], &str, &str)] = &[
    (
        "k:\n  - { a: 1, b: 2 }\n  # c\n",
        &[],
        ".",
        "k:\n  - {a: 1, b: 2,}\n  # c\n",
    ),
    (
        "k:\n  - { a: [1] }\n  # c\n",
        &[],
        ".",
        "k:\n  - {a: [1],}\n  # c\n",
    ),
    (
        "k:\n  - { a: 1 } # l\n  # c\n",
        &[],
        ".",
        "k:\n  - {a: 1,} # l\n  # c\n",
    ),
    (
        "a:\n  - k:\n      - { a: 1 }\n      # c\n",
        &[],
        ".",
        "a:\n  - k:\n      - {a: 1,}\n      # c\n",
    ),
    (
        "k:\n  - { a: 1 }\n  # c\n",
        &[],
        ".k[0].a = 2",
        "k:\n  - {a: 2,}\n  # c\n",
    ),
    (
        "k:\n  - { a: 1 }\n  # c\n",
        &[],
        ".k += [3]",
        "k:\n  - {a: 1,}\n  # c\n\n  - 3\n",
    ),
    (
        "k:\n  - { a: 1, b: 2 }\n  # c\n",
        &[],
        "sort_keys(..)",
        "k:\n  - {a: 1, b: 2,}\n  # c\n",
    ),
    ("k:\n  - { a: 1 }\n  # c\n", &[], ".k", "- {a: 1,}\n# c\n"),
    ("k:\n  - {}\n  # c\n", &[], ".", "k:\n  - {}\n  # c\n"),
    (
        "k:\n  - [1, 2]\n  # c\n",
        &[],
        ".",
        "k:\n  - [1, 2]\n  # c\n",
    ),
    (
        "k:\n  - { a: 1 }\n  # c\n  - 2\n",
        &[],
        ".",
        "k:\n  - {a: 1}\n  # c\n  - 2\n",
    ),
    ("{ a: 1 }\n# c\n", &[], ".", "{a: 1}\n# c\n"),
];

#[test]
fn test_flow_mapping_that_owns_a_foot_keeps_a_trailing_comma_5058() -> Result<()> {
    for (doc, args, filter, expected) in ROWS {
        let (stdout, code) = run_yq_stdin(filter, doc, args)?;
        assert_eq!(
            (stdout.as_str(), code),
            (*expected, 0),
            "{doc:?} {args:?} {filter}"
        );
    }
    Ok(())
}
