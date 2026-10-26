//! #5339: a `~/.jq` file's own `import`/`include` directives load like a module's: they search the
//! current directory, then `-L`; the main program sees the file's defs and nothing its directives
//! brought in; and a directive that fails to load is a compile error, reported after the
//! program's own. Expected values and messages captured from jq 1.7.1 (`/usr/bin/jq`).

use std::path::Path;
use std::process::Command;

use anyhow::Result;

use crate::{spawn_with_signal_retry, succinctly_bin};

/// `succinctly jq -nc <args>` in `cwd` with `HOME` set to a directory holding `home_jq`.
fn run_with_home(cwd: &Path, home_jq: &str, args: &[&str]) -> Result<(String, String, i32)> {
    let home = tempfile::tempdir()?;
    std::fs::write(home.path().join(".jq"), home_jq)?;
    let (output, code) = spawn_with_signal_retry(
        || {
            let mut command = Command::new(succinctly_bin());
            command
                .current_dir(cwd)
                .env("HOME", home.path())
                .env_remove("JQ_LIBRARY_PATH")
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

fn fixture() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    let p = dir.path();
    std::fs::create_dir(p.join("lib"))?;
    std::fs::write(p.join("m.jq"), "def a: \"cwd\";")?;
    std::fs::write(p.join("d.json"), "[7]\n")?;
    std::fs::write(p.join("lib/k.jq"), "def k: \"k\";")?;
    std::fs::write(p.join("lib/n.jq"), "import \"m\" as m; def b: m::a;")?;
    Ok(dir)
}

fn undefined(site: &str, what: &str) -> String {
    format!(
        "jq: error: {what} is not defined at <top-level>, line 1:\n{site}\njq: 1 compile error\n"
    )
}

#[test]
fn test_home_file_directives_bind_privately_5339() -> Result<()> {
    let dir = fixture()?;
    let cwd = dir.path();
    for (home_jq, args, want_out, want_err, want_code) in [
        // An `import`: the file's defs reach it, the main program does not.
        (
            "import \"m\" as hm; def hb: hm::a;",
            vec!["hb"],
            "\"cwd\"\n",
            String::new(),
            0,
        ),
        (
            "import \"m\" as hm; def hb: hm::a;",
            vec!["hm::a"],
            "",
            undefined("hm::a", "hm::a/0"),
            3,
        ),
        // An `include`: the same.
        (
            "include \"m\"; def hb: a;",
            vec!["hb"],
            "\"cwd\"\n",
            String::new(),
            0,
        ),
        (
            "include \"m\"; def hb: a;",
            vec!["a"],
            "",
            undefined("a", "a/0"),
            3,
        ),
        // A data import binds `$dd` inside the file's defs only.
        (
            "import \"d\" as $dd; def hb: $dd;",
            vec!["hb"],
            "[[7]]\n",
            String::new(),
            0,
        ),
        (
            "import \"d\" as $dd; def hb: $dd;",
            vec!["$dd"],
            "",
            undefined("$dd", "$dd"),
            3,
        ),
        // The file's `include` does not collide with the program's own import of the same module.
        (
            "include \"m\"; def hb: a;",
            vec!["import \"m\" as hm; [hb, hm::a]"],
            "[\"cwd\",\"cwd\"]\n",
            String::new(),
            0,
        ),
        // All three kinds together, and a dependency's own dependency.
        (
            "import \"m\" as hm; import \"d\" as $dd; include \"k\"; def hb: [hm::a, $dd, k];",
            vec!["-L", "lib", "hb"],
            "[\"cwd\",[[7]],\"k\"]\n",
            String::new(),
            0,
        ),
        (
            "import \"n\" as nn; def hb: nn::b;",
            vec!["-L", "lib", "hb"],
            "\"cwd\"\n",
            String::new(),
            0,
        ),
    ] {
        let (stdout, stderr, code) = run_with_home(cwd, home_jq, &args)?;
        assert_eq!(
            (stdout.as_str(), stderr.as_str(), code),
            (want_out, want_err.as_str(), want_code),
            "{home_jq} :: {args:?}"
        );
    }
    Ok(())
}

#[test]
fn test_home_file_directive_failure_is_a_compile_error_5339() -> Result<()> {
    let dir = fixture()?;
    let cwd = dir.path();
    let missing = "jq: error: module not found: nonexistent\n\njq: 1 compile error\n";
    let (stdout, stderr, code) =
        run_with_home(cwd, "import \"nonexistent\" as n; def hb: 1;", &["1"])?;
    assert_eq!((stdout.as_str(), stderr.as_str(), code), ("", missing, 3));

    // The program's own failure is the one reported, not `~/.jq`'s.
    let (stdout, stderr, code) = run_with_home(
        cwd,
        "import \"nonexistent\" as n; def hb: 1;",
        &["import \"alsomissing\" as z; 1"],
    )?;
    assert_eq!(
        (stdout.as_str(), stderr.as_str(), code),
        (
            "",
            "jq: error: module not found: alsomissing\n\njq: 1 compile error\n",
            3
        )
    );

    // A missing module stops its level, and the walk is last-declared first: only the last
    // `import` is named, counted once.
    let (stdout, stderr, code) = run_with_home(
        cwd,
        "import \"nonexistent\" as n; import \"nonexistent2\" as n2; def hb: 1;",
        &["1"],
    )?;
    assert_eq!(
        (stdout.as_str(), stderr.as_str(), code),
        (
            "",
            "jq: error: module not found: nonexistent2\n\njq: 1 compile error\n",
            3
        )
    );
    Ok(())
}
