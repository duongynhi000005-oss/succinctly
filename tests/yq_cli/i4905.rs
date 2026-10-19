//! #4905: yq's `sub(re; replacement)` evaluates its replacement once, against the input.

use anyhow::Result;

use crate::{run_yq_stdin, run_yq_stdin_with_stderr};

/// #4905: real yq evaluates the replacement of a bare `sub` **once, against `sub`'s own input**
/// (before anything matches, and whether or not anything does), takes its first output (an empty
/// stream is the empty string), and then runs Go's `ReplaceAllString` with that text as the template:
/// `$1`, `${name}` and `$$` expand, a capture group is never visible to the replacement itself.
/// `succinctly yq` followed jq's model (the replacement per match, against a capture object), which
/// agrees only for a literal. Every row captured from yq v4.53.3 (`-o=json -I=0`, a `null` document).
#[test]
fn test_4905_yq_sub_replacement_is_read_once_against_the_input() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        (r#""a1b" | sub("a"; .)"#, r#""a1b1b""#),
        (r#""a1b" | sub("a|1"; .)"#, r#""a1ba1bb""#),
        (r#""a1b" | sub("a"; "[" + . + "]")"#, r#""[a1b]1b""#),
        (r#""a1b" | sub("a"; length)"#, r#""31b""#),
        (r#""a1b" | sub("(?<x>a)"; .x)"#, r#""1b""#),
        (r#""a1b" | sub("(?<x>a)"; "<\(.x)>")"#, r#""<>1b""#),
        (r#""a1b" | sub("a"; .foo)"#, r#""1b""#),
        (r#""a1b" | sub("a"; select(false))"#, r#""1b""#),
        (r#""a1b" | sub("a"; ("X","Y"))"#, r#""X1b""#),
        (r#""a1b" | [sub("a"; ("X","Y"))]"#, r#"["X1b"]"#),
        (r#""a1b" | sub("a"; 1.5)"#, r#""1.51b""#),
        (r#""a1b" | sub("a"; null)"#, r#""null1b""#),
        (r#""a1b" | sub("a"; {})"#, r#""{}1b""#),
        (r#""a1b" | sub("a"; [1])"#, r#""1b""#),
        (r#""a1b" | sub("a"; . + "!")"#, r#""a1b!1b""#),
        (r#""a1b" | sub("a"; ., "Z")"#, r#""a1b1b""#),
        (r#""a1b" | sub("a"; (.foo // "d"))"#, r#""d1b""#),
        (r#""a1b" | sub("a"; @base64)"#, r#""YTFi1b""#),
        (r#""a1b" | sub("a"; tostring)"#, r#""a1b1b""#),
        (r#""abab" | sub("b"; .)"#, r#""aababaabab""#),
        (r#""abab" | sub("b"; "[" + "]")"#, r#""a[]a[]""#),
        (r#""abab" | sub("b"; (. | length))"#, r#""a4a4""#),
        (r#""abab" | sub("b"; ("q" | length))"#, r#""a1a1""#),
        (r#""abab" | sub("(b)"; "$1")"#, r#""abab""#),
        (r#""abab" | sub("(?P<n>b)"; "${n}")"#, r#""abab""#),
        (r#""abab" | sub("(?P<n>b)"; .n)"#, r#""aa""#),
        (r#""abab" | sub("(b)"; "<$1$1>")"#, r#""a<bb>a<bb>""#),
        (r#""abab" | sub("(b)"; "$$1")"#, r#""a$1a$1""#),
        (r#""abab" | sub("(b)"; "$1x")"#, r#""aa""#),
        (r#""abab" | sub("(b)"; "${1}x")"#, r#""abxabx""#),
        (r#""abab" | sub("(b)"; "$01")"#, r#""aa""#),
        (r#""abab" | sub("(b)"; "$0")"#, r#""abab""#),
        (r#""abab" | sub("(b)"; "$")"#, r#""a$a$""#),
        (r#""abab" | sub("(b)"; "${1")"#, r#""a${1a${1""#),
        (r#""abab" | sub("(b)"; "$ 1")"#, r#""a$ 1a$ 1""#),
        (r#""abab" | sub("(b)"; "$2")"#, r#""aa""#),
        (r#""abab" | sub("(?P<n>b)"; "${n}${n}")"#, r#""abbabb""#),
        (r#""abab" | sub("(?P<n>b)"; "$nx")"#, r#""aa""#),
        (r#""a1b" | sub("(a)"; "<\\1>")"#, r#""<\\1>1b""#),
        (r#""" | sub(""; .)"#, r#""""#),
        (r#""" | sub(""; "X")"#, r#""X""#),
        (r#""ab" | sub(""; .)"#, r#""abaabbab""#),
        (r#""ab" | sub("x*"; "-")"#, r#""-a-b-""#),
        (r#""a1b" | sub("z"; .)"#, r#""a1b""#),
        (r#""a1b" | sub("z"; "X")"#, r#""a1b""#),
        (r#""a1b" | sub("a"; "X"; "g")"#, r#""1b""#),
        (r#""a1b" | sub("a"; .; "g")"#, r#""1b""#),
        (r#""a1b" | sub("a"; error("boom"); "g")"#, r#""1b""#),
        (r#""a1b" | sub("a"; sub("1"; "ZZ"))"#, r#""aZZb1b""#),
        (r#""a1b" | sub("a"; sub("1"; .))"#, r#""aa1bb1b""#),
        (r#""a1b" | sub("a"; split("1") | .[0])"#, r#""a1b""#),
        // A name is letters (category L), digits (Nd) and `_`: `²` and a Roman numeral end it.
        (r#""ab" | sub("(a)"; "$1²")"#, r#""a²b""#),
        (r#""xbx" | sub("b"; "$ⅷ")"#, r#""x$ⅷx""#),
        (r#""abab" | sub("(b)"; "$é")"#, r#""aa""#),
        (r#""abab" | sub("(b)"; "${é}")"#, r#""aa""#),
        (r#""abab" | sub("(b)"; "$1é")"#, r#""aa""#),
        (r#""abab" | sub("(b)"; "$2")"#, r#""aa""#),
        (r#""abab" | sub("(b)(c)?"; "[$2]")"#, r#""a[]a[]""#),
        (r#""abab" | sub("(b)"; "$٣")"#, r#""aa""#),
        (r#""abab" | sub("(?P<n>b)"; "$nⅧ")"#, r#""abⅧabⅧ""#),
        (r#""abab" | sub("(?P<n>b)"; "${n}²")"#, r#""ab²ab²""#),
    ];
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, "null", &["-o=json", "-I=0"])?;
        assert_eq!(code, 0, "filter {filter:?}: {out:?}");
        assert_eq!(out.trim(), *want, "filter {filter:?}");
    }
    Ok(())
}

/// #4905: the same over a YAML document, where `.` in the replacement is the node `sub` is applied to
/// (`.v |= sub("a"; .)`), and an absent key (`.w`) reads as the empty string.
#[test]
fn test_4905_yq_sub_replacement_over_a_document() -> Result<()> {
    let doc = r"v: a1b
w: WW
n: 5
nul: null
";
    let rows: &[(&str, &str)] = &[
        (r#".v | sub("a"; .)"#, r#""a1b1b""#),
        (
            r#".v |= sub("a"; .)"#,
            r#"{"v":"a1b1b","w":"WW","n":5,"nul":null}"#,
        ),
        (
            r#".v |= sub("a"; .w)"#,
            r#"{"v":"1b","w":"WW","n":5,"nul":null}"#,
        ),
        (r#".v | sub("a"; .w)"#, r#""1b""#),
        (r#".v | sub("a"; "{\(.)}")"#, r#""{a1b}1b""#),
        (r#".v | sub("a"; "X" + .)"#, r#""Xa1b1b""#),
        (r#".v | sub("a"; . + .)"#, r#""a1ba1b1b""#),
        (r#".v | sub("a"; upcase)"#, r#""A1B1b""#),
        (r#".v | sub("a"; @base64)"#, r#""YTFi1b""#),
        (r#".w | sub("W"; .)"#, r#""WWWW""#),
        (
            r#".[] |= (select(tag == "!!str") | sub("W"; .))"#,
            r#"{"v":"a1b","w":"WWWW","n":5,"nul":null}"#,
        ),
    ];
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, doc, &["-o=json", "-I=0"])?;
        assert_eq!(code, 0, "filter {filter:?}: {out:?}");
        assert_eq!(out.trim(), *want, "filter {filter:?}");
    }
    Ok(())
}

/// #4905: the replacement is evaluated even when nothing matches, an escape anywhere in it raises, and
/// it is evaluated after the pattern and before the input is type-checked or the regex compiled. Rows
/// are `(filter, document, the message yq prints after "Error: ")`.
#[test]
fn test_4905_yq_sub_replacement_errors_and_order() -> Result<()> {
    let rows: &[(&str, &str, &str)] = &[
        (r#""a1b" | sub("z"; error("boom"))"#, r"null", r"boom"),
        (r#""a1b" | sub("a"; error("boom"))"#, r"null", r"boom"),
        (r#""a" | sub("a"; ("X", error("r")))"#, r"null", r"r"),
        (r#""a" | sub("z"; ("X", error("r")))"#, r"null", r"r"),
        (r#""a" | sub("a"; (error("r"), "X"))"#, r"null", r"r"),
        (r#"5 | sub("a"; error("r"))"#, r"null", r"r"),
        (r#""a" | sub(error("p"); error("r"))"#, r"null", r"p"),
        (r#"5 | sub(error("p"); "X")"#, r"null", r"p"),
        (
            r#".n | sub("a"; error("boom"))"#,
            r"v: a1b
w: WW
n: 5
nul: null
",
            r"boom",
        ),
        (
            r#".nul | sub("a"; error("boom"))"#,
            r"v: a1b
w: WW
n: 5
nul: null
",
            r"boom",
        ),
        (
            r#".v | sub("("; error("boom"))"#,
            r"v: a1b
w: WW
n: 5
nul: null
",
            r"boom",
        ),
    ];
    for (filter, doc, message) in rows {
        let (out, err, code) = run_yq_stdin_with_stderr(filter, doc, &["-o=json", "-I=0"])?;
        assert_ne!(code, 0, "filter {filter:?}: {out:?}");
        assert_eq!(out, "", "filter {filter:?}");
        assert!(
            err.contains(message),
            "filter {filter:?}: stderr {err:?} lacks {message:?}"
        );
    }
    Ok(())
}
