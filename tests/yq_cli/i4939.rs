//! #4939: `sort_by` over a root array of block records ran in time quadratic in the record count
//! because the U+2028/U+2029 prefilter (#4865) scanned the whole document buffer from each
//! scalar to its end. The cost itself is pinned by the unit test beside
//! `YamlString::may_hold_unicode_line_separator`; this pins the output the filter feeds.

use anyhow::Result;

use crate::run_yq_stdin;

/// A separator in a later record must not leak into an earlier one, and must still be spelled the
/// way yq spells it in the record that holds it.
#[test]
fn test_4939_sort_by_block_records_keep_the_separator_with_its_own_record() -> Result<()> {
    let input =
        "- id: 3\n  name: 'x\u{2028}y'\n- id: 1\n  name: plain\n- id: 2\n  name: 'z\u{2029}w'\n";
    let (out, code) = run_yq_stdin("sort_by(.id)", input, &[])?;
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "- id: 1\n  name: plain\n- id: 2\n  name: 'z\u{2029}    w'\n- id: 3\n  name: 'x\u{2028}    y'\n"
    );
    Ok(())
}
