//! #2186: the routes that print a repeated mapping key once, where yq v4.53.3 prints it twice.
//!
//! The DOM route's result is an `OwnedValue` (`ResultWithComments`), so anything that reaches
//! it has merged `a: 1 / a: 2` into `a: 2` before output; `docs/compliance/yq/limitations.md`
//! lists the routes. The collapsed rows pin today's answer, not yq's: when a duplicate-capable
//! value (#1344 (b)) lands, a row flips to `KEPT` and its entry in that page goes.
//! The `KEPT` rows are the controls that show the document really has a repeated key, and
//! that the cursor route, which a plain `.` takes, still keeps it.

use anyhow::Result;

use crate::run_yq_stdin;

const DUP: &str = "a: 1\na: 2\nb: 3\n";
const KEPT: &str = "a: 1\na: 2\nb: 3\n";
const COLLAPSED: &str = "a: 2\nb: 3\n";

#[test]
fn the_cursor_route_keeps_the_repeated_key_2186() -> Result<()> {
    assert_eq!(run_yq_stdin(".", DUP, &[])?, (KEPT.to_string(), 0));
    assert_eq!(
        run_yq_stdin(".", DUP, &["--slurp"])?.0,
        "- a: 1\n  a: 2\n  b: 3\n"
    );
    Ok(())
}

#[test]
fn the_dom_forcing_routes_collapse_the_repeated_key_2186() -> Result<()> {
    let rows: &[(&str, &[&str], &str)] = &[
        (".", &["-P"], COLLAPSED),
        (".", &["--arg", "z", "y"], COLLAPSED),
        (".", &["--eval-all"], COLLAPSED),
        (".[0]", &["--slurp"], COLLAPSED),
        (". , .", &[], COLLAPSED),
        (".b = 5", &[], "a: 2\nb: 5\n"),
    ];
    for (filter, args, expected) in rows {
        let (out, code) = run_yq_stdin(filter, DUP, args)?;
        assert_eq!(code, 0, "{filter} {args:?}");
        assert_eq!(out, *expected, "{filter} {args:?}");
    }
    Ok(())
}

/// The part of #2186 that was a wiring fault: the DOM route's `LazySeq` arm printed a large
/// integer as `1e+20`; it now keeps yq's spelling (the JSON form is `100000000000000000000.0`).
#[test]
fn the_dom_route_keeps_a_large_integers_spelling_through_a_lazy_seq_2186() -> Result<()> {
    let doc = "- 99999999999999999999\n- 1\n";
    for filter in ["map(.)", "reverse"] {
        let (out, code) = run_yq_stdin(filter, doc, &["-P"])?;
        assert_eq!(code, 0, "{filter}");
        assert!(out.contains("99999999999999999999"), "{filter}: {out}");
        assert!(!out.contains("e+"), "{filter}: {out}");
    }
    Ok(())
}
