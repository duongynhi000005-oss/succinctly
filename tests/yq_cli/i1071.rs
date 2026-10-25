//! #1071: a *computed* non-finite is Go's `NaN`/`+Inf`/`-Inf` in yq, and a *document* `.inf`/`.NaN`/`-.INF`/`+.inf`
//! scalar keeps the bytes it was written with. A bare `Float` could not tell the two apart, so a document one is now a
//! `NumberLiteral` and the reindex bridge carries its spelling. Every row is a live yq v4.53.3 capture.
//! JSON output of a non-finite stays `null`, where yq errors (#1087).

use anyhow::Result;

use crate::run_yq_stdin;

const DOC: &str = "a: .inf\nb: .NaN\nc: -.INF\nd: +.inf\n";

#[test]
fn test_nonfinite_spelling_matches_yq_1071() -> Result<()> {
    let rows: &[(&str, &str, &str)] = &[
        (".a | tostring", DOC, ".inf\n"),
        (".b | tostring", DOC, ".NaN\n"),
        (".c | tostring", DOC, "-.INF\n"),
        (".d | tostring", DOC, "+.inf\n"),
        ("[.a]", DOC, "- .inf\n"),
        ("[.b]", DOC, "- .NaN\n"),
        ("[.c]", DOC, "- -.INF\n"),
        ("[.d]", DOC, "- +.inf\n"),
        (".c | [.] | .[0] | tostring", DOC, "-.INF\n"),
        ("[.a, .b] | .[1] | tostring", DOC, ".NaN\n"),
        ("(0/0) | tostring", DOC, "NaN\n"),
        ("0 * (1e300 * 1e300)", DOC, "NaN\n"),
        ("(-1e300 * 1e300) | tostring", DOC, "-Inf\n"),
        ("(1e300 * 1e300) | tostring", DOC, "+Inf\n"),
        ("[.a, (1e300 * 1e300)] | join(\",\")", DOC, ".inf,+Inf\n"),
        (
            "[(1e300 * 1e300), .c, (0 * (1e300 * 1e300))] | join(\",\")",
            DOC,
            "+Inf,-.INF,NaN\n",
        ),
        ("{\"v\": (1e300 * 1e300)}", DOC, "v: !!float +Inf\n"),
        ("{\"v\": 0 * (1e300 * 1e300)}", DOC, "v: !!float NaN\n"),
        (
            "[(1e300 * 1e300), 0 * (1e300 * 1e300)]",
            DOC,
            "- !!float +Inf\n- !!float NaN\n",
        ),
        ("(1e300 * 1e300) | @yaml", DOC, "+Inf\n\n"),
        ("(-1e300 * 1e300) | @props", DOC, "-Inf\n\n"),
        (".b | @props", DOC, ".NaN\n\n"),
        (".c | @yaml", DOC, "-.INF\n\n"),
        (
            ".a = (1e300 * 1e300)",
            DOC,
            "a: !!float +Inf\nb: .NaN\nc: -.INF\nd: +.inf\n",
        ),
        (
            ".q = 0 * (1e300 * 1e300)",
            DOC,
            "a: .inf\nb: .NaN\nc: -.INF\nd: +.inf\nq: !!float NaN\n",
        ),
        ("sub(\"a\"; 1e300 * 1e300)", "abc\n", "+Infbc\n"),
        ("sub(\"a\"; 0 * (1e300 * 1e300))", "abc\n", "NaNbc\n"),
    ];
    for (filter, input, want) in rows {
        let (stdout, code) = run_yq_stdin(filter, input, &[])?;
        assert_eq!(code, 0, "for {filter:?}: {stdout:?}");
        assert_eq!(&stdout, want, "for {filter:?}");
    }
    Ok(())
}

/// yq's JSON encoder errors on a non-finite (#1087); succinctly prints `null`, a document spelling included, where
/// it must not leak `.inf` as JSON.
#[test]
fn test_nonfinite_json_output_stays_null_1071() -> Result<()> {
    for filter in [".", "[.a, .b, .c, .d]", ".a", "[.a, (1e300 * 1e300)]"] {
        let (stdout, code) = run_yq_stdin(filter, DOC, &["-o", "json", "-I0"])?;
        assert_eq!(code, 0, "for {filter:?}: {stdout:?}");
        assert!(
            !stdout.contains(".inf") && !stdout.contains("INF") && !stdout.contains("NaN"),
            "for {filter:?}: {stdout:?}"
        );
    }
    Ok(())
}
