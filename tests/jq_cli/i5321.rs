//! #5321: `modulemeta` resolves its input name like an `import` does, on the library directories
//! (`-L`) alone -- not the current directory -- and answers the module's header metadata, then
//! `deps` (one object per `import`/`include`, in source order) and `defs` (`name/arity` of each
//! top-level `def`). Every expected value was captured from jq 1.7.1 (`/usr/bin/jq`).

use anyhow::Result;

use crate::run_jq_in_dir;

fn fixture() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    let lib = dir.path().join("lib");
    std::fs::create_dir_all(lib.join("dir"))?;
    std::fs::write(lib.join("b.jq"), "def a: \"b\";\n")?;
    std::fs::write(
        lib.join("c.jq"),
        concat!(
            "module {name:\"c\", version:1, x:[1,{\"y\":null}], \"quoted\":true};\n",
            "import \"b\" as bb;\n",
            "import \"d\" as $dd;\n",
            "include \"b\" {search:\"./lib\"};\n",
            "import \"b\" as e {search:[\"./a\",\"./b\"]};\n",
            "def f: 1; def g(x; $y): 2; def f(a): 3;\n",
        ),
    )?;
    std::fs::write(
        lib.join("nest.jq"),
        "def outer: def inner: 1; inner; def o2($a): $a;",
    )?;
    std::fs::write(lib.join("dir/dir.jq"), "def q: 1;")?;
    std::fs::write(
        lib.join("clash.jq"),
        "module {deps:5, defs:6};\ndef z: 1;\n",
    )?;
    std::fs::write(lib.join("mainexpr.jq"), "def z: 1; z\n")?;
    std::fs::write(lib.join("empty.jq"), "module {};\n")?;
    std::fs::write(dir.path().join("cw.jq"), "def cw: 1;")?;
    Ok(dir)
}

fn modulemeta(dir: &std::path::Path, name: &str, filter: &str) -> Result<(String, String, i32)> {
    let program = format!("{name:?} | {filter}");
    run_jq_in_dir(dir, &["-nc", "-L", "lib", &program], None)
}

#[test]
fn test_modulemeta_answers_deps_and_defs_5321() -> Result<()> {
    let dir = fixture()?;
    for (name, want) in [
        ("b", "{\"deps\":[],\"defs\":[\"a/0\"]}\n"),
        (
            "c",
            concat!(
                "{\"name\":\"c\",\"version\":1,\"x\":[1,{\"y\":null}],\"quoted\":true,",
                "\"deps\":[{\"as\":\"bb\",\"is_data\":false,\"relpath\":\"b\"},",
                "{\"as\":\"dd\",\"is_data\":true,\"relpath\":\"d\"},",
                "{\"search\":\"./lib\",\"is_data\":false,\"relpath\":\"b\"},",
                "{\"search\":[\"./a\",\"./b\"],\"as\":\"e\",\"is_data\":false,\"relpath\":\"b\"}],",
                "\"defs\":[\"f/0\",\"g/2\",\"f/1\"]}\n",
            ),
        ),
        // Only top-level defs are listed, each with its parameter count.
        ("nest", "{\"deps\":[],\"defs\":[\"outer/0\",\"o2/1\"]}\n"),
        // A directory module resolves as `import` resolves it (#5294).
        ("dir", "{\"deps\":[],\"defs\":[\"q/0\"]}\n"),
        ("empty", "{\"deps\":[],\"defs\":[]}\n"),
        // The real `deps`/`defs` replace metadata keys of the same name, in place.
        ("clash", "{\"deps\":[],\"defs\":[\"z/0\"]}\n"),
    ] {
        let (stdout, stderr, code) = modulemeta(dir.path(), name, "modulemeta")?;
        assert_eq!((stdout.as_str(), code), (want, 0), "{name}: {stderr}");
    }
    Ok(())
}

#[test]
fn test_modulemeta_errors_5321() -> Result<()> {
    let dir = fixture()?;
    // jq searches the library directories only: `cw.jq` in the working directory is not found,
    // and a data file or a missing module is `module not found` (catchable).
    for (name, want) in [
        ("cw", "module not found: cw"),
        ("d", "module not found: d"),
        ("nope", "module not found: nope"),
        ("lib/b", "module not found: lib/b"),
        ("b.jq", "module not found: b.jq"),
        (
            "../x",
            "Relative paths to modules may not traverse to parent directories (../x)",
        ),
        (
            "b/b",
            "module names must not have equal consecutive components: b/b",
        ),
    ] {
        let (stdout, stderr, code) = modulemeta(dir.path(), name, "try modulemeta catch .")?;
        assert_eq!(
            (stdout.as_str(), code),
            (format!("{want:?}\n").as_str(), 0),
            "{name}: {stderr}"
        );
    }
    // Uncaught, the error ends the run with jq's exit 5.
    let (stdout, stderr, code) = modulemeta(dir.path(), "nope", "modulemeta")?;
    assert_eq!(
        (stdout.as_str(), stderr.as_str(), code),
        ("", "jq: error (at <unknown>): module not found: nope\n", 5)
    );
    // `?` swallows it.
    let (stdout, _, code) = modulemeta(dir.path(), "nope", "modulemeta?")?;
    assert_eq!((stdout.as_str(), code), ("", 0));
    // Not a string.
    let (_, stderr, code) =
        run_jq_in_dir(dir.path(), &["-nc", "-L", "lib", "1 | modulemeta"], None)?;
    assert_eq!(
        (stderr.as_str(), code),
        (
            "jq: error (at <unknown>): modulemeta input module name must be a string\n",
            5
        )
    );
    Ok(())
}

/// A module that is found but cannot be loaded is not an error of the filter: the diagnostic goes
/// to stderr and the answer is `null`, exit 0. Its wording is succinctly's own past the
/// `jq: error:` prefix (see the module-syntax-error note in `limitations.md`).
#[test]
fn test_modulemeta_load_failure_prints_and_answers_null_5321() -> Result<()> {
    let dir = fixture()?;
    let (stdout, stderr, code) = modulemeta(dir.path(), "mainexpr", "modulemeta")?;
    assert_eq!(
        (stdout.as_str(), stderr.as_str(), code),
        (
            "null\n",
            "jq: error: jq: error: library should only have function definitions, not a main expression\n<unknown location>\n",
            0
        )
    );

    std::fs::write(dir.path().join("lib/syn.jq"), "def (")?;
    let (stdout, stderr, code) = modulemeta(dir.path(), "syn", "modulemeta")?;
    assert_eq!((stdout.as_str(), code), ("null\n", 0), "{stderr}");
    assert!(
        stderr.starts_with("jq: error: ") && stderr.contains("syn.jq, line 1:\ndef ("),
        "{stderr}"
    );
    Ok(())
}
