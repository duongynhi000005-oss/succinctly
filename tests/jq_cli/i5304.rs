//! #5304: `builtins` answers jq 1.7.1's roster, `name/arity` for each of its 218 builtins and no
//! other. It listed 187 names: it lacked ~50 callable jq builtins (`match/2`, `capture/1`, `JOIN/4`,
//! the libm family, `format/1`, ...) and listed names jq does not have (`trim/0`, `leaf_paths/0`,
//! `reduce/3`, the yq metadata getters, ...). The roster is `tests/data/jq-builtin-names.txt`,
//! captured from the pinned jq by `scripts/sync-jq-builtin-names.sh`.

use anyhow::Result;

use crate::run_jq_full;

/// The pinned jq's roster, one `name/arity` per line, in the order jq prints it.
fn roster() -> Vec<&'static str> {
    include_str!("../data/jq-builtin-names.txt")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .collect()
}

#[test]
fn test_builtins_is_jqs_roster_5304() -> Result<()> {
    let (stdout, stderr, code) = run_jq_full(&["-n", "-c", "builtins"], None)?;
    assert_eq!(code, 0, "{stderr}");
    let want = format!(
        "[{}]\n",
        roster()
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_eq!(stdout, want);
    assert_eq!(roster().len(), 218);
    Ok(())
}

/// The order is jq's own too, so `builtins[0]`, `first(builtins)` and a program that never sorts
/// see what jq shows; sorted, it is the roster sorted.
#[test]
fn test_builtins_order_is_jqs_5304() -> Result<()> {
    let (first, _, _) = run_jq_full(&["-n", "-c", "builtins[:3]"], None)?;
    assert_eq!(
        first,
        "[\"input_line_number/0\",\"input_filename/0\",\"now/0\"]\n"
    );
    let (sorted, _, _) = run_jq_full(&["-n", "-c", "builtins | sort"], None)?;
    let mut names = roster();
    names.sort_unstable();
    let want = format!(
        "[{}]\n",
        names
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_eq!(sorted, want);
    Ok(())
}

/// Callable here, and not jq's: extensions and yq's names are not in jq's list, whichever way a
/// program asks.
#[test]
fn test_builtins_omits_names_jq_does_not_define_5304() -> Result<()> {
    for name in [
        "trim/0",
        "ltrim/0",
        "rtrim/0",
        "leaf_paths/0",
        "skip/2",
        "toboolean/0",
        "isvalid/1",
        "env/1",
        "strenv/1",
        "isnull/0",
        "isstring/0",
        "reduce/3",
        "foreach/3",
        "foreach/4",
        "key/0",
        "parent/0",
        "parent/1",
        "line/0",
        "column/0",
        "tag/0",
        "style/0",
        "anchor/0",
        "kind/0",
        "line_comment/0",
        "tojsonstream/0",
        "fromjsonstream/0",
    ] {
        let (stdout, stderr, code) = run_jq_full(
            &["-n", "-c", &format!("builtins | index(\"{name}\")")],
            None,
        )?;
        assert_eq!((stdout.as_str(), code), ("null\n", 0), "{name}: {stderr}");
    }
    Ok(())
}

/// The entries jq has and this list lacked, now present.
#[test]
fn test_builtins_lists_what_jq_lists_5304() -> Result<()> {
    for name in [
        "match/1",
        "match/2",
        "capture/1",
        "capture/2",
        "test/2",
        "scan/1",
        "scan/2",
        "split/2",
        "splits/1",
        "splits/2",
        "sub/2",
        "sub/3",
        "gsub/2",
        "gsub/3",
        "JOIN/2",
        "JOIN/3",
        "JOIN/4",
        "not/0",
        "isempty/1",
        "format/1",
        "input_filename/0",
        "get_search_list/0",
        "get_prog_origin/0",
        "get_jq_origin/0",
        "strflocaltime/1",
        "cbrt/0",
        "fma/3",
        "pow10/0",
        "yn/2",
        "modulemeta/0",
        "builtins/0",
    ] {
        let (stdout, stderr, code) = run_jq_full(
            &["-n", "-c", &format!("builtins | any(. == \"{name}\")")],
            None,
        )?;
        assert_eq!((stdout.as_str(), code), ("true\n", 0), "{name}: {stderr}");
    }
    Ok(())
}

/// Not listed does not mean not callable: the extensions and the yq getters still run.
#[test]
fn test_unlisted_extensions_are_still_callable_5304() -> Result<()> {
    for (filter, input, want) in [
        ("trim", "\"  a \"", "\"a\"\n"),
        ("ltrim", "\"  a \"", "\"a \"\n"),
        ("[leaf_paths]", "{\"a\":1}", "[[\"a\"]]\n"),
        ("toboolean", "\"true\"", "true\n"),
        ("[skip(1; .[])]", "[1,2,3]", "[2,3]\n"),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-c", filter], Some(input))?;
        assert_eq!((stdout.as_str(), code), (want, 0), "`{filter}`: {stderr}");
    }
    Ok(())
}
