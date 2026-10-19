//! #4909: yq evaluates a `sub` replacement against the subject string, takes its first output, and
//! then expands `$1`, `${1}`, `$name` and `${name}` in the result with Go's `regexp.Expand`
//! rules; jq binds `.` to a captures object and does no expansion.

use anyhow::Result;

use crate::run_yq_stdin;

/// Every row is captured from yq v4.53.3 (`yq -n -o=json -I0`). `.` is the whole subject
/// (`sub("a"; .)` on `"a1b"` is `"a1b1b"`), a named group has no binding (`.n` is nothing, so
/// `""`), and the template rules include the malformed forms that stay literal (`$`, `${`, `${1`,
/// `${}`, `$ 1`), `$$`, a name that runs on (`$1x`, `$1_`), a leading zero (`$01` is a name no
/// group has) and a group that did not take part.
#[test]
fn test_4909_sub_replacement_reads_the_subject_and_expands_go_templates() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        ("\"a1b\" | sub(\"(a)\"; \"[$1]\")", "\"[a]1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"[${1}]\")", "\"[a]1b\"\n"),
        ("\"a1b\" | sub(\"(?P<n>a)\"; \"[$n]\")", "\"[a]1b\"\n"),
        ("\"a1b\" | sub(\"(?P<n>a)\"; \"[${n}]\")", "\"[a]1b\"\n"),
        ("\"a1b\" | sub(\"a\"; \"$0\")", "\"a1b\"\n"),
        ("\"a1b\" | sub(\"a\"; \"\\\\1\")", "\"\\\\11b\"\n"),
        ("\"a1b\" | sub(\"(a)(1)\"; \"$2$1\")", "\"1ab\"\n"),
        ("\"a1b\" | sub(\"a\"; .)", "\"a1b1b\"\n"),
        ("\"a1b\" | sub(\"a\"; . + \"!\")", "\"a1b!1b\"\n"),
        ("\"a1b\" | sub(\"a\"; length)", "\"31b\"\n"),
        ("\"a1b\" | sub(\"a\"; upcase)", "\"A1B1b\"\n"),
        ("\"a1b\" | sub(\"a\"; \"x\" + .)", "\"xa1b1b\"\n"),
        ("\"a1b\" | sub(\"a\"; .foo)", "\"1b\"\n"),
        ("\"a1b\" | sub(\"a\"; .foo?)", "\"1b\"\n"),
        ("\"a1b\" | sub(\"(?P<n>a)\"; .n)", "\"1b\"\n"),
        (
            "\"a1b\" | sub(\"(?P<n>a)\"; \"[\" + .n + \"]\")",
            "\"[]1b\"\n",
        ),
        ("\"a1b\" | sub(\"a\"; \"[\" + . + \"]\")", "\"[a1b]1b\"\n"),
        ("\"abab\" | sub(\"a\"; .)", "\"ababbababb\"\n"),
        (
            "\"abab\" | sub(\"a\"; \"[\" + . + \"]\")",
            "\"[abab]b[abab]b\"\n",
        ),
        ("\"a1b\" | sub(\"a\"; 5)", "\"51b\"\n"),
        ("\"a1b\" | sub(\"a\"; null)", "\"null1b\"\n"),
        ("\"a1b\" | sub(\"a\"; \"X\",\"Y\")", "\"X1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"$1x\")", "\"1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"$$\")", "\"$1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"$$1\")", "\"$11b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"$\")", "\"$1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"x$\")", "\"x$1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"${\")", "\"${1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"${1\")", "\"${11b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"${}\")", "\"${}1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"$ 1\")", "\"$ 11b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"${1}1\")", "\"a11b\"\n"),
        ("\"a1b\" | sub(\"(a)|(z)\"; \"[$2]\")", "\"[]1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"$9\")", "\"1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"$_\")", "\"1b\"\n"),
        ("\"a1b\" | sub(\"(?P<n>a)\"; \"$N\")", "\"1b\"\n"),
        ("\"a1b\" | sub(\"(?P<n1>a)\"; \"$n1\")", "\"a1b\"\n"),
        ("\"a1b\" | sub(\"a\"; \"$1\")", "\"1b\"\n"),
        ("\"éb\" | sub(\"(é)\"; \"$1é\")", "\"b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"\\\\$1\")", "\"\\\\a1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"${1}${1}\")", "\"aa1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"$1\"; \"g\")", "\"1b\"\n"),
        ("\"a1b\" | sub(\"a\"; \"$1\" + .)", "\"1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"$-1\")", "\"$-11b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"${ 1}\")", "\"${ 1}1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"$1.\")", "\"a.1b\"\n"),
        ("\"ab\" | sub(\"(?:a)\"; \"$0\")", "\"ab\"\n"),
        ("\"a1b\" | sub(\"a\"; \"$00\")", "\"1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"$01\")", "\"1b\"\n"),
        ("\"a1b\" | sub(\"(a)\"; \"$1_\")", "\"1b\"\n"),
        ("\"a1b\" | sub(\"a\"; select(false))", "\"1b\"\n"),
        ("\"abab\" | sub(\"a\"; select(false))", "\"bb\"\n"),
        (
            "{\"s\":\"a1b\",\"r\":\"Z\"} | .s |= sub(\"a\"; .)",
            "{\"s\":\"a1b1b\",\"r\":\"Z\"}\n",
        ),
        (
            "{\"s\":\"a1b\",\"r\":\"Z\"} | .s |= sub(\"a\"; parent)",
            "{\"s\":\"1b\",\"r\":\"Z\"}\n",
        ),
        (
            "{\"s\":\"a1b\",\"r\":\"Z\"} | .s | sub(\"a\"; .r)",
            "\"1b\"\n",
        ),
        (
            "{\"s\":\"a1b\",\"r\":\"Z\"} | .s |= sub(\"a\"; $x)",
            "{\"s\":\"1b\",\"r\":\"Z\"}\n",
        ),
        (
            "{\"s\":\"a1b\",\"r\":\"Z\"} | .s |= sub(\"a\"; \"[\" + ({\"k\":\"v\"} | .k) + \"]\")",
            "{\"s\":\"[v]1b\",\"r\":\"Z\"}\n",
        ),
        ("\"a$1b\" | sub(\"a\"; .)", "\"a$1b\"\n"),
        ("\"a$1b\" | sub(\"(a)\"; \"<\" + . + \">\")", "\"<a>$1b\"\n"),
        ("\"a${1}b\" | sub(\"(a)\"; .)", "\"aab${1}b\"\n"),
    ];
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, "null", &["-n", "-o=json", "-I=0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(out, *want, "{filter}");
    }
    Ok(())
}
