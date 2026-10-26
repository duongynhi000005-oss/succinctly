//! #1791: in yq a `{}` *element* of the array `join` joins is the empty string when it is a node of
//! the document -- `- {}` / `- x` joins as `,x`, and so does one reached through `$x`, `[.[0]]` or
//! a `del` -- and is the text `{}` only when the expression itself spelled `{}`
//! (`[{}] | join(",")`). This evaluator cannot tell the two apart (a constructed array reaches
//! `join` the way a document one does), so it follows the document case, the common one; the
//! literal element is recorded on the limitations page as not matched. Every answer was captured
//! from yq v4.53.3.

use anyhow::Result;

use crate::run_yq_stdin;

#[test]
fn test_a_document_empty_mapping_element_joins_as_empty_1791() -> Result<()> {
    for (input, filter, want) in [
        ("- {}\n- x\n", "join(\",\")", ",x"),
        ("- {}\n", "join(\",\")", ""),
        ("- {}\n- {}\n", "join(\",\")", ","),
        ("- {}\n- b\n- {}\n", "join(\"-\")", "-b-"),
        ("- {\"a\":1}\n- {}\n", "join(\",\")", ","),
        ("- [{}]\n- x\n", "join(\",\")", ",x"),
        ("- {}\n- x\n", ". as $d | $d | join(\",\")", ",x"),
    ] {
        let (out, code) = run_yq_stdin(filter, input, &["-r"])?;
        assert_eq!((out.trim_end(), code), (want, 0), "{filter} on {input:?}");
    }
    Ok(())
}

/// The separator is an evaluated expression, so the `{}` literal there keeps its text.
#[test]
fn test_an_empty_mapping_separator_still_joins_as_text_1791() -> Result<()> {
    let (out, code) = run_yq_stdin("[1,2] | join({})", "", &["-n", "-r"])?;
    assert_eq!((out.trim_end(), code), ("1{}2", 0));
    Ok(())
}
