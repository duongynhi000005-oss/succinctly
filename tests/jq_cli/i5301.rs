//! #5301: `strftime`, `strflocaltime`, `todate` and `mktime` read all 8 fields of a broken-down time
//! (jq's `jv2tm`), and raise `<name> requires parsed datetime inputs` when one is missing or is not
//! a number -- where a string element used to read as 0 and a short `mktime` array was accepted.
//! Elements past the 8th are ignored. Every answer below was captured from jq 1.7.1.

use anyhow::Result;

use crate::run_jq_stdin;

#[test]
fn test_broken_down_time_needs_eight_number_fields_5301() -> Result<()> {
    for (filter, input, want) in [
        (
            "try strftime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[\"a\",1,2,3,4,5,6,7]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strftime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,\"b\",2,3,4,5,6,7]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strftime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,5,6,null]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strftime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,5,6,\"x\"]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strftime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,\"s\",6,7]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strftime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[true,1,2,3,4,5,6,7]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strftime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[[2020],1,2,3,4,5,6,7]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strftime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,5,6]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strftime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,5]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strftime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strftime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,5,6,7]",
            "\"2020-02-02T03:04:05Z\"\n",
        ),
        (
            "try strftime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,5,6,7.5,\"extra\"]",
            "\"2020-02-02T03:04:05Z\"\n",
        ),
        (
            "try strflocaltime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[\"a\",1,2,3,4,5,6,7]",
            "\"strflocaltime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strflocaltime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,\"b\",2,3,4,5,6,7]",
            "\"strflocaltime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strflocaltime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,5,6,null]",
            "\"strflocaltime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strflocaltime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,5,6,\"x\"]",
            "\"strflocaltime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strflocaltime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,\"s\",6,7]",
            "\"strflocaltime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strflocaltime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[true,1,2,3,4,5,6,7]",
            "\"strflocaltime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strflocaltime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[[2020],1,2,3,4,5,6,7]",
            "\"strflocaltime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strflocaltime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,5,6]",
            "\"strflocaltime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strflocaltime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,5]",
            "\"strflocaltime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strflocaltime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[]",
            "\"strflocaltime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try strflocaltime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,5,6,7]",
            "\"2020-02-02T03:04:05Z\"\n",
        ),
        (
            "try strflocaltime(\"%Y-%m-%dT%H:%M:%SZ\") catch .",
            "[2020,1,2,3,4,5,6,7.5,\"extra\"]",
            "\"2020-02-02T03:04:05Z\"\n",
        ),
        (
            "try todate catch .",
            "[\"a\",1,2,3,4,5,6,7]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try todate catch .",
            "[2020,\"b\",2,3,4,5,6,7]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try todate catch .",
            "[2020,1,2,3,4,5,6,null]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try todate catch .",
            "[2020,1,2,3,4,5,6,\"x\"]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try todate catch .",
            "[2020,1,2,3,4,\"s\",6,7]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try todate catch .",
            "[true,1,2,3,4,5,6,7]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try todate catch .",
            "[[2020],1,2,3,4,5,6,7]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try todate catch .",
            "[2020,1,2,3,4,5,6]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try todate catch .",
            "[2020,1,2,3,4,5]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try todate catch .",
            "[]",
            "\"strftime/1 requires parsed datetime inputs\"\n",
        ),
        (
            "try todate catch .",
            "[2020,1,2,3,4,5,6,7]",
            "\"2020-02-02T03:04:05Z\"\n",
        ),
        (
            "try todate catch .",
            "[2020,1,2,3,4,5,6,7.5,\"extra\"]",
            "\"2020-02-02T03:04:05Z\"\n",
        ),
        (
            "try mktime catch .",
            "[\"a\",1,2,3,4,5,6,7]",
            "\"mktime requires parsed datetime inputs\"\n",
        ),
        (
            "try mktime catch .",
            "[2020,\"b\",2,3,4,5,6,7]",
            "\"mktime requires parsed datetime inputs\"\n",
        ),
        (
            "try mktime catch .",
            "[2020,1,2,3,4,5,6,null]",
            "\"mktime requires parsed datetime inputs\"\n",
        ),
        (
            "try mktime catch .",
            "[2020,1,2,3,4,5,6,\"x\"]",
            "\"mktime requires parsed datetime inputs\"\n",
        ),
        (
            "try mktime catch .",
            "[2020,1,2,3,4,\"s\",6,7]",
            "\"mktime requires parsed datetime inputs\"\n",
        ),
        (
            "try mktime catch .",
            "[true,1,2,3,4,5,6,7]",
            "\"mktime requires parsed datetime inputs\"\n",
        ),
        (
            "try mktime catch .",
            "[[2020],1,2,3,4,5,6,7]",
            "\"mktime requires parsed datetime inputs\"\n",
        ),
        (
            "try mktime catch .",
            "[2020,1,2,3,4,5,6]",
            "\"mktime requires parsed datetime inputs\"\n",
        ),
        (
            "try mktime catch .",
            "[2020,1,2,3,4,5]",
            "\"mktime requires parsed datetime inputs\"\n",
        ),
        (
            "try mktime catch .",
            "[]",
            "\"mktime requires parsed datetime inputs\"\n",
        ),
        ("try mktime catch .", "[2020,1,2,3,4,5,6,7]", "1580612645\n"),
        (
            "try mktime catch .",
            "[2020,1,2,3,4,5,6,7.5,\"extra\"]",
            "1580612645\n",
        ),
    ] {
        let (out, code) = run_jq_stdin(filter, input, &["-c"])?;
        assert_eq!((out.as_str(), code), (want, 0), "{filter} on {input}");
    }
    Ok(())
}
