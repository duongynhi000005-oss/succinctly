//! #5312: jq's parser skips one UTF-8 byte-order mark at the front of the text it is handed, so
//! `fromjson` and `tonumber` (which hand their string to it) answer through a BOM. Expected
//! values and messages captured live against jq 1.7.1 (`/usr/bin/jq`).

use anyhow::Result;

use crate::run_jq_full;

const BOM: &str = "\u{feff}";

fn run(filter: &str) -> Result<(String, String, i32)> {
    run_jq_full(&["-nc", filter], None)
}

#[test]
fn test_fromjson_and_tonumber_skip_one_leading_bom_5312() -> Result<()> {
    for (filter, want) in [
        (r#""﻿[1]" | fromjson"#, "[1]\n"),
        (r#""﻿{\"a\":1}" | fromjson | .a"#, "1\n"),
        // The BOM comes off first; whitespace after it is the parser's ordinary whitespace.
        (r#""﻿ 1 " | fromjson"#, "1\n"),
        (r#""﻿1" | tonumber"#, "1\n"),
        (r#""﻿ 2.50 " | tonumber"#, "2.50\n"),
        (r#""﻿[1]" | try fromjson catch "bad""#, "[1]\n"),
    ] {
        let (out, err, code) = run(filter)?;
        assert_eq!((out.as_str(), code), (want, 0), "{filter}: {err}");
    }
    Ok(())
}

#[test]
fn test_a_second_bom_or_one_behind_whitespace_is_refused_5312() -> Result<()> {
    for (filter, message) in [
        // The column counts what follows the first BOM; the text is quoted whole.
        (
            r#""﻿﻿1" | fromjson"#,
            format!(
                "Invalid numeric literal at EOF at line 1, column 4 (while parsing '{BOM}{BOM}1')"
            ),
        ),
        (
            r#"" ﻿1" | fromjson"#,
            format!("Invalid numeric literal at EOF at line 1, column 5 (while parsing ' {BOM}1')"),
        ),
        (
            r#""﻿" | fromjson"#,
            format!("Expected JSON value (while parsing '{BOM}')"),
        ),
    ] {
        let (out, err, code) = run(filter)?;
        assert_eq!((out.as_str(), code), ("", 5), "{filter}");
        assert!(err.contains(&message), "{filter}: {err}");
    }
    let (out, _, code) = run(r#"" ﻿1" | try tonumber catch "no""#)?;
    assert_eq!((out.as_str(), code), ("\"no\"\n", 0));
    // A BOM-prefixed string that is JSON but not a number is still named as the string it is.
    let (out, _, code) = run(r#""﻿null" | tonumber? // "x""#)?;
    assert_eq!((out.as_str(), code), ("\"x\"\n", 0));
    Ok(())
}
