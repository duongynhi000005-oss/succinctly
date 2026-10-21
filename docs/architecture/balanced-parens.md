# Balanced Parentheses

[Home](../../) > [Docs](../) > [Architecture](./) > Balanced Parentheses

Succinct tree representation using balanced parentheses.

## Overview

`BalancedParens` encodes trees as sequences of opening and closing parentheses, supporting O(1) navigation operations.

## Tree Encoding

Any tree can be represented as balanced parentheses:

```
Tree Structure:
       A
      /|\
     B C D
       |
       E

Encoding (depth-first):
A( B() C( E() ) D() )

As bits (1=open, 0=close):
1 1 0 1 1 0 0 1 0 0
A B   C E     D
```

## Data Structure

```rust
pub struct BalancedParens<W = Vec<u64>, S: SelectSupport = NoSelect> {
    words: W,                  // Parentheses as bits (1=open, 0=close)
    len: usize,                // Number of valid bits
    total_ones: usize,         // Cached count of open parens

    // Three-level min-excess index for find_close
    l0_min_excess: Vec<i8>,    l0_word_excess: Vec<i16>,   // per word
    l1_min_excess: Vec<i16>,   l1_block_excess: Vec<i16>,  // per 32 words
    l2_min_excess: Vec<i32>,   l2_block_excess: Vec<i32>,  // per 1024 words

    // Cumulative rank directory for O(1) rank1()
    rank_l1: Vec<u32>,         // absolute count per 512-bit block
    rank_l2: Vec<u64>,         // 7 x 9-bit within-block offsets

    select: S,                 // ZST for NoSelect; WithCsPoppy (#64) or deprecated WithSelect (P11)
}
```

Note it is **not** built on [`BitVec`](bitvec.md): it holds plain words plus its own rank arrays, so
that `W` can be a borrowed slice (mmap) and so that select support is a compile-time choice. See
[ADR-0011](../adrs/adr-0011.md#consequences).

## Core Operations

### find_close(i)

Find the matching closing parenthesis for position i.

Algorithm:
1. If the next bit is a close, `i` is a leaf and `i + 1` is the answer. Leaves are most of a
   document's nodes (every key and scalar), so this is checked before anything else: it takes a
   sibling hop from ~170 instructions to a word load (#3140, −14% instructions on `jq .` over a wide
   object)
2. Otherwise track "excess" (opens - closes)
3. Find first position where excess drops to starting level
4. Use RangeMin index for O(1) amortized lookup

### find_open(i)

Find the matching opening parenthesis for position i.

Algorithm: Reverse of find_close, scan backwards.

### enclose(i)

Find the parent of the node at position i.

Algorithm: Find the first unmatched open paren before i.

## RangeMin Index

For O(1) find_close, we precompute range minimum queries:

```
Bits:     1 1 0 1 1 0 0 1 0 0
Excess:   1 2 1 2 3 2 1 2 1 0
Position: 0 1 2 3 4 5 6 7 8 9

RangeMin[0..4] = 1 (minimum excess in range)
```

The index is a 3-level hierarchy, not a sparse table. Each level stores, per block, the block's
total excess and the minimum excess reached inside it (both relative to the block's start):

| Level | Block           | Fields                                       |
|-------|-----------------|----------------------------------------------|
| L0    | one 64-bit word | `l0_min_excess: i8`, `l0_word_excess: i16`   |
| L1    | 32 words        | `l1_min_excess: i16`, `l1_block_excess: i16` |
| L2    | 1,024 words     | `l2_min_excess: i32`, `l2_block_excess: i32` |

`find_close` skips any block whose minimum cannot bring the running excess back to the target,
climbing to L1 and L2 to skip larger runs, and scans bits only inside the block that holds the
match. `enclose` uses the same arrays backwards (#4102). `find_open` does not: it scans backwards bit
by bit.

## Applications

### JSON Navigation

```
JSON: {"a": [1, 2]}

BP:   ( ( ( ) ( ) ) )
       { a [ 1   2 ] }
```

### Tree Queries

- Parent: `enclose(i)`
- First child: `i + 1` if open
- Next sibling: `find_close(i) + 1`
- Subtree size: `(find_close(i) - i + 1) / 2`

## Space Overhead

For n nodes the parentheses take 2n bits. The indices, as a share of that bitmap, are:

| Part                        | Fields                                      | Bits per      | Overhead  |
|-----------------------------|---------------------------------------------|---------------|-----------|
| L0 min-excess               | `l0_min_excess: i8` + `l0_word_excess: i16` | 24 per 64     | 37.5%     |
| L1 min-excess               | `l1_min_excess` + `l1_block_excess` (`i16`) | 32 per 2,048  | 1.6%      |
| L2 min-excess               | `l2_min_excess` + `l2_block_excess` (`i32`) | 64 per 65,536 | 0.1%      |
| Rank directory              | `rank_l1: u32` + `rank_l2: u64`             | 96 per 512    | 18.75%    |
| **Total (`NoSelect`)**      |                                             |               | **≈ 58%** |
| Select (`WithCsPoppy`, #64) | one `u32` per 256 opens                     | 32 per 512    | +6.25%    |

`BalancedParens::index_heap_size()` reports these bytes (it leaves out the parenthesis words, which
`W` may borrow). A unit test pins 9,488 bytes of index for a 16,384-byte bitmap, 57.9%. JSON uses
`NoSelect` (≈ 58%); YAML uses `WithCsPoppy` (≈ 64%).

Most of it is L0: two values per 64-bit word. `l0_word_excess` ranges over [-64, 64], so it is
stored wider than it needs to be. This page used to say "~6%", which matches no subset of these
fields; it may date from an earlier index.

## Implementation Optimizations

The RangeMin index construction uses SIMD acceleration on supported platforms:

| Platform | Instruction       | Speedup               | Notes                                 |
|----------|-------------------|-----------------------|---------------------------------------|
| ARM64    | NEON VMINV        | **2.8x**              | Direct signed horizontal minimum      |
| x86_64   | SSE4.1 PHMINPOSUW | **1-3%** (large data) | Requires bias trick for signed values |

ARM NEON provides `vminvq_s16` which directly computes the minimum across 8 signed 16-bit values. SSE4.1's `PHMINPOSUW` only handles unsigned values, requiring a bias/unbias workaround that adds overhead.

See [SIMD Optimizations](../optimizations/simd.md#x86-sse41-horizontal-minimum-phminposuw) for implementation details.

### Generic Select Support

`BalancedParens<W, S>` is generic over select support:
- `NoSelect` (ZST) — used by JSON, zero overhead
- `WithCsPoppy` — used by YAML for `at_offset`/`yq-locate`: combined-sampling select1 that
  reads entry points into BP's own rank directory instead of a parallel sampled index (#64)
- `WithSelect` — deprecated in favour of `WithCsPoppy`; sampled `(word, cumulative)` pairs (P11)

## Used By

- [JsonIndex](../parsing/json-index.md) — encodes JSON nesting structure (objects, arrays, values)
- [YamlIndex](../parsing/yaml-index.md) — encodes virtual brackets from indentation
- [DsvIndex](../parsing/dsv-index.md) — encodes row/field structure

## Depends On

Nothing in [`bits/`](../../src/bits/) — the parenthesis sequence is a plain `Vec<u64>` (or borrowed
slice) and the RangeMin and rank indices are its own arrays. See
[BitVec](bitvec.md#used-by) for why it does not use that type.

## Academic Papers

- Sadakane & Navarro 2010 — fully-functional succinct trees, RangeMin for O(1) navigation
- Navarro & Sadakane 2014 — space-optimal BP representation

## Source & Docs

- Implementation: [src/trees/bp.rs](../../src/trees/bp.rs)
- Optimization: [optimizations/hierarchical-structures.md](../optimizations/hierarchical-structures.md)

## See Also

- [Core Concepts](core-concepts.md) - Theory background
- [Semi-Indexing](semi-indexing.md) - How BP is used in parsing
- [Hierarchical Structures](../optimizations/hierarchical-structures.md) - Index optimization
