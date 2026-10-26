//! #5293: the order of `//` and the assignment operators is the one place jq and yq disagree
//! that moved with this change. yq gives `//` precedence 42 and `=` 40 (`pkg/yqlib/operation.go`),
//! so `succinctly yq` keeps `.b = .a // "d"` as `.b = (.a // "d")`; the jq order is pinned in
//! `tests/jq_cli/i5293.rs`. Every expected output is what yq v4.53.3 prints.

use anyhow::Result;

use crate::run_yq_stdin;

/// (document, filter, expected `-o=json -I=0` output).
const ROWS: &[(&str, &str, &str)] = &[
    (
        "a: ~\nb: 2\n",
        ".b = .a // \"d\"",
        "{\"a\":null,\"b\":\"d\"}\n",
    ),
    ("a: ~\n", ".a = null // 7", "{\"a\":7}\n"),
    ("{}", ".a |= . // 5", "{\"a\":5}\n"),
    (
        "a: ~\nb: 2\n",
        ".b = .a // .b // \"z\"",
        "{\"a\":null,\"b\":2}\n",
    ),
    (
        "a: ~\nb: 2\n",
        "(.b = .a) // \"d\"",
        "{\"a\":null,\"b\":null}\n",
    ),
    ("a: 1\n", ".a = 1 // 2 | .b = 3 // 4", "{\"a\":1,\"b\":3}\n"),
    (
        "a: ~\nb: 2\n",
        ".b = (.a // \"d\")",
        "{\"a\":null,\"b\":\"d\"}\n",
    ),
];

#[test]
fn test_yq_keeps_alternative_tighter_than_assignment_5293() -> Result<()> {
    for (doc, filter, want) in ROWS {
        let (out, code) = run_yq_stdin(filter, doc, &["-o=json", "-I=0"])?;
        assert_eq!((out.as_str(), code), (*want, 0), "`{filter}`");
    }
    Ok(())
}
