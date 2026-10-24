#!/usr/bin/env python3
"""Three-way differential run for the #5223 Phase 0 go/no-go: jq 1.7.1, a
`main` build, and the path-mode spike, row by row.

Rows are JSON lines `{"args": [...], "stdin": "...", "source": "..."}`, where
`args` is the argv after the binary, `["jq", "-c", PROG]` as a sweep calls it
(a row for any other subcommand is skipped). They come from `scripts/jq-path-spike-rows/*.jsonl`
(hand-written repros) or from a directory `scripts/jq-path-spike-record.py`
filled while a sweep ran against `main`. Duplicates are run once.

Each row compares **stdout and exit code** with jq's, as the sweeps do:

  both          main and the spike match jq
  gain          only the spike matches jq
  LOSS          only main matches jq, and the spike's run touched no stub:
                the gate's failure class
  LOSS-stub     only main matches jq, on a run that answered a builtin
                through the fresh stub where jq's own version can hand back
                its input or navigates it (`path-spike: stubbed: ...`)
  neither       neither matches jq
  neither-stub  neither matches jq, on a stubbed run
  unsupported   the spike declined the row (exit 64, `path-spike: unsupported`)
  oracle-skip   jq timed out, so the row says nothing

A `-stub` row is the issue's "unstubbed builtin" case, which does not fail the
gate; a LOSS or a neither still needs tracing to a construct rule (which
falsifies the model) or to something else. `--out` writes every row that is
not `both`, with all three answers, for that tracing. Exit status 1 on any
LOSS.

Usage:
    cargo build --release --features cli          # the spike, this branch
    ./scripts/jq-path-spike-3way.py --main <main build> \
        --rows scripts/jq-path-spike-rows/repros.jsonl --rows <record dir> \
        [--sample N --seed S] [--jobs N] [--out FILE]
"""
import argparse
import collections
import concurrent.futures as cf
import json
import os
import random
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
UNSUPPORTED_EXIT = 64


def load_rows(paths):
    rows, seen = [], set()
    for path in paths:
        files = (
            [os.path.join(path, f) for f in sorted(os.listdir(path)) if f.endswith(".jsonl")]
            if os.path.isdir(path)
            else [path]
        )
        for name in files:
            with open(name, encoding="utf-8") as f:
                for line in f:
                    if not line.strip():
                        continue
                    row = json.loads(line)
                    args = row["args"]
                    if not args or args[0] != "jq":
                        continue  # another subcommand, or a probe of the binary
                    args = args[1:]
                    if any(a in ("--version", "-h", "--help") for a in args):
                        continue
                    key = (tuple(args), row.get("stdin", ""))
                    if key in seen:
                        continue
                    seen.add(key)
                    rows.append(
                        {"args": args, "stdin": row.get("stdin", ""), "source": row.get("source", name)}
                    )
    return rows


def run(argv, stdin, timeout):
    try:
        done = subprocess.run(
            argv, input=stdin.encode("utf-8", "surrogateescape"), capture_output=True, timeout=timeout
        )
    except subprocess.TimeoutExpired:
        return None
    return done.returncode, done.stdout.decode("utf-8", "replace"), done.stderr.decode("utf-8", "replace")


def classify(row, opts):
    jq = run([opts.jq] + row["args"], row["stdin"], opts.timeout)
    if jq is None:
        return "oracle-skip", row, None, None, None
    main = run([opts.main, "jq"] + row["args"], row["stdin"], opts.timeout)
    spike = run([opts.spike, "jq"] + row["args"], row["stdin"], opts.timeout)
    if spike is not None and spike[0] == UNSUPPORTED_EXIT and "path-spike: unsupported" in spike[2]:
        return "unsupported", row, jq, main, spike
    same = lambda r: r is not None and (r[0], r[1]) == (jq[0], jq[1])
    m, s = same(main), same(spike)
    cls = "both" if m and s else "gain" if s else "LOSS" if m else "neither"
    if cls in ("LOSS", "neither") and spike is not None and "path-spike: stubbed:" in spike[2]:
        cls += "-stub"
    return cls, row, jq, main, spike


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--rows", action="append", required=True, help="a .jsonl file or a record directory")
    ap.add_argument("--main", required=True, help="a succinctly build of main")
    ap.add_argument("--spike", default=os.path.join(HERE, "jq-path-spike.sh"))
    ap.add_argument("--jq", default="/usr/bin/jq")
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 4)
    ap.add_argument("--timeout", type=float, default=10.0)
    ap.add_argument("--sample", type=int, help="run a seeded random sample of this many rows")
    ap.add_argument("--seed", type=int, default=5223)
    ap.add_argument("--out", help="write every non-`both` row here as JSON lines")
    opts = ap.parse_args()

    rows = load_rows(opts.rows)
    total = len(rows)
    if opts.sample and opts.sample < total:
        rows = random.Random(opts.seed).sample(rows, opts.sample)
    print(f"rows: {len(rows)} of {total} distinct (seed {opts.seed})", file=sys.stderr)

    counts = collections.Counter()
    by_source = collections.defaultdict(collections.Counter)
    out = open(opts.out, "w", encoding="utf-8") if opts.out else None
    with cf.ThreadPoolExecutor(max_workers=opts.jobs) as pool:
        futures = [pool.submit(classify, row, opts) for row in rows]
        for done, fut in enumerate(cf.as_completed(futures), 1):
            cls, row, jq, main_r, spike = fut.result()
            counts[cls] += 1
            by_source[row["source"]][cls] += 1
            if out and cls != "both":
                show = lambda r: None if r is None else {"rc": r[0], "stdout": r[1], "stderr": r[2][:400]}
                out.write(
                    json.dumps(
                        {"class": cls, **row, "jq": show(jq), "main": show(main_r), "spike": show(spike)}
                    )
                    + "\n"
                )
            if done % 500 == 0:
                print(f"  {done}/{len(rows)} {dict(counts)}", file=sys.stderr)
    if out:
        out.close()

    order = ["both", "gain", "LOSS", "LOSS-stub", "neither", "neither-stub", "unsupported", "oracle-skip"]
    print("| class | rows |")
    print("|---|---|")
    for cls in order:
        print(f"| {cls} | {counts[cls]} |")
    if len(by_source) > 1:
        print()
        print("| source | " + " | ".join(order) + " |")
        print("|---|" + "---|" * len(order))
        for source in sorted(by_source):
            c = by_source[source]
            print(f"| {source} | " + " | ".join(str(c[k]) for k in order) + " |")
    sys.exit(1 if counts["LOSS"] else 0)


if __name__ == "__main__":
    main()
