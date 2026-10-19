//! Per-issue jq CLI tests (#4329), compiled into the `jq_cli_tests` binary.
//!
//! `tests/jq_cli_tests.rs` is 160k+ lines and every new test used to be appended at its end,
//! so any two PRs in flight conflicted there. **Put a new test for issue `N` in
//! `tests/jq_cli/iN.rs` and add one `mod iN;` line to the list below, in numeric order.**
//! Two PRs for different issues then touch different files, and their `mod` lines land in
//! different gaps of the sorted list. `.gitattributes` also marks this file `merge=union`,
//! so two lines added into the same gap keep both instead of conflicting.
//!
//! - A module reaches the binary's shared helpers through `crate::`
//!   (`use crate::{run_jq_full, run_jq_stdin};`); they stay private to the binary, and every
//!   descendant module can see them.
//! - This directory is not a test target (Cargo discovers `tests/*.rs` and
//!   `tests/*/main.rs`, not `tests/*/mod.rs`), so it adds no binary and no link time.
//! - Existing tests stay where they are; do not move them (a mass move conflicts with every
//!   open PR). Extend an existing test in place when it already covers the same behaviour.
//! - A test that belongs with a topical group already in `jq_cli_tests.rs` (a `mod ... {}`
//!   block) may still go there, next to its siblings, but never at the end of the file.
//! - `scripts/deep-recursion-tests.sh` matches its tests by bare name with `--exact`, so a
//!   test it lists must stay at the root of `jq_cli_tests.rs`: inside this module its name
//!   would be `per_issue::iN::name` and neither the skip nor the run would find it.

mod i4800;
mod i4800_first_bind_sibling;
mod i4800_first_limit_if;
mod i4832;
mod i4886;
mod i4899;
mod i4901;
mod i4903;
mod i4909;
mod i4932;
