//! #5152: `$ARGS` is `{"positional": .., "named": ..}`, in that key order, as in jq.

use anyhow::Result;

use crate::run_jq_full;

/// #5152: `$ARGS` printed whole, its keys in order, as JSON text and as entries, over `--arg`, `--argjson`, `--args` and
/// `--jsonargs`. Every expected output captured from `/usr/bin/jq` 1.7.1.
#[test]
fn test_5152_args_lists_positional_before_named() -> Result<()> {
    let rows: &[(&[&str], &str)] = &[
        (&[r"-nc", r"$ARGS"], "{\"positional\":[],\"named\":{}}\n"),
        (
            &[
                r"-nc", r"--arg", r"a", r"1", r"$ARGS", r"--args", r"x", r"y",
            ],
            "{\"positional\":[\"x\",\"y\"],\"named\":{\"a\":\"1\"}}\n",
        ),
        (
            &[r"-nc", r"$ARGS|keys_unsorted"],
            "[\"positional\",\"named\"]\n",
        ),
        (
            &[
                r"-nc",
                r"--arg",
                r"b",
                r"2",
                r"--arg",
                r"a",
                r"1",
                r"$ARGS|tojson",
            ],
            "\"{\\\"positional\\\":[],\\\"named\\\":{\\\"b\\\":\\\"2\\\",\\\"a\\\":\\\"1\\\"}}\"\n",
        ),
        (
            &[
                r"-nc",
                r"$ARGS|to_entries|map(.key)",
                r"--jsonargs",
                r"1",
                r#""z""#,
            ],
            "[\"positional\",\"named\"]\n",
        ),
        (
            &[r"-n", r"--tab", r"--arg", r"a", r"1", r"$ARGS"],
            "{\n\t\"positional\": [],\n\t\"named\": {\n\t\t\"a\": \"1\"\n\t}\n}\n",
        ),
    ];
    for &(args, expected) in rows {
        let (stdout, stderr, code) = run_jq_full(args, None)?;
        assert_eq!(
            (stdout.as_str(), code),
            (expected, 0),
            "jq {args:?}: stderr {stderr:?}"
        );
    }
    Ok(())
}
