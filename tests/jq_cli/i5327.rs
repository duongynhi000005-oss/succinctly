//! #5327: jq 1.7.1 prepends the current directory to the search list of every `import`/`include`
//! that carries no `search` metadata, ahead of the `-L` directories -- for the main program and for
//! a module's own nested imports alike. It is the process's current directory, not the program
//! file's and not the importing module's. Expected values captured live against jq 1.7.1
//! (`/usr/bin/jq`).

use anyhow::Result;

use crate::run_jq_in_dir;

/// A scratch directory holding `m.jq` (answers `cwd`), `other/m.jq` (`other`), `other/n.jq`
/// (imports `k` and answers its `a`), `other/k.jq` (`otherk`), `zz.json` and `sub/prog.jq`
/// (imports `m`).
fn fixture() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    std::fs::create_dir(p.join("other"))?;
    std::fs::create_dir(p.join("sub"))?;
    std::fs::write(p.join("m.jq"), "def a: \"cwd\";")?;
    std::fs::write(p.join("other/m.jq"), "def a: \"other\";")?;
    std::fs::write(p.join("other/n.jq"), "import \"k\" as k; def b: k::a;")?;
    std::fs::write(p.join("other/k.jq"), "def a: \"otherk\";")?;
    std::fs::write(p.join("zz.json"), "[9]\n")?;
    std::fs::write(p.join("sub/prog.jq"), "import \"m\" as m; m::a")?;
    Ok(dir)
}

#[test]
fn test_import_searches_the_current_directory_ahead_of_l_5327() -> Result<()> {
    let dir = fixture()?;
    for (args, want) in [
        // Found with no `-L` at all.
        (vec!["-nc", "import \"m\" as m; m::a"], "\"cwd\"\n"),
        // The current directory beats an `-L` directory holding the same name.
        (
            vec!["-nc", "-L", "other", "import \"m\" as m; m::a"],
            "\"cwd\"\n",
        ),
        (vec!["-nc", "include \"m\"; a"], "\"cwd\"\n"),
        (vec!["-nc", "-L", "other", "include \"m\"; a"], "\"cwd\"\n"),
        // A data import is searched the same way.
        (vec!["-nc", "import \"zz\" as $d; $d"], "[[9]]\n"),
        // ... and `./`-prefixed and nested-directory spellings resolve against it.
        (vec!["-nc", "import \"./m\" as m; m::a"], "\"cwd\"\n"),
        (vec!["-nc", "import \"other/m\" as m; m::a"], "\"other\"\n"),
        // A name the current directory lacks still falls through to `-L`.
        (
            vec!["-nc", "-L", "other", "import \"k\" as k; k::a"],
            "\"otherk\"\n",
        ),
    ] {
        let (stdout, stderr, code) = run_jq_in_dir(dir.path(), &args, None)?;
        assert_eq!((stdout.as_str(), code), (want, 0), "{args:?}: {stderr}");
    }
    Ok(())
}

#[test]
fn test_nested_import_searches_the_current_directory_not_the_modules_5327() -> Result<()> {
    let dir = fixture()?;
    let args = ["-nc", "-L", "other", "import \"n\" as n; n::b"];

    // `other/n.jq` imports `k`, which only `other/` holds: found through `-L`.
    let (stdout, stderr, code) = run_jq_in_dir(dir.path(), &args, None)?;
    assert_eq!((stdout.as_str(), code), ("\"otherk\"\n", 0), "{stderr}");

    // With `k.jq` also in the current directory, that one wins over the module's own directory.
    std::fs::write(dir.path().join("k.jq"), "def a: \"cwdk\";")?;
    let (stdout, stderr, code) = run_jq_in_dir(dir.path(), &args, None)?;
    assert_eq!((stdout.as_str(), code), ("\"cwdk\"\n", 0), "{stderr}");
    Ok(())
}

#[test]
fn test_current_directory_is_the_process_s_not_the_program_files_5327() -> Result<()> {
    let dir = fixture()?;
    let want_err = "jq: error: module not found: m\n\njq: 1 compile error\n";

    // `sub/prog.jq` imports `m`; `m.jq` sits in the parent, so running from `sub/` finds nothing.
    let sub = dir.path().join("sub");
    let (stdout, stderr, code) = run_jq_in_dir(&sub, &["-nc", "-f", "prog.jq"], None)?;
    assert_eq!((stdout.as_str(), stderr.as_str(), code), ("", want_err, 3));

    // Run from the directory that holds `m.jq`, the same program file resolves it.
    let (stdout, stderr, code) = run_jq_in_dir(dir.path(), &["-nc", "-f", "sub/prog.jq"], None)?;
    assert_eq!((stdout.as_str(), code), ("\"cwd\"\n", 0), "{stderr}");
    Ok(())
}
