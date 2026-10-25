//! The galloping select over an interest-bit rank directory, shared by
//! `JsonIndex::ib_select1_from` and `YamlIndex::ib_select1_from` (#2580).
//!
//! The two indices used to hold line-for-line copies of this search, and #2578 changed how the
//! JSON copy is *seeded* while the YAML one stayed behind -- duplicated predicates diverge
//! silently (#106). One definition, plus a test that both call sites agree
//! (`tests/ib_select1_agreement.rs`).

use crate::util::broadword::select_in_word;

/// Select the `k`-th (0-indexed) set bit of `words`, galloping outward from word `hint`.
///
/// `ib_rank[i]` is the number of set bits in `words[..i]` (so it holds `words.len() + 1`
/// entries) and `ib_len` the number of valid bits. Returns the bit position and the number of
/// `ib_rank` probes made (#40), or `None` when there is no such bit.
///
/// Exponential search from the hint is O(log d) for a seed `d` words from the answer, which is
/// what makes a sequential walk cheap; a random ask costs O(log n) with ~37% overhead over a
/// plain bisection.
#[inline(always)]
pub(crate) fn select1_from(
    words: &[u64],
    ib_rank: &[u32],
    ib_len: usize,
    k: usize,
    hint: usize,
) -> Option<(usize, usize)> {
    if words.is_empty() {
        return None;
    }

    let k32 = k as u32;
    let n = words.len();

    // Counts `ib_rank` probes; starts at 1 for the `hint_rank` probe below. Callers that do not
    // record it let the optimizer drop the counter.
    let mut probes = 1usize;

    // Clamp hint to valid range
    let hint = hint.min(n.saturating_sub(1));

    // Check if hint is already past k
    let hint_rank = ib_rank[hint + 1];
    let lo;
    let hi;

    if hint_rank <= k32 {
        // k is at or after hint - search forward with exponential expansion
        let mut bound = 1usize;
        let mut prev = hint;

        // Gallop forward: double the step size until we overshoot
        loop {
            probes += 1;
            let next = (hint + bound).min(n);
            if next >= n || ib_rank[next + 1] > k32 {
                // Found the range: [prev, next]
                lo = prev;
                hi = next;
                break;
            }
            prev = next;
            bound *= 2;
        }
    } else {
        // k is before hint - search backward with exponential expansion
        let mut bound = 1usize;
        let mut prev = hint;

        // Gallop backward
        loop {
            probes += 1;
            let next = hint.saturating_sub(bound);
            if next == 0 || ib_rank[next + 1] <= k32 {
                // Found the range: [next, prev]
                lo = next;
                hi = prev;
                break;
            }
            prev = next;
            bound *= 2;
        }
    }

    // Binary search within [lo, hi]
    let mut lo = lo;
    let mut hi = hi;
    while lo < hi {
        probes += 1;
        let mid = lo + (hi - lo) / 2;
        if ib_rank[mid + 1] <= k32 {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    if lo >= n {
        return None;
    }

    // Now lo is the word index, and ib_rank[lo] is count before this word
    let remaining = k - ib_rank[lo] as usize;
    let bit_pos = select_in_word(words[lo], remaining as u32) as usize;
    let result = lo * 64 + bit_pos;

    if result < ib_len {
        Some((result, probes))
    } else {
        None
    }
}
