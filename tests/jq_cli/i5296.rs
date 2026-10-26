//! #5296: a later `import ... as f` shadows an earlier one under the same alias, for the names it
//! defines -- `f::e` answers from the last module defining `e`, and a name only the earlier module
//! defines is still reachable. The first import won. Every answer below was captured from jq 1.7.1
//! on the same modules, including a repeated alias inside a module, one module under
//! several aliases, and an `include` beside an `import`.

use anyhow::Result;

use crate::run_jq_with_modules;

const MODULES: &[(&str, &str)] = &[
    ("s1", "def e: 1;"),
    ("s2", "def e: 2;"),
    ("s3", "def g: 3;"),
    (
        "s4",
        "import \"s1\" as inner; def e: inner::e + 10; def h: e + 100;",
    ),
    ("s5", "include \"s1\"; def e2: e + 1000;"),
    ("s6", "import \"s2\" as inner; def k: inner::e;"),
    ("m1", "def e: \"e1\"; def only1: 1;"),
    ("m2", "def e: \"e2\";"),
    ("s7", "import \"s1\" as f; import \"s2\" as f; def k: f::e;"),
    (
        "s8",
        "import \"s2\" as f; import \"s1\" as f; def k: f::e; def kk: k;",
    ),
    (
        "s9",
        "import \"s4\" as f; import \"s2\" as f; import \"s4\" as f; def k: [f::e, f::h];",
    ),
];

#[test]
fn test_a_later_import_under_the_same_alias_shadows_an_earlier_one_5296() -> Result<()> {
    for (program, want) in [
        ("import \"s1\" as f; import \"s2\" as f; f::e", "2\n"),
        (
            "import \"m1\" as f; import \"m2\" as f; [f::e, f::only1]",
            "[\"e2\",1]\n",
        ),
        (
            "import \"m2\" as f; import \"m1\" as f; [f::e, f::only1]",
            "[\"e1\",1]\n",
        ),
        (
            "import \"s1\" as f; import \"s2\" as f; import \"s3\" as f; [f::e, f::g]",
            "[2,3]\n",
        ),
        (
            "import \"s2\" as f; import \"s1\" as f; import \"s3\" as f; [f::e, f::g]",
            "[1,3]\n",
        ),
        (
            "import \"s1\" as f; import \"s2\" as f; import \"s1\" as f; f::e",
            "1\n",
        ),
        (
            "import \"s4\" as f; import \"s2\" as f; [f::e, f::h]",
            "[2,111]\n",
        ),
        (
            "import \"s2\" as f; import \"s4\" as f; [f::e, f::h]",
            "[11,111]\n",
        ),
        (
            "import \"s4\" as a; import \"s6\" as b; import \"s2\" as a; [a::e, b::k, a::h]",
            "[2,2,111]\n",
        ),
        (
            "import \"s2\" as f; import \"s5\" as f; [f::e, f::e2]",
            "[2,1001]\n",
        ),
        (
            "import \"s1\" as f; import \"s2\" as f; def e: 77; [f::e, e]",
            "[2,77]\n",
        ),
        (
            "import \"s1\" as f; import \"s2\" as f; def g(x): f::e + x; g(1)",
            "3\n",
        ),
        (
            "import \"s1\" as f; import \"s2\" as f; [limit(3; repeat(f::e))]",
            "[2,2,2]\n",
        ),
        ("import \"s1\" as f; include \"s2\"; [f::e, e]", "[1,2]\n"),
        (
            "import \"s7\" as m; m::k",
            "2\n",
        ),
        (
            "import \"s8\" as m; [m::k, m::kk]",
            "[1,1]\n",
        ),
        (
            "import \"s9\" as m; m::k",
            "[11,111]\n",
        ),
        (
            "import \"s7\" as m; import \"s8\" as m; [m::k, m::kk]",
            "[1,1]\n",
        ),
        (
            "import \"s8\" as m; import \"s7\" as m; [m::k, m::kk]",
            "[2,1]\n",
        ),
        (
            "import \"s4\" as f; import \"s2\" as f; import \"s4\" as f; [f::e, f::h]",
            "[11,111]\n",
        ),
        (
            "import \"s4\" as f; import \"s2\" as f; import \"s6\" as f; import \"s4\" as f; [f::e, f::h, f::k]",
            "[11,111,2]\n",
        ),
        (
            "import \"s1\" as f; import \"s2\" as g; import \"s1\" as g; [f::e, g::e]",
            "[1,1]\n",
        ),
        (
            "import \"s4\" as f; import \"s4\" as g; import \"s2\" as g; [f::e, g::e, g::h]",
            "[11,2,111]\n",
        ),
        (
            "import \"s2\" as g; import \"s4\" as f; import \"s4\" as g; [f::e, g::e, g::h]",
            "[11,11,111]\n",
        ),
        (
            "import \"s1\" as f; include \"s2\"; import \"s2\" as f; [f::e, e]",
            "[2,2]\n",
        ),
        (
            "include \"s4\"; import \"s2\" as f; import \"s4\" as f; [e, h, f::e, f::h]",
            "[11,111,11,111]\n",
        ),
        (
            "import \"s4\" as f; import \"s6\" as g; import \"s6\" as f; import \"s4\" as g; [f::e, f::k, g::e, g::k]",
            "[11,2,11,2]\n",
        ),
    ] {
        let (stdout, stderr, code) = run_jq_with_modules(MODULES, &["-nc", program])?;
        assert_eq!((stdout.as_str(), code), (want, 0), "{program}: {stderr}");
    }
    Ok(())
}
