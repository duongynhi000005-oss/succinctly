# BitVec Design

[Home](../../) > [Docs](../) > [Architecture](./) > BitVec

Design and implementation of the bit vector with rank/select support.

## Overview

`BitVec` is a bit vector that supports:
- O(1) rank operations
- sampled select: an O(1) jump to the nearest sample, then a short forward word scan
- **27.5-47.5% space overhead**, rising with bit density (see [Space Analysis](#space-analysis)) —
  *not* the ~3% of a compact Poppy index, a figure this page previously quoted from the paper

## Data Layout

```rust
pub struct BitVec {
    words: Vec<u64>,          // Raw bit storage
    len: usize,               // Number of valid bits
    ones_count: usize,        // Cached popcount
    rank_dir: RankDirectory,  // 3-level Poppy directory (l0 + cache-aligned l1/l2)
    select_idx: SelectIndex,  // One 16-byte sample per `select_sample_rate` ones
}
```

`heap_size()` reports the total of all three parts.

## Index Structure

The rank directory ([src/bits/rank.rs](../../src/bits/rank.rs)) is a 3-level Poppy-style layout:

| Level | Granularity                 | Stores                                                   | Width      |
|-------|-----------------------------|----------------------------------------------------------|------------|
| L0    | one entry per 2^32 bits     | absolute count of ones before the superblock             | `u64`      |
| L1    | one entry per 512-bit block | count of ones before the block, relative to its L0 entry | 32 bits    |
| L2    | words 1-7 of each block     | count of ones before the word, within the block          | 7 x 9 bits |

L1 and L2 for one block share a single `u128`: the low 32 bits hold L1 and bits 32-94 hold the
seven 9-bit L2 offsets (the largest is 7 x 64 = 448, which fits). Word 0 of a block needs no
offset. The `u128` entries live in a 64-byte-aligned allocation, four per cache line, so a rank
query touches one cache line of directory. L0 only gets an entry once a vector passes 2^32 bits
(512 MiB), so below that it is empty.

## Rank Algorithm

`RankDirectory::rank_at_word` returns the ones before a word; `rank1` adds the partial word:

```rust
fn rank1(&self, i: usize) -> usize {
    let word_idx = i / 64;
    let bit_idx = i % 64;

    // Directory: L0 + L1 + L2 for the ones before word_idx
    let block_idx = word_idx / 8;
    let word_in_block = word_idx % 8;
    let entry = l1_l2[block_idx];                        // one u128
    let l0 = if block_idx >= 1 << 23 { l0[block_idx / (1 << 23) - 1] } else { 0 };
    let l1 = (entry & 0xFFFF_FFFF) as u64;
    let l2 = if word_in_block == 0 {
        0
    } else {
        ((entry >> (32 + (word_in_block - 1) * 9)) & 0x1FF) as u64
    };

    // Partial word: popcount of the bits below i
    let mask = (1u64 << bit_idx) - 1;
    (l0 + l1 + l2) as usize + popcount_word(words[word_idx] & mask) as usize
}
```

(Simplified: the real code also clamps `i` to the vector's length and returns early for `0`.)

## Select Algorithm

`select1(k)` uses the sampled `SelectIndex` ([src/bits/select.rs](../../src/bits/select.rs)), not
the rank directory:

1. The index stores one `(word index, ones before that word)` pair per `select_sample_rate` set
   bits (256 by default; `Config::select_sample_rate` changes it). `jump_to(k)` reads sample
   `k / rate` in O(1) and returns the word to start from and how many ones are left to skip.
2. `scan_select` ([src/bits/scan.rs](../../src/bits/scan.rs)) walks forward, popcounting words
   (whole blocks at a time once the scan runs long) until the running total passes the target.
3. `select_in_word` finds the bit inside the crossing word.

The cost is the O(1) jump plus a scan that grows with the gap between samples, so it depends on
density: at most `rate` ones lie between the sample and the target, however many words hold them.

`select0` has no sample index of its own: it binary-searches `rank0`, so it is O(log n) (#602).

## Space Analysis

For n bits:

- Bits: n bits
- L1 + L2: one 128-bit entry per 512 bits = n/4 bits (**25%**)
- L0: 64 bits per 2^32 bits, negligible (zero below 2^32 bits)
- Select samples: 128 bits (two `u64`s) per `rate` set bits, i.e. 128 x d / rate of n for
  density d: 25% at d = 0.5, 2.5% at d = 0.05 with the default rate 256

So the rank directory costs a fixed 25%, and `heap_size()` adds the samples on top.

This is a deliberate trade of space for query speed, and is *not* the ~3% of a compact Poppy index;
see the note in [src/bits/rank.rs](../../src/bits/rank.rs). With the select samples a full `BitVec`
measures **27.5-47.5%** resident overhead, rising with bit density
([../benchmarks/rust-succinct-libs.md](../benchmarks/rust-succinct-libs.md)).

## SIMD Optimization

A single word is popcounted with `u64::count_ones()` (or, under the `portable-popcount` feature,
a broadword routine). On ARM64 that compiles to NEON `cnt`. On
x86_64 it is the `POPCNT` instruction only when the build enables that target feature (for example
`-C target-cpu=native`). This repo's builds set no `target-cpu`, so they get the baseline
`x86_64` target's bit-twiddling sequence instead. Bulk popcounts (`popcount_words`) choose an implementation at
compile time ([src/bits/popcount.rs](../../src/bits/popcount.rs)):
- `simd` feature: NEON intrinsics on ARM64; on x86_64, AVX-512 `VPOPCNTDQ` when the CPU has it
  (runtime detection, needs `std`), else a `count_ones()` loop
- default: a `count_ones()` loop that LLVM auto-vectorises
- `portable-popcount` feature: a broadword (SWAR) bit-counting loop with no intrinsics

## Used By

**No data path in the crate, as of [#228](https://github.com/rust-works/succinctly/issues/228).**
`BitVec` is public API — exported from the crate root, exercised by doctests and the serde
round-trip tests — but nothing parses, indexes or queries through it. Its one remaining caller is
measurement scaffolding: the corpus-shape report reconstructs a dense bitmap per file purely to
print what `LineIndex` saves against it ([src/bin/succinctly/corpus_stats.rs](../../src/bin/succinctly/corpus_stats.rs)).
This section previously listed four callers, none of which were accurate:

| Structure                             | What it actually stores                                                                                                                        |
|---------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------|
| [BalancedParens](balanced-parens.md)  | `words: W` (a plain `Vec<u64>` or borrowed slice) plus its own `rank_l1`/`rank_l2` arrays — never a `BitVec`                                   |
| [JsonIndex](../parsing/json-index.md) | interest bits as `W` plus a cumulative-rank `Vec<u32>`; the newline index moved to [`text::LineIndex`](../../src/text/lines.rs)                |
| [YamlIndex](../parsing/yaml-index.md) | same pattern, plus type bits; newline index likewise now `LineIndex`                                                                           |
| [DsvIndex](../parsing/dsv-index.md)   | `DsvIndexLightweight` — plain `Vec<u64>` markers and newlines with cumulative-rank `Vec<u32>`, measured 5-9x faster to iterate than a `BitVec` |

The parsers bypass `BitVec` because they need storage genericity (`W: AsRef<[u64]>` for mmap),
hinted select, or cheap sequential iteration, none of which it offers. See
[ADR-0011](../adrs/adr-0011.md) for why the structures are in-crate at all, and
[ADR-0012](../adrs/adr-0012.md) for why the last auxiliary uses moved to Elias-Fano.

## Academic Papers

- [Vigna 2008](https://vigna.di.unimi.it/ftp/papers/Broadword.pdf) — broadword rank/select algorithms
- [Zhou, Andersen, Kaminsky 2013](https://www.cs.cmu.edu/~dga/papers/zhou-sea2013.pdf) — Poppy structure (3-level directory)
- [Mula, Kurz, Lemire 2016](https://arxiv.org/abs/1611.07612) — Harley-Seal popcount with AVX2

## Source & Docs

- Implementation: [src/bits/](../../src/bits/) (bitvec.rs, rank.rs, select.rs, popcount.rs)
- Optimization techniques: [optimizations/bit-manipulation.md](../optimizations/bit-manipulation.md)

## See Also

- [Core Concepts](core-concepts.md) - Rank/select theory
- [Bit Manipulation](../optimizations/bit-manipulation.md) - Popcount techniques
