//! #5339: a `~/.jq` file's own `import`/`include`/data-import directives are resolved like a
//! module's: they search the current directory and then `-L`, bind only inside that file's defs
//! (neither the main filter nor a module sees them), and a missing one is a compile error that
//! comes last in jq's report. Before, only the file's defs were kept, so a def calling into an
//! imported module failed to compile. Expected values captured live against jq 1.7.1
//! (`/usr/bin/jq`).

use anyhow::Result;

use std::process::Command;

use crate::{spawn_with_signal_retry, succinctly_bin};

/// A scratch directory (the process's current directory) with modules `m.jq` (`a` answers
/// `cwd`), `inc.jq` (`g` is 42), `mid.jq` (includes `inc`, `h` is `g + 1`), `d.json` and a
/// `lib/m.jq` (`a` answers `lib`), plus an empty directory standing in for `$HOME`.
fn fixture() -> Result<(tempfile::TempDir, tempfile::TempDir)> {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    std::fs::create_dir(p.join("lib"))?;
    std::fs::write(p.join("m.jq"), "def a: \"cwd\";")?;
    std::fs::write(p.join("lib/m.jq"), "def a: \"lib\";")?;
    std::fs::write(p.join("inc.jq"), "def g: 42;")?;
    std::fs::write(p.join("mid.jq"), "include \"inc\"; def h: g + 1;")?;
    std::fs::write(p.join("d.json"), "{\"k\":1}\n")?;
    Ok((dir, tempfile::tempdir()?))
}

/// `succinctly jq -nc <args>` in `dir` with `home/.jq` holding `dot_jq`.
fn run(
    dir: &tempfile::TempDir,
    home: &tempfile::TempDir,
    dot_jq: &str,
    args: &[&str],
) -> Result<(String, String, i32)> {
    std::fs::write(home.path().join(".jq"), dot_jq)?;
    let (output, code) = spawn_with_signal_retry(
        || {
            let mut command = Command::new(succinctly_bin());
            command
                .current_dir(dir.path())
                .env("HOME", home.path())
                .args(["jq", "-nc"])
                .args(args);
            command
        },
        None,
    )?;
    Ok((
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?,
        code,
    ))
}

#[test]
fn test_dot_jq_directives_resolve_for_its_own_defs_5339() -> Result<()> {
    let (dir, home) = fixture()?;
    for (dot_jq, args, want) in [
        // import, include and data import, each reached from a def of the file.
        (
            "import \"m\" as hm; def hb: hm::a;",
            &["hb"][..],
            "\"cwd\"\n",
        ),
        ("include \"inc\"; def hb: g;", &["hb"][..], "42\n"),
        (
            "import \"d\" as $d; def hb: $d;",
            &["hb"][..],
            "[{\"k\":1}]\n",
        ),
        // A module's own dependency reaches the file through the module.
        ("include \"mid\"; def hb: h;", &["hb"][..], "43\n"),
        // The current directory ranks ahead of `-L`, as for the main program (#5327).
        (
            "import \"m\" as hm; def hb: hm::a;",
            &["-L", "lib", "hb"][..],
            "\"cwd\"\n",
        ),
        // The same module imported by the file and by the main filter is one module.
        (
            "import \"m\" as hm; def hb: hm::a;",
            &["import \"m\" as hm; hm::a, hb"][..],
            "\"cwd\"\n\"cwd\"\n",
        ),
        // The main filter's own `include` outranks the file's of the same name.
        (
            "import \"m\" as hm; def hb: hm::a;",
            &["include \"m\"; a, hb"][..],
            "\"cwd\"\n\"cwd\"\n",
        ),
        // A `$d` the file imports is its own: `--argjson` of the same name does not replace it.
        (
            "import \"d\" as $d; def hb: $d;",
            &["--argjson", "d", "5", "hb"][..],
            "[{\"k\":1}]\n",
        ),
    ] {
        let (stdout, stderr, code) = run(&dir, &home, dot_jq, args)?;
        assert_eq!(
            (stdout.as_str(), code),
            (want, 0),
            "{dot_jq} | {args:?}: {stderr}"
        );
    }
    Ok(())
}

/// jq 1.7.1 does not look for a `~/.jq` file's modules next to it: a module that lives only in
/// `$HOME` is `module not found` until `-L` names that directory.
#[test]
fn test_dot_jq_modules_are_not_searched_next_to_the_file_5339() -> Result<()> {
    let (dir, home) = fixture()?;
    std::fs::write(home.path().join("helpers.jq"), "def helper: \"home\";")?;
    let dot_jq = "include \"helpers\"; def hb: helper;";
    let (stdout, stderr, code) = run(&dir, &home, dot_jq, &["hb"])?;
    assert_eq!(
        (stdout.as_str(), stderr.as_str(), code),
        (
            "",
            "jq: error: module not found: helpers\n\njq: 1 compile error\n",
            3
        )
    );
    let home_dir = home.path().to_str().expect("a temp dir path is UTF-8");
    let (stdout, stderr, code) = run(&dir, &home, dot_jq, &["-L", home_dir, "hb"])?;
    assert_eq!((stdout.as_str(), code), ("\"home\"\n", 0), "{stderr}");
    Ok(())
}

#[test]
fn test_dot_jq_dependencies_stay_private_to_the_file_5339() -> Result<()> {
    let (dir, home) = fixture()?;
    for (dot_jq, args, undefined) in [
        // A namespace, a module's def and a data variable the file imports are not the main
        // filter's.
        (
            "import \"m\" as hm; def hb: hm::a;",
            "hm::a",
            "hm::a/0 is not defined",
        ),
        ("include \"inc\"; def hb: g;", "g", "g/0 is not defined"),
        ("import \"d\" as $d; def hb: $d;", "$d", "$d is not defined"),
        // ... nor a def of the file's, through a module it includes (`h` is `mid`'s).
        ("include \"mid\"; def hb: h;", "h", "h/0 is not defined"),
        // ... and a module's dependency is not re-exported to the file.
        ("include \"mid\"; def hb: g;", "hb", "g/0 is not defined"),
    ] {
        let (stdout, stderr, code) = run(&dir, &home, dot_jq, &[args])?;
        assert_eq!((stdout.as_str(), code), ("", 3), "{dot_jq} | {args}");
        assert!(stderr.contains(undefined), "{dot_jq} | {args}: {stderr}");
        assert!(
            stderr.ends_with("jq: 1 compile error\n"),
            "{dot_jq} | {args}: {stderr}"
        );
    }
    Ok(())
}

#[test]
fn test_dot_jq_missing_dependency_is_a_compile_error_reported_last_5339() -> Result<()> {
    let (dir, home) = fixture()?;
    // Raised whether or not the filter uses the file's defs.
    for dot_jq in [
        "import \"nonexist\" as x; def hb: 1;",
        "include \"nonexist\"; def hb: 1;",
        "import \"nonexist\" as $d; def hb: 1;",
    ] {
        let (stdout, stderr, code) = run(&dir, &home, dot_jq, &["1"])?;
        assert_eq!(
            (stdout.as_str(), stderr.as_str(), code),
            (
                "",
                "jq: error: module not found: nonexist\n\njq: 1 compile error\n",
                3
            ),
            "{dot_jq}"
        );
    }
    // The file is declared ahead of the program, so jq reaches it after the program's own
    // directives, and a missing one there stops the level before the file.
    let (_, stderr, code) = run(
        &dir,
        &home,
        "include \"nonexist\"; def hb: 1;",
        &["include \"nonexist2\"; 1"],
    )?;
    assert_eq!(
        (stderr.as_str(), code),
        (
            "jq: error: module not found: nonexist2\n\njq: 1 compile error\n",
            3
        )
    );
    // With the program's directives all found, the file's failure is the one reported.
    let (_, stderr, code) = run(
        &dir,
        &home,
        "include \"nonexist\"; def hb: 1;",
        &["import \"d\" as $d; $d"],
    )?;
    assert_eq!(
        (stderr.as_str(), code),
        (
            "jq: error: module not found: nonexist\n\njq: 1 compile error\n",
            3
        )
    );
    Ok(())
}

#[test]
fn test_dot_jq_include_does_not_shadow_a_builtin_in_the_main_filter_5339() -> Result<()> {
    let (dir, home) = fixture()?;
    std::fs::write(dir.path().join("shad.jq"), "def length: \"shadow\";")?;
    let dot_jq = "include \"shad\"; def hb: [1,2] | length;";
    // Inside the file the module's `length` wins; in the main filter it stays the builtin.
    let (stdout, stderr, code) = run(&dir, &home, dot_jq, &["hb"])?;
    assert_eq!((stdout.as_str(), code), ("\"shadow\"\n", 0), "{stderr}");
    let (stdout, stderr, code) = run(&dir, &home, dot_jq, &["[1,2] | length"])?;
    assert_eq!((stdout.as_str(), code), ("2\n", 0), "{stderr}");
    Ok(())
}
