//! #1998: `sort`, `sort_by`, `reverse` and `shuffle` on a non-array raise yq's own
//! `node at path [] is not an array (or map) (it's a !!tag)`. The path is fixed at `[]` (yq prints the node's
//! lineage after a navigation, which is recorded in `docs/compliance/yq/limitations.md`), so only
//! root and literal inputs are pinned here. Each row is a live yq v4.53.3 capture.

use anyhow::Result;

use crate::run_yq_stdin_with_stderr;

#[test]
fn test_non_array_sort_family_wording_matches_yq_1998() -> Result<()> {
    let rows: &[(&str, &str)] = &[
        (
            "5 | sort",
            "node at path [] is not an array or map (it's a !!int)",
        ),
        (
            "1.5 | sort_by(.)",
            "node at path [] is not an array or map (it's a !!float)",
        ),
        (
            "true | sort",
            "node at path [] is not an array or map (it's a !!bool)",
        ),
        (
            r#""s" | sort_by(.)"#,
            "node at path [] is not an array or map (it's a !!str)",
        ),
        (
            "null | reverse",
            "node at path [] is not an array (it's a !!null)",
        ),
        (
            r#""s" | reverse"#,
            "node at path [] is not an array (it's a !!str)",
        ),
        (
            "{} | reverse",
            "node at path [] is not an array (it's a !!map)",
        ),
        (
            "5 | shuffle",
            "node at path [] is not an array (it's a !!int)",
        ),
    ];
    for (filter, message) in rows {
        let (stdout, stderr, code) = run_yq_stdin_with_stderr(filter, "a: 1\n", &[])?;
        assert_eq!((stdout.as_str(), code), ("", 1), "{filter}");
        assert_eq!(
            stderr.lines().next(),
            Some(format!("Error: {message}").as_str()),
            "{filter}"
        );
    }
    Ok(())
}
