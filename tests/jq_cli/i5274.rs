//! #5274: a fold whose UPDATE is a slice (`.[1:]`) cuts the owned value instead of writing it out as
//! JSON and indexing it again on every step. These pin the answers jq 1.7.1 gives (captured live),
//! including that a value another binding still holds is not written through.

use anyhow::Result;

use crate::run_jq_full;

#[test]
fn test_slice_folds_answer_as_jq_5274() -> Result<()> {
    for (input, filter, want) in [
        (
            "[1,2,3,4,5,6]",
            "reduce range(3) as $i (.; .[1:])",
            "[4,5,6]",
        ),
        (
            "[1,2,3,4,5,6]",
            "reduce range(4) as $i (.; .[:-1])",
            "[1,2]",
        ),
        (
            "[1,2,3,4,5,6]",
            "reduce range(2) as $i (.; .[1:-1])",
            "[3,4]",
        ),
        (
            "[1,2,3,4,5,6]",
            "[foreach range(3) as $i (.; .[1:]; .)]",
            "[[2,3,4,5,6],[3,4,5,6],[4,5,6]]",
        ),
        (
            "[1,2,3,4,5,6]",
            ". as $a | reduce range(3) as $i ($a; .[1:]) | [., $a]",
            "[[4,5,6],[1,2,3,4,5,6]]",
        ),
        ("[1,2,3,4,5,6]", "[. | until(length < 3; .[1:])]", "[[5,6]]"),
        (
            "[1,2,3,4,5,6]",
            "[. | while(length > 0; .[1:])]",
            "[[1,2,3,4,5,6],[2,3,4,5,6],[3,4,5,6],[4,5,6],[5,6],[6]]",
        ),
        ("[1,2,3,4,5,6]", "reduce range(10) as $i (.; .[2:])", "[]"),
        (
            r#""héllo wörld 😀""#,
            "reduce range(3) as $i (.; .[1:])",
            r#""lo wörld 😀""#,
        ),
        (
            r#""héllo wörld 😀""#,
            "reduce range(2) as $i (.; .[:-1])",
            r#""héllo wörld""#,
        ),
        ("null", "reduce range(2) as $i (.; .[1:])", "null"),
        ("[[1],[2],[3]]", "reduce range(2) as $i (.; .[1:])", "[[3]]"),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-c", filter], Some(input))?;
        assert_eq!(
            (stdout.trim_end(), code),
            (want, 0),
            "`{filter}` on {input}: {stderr}"
        );
    }
    Ok(())
}
