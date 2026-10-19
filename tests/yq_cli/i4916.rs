//! #4916: a yq string literal keeps every backslash sequence it does not decode.

use anyhow::Result;

use crate::{run_jq_stdin, run_yq_stdin};

/// #4916: real yq's lexer decodes `\n \t \r \b \f \a \v \" \\` in a string literal and keeps any other
/// backslash sequence as written, the backslash included. `"a1" | test("\d")` therefore reaches the
/// regex engine as `\d`, `"\u0041"` is six characters (not `A`) and `"\/"` stays `\/`; `succinctly yq`
/// read the literal as jq does and decoded `\u` and `\/` and failed to parse the rest. Every row is
/// captured from yq v4.53.3 (`-n -o=json -I=0`): each ASCII punctuation, letter and digit after a
/// backslash, `\u` forms (short, non-hex, a surrogate pair), a multi-byte character, an interpolation
/// beside an escape, object keys, path steps and regex builtins.
#[test]
fn test_4916_yq_string_literal_keeps_unknown_backslash_escapes() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        (r#""a\ b""#, r#""a\\ b""#),
        (r#""a\!b""#, r#""a\\!b""#),
        (r#""a\#b""#, r#""a\\#b""#),
        (r#""a\$b""#, r#""a\\$b""#),
        (r#""a\%b""#, r#""a\\%b""#),
        (r#""a\&b""#, r#""a\\&b""#),
        (r#""a\'b""#, r#""a\\'b""#),
        (r#""a\)b""#, r#""a\\)b""#),
        (r#""a\*b""#, r#""a\\*b""#),
        (r#""a\+b""#, r#""a\\+b""#),
        (r#""a\,b""#, r#""a\\,b""#),
        (r#""a\-b""#, r#""a\\-b""#),
        (r#""a\.b""#, r#""a\\.b""#),
        (r#""a\/b""#, r#""a\\/b""#),
        (r#""a\0b""#, r#""a\\0b""#),
        (r#""a\1b""#, r#""a\\1b""#),
        (r#""a\2b""#, r#""a\\2b""#),
        (r#""a\3b""#, r#""a\\3b""#),
        (r#""a\4b""#, r#""a\\4b""#),
        (r#""a\5b""#, r#""a\\5b""#),
        (r#""a\6b""#, r#""a\\6b""#),
        (r#""a\7b""#, r#""a\\7b""#),
        (r#""a\8b""#, r#""a\\8b""#),
        (r#""a\9b""#, r#""a\\9b""#),
        (r#""a\:b""#, r#""a\\:b""#),
        (r#""a\;b""#, r#""a\\;b""#),
        (r#""a\<b""#, r#""a\\<b""#),
        (r#""a\=b""#, r#""a\\=b""#),
        (r#""a\>b""#, r#""a\\>b""#),
        (r#""a\?b""#, r#""a\\?b""#),
        (r#""a\@b""#, r#""a\\@b""#),
        (r#""a\Ab""#, r#""a\\Ab""#),
        (r#""a\Bb""#, r#""a\\Bb""#),
        (r#""a\Cb""#, r#""a\\Cb""#),
        (r#""a\Db""#, r#""a\\Db""#),
        (r#""a\Eb""#, r#""a\\Eb""#),
        (r#""a\Fb""#, r#""a\\Fb""#),
        (r#""a\Gb""#, r#""a\\Gb""#),
        (r#""a\Hb""#, r#""a\\Hb""#),
        (r#""a\Ib""#, r#""a\\Ib""#),
        (r#""a\Jb""#, r#""a\\Jb""#),
        (r#""a\Kb""#, r#""a\\Kb""#),
        (r#""a\Lb""#, r#""a\\Lb""#),
        (r#""a\Mb""#, r#""a\\Mb""#),
        (r#""a\Nb""#, r#""a\\Nb""#),
        (r#""a\Ob""#, r#""a\\Ob""#),
        (r#""a\Pb""#, r#""a\\Pb""#),
        (r#""a\Qb""#, r#""a\\Qb""#),
        (r#""a\Rb""#, r#""a\\Rb""#),
        (r#""a\Sb""#, r#""a\\Sb""#),
        (r#""a\Tb""#, r#""a\\Tb""#),
        (r#""a\Ub""#, r#""a\\Ub""#),
        (r#""a\Vb""#, r#""a\\Vb""#),
        (r#""a\Wb""#, r#""a\\Wb""#),
        (r#""a\Xb""#, r#""a\\Xb""#),
        (r#""a\Yb""#, r#""a\\Yb""#),
        (r#""a\Zb""#, r#""a\\Zb""#),
        (r#""a\[b""#, r#""a\\[b""#),
        (r#""a\\b""#, r#""a\\b""#),
        (r#""a\]b""#, r#""a\\]b""#),
        (r#""a\^b""#, r#""a\\^b""#),
        (r#""a\_b""#, r#""a\\_b""#),
        (r#""a\`b""#, r#""a\\`b""#),
        (r#""a\ab""#, r#""a\u0007b""#),
        (r#""a\bb""#, r#""a\u0008b""#),
        (r#""a\cb""#, r#""a\\cb""#),
        (r#""a\db""#, r#""a\\db""#),
        (r#""a\eb""#, r#""a\\eb""#),
        (r#""a\fb""#, r#""a\u000cb""#),
        (r#""a\gb""#, r#""a\\gb""#),
        (r#""a\hb""#, r#""a\\hb""#),
        (r#""a\ib""#, r#""a\\ib""#),
        (r#""a\jb""#, r#""a\\jb""#),
        (r#""a\kb""#, r#""a\\kb""#),
        (r#""a\lb""#, r#""a\\lb""#),
        (r#""a\mb""#, r#""a\\mb""#),
        (r#""a\nb""#, r#""a\nb""#),
        (r#""a\ob""#, r#""a\\ob""#),
        (r#""a\pb""#, r#""a\\pb""#),
        (r#""a\qb""#, r#""a\\qb""#),
        (r#""a\rb""#, r#""a\rb""#),
        (r#""a\sb""#, r#""a\\sb""#),
        (r#""a\tb""#, r#""a\tb""#),
        (r#""a\ub""#, r#""a\\ub""#),
        (r#""a\vb""#, r#""a\u000bb""#),
        (r#""a\wb""#, r#""a\\wb""#),
        (r#""a\xb""#, r#""a\\xb""#),
        (r#""a\yb""#, r#""a\\yb""#),
        (r#""a\zb""#, r#""a\\zb""#),
        (r#""a\{b""#, r#""a\\{b""#),
        (r#""a\|b""#, r#""a\\|b""#),
        (r#""a\}b""#, r#""a\\}b""#),
        (r#""a\~b""#, r#""a\\~b""#),
        (r#""a\"b""#, r#""a\"b""#),
        (r#""a\\b""#, r#""a\\b""#),
        (r#""a\u0041b""#, r#""a\\u0041b""#),
        (r#""\u00e9""#, r#""\\u00e9""#),
        (r#""\ud83d\ude00""#, r#""\\ud83d\\ude00""#),
        (r#""\u""#, r#""\\u""#),
        (r#""\u00""#, r#""\\u00""#),
        (r#""\uZZZZ""#, r#""\\uZZZZ""#),
        (r#""\u12""#, r#""\\u12""#),
        (r#""\U00000041""#, r#""\\U00000041""#),
        (r#""\ud83d""#, r#""\\ud83d""#),
        (r#""a\é""#, r#""a\\é""#),
        (r#""€\q""#, r#""€\\q""#),
        (r#""x\(1+2)y\q""#, r#""x3y\\q""#),
        (r#""\q\(1)""#, r#""\\q1""#),
        (r#""\(1)\u0041""#, r#""1\\u0041""#),
        (r#"{"a\q": 1} | keys"#, r#"["a\\q"]"#),
        (r#"{"a\u0041": 1} | keys"#, r#"["a\\u0041"]"#),
        (r#".["a\q"]"#, r"null"),
        (r#"."a\q""#, r"null"),
        (r#".a."b\q""#, r"null"),
        (r#""a\q" | @base64"#, r#""YVxx""#),
        (r#""a1" | test("\d")"#, r"true"),
        (r#""a1" | sub("\d"; "X")"#, r#""aX""#),
        (r#""a1" | sub("\d"; "\q")"#, r#""a\\q""#),
        (r#""a-b" | split("\-")"#, r#"["a-b"]"#),
        (r#""a1" | capture("(?<n>\d)")"#, r#"{"n":"1"}"#),
        (r#""a.b" | test("a\.b")"#, r"true"),
        (r#""a b" | test("a\sb")"#, r"true"),
        (r#"["a\qb"] | join("\\")"#, r#""a\\qb""#),
    ];
    for (filter, want) in rows {
        let (out, code) = run_yq_stdin(filter, "", &["-n", "-o=json", "-I=0"])?;
        assert_eq!(code, 0, "filter {filter:?}: {out:?}");
        assert_eq!(out.trim(), *want, "filter {filter:?}");
    }
    Ok(())
}

/// #4916: jq mode keeps its own table (`\u` and `\/` decode, every other unknown escape is an error).
#[test]
fn test_4916_jq_string_literal_escapes_are_unchanged() -> Result<()> {
    for (filter, want) in [(r#""a\u0041b""#, r#""aAb""#), (r#""a\/b""#, r#""a/b""#)] {
        let (out, code) = run_jq_stdin(filter, "null", &["-c"])?;
        assert_eq!(code, 0, "filter {filter:?}: {out:?}");
        assert_eq!(out.trim(), want, "filter {filter:?}");
    }
    let (out, code) = run_jq_stdin(r#""a\qb""#, "null", &["-c"])?;
    assert_ne!(code, 0, "{out:?}");
    Ok(())
}
