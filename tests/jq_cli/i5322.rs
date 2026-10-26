//! #5322: jq validates a relative module path before it searches for it (`validate_relpath`) and
//! reports a malformed one in the same compile-error report as a missing module: a backslash, then
//! left to right the first of a component equal to the one before it and a `..` component. A path
//! that passes is merely not found. The same three checks apply to `import`, a data import and
//! `include`. Every stderr below was captured from jq 1.7.1, run in a directory holding none of the
//! modules.

use anyhow::Result;

use crate::run_jq_full;

#[test]
fn test_malformed_module_path_is_rejected_before_the_search_5322() -> Result<()> {
    for (program, want_stderr) in [
        (
            "import \"../x\" as m; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (../x)\n\njq: 1 compile error\n",
        ),
        (
            "import \"../x\" as $d; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (../x)\n\njq: 1 compile error\n",
        ),
        (
            "include \"../x\"; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (../x)\n\njq: 1 compile error\n",
        ),
        (
            "import \"..\" as m; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (..)\n\njq: 1 compile error\n",
        ),
        (
            "import \"..\" as $d; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (..)\n\njq: 1 compile error\n",
        ),
        (
            "include \"..\"; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (..)\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/..\" as m; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (a/..)\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/..\" as $d; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (a/..)\n\njq: 1 compile error\n",
        ),
        (
            "include \"a/..\"; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (a/..)\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/../b\" as m; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (a/../b)\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/../b\" as $d; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (a/../b)\n\njq: 1 compile error\n",
        ),
        (
            "include \"a/../b\"; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (a/../b)\n\njq: 1 compile error\n",
        ),
        (
            "import \"../a/a\" as m; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (../a/a)\n\njq: 1 compile error\n",
        ),
        (
            "import \"../a/a\" as $d; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (../a/a)\n\njq: 1 compile error\n",
        ),
        (
            "include \"../a/a\"; 1",
            "jq: error: Relative paths to modules may not traverse to parent directories (../a/a)\n\njq: 1 compile error\n",
        ),
        (
            "import \"b/b\" as m; 1",
            "jq: error: module names must not have equal consecutive components: b/b\n\njq: 1 compile error\n",
        ),
        (
            "import \"b/b\" as $d; 1",
            "jq: error: module names must not have equal consecutive components: b/b\n\njq: 1 compile error\n",
        ),
        (
            "include \"b/b\"; 1",
            "jq: error: module names must not have equal consecutive components: b/b\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/b/b\" as m; 1",
            "jq: error: module names must not have equal consecutive components: a/b/b\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/b/b\" as $d; 1",
            "jq: error: module names must not have equal consecutive components: a/b/b\n\njq: 1 compile error\n",
        ),
        (
            "include \"a/b/b\"; 1",
            "jq: error: module names must not have equal consecutive components: a/b/b\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/a/..\" as m; 1",
            "jq: error: module names must not have equal consecutive components: a/a/..\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/a/..\" as $d; 1",
            "jq: error: module names must not have equal consecutive components: a/a/..\n\njq: 1 compile error\n",
        ),
        (
            "include \"a/a/..\"; 1",
            "jq: error: module names must not have equal consecutive components: a/a/..\n\njq: 1 compile error\n",
        ),
        (
            "import \"a///b\" as m; 1",
            "jq: error: module names must not have equal consecutive components: a///b\n\njq: 1 compile error\n",
        ),
        (
            "import \"a///b\" as $d; 1",
            "jq: error: module names must not have equal consecutive components: a///b\n\njq: 1 compile error\n",
        ),
        (
            "include \"a///b\"; 1",
            "jq: error: module names must not have equal consecutive components: a///b\n\njq: 1 compile error\n",
        ),
        (
            "import \"//\" as m; 1",
            "jq: error: module names must not have equal consecutive components: //\n\njq: 1 compile error\n",
        ),
        (
            "import \"//\" as $d; 1",
            "jq: error: module names must not have equal consecutive components: //\n\njq: 1 compile error\n",
        ),
        (
            "include \"//\"; 1",
            "jq: error: module names must not have equal consecutive components: //\n\njq: 1 compile error\n",
        ),
        (
            "import \"/a/a\" as m; 1",
            "jq: error: module names must not have equal consecutive components: /a/a\n\njq: 1 compile error\n",
        ),
        (
            "import \"/a/a\" as $d; 1",
            "jq: error: module names must not have equal consecutive components: /a/a\n\njq: 1 compile error\n",
        ),
        (
            "include \"/a/a\"; 1",
            "jq: error: module names must not have equal consecutive components: /a/a\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/ / /b\" as m; 1",
            "jq: error: module names must not have equal consecutive components: a/ / /b\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/ / /b\" as $d; 1",
            "jq: error: module names must not have equal consecutive components: a/ / /b\n\njq: 1 compile error\n",
        ),
        (
            "include \"a/ / /b\"; 1",
            "jq: error: module names must not have equal consecutive components: a/ / /b\n\njq: 1 compile error\n",
        ),
        (
            "import \"a\\\\b\" as m; 1",
            "jq: error: Modules must be named by relative paths using '/', not '\\' (a\\b)\n\njq: 1 compile error\n",
        ),
        (
            "import \"a\\\\b\" as $d; 1",
            "jq: error: Modules must be named by relative paths using '/', not '\\' (a\\b)\n\njq: 1 compile error\n",
        ),
        (
            "include \"a\\\\b\"; 1",
            "jq: error: Modules must be named by relative paths using '/', not '\\' (a\\b)\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/b\\\\\" as m; 1",
            "jq: error: Modules must be named by relative paths using '/', not '\\' (a/b\\)\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/b\\\\\" as $d; 1",
            "jq: error: Modules must be named by relative paths using '/', not '\\' (a/b\\)\n\njq: 1 compile error\n",
        ),
        (
            "include \"a/b\\\\\"; 1",
            "jq: error: Modules must be named by relative paths using '/', not '\\' (a/b\\)\n\njq: 1 compile error\n",
        ),
        (
            "import \"a\\\\../b\" as m; 1",
            "jq: error: Modules must be named by relative paths using '/', not '\\' (a\\../b)\n\njq: 1 compile error\n",
        ),
        (
            "import \"a\\\\../b\" as $d; 1",
            "jq: error: Modules must be named by relative paths using '/', not '\\' (a\\../b)\n\njq: 1 compile error\n",
        ),
        (
            "include \"a\\\\../b\"; 1",
            "jq: error: Modules must be named by relative paths using '/', not '\\' (a\\../b)\n\njq: 1 compile error\n",
        ),
        (
            "import \"a//b\" as m; 1",
            "jq: error: module not found: a//b\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/./b\" as m; 1",
            "jq: error: module not found: a/./b\n\njq: 1 compile error\n",
        ),
        (
            "import \"x/y/x\" as m; 1",
            "jq: error: module not found: x/y/x\n\njq: 1 compile error\n",
        ),
        (
            "import \"/abs\" as m; 1",
            "jq: error: module not found: /abs\n\njq: 1 compile error\n",
        ),
        (
            "import \"b/\" as m; 1",
            "jq: error: module not found: b/\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/...\" as m; 1",
            "jq: error: module not found: a/...\n\njq: 1 compile error\n",
        ),
        (
            "import \"..a\" as m; 1",
            "jq: error: module not found: ..a\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/..b\" as m; 1",
            "jq: error: module not found: a/..b\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/ /b\" as m; 1",
            "jq: error: module not found: a/ /b\n\njq: 1 compile error\n",
        ),
        (
            "import \"x//x\" as m; 1",
            "jq: error: module not found: x//x\n\njq: 1 compile error\n",
        ),
        (
            "import \"A/a\" as m; 1",
            "jq: error: module not found: A/a\n\njq: 1 compile error\n",
        ),
        (
            "import \"a/.\" as m; 1",
            "jq: error: module not found: a/.\n\njq: 1 compile error\n",
        ),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-nc", program], None)?;
        assert_eq!(
            (stdout.as_str(), stderr.as_str(), code),
            ("", want_stderr, 3),
            "{program}"
        );
    }
    Ok(())
}
