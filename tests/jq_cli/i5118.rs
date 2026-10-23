//! #5118: a slice past offset 65,535 of a value that is itself a view of a longer array.

use anyhow::Result;

use crate::run_jq_full;

// jq's slice is a view of the array's buffer, with a 16-bit offset: from 65,536 it copies. The resolver records every node
// below the root at offset 0, so `[range(70003)] | .[60000:] | del(.[6000:] as $x | .[6000:] | $x)` deleted through a
// slice jq refuses. Once a slice has been resolved against a container of more than 65,535 elements, every slice that
// starts past 0 is read as possibly copied: loud where jq refuses, and over-refusing the views jq still shares.

/// #5118: the rows jq refuses and `main` answered (a view whose offset reaches 65,536 through a second slice, below a field,
/// after a write and after `+`). Each must refuse, as jq does (exit 5, no output).
#[test]
fn test_5118_a_slice_past_65535_of_a_view_is_refused_as_jq_refuses() -> Result<()> {
    for filter in [
        "[range(70003)] | .[60000:] | del(.[6000:] as $x | .[6000:] | $x) | length",
        "[range(70003)] | .[60000:] | del(.[5536:] as $x | .[5536:] | $x) | length",
        "{a: ([range(70003)] | .[60000:])} | del(.a[6000:] as $x | .a[6000:] | $x) | .a | length",
        "[range(70003)] | .[60000:] | .[0] = 0 | del(.[6000:] as $x | .[6000:] | $x) | length",
        "[range(70003)] | .[60000:] | . + [] | del(.[6000:] as $x | .[6000:] | $x) | length",
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-nc", filter], None)?;
        assert!(
            code == 5 && stdout.is_empty() && stderr.contains("Invalid path expression"),
            "`{filter}`: jq refuses, got {stdout:?} {stderr:?} exit {code}"
        );
    }
    Ok(())
}

/// #5118: small arrays are untouched by the gate, so the rows that match jq keep matching (`[1,2,3]`, and a slice of a
/// 70,003-element array that stays below the 65,536 offset answers as jq does or refuses).
#[test]
fn test_5118_small_arrays_still_answer_as_jq_does() -> Result<()> {
    let (stdout, stderr, code) =
        run_jq_full(&["-nc", "[1,2,3] | del(.[1:] as $x | .[1:] | $x)"], None)?;
    assert_eq!((stdout.as_str(), code), ("[1]\n", 0), "{stderr}");
    for filter in [
        "[range(70003)] | del(.[6000:] as $x | .[6000:] | $x) | length",
        "[range(70003)] | .[60000:] | del(.[5535:] as $x | .[5535:] | $x) | length",
    ] {
        // jq answers 6000 and 5535; the gate over-refuses them (loud) rather than guess a view's offset.
        let (stdout, stderr, code) = run_jq_full(&["-nc", filter], None)?;
        assert!(
            (code == 0 && (stdout == "6000\n" || stdout == "5535\n"))
                || (code == 5 && stdout.is_empty()),
            "`{filter}`: {stdout:?} {stderr:?} exit {code}"
        );
    }
    Ok(())
}
