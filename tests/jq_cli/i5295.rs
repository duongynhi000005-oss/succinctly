//! #5295: jq's lexer reads `$ns::name` as one variable name, so a data import `import "d" as $a;`
//! is bound as `$a` and as `$a::a`, and `$x::y` is an ordinary (here, undefined) variable
//! otherwise, usable as a pattern binder and in `{$x::y}`. `$a::b` did not parse. Every answer
//! below was captured from jq 1.7.1; the data files are `d1.json` = `{"v":1}`, `d2.json` =
//! `{"v":2}`, and the module `s1.jq` = `def e: 1;`.

use std::process::Command;

use anyhow::Result;

use crate::{spawn_with_signal_retry, succinctly_bin};

/// Run `succinctly jq -nc -L <dir> <program>` over a directory holding the files above.
fn run_with_data(program: &str) -> Result<(String, String, i32)> {
    let dir = tempfile::tempdir()?;
    for (name, contents) in [
        ("d1.json", "{\"v\":1}"),
        ("d2.json", "{\"v\":2}"),
        ("s1.jq", "def e: 1;"),
    ] {
        std::fs::write(dir.path().join(name), contents)?;
    }
    let (output, code) = spawn_with_signal_retry(
        || {
            let mut command = Command::new(succinctly_bin());
            command
                .args(["jq", "-nc", "-L"])
                .arg(dir.path())
                .arg(program);
            command
        },
        None,
    )?;
    Ok((
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?,
        code,
    ))
}

#[test]
fn test_a_data_import_is_also_bound_as_alias_colon_colon_alias_5295() -> Result<()> {
    for (program, want) in [
        (
            "import \"d1\" as $d; [$d, $d::d]",
            "[[{\"v\":1}],[{\"v\":1}]]\n",
        ),
        ("import \"d1\" as $d; $d::d", "[{\"v\":1}]\n"),
        ("import \"d1\" as $d; def f: $d::d; f", "[{\"v\":1}]\n"),
        ("import \"d1\" as $d; [$d::d[] | .v]", "[1]\n"),
        ("import \"d1\" as $d; $d::d | length", "1\n"),
        (
            "import \"d1\" as $d; import \"d2\" as $d; $d::d",
            "[{\"v\":2}]\n",
        ),
        (
            "import \"d1\" as $d; import \"d2\" as $e; [$d::d, $e::e]",
            "[[{\"v\":1}],[{\"v\":2}]]\n",
        ),
        ("import \"d1\" as $d; $d::d as $x | $x", "[{\"v\":1}]\n"),
        (
            "import \"d1\" as $d; \"\\($d::d)\"",
            "\"[{\\\"v\\\":1}]\"\n",
        ),
        ("import \"d1\" as $d; {$d::d}", "{\"d::d\":[{\"v\":1}]}\n"),
        (
            "import \"d1\" as $d; {\"k\": $d::d}",
            "{\"k\":[{\"v\":1}]}\n",
        ),
        ("import \"d1\" as $d; $d::d?", "[{\"v\":1}]\n"),
        (
            "import \"d1\" as $d; reduce range(2) as $i (0; . + ($d::d|length))",
            "2\n",
        ),
        (
            "import \"s1\" as f; import \"d1\" as $f; [f::e, $f::f]",
            "[1,[{\"v\":1}]]\n",
        ),
        (
            "include \"s1\"; import \"d1\" as $d; [e, $d::d]",
            "[1,[{\"v\":1}]]\n",
        ),
        ("import \"d1\" as $d; $d::d.v?", ""),
    ] {
        let (stdout, stderr, code) = run_with_data(program)?;
        assert_eq!(stdout, want, "{program}: {stderr}");
        assert_eq!(code, 0, "{program}");
    }
    Ok(())
}

#[test]
fn test_a_namespaced_variable_that_is_not_bound_is_not_defined_5295() -> Result<()> {
    for (program, want) in [
        (
            "import \"d1\" as $d; $d::x",
            "jq: error: $d::x is not defined at <top-level>, line 1:\nimport \"d1\" as $d; $d::x                   \njq: 1 compile error\n",
        ),
        (
            "import \"d1\" as $d; $x::d",
            "jq: error: $x::d is not defined at <top-level>, line 1:\nimport \"d1\" as $d; $x::d                   \njq: 1 compile error\n",
        ),
        (
            "import \"d1\" as $d; $d::d::d",
            "jq: error: $d::d::d is not defined at <top-level>, line 1:\nimport \"d1\" as $d; $d::d::d                   \njq: 1 compile error\n",
        ),
        (
            "import \"d1\" as $d; $ENV::d",
            "jq: error: $ENV::d is not defined at <top-level>, line 1:\nimport \"d1\" as $d; $ENV::d                   \njq: 1 compile error\n",
        ),
        (
            "1 as $a::b | $a",
            "jq: error: $a is not defined at <top-level>, line 1:\n1 as $a::b | $a             \njq: 1 compile error\n",
        ),
    ] {
        let (stdout, stderr, code) = run_with_data(program)?;
        assert_eq!(
            (stdout.as_str(), stderr.as_str(), code),
            ("", want, 3),
            "{program}"
        );
    }
    Ok(())
}

#[test]
fn test_a_namespaced_variable_is_an_ordinary_binder_5295() -> Result<()> {
    for (program, want) in [
        ("1 as $a::a | $a::a", "1\n"),
        ("1 as $a::b::c | $a::b::c", "1\n"),
        ("[1,2] as [$a::x, $b::y] | [$a::x, $b::y]", "[1,2]\n"),
        ("{\"k\":5} as {k: $a::z} | $a::z", "5\n"),
        ("{\"k\":5} as {$a::z} | $a::z", "null\n"),
        ("[1,2] | reduce .[] as $q::r (0; . + $q::r)", "3\n"),
        ("[1,2] | [foreach .[] as $q::r (0; . + $q::r)]", "[1,3]\n"),
        ("def f($a::b): $a::b + 1; f(1)", "2\n"),
        ("1 as $A_1::B_2 | $A_1::B_2", "1\n"),
        ("[1] | .[] as $a::b | {$a::b}", "{\"a::b\":1}\n"),
        ("\"x\" as $a::b | \"\\($a::b)y\"", "\"xy\"\n"),
    ] {
        let (stdout, stderr, code) = run_with_data(program)?;
        assert_eq!((stdout.as_str(), code), (want, 0), "{program}: {stderr}");
    }
    Ok(())
}
