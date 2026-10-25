//! #5245 is jq's NaN index key; yq has no NaN path component, so its write keeps refusing at resolution.

use anyhow::Result;

use crate::run_yq_stdin_with_stderr;

#[test]
fn test_yq_nan_index_key_still_refuses_the_write_5245() -> Result<()> {
    let rows: &[(&str, &[&str])] = &[
        (".[0/0] = 5", &[]),
        ("(0/0) as $v | .[$v] = 5", &[]),
        (".[0/0]? = 5", &[]),
        (".[0/0] |= 5", &[]),
        // `getpath` and a variable key in `path()` are `--jq-extensions` names in yq.
        ("path(getpath([0/0]))", &["--jq-extensions"]),
        ("(0/0) as $v | path(.[$v])", &["--jq-extensions"]),
    ];
    for (filter, args) in rows {
        let (_, stderr, code) = run_yq_stdin_with_stderr(filter, "[1, 2]\n", args)?;
        assert_ne!(code, 0, "{filter}");
        assert!(
            stderr.contains("Cannot set array element at NaN index"),
            "{filter}: {stderr}"
        );
    }
    Ok(())
}
