//! #4921: jq mode never sees the `+` or the zero padding of a YAML integer past `i64`.

use std::io::Write;

use anyhow::Result;
use tempfile::NamedTempFile;

use crate::run_jq_full;

/// #4921: yq keeps `+99999999999999999999` verbatim and the yq-mode reindex bridge now keeps its `+`
/// too, but `succinctly jq` follows jq, which stores `+X` as `X` (#2877) and a padded run as its
/// value: every spelling `load()`s from YAML as the same literal, however the loaded value is
/// walked, rebuilt or printed. Each row is the plain `99999999999999999999` jq 1.7.1 prints.
#[test]
fn test_4921_jq_load_of_yaml_overflowed_int_spellings_prints_the_bare_digits() -> Result<()> {
    let mut file = NamedTempFile::new()?;
    file.write_all(b"a: +99999999999999999999\nb: 0099999999999999999999\nc: !!float 99999999999999999999\nd: 99999999999999999999\n")?;
    file.flush()?;
    let path = file.path().to_str().unwrap();

    let digits = "99999999999999999999";
    for (suffix, want) in [
        (
            "",
            format!(r#"{{"a":{digits},"b":{digits},"c":{digits},"d":{digits}}}"#),
        ),
        (
            " | [.[] | tostring]",
            format!(r#"["{digits}","{digits}","{digits}","{digits}"]"#),
        ),
        (
            " | tojson",
            format!(r#""{{\"a\":{digits},\"b\":{digits},\"c\":{digits},\"d\":{digits}}}""#),
        ),
        (
            " | with_entries(.value |= .)",
            format!(r#"{{"a":{digits},"b":{digits},"c":{digits},"d":{digits}}}"#),
        ),
        (
            " | reduce .[] as $x ([]; . + [$x])",
            format!("[{digits},{digits},{digits},{digits}]"),
        ),
        (" | [.[]] | unique", format!("[{digits}]")),
    ] {
        let expr = format!("load({path:?}){suffix}");
        let (out, err, code) = run_jq_full(&["-cn", &expr], None)?;
        assert_eq!(code, 0, "{suffix}: err={err}");
        assert_eq!(out.trim(), want, "{suffix}");
    }
    Ok(())
}
