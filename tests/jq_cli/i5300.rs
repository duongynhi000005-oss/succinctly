//! #5300: string repetition tests the count's sign and NaN *before* truncating it, so a negative
//! fraction and `nan` give `null` where truncating first gave `""`. Every answer below was captured
//! from jq 1.7.1.

use anyhow::Result;

use crate::run_jq_stdin;

#[test]
fn test_string_repeat_by_a_negative_fraction_or_nan_is_null_5300() -> Result<()> {
    for (filter, input, want) in [
        (
            "[.[] * \"abc\"]",
            "[-1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 3.7]",
            "[null,null,\"\",\"\",\"abc\",\"abc\",\"abcabcabc\"]\n",
        ),
        (
            "[\"abc\" * .[]]",
            "[-1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 3.7]",
            "[null,null,\"\",\"\",\"abc\",\"abc\",\"abcabcabc\"]\n",
        ),
        ("[. * (nan,-nan)]", "\"abc\"", "[null,null]\n"),
        ("[(nan,-nan) * .]", "\"abc\"", "[null,null]\n"),
        (
            "[. * (-1e300, -1e-300, 1e-300, 0.999999, -0.0, -1)]",
            "\"abc\"",
            "[null,null,\"\",\"\",\"\",null]\n",
        ),
    ] {
        let (out, code) = run_jq_stdin(filter, input, &["-c"])?;
        assert_eq!((out.as_str(), code), (want, 0), "{filter}");
    }
    Ok(())
}
