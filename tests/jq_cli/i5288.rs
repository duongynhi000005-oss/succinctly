//! #5288: a fold step whose operand reads the state into scalars (`.a += [.i]`) is read off the
//! tree instead of through a serialised, re-indexed copy of the whole state, which made every step
//! cost the size of the state. These pin the answers jq 1.7.1 gives (captured live), including the
//! operands that are not answered from the tree -- a float, a container read off the state, a
//! division -- and so must still spell the way the rebuilt route does. The cost itself is pinned by
//! `state_reading_operand_is_read_off_the_tree_5288` in `src/jq/eval.rs`, which counts the operands
//! that still go through the rebuilt state; a wall-clock bound here would only be flaky.

use anyhow::Result;

use crate::run_jq_full;

#[test]
fn test_state_reading_fold_operands_answer_as_jq_5288() -> Result<()> {
    for (filter, want) in [
        (
            "reduce range(5) as $k ({i:0,a:[]}; .i += 1 | .a += [.i])",
            "{\"i\":5,\"a\":[1,2,3,4,5]}",
        ),
        (
            "reduce range(5) as $k ({i:0,a:[]}; .a += [.i]) | .a",
            "[0,0,0,0,0]",
        ),
        (
            "reduce range(4) as $k ({i:2,j:\"x\",a:[]}; .a += [.i, .j]) | .a",
            "[2,\"x\",2,\"x\",2,\"x\",2,\"x\"]",
        ),
        (
            "reduce range(4) as $k ({n:10,a:[]}; .a += [.n > 3]) | .a",
            "[true,true,true,true]",
        ),
        (
            "reduce range(3) as $k ({b:true,z:null,a:[]}; .a += [.b, .z, .zz]) | .a",
            "[true,null,null,true,null,null,true,null,null]",
        ),
        (
            "reduce range(3) as $k ({o:{x:7},a:[]}; .a += [.o.x]) | .a",
            "[7,7,7]",
        ),
        (
            "reduce range(3) as $k ({f:1.5,a:[]}; .a += [.f]) | .a",
            "[1.5,1.5,1.5]",
        ),
        (
            "reduce range(3) as $k ({f:1.0,a:[]}; .a += [.f]) | .a",
            "[1.0,1.0,1.0]",
        ),
        (
            "reduce range(3) as $k ({e:1e3,a:[]}; .a += [.e]) | .a",
            "[1E+3,1E+3,1E+3]",
        ),
        (
            "reduce range(2) as $k ({big:100000000000000000000,a:[]}; .a += [.big]) | .a",
            "[100000000000000000000,100000000000000000000]",
        ),
        (
            "reduce range(3) as $k ({i:0,a:[]}; .i += 1 | .a += [.i, (.i > 1)]) | .a",
            "[1,false,2,true,3,true]",
        ),
        (
            "reduce range(3) as $k ({i:0,a:[]}; .a = [.i, $k] | .i += 1) | [.a, .i]",
            "[[2,2],3]",
        ),
        (
            "reduce range(3) as $k ({i:1,a:[]}; .a += [.i + 1]) | .a",
            "[2,2,2]",
        ),
        (
            "reduce range(3) as $k ({i:5,a:[]}; .a += [.i | not]) | .a",
            "[false,false,false]",
        ),
        (
            "reduce range(3) as $k ({i:5,a:[]}; .a += [.i | type]) | .a",
            "[\"number\",\"number\",\"number\"]",
        ),
        (
            "reduce range(3) as $k ({s:\"hi\",a:[]}; .a += [.s]) | .a",
            "[\"hi\",\"hi\",\"hi\"]",
        ),
        (
            "reduce range(2) as $k ({a:[[1]]}; .a += [.a]) | .a",
            "[[1],[[1]],[[1],[[1]]]]",
        ),
        (
            "reduce range(3) as $k ({neg:-4,a:[]}; .a += [.neg]) | .a",
            "[-4,-4,-4]",
        ),
        (
            "[limit(3; foreach range(5) as $k ({i:0,a:[]}; .i += 1 | .a += [.i]; .a))]",
            "[[1],[1,2],[1,2,3]]",
        ),
        (
            "reduce range(3) as $k ({i:1,n:10,a:[]}; .a += [.i * .n - 2]) | .a",
            "[8,8,8]",
        ),
        (
            "reduce range(3) as $k ({i:0,a:[]}; .i += 1 | .a += [.i + 1, .i * .i]) | .a",
            "[2,1,3,4,4,9]",
        ),
        (
            "reduce range(3) as $k ({neg:-4,a:[]}; .a += [.neg - 1, .neg * .neg]) | .a",
            "[-5,16,-5,16,-5,16]",
        ),
        (
            "reduce range(2) as $k ({m:9007199254740992,a:[]}; .a += [.m + 1]) | .a",
            "[9007199254740992,9007199254740992]",
        ),
        (
            "reduce range(2) as $k ({m:9007199254740992,a:[]}; .a += [.m * 2]) | .a",
            "[18014398509481984,18014398509481984]",
        ),
        (
            "reduce range(2) as $k ({m:9007199254740993,a:[]}; .a += [.m + 0]) | .a",
            "[9007199254740992,9007199254740992]",
        ),
        (
            "reduce range(2) as $k ({f:1.5,a:[]}; .a += [.f + 1]) | .a",
            "[2.5,2.5]",
        ),
        (
            "reduce range(2) as $k ({i:7,a:[]}; .a += [.i / 2, .i % 4]) | .a",
            "[3.5,3,3.5,3]",
        ),
        (
            "reduce range(2) as $k ({s:\"a\",a:[]}; .a += [.s + \"x\"]) | .a",
            "[\"ax\",\"ax\"]",
        ),
        (
            "reduce range(2) as $k ({z:null,a:[]}; .a += [.z + 1, .zz + 1]) | .a",
            "[1,1,1,1]",
        ),
        (
            "reduce range(3) as $k ({i:0,a:[]}; .i += 1 | .a += [.i - 1]) | .a",
            "[0,1,2]",
        ),
        // The other callers of the operand: a key, a condition, a path, the other compound operators.
        (
            "reduce range(3) as $k ({i:0,a:[]}; .i += 1 | .a[.i] = $k) | .a",
            "[null,0,1,2]",
        ),
        (
            "reduce range(4) as $k ({n:0,a:[]}; if .n > 1 then .a += [.n] else .n += 1 end)",
            "{\"n\":2,\"a\":[2,2]}",
        ),
        (
            "reduce range(3) as $k ({i:0,a:{}}; .i += 1 | setpath([\"a\",\"x\"]; .i * 2)) | .a",
            "{\"x\":6}",
        ),
        (
            "reduce range(3) as $k ({i:2,n:10}; .n -= .i)",
            "{\"i\":2,\"n\":4}",
        ),
        (
            "reduce range(3) as $k ({i:2,n:10}; .n *= .i)",
            "{\"i\":2,\"n\":80}",
        ),
        (
            "reduce range(3) as $k ({\"i\":2}; .z //= .i)",
            "{\"i\":2,\"z\":2}",
        ),
        (
            "reduce range(3) as $k ({i:0,m:{}}; .i += 1 | .m[(.i|tostring)] = .i)",
            "{\"i\":3,\"m\":{\"1\":1,\"2\":2,\"3\":3}}",
        ),
        (
            "[foreach range(3) as $k ({i:0,a:[]}; .i += 1 | .a += [.i * 2]; .a)]",
            "[[2],[2,4],[2,4,6]]",
        ),
    ] {
        let (stdout, stderr, code) = run_jq_full(&["-nc", filter], None)?;
        assert_eq!(code, 0, "{filter}: stderr: {stderr:?}");
        assert_eq!(stdout.trim_end(), want, "{filter}");
    }
    Ok(())
}
