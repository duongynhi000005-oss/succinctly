#!/usr/bin/env bash
# The #5223 Phase 0 path-mode spike under jq's argv, so a differential sweep can
# take it as its `--bin`/`--candidate` unchanged: `jq-path-spike.sh jq -c PROG`
# runs `succinctly dev jq-path-spike -c -- PROG` (stdin is passed through).
#
# The binary is $SUCCINCTLY_SPIKE_BIN, else target/release/succinctly in this
# checkout (build it with `cargo build --release --features cli`). An option
# the spike does not take exits 64 with a `path-spike: unsupported` line, the
# status `scripts/jq-path-spike-3way.py` counts apart from verdicts.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
bin=${SUCCINCTLY_SPIKE_BIN:-$root/target/release/succinctly}
if [[ ${1:-} == jq ]]; then shift; fi
opts=()
while (($#)); do
  case $1 in
    -c | -r | -n | --compact-output | --raw-output | --null-input) opts+=("$1") ;;
    -[crn][crn] | -[crn][crn][crn]) for ((i = 1; i < ${#1}; i++)); do opts+=("-${1:i:1}"); done ;;
    --) shift; break ;;
    -[a-zA-Z]* | --[a-z]*) echo "path-spike: unsupported: option $1" >&2; exit 64 ;;
    *) break ;;
  esac
  shift
done
exec "$bin" dev jq-path-spike "${opts[@]}" -- "$@"
