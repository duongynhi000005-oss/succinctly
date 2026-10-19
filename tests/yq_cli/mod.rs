//! Per-issue yq CLI tests (#4329), compiled into the `yq_cli_tests` binary.
//!
//! `tests/yq_cli_tests.rs` is 70k+ lines and every new test used to be appended at its end,
//! so any two PRs in flight conflicted there. **Put a new test for issue `N` in
//! `tests/yq_cli/iN.rs` and add one `mod iN;` line to the list below, in numeric order.**
//! Two PRs for different issues then touch different files, and their `mod` lines land in
//! different gaps of the sorted list. `.gitattributes` also marks this file `merge=union`,
//! so two lines added into the same gap keep both instead of conflicting.
//!
//! - A module reaches the binary's shared helpers through `crate::`
//!   (`use crate::{run_yq_stdin, run_yq_stdin_with_stderr};`, or `use crate::run_yq_stdin;` for one); they stay private to the
//!   binary, and every descendant module can see them.
//! - This directory is not a test target (Cargo discovers `tests/*.rs` and
//!   `tests/*/main.rs`, not `tests/*/mod.rs`), so it adds no binary and no link time.
//! - Existing tests stay where they are; do not move them (a mass move conflicts with every
//!   open PR; `i4136` is the one exception, moved here as this layout's first resident).
//!   Extend an existing test in place when it already covers the same behaviour.
//! - A test that belongs with a topical group already in `yq_cli_tests.rs` (a `mod ... {}`
//!   block) may still go there, next to its siblings, but never at the end of the file.

mod i1128;
mod i1129;
mod i2126;
mod i3576;
mod i4136;
mod i4206;
mod i4386;
mod i4856;
mod i4865;
mod i4886;
mod i4899;
mod i4903;
mod i4905;
mod i4909;
mod i4916;
mod i4922;
mod i4926;
