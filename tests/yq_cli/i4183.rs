//! #4183: yq's `,` skips its right operand when both operands evaluate to the very same list (a pointer
//! compare in `unionOperator`), which a variable read does: `GET_VARIABLE` hands back the list the variable is
//! stored in, so `$z, $z` collapses as `., .` does. And the body of `X as $v | body` is a pipe chain, which
//! binds tighter than `,` and `and`/`or` in yq, so the binding does not reach past a comma: `1 as $z | $z, $y`
//! is `(1 as $z | $z), $y`.
//!
//! Every output is captured from yq v4.53.3 (`-n -o=json -I0`, outputs joined by a space).

use anyhow::Result;

use crate::run_yq_stdin;

/// (filter, expected outputs, one per line).
const ROWS: &[(&str, &str)] = &[
    ("[1] as $a | ($a, $a)", "[1]\n"),
    ("[1] as $a | [$a, $a]", "[[1]]\n"),
    ("[1] as $a | [($a), $a]", "[[1]]\n"),
    ("[1] as $a | [$a, ($a)]", "[[1]]\n"),
    ("[1] as $a | [($a), (($a))]", "[[1]]\n"),
    ("[1] as $a | [($a | .), $a]", "[[1]]\n"),
    ("[1] as $a | [$a, $a | .]", "[[1]]\n"),
    ("[1] as $a | [($a | .), ($a | .)]", "[[1]]\n"),
    ("[[1]] as $z | [$z, $z]", "[[[1]]]\n"),
    ("[1] as $a | [$a, $a, $a]", "[[1],[1]]\n"),
    ("[1] as $a | [$a, $a, $a, $a]", "[[1],[1],[1]]\n"),
    ("1 as $a | [$a, $a]", "[1]\n"),
    ("(1,2) as $v | [$v, $v]", "[1]\n[2]\n"),
    ("[1] as $a | [1] as $b | ($a, $b)", "[1]\n[1]\n"),
    ("[1] as $a | [., $a]", "[null,[1]]\n"),
    ("[1] as $a | [. | $a, $a]", "[[1],[1]]\n"),
    ("[1] as $a | [$a | $a, $a]", "[[1],[1]]\n"),
    ("[1] as $a | [$a, . | $a]", "[[1],[1]]\n"),
    ("[1] as $a | [.a | $a, $a]", "[[1],[1]]\n"),
    ("[1] as $a | [(. | $a), (. | $a)]", "[[1],[1]]\n"),
    ("1 as $a | 2 as $b | [$a, $b]", "[1,2]\n"),
    ("[1] as $a | $a, $a", "[1]\n"),
    ("1 as $z | 2 as $y | $z, $y", "1\n"),
    ("1 as $z | 2 as $y | ($z, $y)", "1\n2\n"),
    ("1 as $a | 2 as $b | $a, $b, 3", "1\n3\n"),
    ("1 as $a | (2 as $b | $a, $b), $a", "1\n"),
    ("1 as $x | 2 as $y | $x + $y, $x", "3\n"),
    ("1 as $z | $z + 1, $z + 2", "2\n2\n"),
    ("1 as $z | 10, $z + 1", "10\n1\n"),
    ("true as $z | false or $z", "false\n"),
    ("false as $z | true and $z", "false\n"),
    ("[1,2] as $z | $z[0], $z[1]", "1\n"),
    ("1 as $z | $z | . + 1, 5", "2\n5\n"),
    ("[[1,2]] as $q | $q | .[] as $r | $r, 9", "[1,2]\n9\n"),
    ("[1] as $a | ., $a", "null\n"),
];

#[test]
fn test_4183_a_variable_union_and_the_scope_of_an_as_body_follow_yq() -> Result<()> {
    for (filter, expected) in ROWS {
        let (out, code) = run_yq_stdin(filter, "null\n", &["-o=json", "-I0"])?;
        assert_eq!(code, 0, "{filter}");
        assert_eq!(out, *expected, "{filter}");
    }
    Ok(())
}
