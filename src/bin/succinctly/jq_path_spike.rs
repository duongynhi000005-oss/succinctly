//! `succinctly dev jq-path-spike` (hidden, #5223 Phase 0): run a filter
//! through the register-carrying path-mode spike
//! (`succinctly::jq::path_spike`) instead of the shipped evaluator.
//!
//! Only the surface the differential sweeps use: one JSON document on stdin
//! (or a file), `-c`, `-r` and `-n`. Output is always compact. An uncaught
//! error exits 5, as `jq` does; a construct the spike does not implement
//! exits [`UNSUPPORTED_EXIT`] with a `path-spike: unsupported` diagnostic, so
//! `scripts/jq-path-spike-3way.py` can count those rows apart from verdicts.
//! `scripts/jq-path-spike.sh` maps a sweep's `BIN jq -c PROG` argv onto it.

use std::io::{Read, Write};
use std::path::PathBuf;

use clap::Parser;
use succinctly::jq::path_spike::{self, Outcome};
use succinctly::jq::{JqSemantics, OwnedValue};
use succinctly::json::JsonIndex;

/// Exit status for a row the spike cannot run.
const UNSUPPORTED_EXIT: i32 = 64;

/// Arguments for `dev jq-path-spike`.
#[derive(Debug, Parser)]
pub struct JqPathSpikeArgs {
    /// Compact output (always on; accepted for argv compatibility with `jq`)
    #[arg(short = 'c', long = "compact-output")]
    compact: bool,
    /// Print strings without quotes
    #[arg(short = 'r', long = "raw-output")]
    raw: bool,
    /// Use `null` as the single input
    #[arg(short = 'n', long = "null-input")]
    null_input: bool,
    /// The jq filter
    filter: String,
    /// Input file (default: stdin)
    file: Option<PathBuf>,
}

/// Run the spike and return the process exit status.
pub fn run(args: JqPathSpikeArgs) -> i32 {
    let _ = args.compact;
    let bytes = if args.null_input {
        None
    } else {
        let mut bytes = Vec::new();
        let read = match &args.file {
            Some(path) => std::fs::File::open(path).and_then(|mut f| f.read_to_end(&mut bytes)),
            None => std::io::stdin().read_to_end(&mut bytes),
        };
        if let Err(e) = read {
            eprintln!("jq: error: could not read input: {e}");
            return 2;
        }
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return 0;
        }
        Some(bytes)
    };
    // A continuation-threaded evaluator nests a frame per pipe stage and per
    // output, so it runs on a thread with room for deep documents (values are
    // `Rc`, so the input is parsed there too).
    let (filter, raw) = (args.filter, args.raw);
    let worker = std::thread::Builder::new()
        .stack_size(1 << 29)
        .spawn(move || {
            let input = match bytes {
                None => OwnedValue::Null,
                Some(bytes) => {
                    let index = JsonIndex::build(&bytes);
                    let cursor = index.root(&bytes);
                    match succinctly::jq::eval_generic::to_owned::<JqSemantics, _>(&cursor.value())
                    {
                        Ok(value) => value,
                        Err(e) => {
                            eprintln!("jq: error (at <stdin>:0): {}", e.message);
                            return 2;
                        }
                    }
                }
            };
            report(&filter, &input, raw)
        })
        .expect("spawn the spike's thread");
    worker.join().unwrap_or(UNSUPPORTED_EXIT)
}

fn report(filter: &str, input: &OwnedValue, raw: bool) -> i32 {
    let run = match path_spike::run(filter, input) {
        Ok(run) => run,
        Err(e) => {
            eprintln!("jq: error: {e}\njq: 1 compile error");
            return 3;
        }
    };
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for value in &run.outputs {
        let text = match value {
            OwnedValue::String(s) if raw => s.as_str().to_string(),
            other => other.to_json_jq_preserve(),
        };
        let _ = writeln!(out, "{text}");
    }
    let _ = out.flush();
    if !run.stubbed.is_empty() {
        eprintln!("path-spike: stubbed: {}", run.stubbed.join(","));
    }
    match run.outcome {
        Outcome::Done => 0,
        Outcome::Error(payload) => {
            match &payload {
                OwnedValue::String(s) => eprintln!("jq: error (at <unknown>): {}", s.as_str()),
                other => eprintln!(
                    "jq: error (at <unknown>) (not a string): {}",
                    other.to_json_jq_preserve()
                ),
            }
            5
        }
        Outcome::Halt(code) => code,
        Outcome::Unsupported(what) => {
            eprintln!("path-spike: unsupported: {what}");
            UNSUPPORTED_EXIT
        }
    }
}
