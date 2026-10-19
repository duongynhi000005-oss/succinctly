//! #1129: an integer too big for `i64` keeps its digits through `tostring`, interpolation
//! and every other place yq echoes a scalar's text, while yq's JSON encoder still prints
//! the double it parsed.

use anyhow::Result;

use crate::run_yq_stdin;

type Row = (&'static [&'static str], &'static str, &'static str);

/// #1129: every row is captured from yq v4.53.3. `99999999999999999999` is a `!!float` to
/// yq (it overflows `i64`), but the node still holds the text it was written with: `tostring`,
/// `"\(.)"`, `join`, `length` and a YAML re-emit all answer those digits, where the JSON encoder
/// prints `100000000000000000000.0`. `9223372036854775807` is the control: it fits `i64`, so
/// it is an int in both spellings.
#[test]
fn test_1129_overflowed_int_keeps_its_digits_and_json_rounds() -> Result<()> {
    let cases: &[(&str, &[Row])] = &[
        (
            "a: 99999999999999999999\n",
            &[
                (&[], ".a | tostring", "99999999999999999999\n"),
                (&[], "\"v=\\(.a)\"", "v=99999999999999999999\n"),
                (&[], "[.a] | .[0]", "99999999999999999999\n"),
                (&[], ".a = .a", "a: 99999999999999999999\n"),
                (&[], ".a", "99999999999999999999\n"),
                (&[], "[.a]", "- 99999999999999999999\n"),
                (&[], "{\"x\": .a}", "x: 99999999999999999999\n"),
                (&[], "[.a] | join(\",\")", "99999999999999999999\n"),
                (&[], ".a | length", "20\n"),
                (&["-o=json", "-I=0"], ".a", "100000000000000000000.0\n"),
                (&["-o=json", "-I=0"], "[.a]", "[100000000000000000000.0]\n"),
                (
                    &["-o=json", "-I=0"],
                    ".a = .a",
                    "{\"a\":100000000000000000000.0}\n",
                ),
                (
                    &["-o=json", "-I=0"],
                    ".a | @json",
                    "\"100000000000000000000.0\"\n",
                ),
                (
                    &["-o=json", "-I=0"],
                    "{\"x\": .a} | tostring",
                    "\"x: 99999999999999999999\"\n",
                ),
                (
                    &["-o=json", "-I=0"],
                    ".a | tostring",
                    "\"99999999999999999999\"\n",
                ),
            ],
        ),
        (
            "a: -99999999999999999999\n",
            &[
                (&[], ".a | tostring", "-99999999999999999999\n"),
                (&[], "\"v=\\(.a)\"", "v=-99999999999999999999\n"),
                (&[], "[.a] | .[0]", "-99999999999999999999\n"),
                (&[], ".a = .a", "a: -99999999999999999999\n"),
                (&[], ".a", "-99999999999999999999\n"),
                (&[], "[.a]", "- -99999999999999999999\n"),
                (&[], "{\"x\": .a}", "x: -99999999999999999999\n"),
                (&[], "[.a] | join(\",\")", "-99999999999999999999\n"),
                (&[], ".a | length", "21\n"),
                (&["-o=json", "-I=0"], ".a", "-100000000000000000000.0\n"),
                (&["-o=json", "-I=0"], "[.a]", "[-100000000000000000000.0]\n"),
                (
                    &["-o=json", "-I=0"],
                    ".a = .a",
                    "{\"a\":-100000000000000000000.0}\n",
                ),
                (
                    &["-o=json", "-I=0"],
                    ".a | @json",
                    "\"-100000000000000000000.0\"\n",
                ),
                (
                    &["-o=json", "-I=0"],
                    "{\"x\": .a} | tostring",
                    "\"x: -99999999999999999999\"\n",
                ),
                (
                    &["-o=json", "-I=0"],
                    ".a | tostring",
                    "\"-99999999999999999999\"\n",
                ),
            ],
        ),
        (
            "a: -9223372036854775809\n",
            &[
                (&[], ".a | tostring", "-9223372036854775809\n"),
                (&[], "\"v=\\(.a)\"", "v=-9223372036854775809\n"),
                (&[], "[.a] | .[0]", "-9223372036854775809\n"),
                (&[], ".a = .a", "a: -9223372036854775809\n"),
                (&[], ".a", "-9223372036854775809\n"),
                (&[], "[.a]", "- -9223372036854775809\n"),
                (&[], "{\"x\": .a}", "x: -9223372036854775809\n"),
                (&[], "[.a] | join(\",\")", "-9223372036854775809\n"),
                (&[], ".a | length", "20\n"),
                (&["-o=json", "-I=0"], ".a", "-9223372036854776000.0\n"),
                (&["-o=json", "-I=0"], "[.a]", "[-9223372036854776000.0]\n"),
                (
                    &["-o=json", "-I=0"],
                    ".a = .a",
                    "{\"a\":-9223372036854776000.0}\n",
                ),
                (
                    &["-o=json", "-I=0"],
                    ".a | @json",
                    "\"-9223372036854776000.0\"\n",
                ),
                (
                    &["-o=json", "-I=0"],
                    "{\"x\": .a} | tostring",
                    "\"x: -9223372036854775809\"\n",
                ),
                (
                    &["-o=json", "-I=0"],
                    ".a | tostring",
                    "\"-9223372036854775809\"\n",
                ),
            ],
        ),
        (
            "a: 9223372036854775807\n",
            &[
                (&[], ".a | tostring", "9223372036854775807\n"),
                (&[], "\"v=\\(.a)\"", "v=9223372036854775807\n"),
                (&[], "[.a] | .[0]", "9223372036854775807\n"),
                (&[], ".a = .a", "a: 9223372036854775807\n"),
                (&[], ".a", "9223372036854775807\n"),
                (&[], "[.a]", "- 9223372036854775807\n"),
                (&[], "{\"x\": .a}", "x: 9223372036854775807\n"),
                (&[], "[.a] | join(\",\")", "9223372036854775807\n"),
                (&[], ".a | length", "19\n"),
                (&["-o=json", "-I=0"], ".a", "9223372036854775807\n"),
                (&["-o=json", "-I=0"], "[.a]", "[9223372036854775807]\n"),
                (
                    &["-o=json", "-I=0"],
                    ".a = .a",
                    "{\"a\":9223372036854775807}\n",
                ),
                (
                    &["-o=json", "-I=0"],
                    ".a | @json",
                    "\"9223372036854775807\"\n",
                ),
                (
                    &["-o=json", "-I=0"],
                    "{\"x\": .a} | tostring",
                    "\"x: 9223372036854775807\"\n",
                ),
                (
                    &["-o=json", "-I=0"],
                    ".a | tostring",
                    "\"9223372036854775807\"\n",
                ),
            ],
        ),
    ];
    for (input, rows) in cases {
        for (args, filter, want) in *rows {
            let (out, code) = run_yq_stdin(filter, input, args)?;
            assert_eq!(
                code, 0,
                "input {input:?} args {args:?} filter {filter:?}: {out:?}"
            );
            assert_eq!(
                out, *want,
                "input {input:?} args {args:?} filter {filter:?}"
            );
        }
    }
    Ok(())
}

/// #1129: the routes other than a plain YAML-sourced value read. Every row is captured from yq
/// v4.53.3. `-r` writes a scalar's own text, so a root `-r -o=json` prints the digits while the same
/// number inside a container is JSON-encoded (rounded); `-P`, an anchor/alias pair and `--eval-all`
/// reach the value through the DOM or owned routes; a JSON-sourced document is a float to yq's JSON
/// decoder (`1e+20`), so the digits are not kept there.
#[test]
fn test_1129_overflowed_int_on_other_routes() -> Result<()> {
    #[allow(clippy::type_complexity)]
    let cases: &[(&str, &[&str], &str, &str)] = &[
        (
            "a: 99999999999999999999\n",
            &["-r", "-o=json"],
            ".a",
            "99999999999999999999\n",
        ),
        (
            "a: 99999999999999999999\n",
            &["-r", "-o=json"],
            "[.a] | .[0]",
            "99999999999999999999\n",
        ),
        (
            "a: 99999999999999999999\n",
            &["-r", "-o=json"],
            "[.a]",
            "[\n  100000000000000000000.0\n]\n",
        ),
        (
            "a: 99999999999999999999\n",
            &["-P"],
            ".a | tostring",
            "99999999999999999999\n",
        ),
        (
            "a: 99999999999999999999\n",
            &["-P"],
            ".a",
            "99999999999999999999\n",
        ),
        (
            "a: 99999999999999999999\nb: 1\n",
            &["-P"],
            ".b = 2",
            "a: 99999999999999999999\nb: 2\n",
        ),
        (
            "a: &x 99999999999999999999\nb: *x\n",
            &[],
            ".a | tostring",
            "99999999999999999999\n",
        ),
        (
            "a: &x 99999999999999999999\nb: *x\n",
            &["-o=json", "-I=0"],
            ".",
            "{\"a\":100000000000000000000.0,\"b\":100000000000000000000.0}\n",
        ),
        (
            "a: 99999999999999999999\n",
            &["--eval-all"],
            ".a | tostring",
            "99999999999999999999\n",
        ),
        (
            "{\"a\": 99999999999999999999}",
            &["-p=json", "-o=yaml"],
            ".a | tostring",
            "1e+20\n",
        ),
        (
            "a: 99999999999999999999\nb: [99999999999999999999, 1]\n",
            &["-o=json", "-I=0"],
            ".",
            "{\"a\":100000000000000000000.0,\"b\":[100000000000000000000.0,1]}\n",
        ),
        (
            "a: 99999999999999999999\nb: [99999999999999999999, 1]\n",
            &[],
            "[.a, .b[0]] | map(tostring)",
            "- \"99999999999999999999\"\n- \"99999999999999999999\"\n",
        ),
    ];
    for (input, args, filter, want) in cases {
        let (out, code) = run_yq_stdin(filter, input, args)?;
        assert_eq!(
            code, 0,
            "input {input:?} args {args:?} filter {filter:?}: {out:?}"
        );
        assert_eq!(
            out, *want,
            "input {input:?} args {args:?} filter {filter:?}"
        );
    }
    Ok(())
}
