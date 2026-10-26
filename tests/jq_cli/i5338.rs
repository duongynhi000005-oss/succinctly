//! #5338: an `import`/`include` directive's `search` metadata replaces the default search list
//! (the current directory, then `-L`) for that one directive. A string is a list of one, an array
//! contributes its string entries, and any other value is an empty list. A relative entry is
//! relative to the directory of the file that holds the directive -- the working directory for
//! the main program, the `-f` file's directory under `-f` -- except exactly `.`, which is always
//! the working directory. A `module {search: ...};` header is not read. Expected values captured
//! live against jq 1.7.1 (`/usr/bin/jq`).

use anyhow::Result;

use crate::run_jq_in_dir;

/// A scratch directory: `m.jq` (`a` answers `cwd`), `L1/m.jq` (`L1`), `L2/m.jq` (`L2`),
/// `L1/sub/m.jq` (`L1/sub`), `abs/m.jq` (`abs`), `sub/inner/m.jq` (`inner`), `L1/deepl/z.jq`
/// (`deepl`), data files `d.json` (`cwd`), `L1/d.json` (`L1`) and `L2/d.json` (`L2`), and the
/// modules `L1/ns*.jq` that carry `search` metadata of their own.
fn fixture() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    for sub in ["L1/deepl", "L1/sub", "L2", "abs", "sub/inner"] {
        std::fs::create_dir_all(p.join(sub))?;
    }
    for (file, text) in [
        ("m.jq", "def a: \"cwd\";"),
        ("L1/m.jq", "def a: \"L1\";"),
        ("L2/m.jq", "def a: \"L2\";"),
        ("L1/sub/m.jq", "def a: \"L1/sub\";"),
        ("abs/m.jq", "def a: \"abs\";"),
        ("sub/m.jq", "def a: \"sub/m\";"),
        ("sub/inner/m.jq", "def a: \"inner\";"),
        ("L1/deepl/z.jq", "def a: \"deepl\";"),
        ("d.json", "{\"v\":\"cwd\"}"),
        ("L1/d.json", "{\"v\":\"L1\"}"),
        ("L2/d.json", "{\"v\":\"L2\"}"),
        (
            "L1/ns.jq",
            "import \"z\" as z {search:\"./deepl\"}; def b: z::a;",
        ),
        (
            "L1/ns2.jq",
            "import \"m\" as q {search:\"./sub\"}; def b: q::a;",
        ),
        (
            "L1/ns3.jq",
            "import \"m\" as q {search:\"L2\"}; def b: q::a;",
        ),
        (
            "L1/ns4.jq",
            "import \"m\" as q {search:\"../L2\"}; def b: q::a;",
        ),
        ("L1/ns5.jq", "include \"m\" {search:\"./deepl\"}; def b: a;"),
        (
            "L1/ns6.jq",
            "module {search:\"./deepl\"};\nimport \"z\" as z; def b: z::a;",
        ),
        ("sub/p1.jq", "import \"m\" as m {search:\"./inner\"}; m::a"),
        ("sub/p2.jq", "import \"m\" as m {search:\"./L1\"}; m::a"),
        ("sub/p3.jq", "import \"m\" as m {search:[\".\"]}; m::a"),
    ] {
        std::fs::write(p.join(file), text)?;
    }
    Ok(dir)
}

fn answers(dir: &tempfile::TempDir, args: &[&str], want: &str) -> Result<()> {
    let (stdout, stderr, code) = run_jq_in_dir(dir.path(), args, None)?;
    assert_eq!((stdout.as_str(), code), (want, 0), "{args:?}: {stderr}");
    Ok(())
}

fn not_found(dir: &tempfile::TempDir, args: &[&str], name: &str) -> Result<()> {
    let (stdout, stderr, code) = run_jq_in_dir(dir.path(), args, None)?;
    assert_eq!(
        (stdout.as_str(), stderr.as_str(), code),
        (
            "",
            format!("jq: error: module not found: {name}\n\njq: 1 compile error\n").as_str(),
            3
        ),
        "{args:?}"
    );
    Ok(())
}

#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object literals read like format arguments
fn test_search_metadata_replaces_the_default_list_5338() -> Result<()> {
    let dir = fixture()?;
    let abs = format!(
        "import \"m\" as m {{search:\"{}/abs\"}}; m::a",
        dir.path().display()
    );
    for (filter, want) in [
        // A string, an array (first directory first), and a missing first entry.
        (r#"import "m" as m {search:"./L1"}; m::a"#, "\"L1\"\n"),
        (r#"import "m" as m {search:"L1"}; m::a"#, "\"L1\"\n"),
        (
            r#"import "m" as m {search:["./L2","./L1"]}; m::a"#,
            "\"L2\"\n",
        ),
        (
            r#"import "m" as m {search:["./nonexist","./L1"]}; m::a"#,
            "\"L1\"\n",
        ),
        // A non-string entry is skipped; the empty string and `.` are the working directory.
        (r#"import "m" as m {search:[1,"./L1"]}; m::a"#, "\"L1\"\n"),
        (r#"import "m" as m {search:""}; m::a"#, "\"cwd\"\n"),
        (r#"import "m" as m {search:"."}; m::a"#, "\"cwd\"\n"),
        // Other metadata keys do not matter, and an absolute path is taken as it stands.
        (
            r#"import "m" as m {search:"./L1", other:1}; m::a"#,
            "\"L1\"\n",
        ),
        (r#"import "m" as m {other:1}; m::a"#, "\"cwd\"\n"),
        (abs.as_str(), "\"abs\"\n"),
        // `include`, and a data import, take `search` too.
        (r#"include "m" {search:"./L1"}; a"#, "\"L1\"\n"),
        (
            r#"import "d" as $d {search:"./L1"}; $d"#,
            "[{\"v\":\"L1\"}]\n",
        ),
        (
            r#"import "d" as $d {search:["./nonexist","./L2"]}; $d"#,
            "[{\"v\":\"L2\"}]\n",
        ),
        // Each directive has its own list; the later `include` of a name wins.
        (
            r#"import "m" as a {search:"./L1"}; import "m" as b; a::a, b::a"#,
            "\"L1\"\n\"cwd\"\n",
        ),
        (
            r#"include "m" {search:"./L1"}; include "m" {search:"./L2"}; a"#,
            "\"L2\"\n",
        ),
        // A nested path under the searched directory.
        (
            r#"import "sub/m" as s {search:"./L1"}; s::a"#,
            "\"L1/sub\"\n",
        ),
    ] {
        answers(&dir, &["-nc", filter], want)?;
    }
    // `-L` is not tried after a `search`, and does not outrank it.
    answers(
        &dir,
        &[
            "-nc",
            "-L",
            "L2",
            r#"import "m" as m {search:"./L1"}; m::a"#,
        ],
        "\"L1\"\n",
    )?;
    Ok(())
}

#[test]
#[allow(clippy::literal_string_with_formatting_args)] // jq object literals read like format arguments
fn test_search_metadata_that_finds_nothing_is_module_not_found_5338() -> Result<()> {
    let dir = fixture()?;
    for filter in [
        // The current directory holds `m.jq`, but a `search` replaces it.
        r#"import "m" as m {search:"./nonexist"}; m::a"#,
        r#"import "m" as m {search:[]}; m::a"#,
        // Anything that is neither a string nor an array is an empty list.
        r#"import "m" as m {search:1}; m::a"#,
        r#"import "m" as m {search:null}; m::a"#,
        r#"import "m" as m {search:{}}; m::a"#,
        r#"include "m" {search:"./nonexist"}; a"#,
    ] {
        not_found(&dir, &["-nc", filter], "m")?;
        not_found(&dir, &["-nc", "-L", "L1", filter], "m")?;
    }
    not_found(
        &dir,
        &["-nc", r#"import "d" as $d {search:"./nonexist"}; $d"#],
        "d",
    )?;
    Ok(())
}

#[test]
fn test_search_in_a_module_is_relative_to_the_module_5338() -> Result<()> {
    let dir = fixture()?;
    // A relative entry resolves against the module's own directory, not the working one.
    answers(
        &dir,
        &["-nc", "-L", "L1", r#"import "ns" as n; n::b"#],
        "\"deepl\"\n",
    )?;
    answers(
        &dir,
        &["-nc", "-L", "L1", r#"import "ns2" as n; n::b"#],
        "\"L1/sub\"\n",
    )?;
    answers(
        &dir,
        &["-nc", "-L", "L1", r#"import "ns4" as n; n::b"#],
        "\"L2\"\n",
    )?;
    // `L2` is not a directory of `L1/`, whatever the working directory holds.
    not_found(
        &dir,
        &["-nc", "-L", "L1", r#"import "ns3" as n; n::b"#],
        "m",
    )?;
    not_found(&dir, &["-nc", "-L", "L1", r#"include "ns5"; b"#], "m")?;
    // A `module {search: ...};` header is not a search list for the module's own imports.
    not_found(
        &dir,
        &["-nc", "-L", "L1", r#"import "ns6" as n; n::b"#],
        "z",
    )?;
    // The program's own `search` is not inherited by the module it loads.
    answers(
        &dir,
        &["-nc", r#"import "ns" as n {search:"./L1"}; n::b"#],
        "\"deepl\"\n",
    )?;
    Ok(())
}

#[test]
fn test_search_in_a_program_file_is_relative_to_the_file_5338() -> Result<()> {
    let dir = fixture()?;
    // `./inner` is `sub/inner` for `-f sub/p1.jq`, and `./L1` is `sub/L1`, which is not there.
    answers(&dir, &["-nc", "-f", "sub/p1.jq"], "\"inner\"\n")?;
    not_found(&dir, &["-nc", "-f", "sub/p2.jq"], "m")?;
    // ... but exactly `.` is the working directory, wherever the program file is.
    answers(&dir, &["-nc", "-f", "sub/p3.jq"], "\"cwd\"\n")?;
    Ok(())
}
