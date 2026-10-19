//! #1128: `@json` spells a computed float the way yq does (its threshold-aware
//! `2e+10`), while a document scalar keeps the spelling its JSON encoder gives it.

use anyhow::Result;

use crate::run_yq_stdin;

/// #1128: `@json` on a computed whole float used to print `20000000000.0` where yq answers
/// `2e+10`, and the obvious fix (a threshold-aware float formatter for the `@json` arm) broke the
/// document-sourced `a: 99999999999999999999` case, which prints `100000000000000000000.0` in yq. The
/// fix landed with #2438's one threshold rule for computed floats and #1129's text-carrying
/// literal; this pins both sides together. Every row is captured from yq v4.53.3.
#[test]
fn test_1128_json_formats_computed_floats_with_yqs_threshold() -> Result<()> {
    let input = "a: 99999999999999999999\nb: \"1e10\"\nc: 1.5\nd: 1e100\n";
    let rows: &[(&str, &str)] = &[
        ("(1e10 * 2) | @json", "2e+10\n"),
        ("(1e10 * 2) | tostring", "2e+10\n"),
        (".a | @json", "100000000000000000000.0\n"),
        (".c | @json", "1.5\n"),
        (".d | @json", "1e100\n"),
        ("[1e10 * 2] | .[0] | @json", "2e+10\n"),
        ("(.b | tonumber) | @json", "1e10\n"),
        ("(.c * 2) | @json", "3.0\n"),
        ("(.c * 1e9) | @json", "1.5e+09\n"),
        ("[1e10 * 2] | @json", "[2e+10]\n"),
        ("{\"x\": (1e10 * 2)} | @json", "{\"x\":2e+10}\n"),
        ("(1.0 + 1) | @json", "2.0\n"),
        ("(-1e10 * 2) | @json", "-2e+10\n"),
        ("(1e-10 * 2) | @json", "2e-10\n"),
    ];
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, input, &["-r"])?;
        assert_eq!(code, 0, "filter {filter:?}: {out:?}");
        assert_eq!(out, *want, "filter {filter:?}");
    }
    Ok(())
}
