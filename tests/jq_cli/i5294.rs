//! #5294: jq resolves `import "b"` / `include "b"` to `<dir>/b.jq`, then the directory-module
//! spellings `<dir>/b/jq/main.jq` and `<dir>/b/b.jq`, trying all three in one search directory
//! before the next (a data import does the same with `.json`). Only `<dir>/b.jq` was tried. Every
//! answer below was captured from jq 1.7.1 on the same layout.

use std::path::Path;
use std::process::Command;

use anyhow::Result;

use crate::{run_jq_with_modules, spawn_with_signal_retry, succinctly_bin};

/// Write each `(relative path, contents)` into a fresh directory, run
/// `succinctly jq -nc -L <dir>... <filter>` over those directories, and return stdout.
fn run_over(layouts: &[&[(&str, &str)]], filter: &str) -> Result<(String, i32)> {
    let dirs = layouts
        .iter()
        .map(|files| {
            let dir = tempfile::tempdir()?;
            for (name, contents) in *files {
                let path = dir.path().join(name);
                std::fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))?;
                std::fs::write(path, contents)?;
            }
            Ok(dir)
        })
        .collect::<Result<Vec<_>>>()?;
    let (output, code) = spawn_with_signal_retry(
        || {
            let mut command = Command::new(succinctly_bin());
            command.arg("jq");
            for dir in &dirs {
                command.arg("-L").arg(dir.path());
            }
            command.args(["-nc", filter]);
            command
        },
        None,
    )?;
    Ok((String::from_utf8(output.stdout)?, code))
}

#[test]
fn test_directory_module_is_found_5294() -> Result<()> {
    for (files, filter, want) in [
        // `b/b.jq` for `import "b"`.
        (
            &[("b/b.jq", "def a: \"b/b\";")][..],
            "import \"b\" as b; b::a",
            "\"b/b\"\n",
        ),
        (
            &[("b/b.jq", "def a: \"b/b\";")][..],
            "include \"b\"; a",
            "\"b/b\"\n",
        ),
        // `b/jq/main.jq`.
        (
            &[("b/jq/main.jq", "def a: \"b/jq/main\";")][..],
            "import \"b\" as b; b::a",
            "\"b/jq/main\"\n",
        ),
        // The last component of a nested path names the file: `p/q/q.jq`.
        (
            &[("p/q/q.jq", "def a: \"p/q/q\";")][..],
            "import \"p/q\" as q; q::a",
            "\"p/q/q\"\n",
        ),
        // `b.jq` beats `b/b.jq`, and `b/jq/main.jq` beats `b/b.jq`.
        (
            &[("b.jq", "def a: \"b\";"), ("b/b.jq", "def a: \"b/b\";")][..],
            "import \"b\" as b; b::a",
            "\"b\"\n",
        ),
        (
            &[
                ("b/b.jq", "def a: \"b/b\";"),
                ("b/jq/main.jq", "def a: \"b/jq/main\";"),
            ][..],
            "import \"b\" as b; b::a",
            "\"b/jq/main\"\n",
        ),
        // A directory that is not a module is not one: no `b/other.jq` match.
        (
            &[("b/other.jq", "def a: 1;")][..],
            "import \"b\" as b; b::a",
            "",
        ),
    ] {
        let (out, code) = run_over(&[files], filter)?;
        assert_eq!(out, want, "{filter} over {files:?}");
        assert_eq!(
            code,
            i32::from(want.is_empty()) * 3,
            "{filter} over {files:?}"
        );
    }
    Ok(())
}

#[test]
fn test_every_spelling_in_one_directory_precedes_the_next_directory_5294() -> Result<()> {
    let earlier: &[(&str, &str)] = &[("b/b.jq", "def a: \"earlier b/b\";")];
    let later: &[(&str, &str)] = &[("b.jq", "def a: \"later b\";")];
    for (layouts, want) in [
        (&[earlier, later], "\"earlier b/b\"\n"),
        (&[later, earlier], "\"later b\"\n"),
    ] {
        let (out, code) = run_over(layouts, "import \"b\" as b; b::a")?;
        assert_eq!((out.as_str(), code), (want, 0));
    }
    Ok(())
}

#[test]
fn test_data_import_finds_a_directory_data_file_5294() -> Result<()> {
    for (files, want) in [
        (&[("d/d.json", "{\"k\":1}")][..], "[{\"k\":1}]\n"),
        (&[("d/jq/main.json", "[3]")][..], "[[3]]\n"),
        (
            &[("d/d.json", "[4]"), ("d/jq/main.json", "[3]")][..],
            "[[3]]\n",
        ),
        (&[("d.json", "[5]"), ("d/d.json", "[4]")][..], "[[5]]\n"),
    ] {
        let (out, code) = run_over(&[files], "import \"d\" as $d; $d")?;
        assert_eq!((out.as_str(), code), (want, 0), "{files:?}");
    }
    Ok(())
}

/// The module this lays out is the multi-file helper's `<name>.jq`, so a nested module name works
/// there too.
#[test]
fn test_directory_module_through_the_multi_module_helper_5294() -> Result<()> {
    let (stdout, stderr, code) = run_jq_with_modules(
        &[("m/m", "def a: \"m/m\";")],
        &["-nc", "import \"m\" as m; m::a"],
    )?;
    assert_eq!((stdout.as_str(), code), ("\"m/m\"\n", 0), "{stderr}");
    Ok(())
}

/// A directory module's own imports resolve through the search path, not its directory: `c.jq`
/// beside `b/b.jq` is not found for `import "c"` there, while `c.jq` at the root of the search
/// directory is. A trailing slash names no module.
#[test]
fn test_directory_module_imports_resolve_through_the_search_path_5294() -> Result<()> {
    let nested = "import \"c\" as c; def a: c::z;";
    for (files, filter, want, want_code) in [
        (
            &[("b/b.jq", nested), ("c.jq", "def z: \"root c\";")][..],
            "import \"b\" as b; b::a",
            "\"root c\"\n",
            0,
        ),
        (
            &[("b/b.jq", nested), ("b/c.jq", "def z: \"sibling c\";")][..],
            "import \"b\" as b; b::a",
            "",
            3,
        ),
        (
            &[("b/b.jq", "def a: 1;")][..],
            "import \"b/\" as b; b::a",
            "",
            3,
        ),
    ] {
        let (out, code) = run_over(&[files], filter)?;
        assert_eq!(
            (out.as_str(), code),
            (want, want_code),
            "{filter} over {files:?}"
        );
    }
    Ok(())
}
