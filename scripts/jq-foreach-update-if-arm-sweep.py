#!/usr/bin/env python3
"""Differential grid for an `if`/`elif` in a `foreach` UPDATE that has a caught foreign-`$x` fold in one arm (#5115).

The #4800 slices (#5084, #5093, #5099, #5104) changed how the arms of an `if` in a `foreach` UPDATE
are resolved when one arm is a literal and the other a `foreach` whose source destructures an outer
`$x` under `try`. Each justified the change with a grid against `/usr/bin/jq` that was never committed;
this is that grid. `jq-fold-body-register-grid-sweep.py` has no `if` UPDATE, so it cannot gate this one.

Grid: UPDATE (`if C then A else B end`, `C` in `.a`, `.`, `.b`, a literal in one arm and the fold FE in
the other, both orders, plus `elif` chains) x FE (a `try`-caught `$x` destructure, a `?//` chain) x
wrapper (none, `try .. catch`, comma siblings, `?`, `| .`, `label`) x loop source x EXTRACT x INIT x
write context x document. The full product is large: `--n` samples it deterministically (`--seed`).

Rows are classified against jq (`/usr/bin/jq`) per build, as `jq-fold-body-register-grid-sweep.py`
does: MATCH, ACCEPT_WRONG (jq refused, the build answered: the dangerous one, `del`/`|=` write through
it), REFUSE_WRONG (the safe direction) and DIFF. Exit status 1 when, against `--base`, a row matched
jq on the base and no longer does (a lost match: a refuse-only fallback has leaked under `try` before,
#3186/#3267) or the candidate is ACCEPT_WRONG where the base was not, or on a candidate timeout.
`--fixed` rows (wrong on the base, MATCH on the candidate) are counted and listed, so a change can
show what it moved. Run it against a build from before the change as `--base`.

`--control` puts the fold in the `then` arm only, the shape a change to the literal-`else` code must
not touch: with `--base` it exits 1 on any row whose output differs between the two builds.

Usage: scripts/jq-foreach-update-if-arm-sweep.py --bin BIN [--base BIN] [--jq JQ] [--n N] [--seed S]
                                                  [--show N] [--only SUBSTR[,SUBSTR...]] [--jobs N]
                                                  [--control] [--dump FILE]
"""
import argparse
import collections
import concurrent.futures
import itertools
import json
import random
import subprocess
import sys

# The caught foreign-`$x` fold: `$x` is the context's variable, not the loop's, so its destructure runs
# on a node that is not jq's path register.
FOLDS = [
    ("fe", "foreach (try ($x as {a:$a} | .)) as $y (.; .; .)"),
    ("fe-chain", "foreach (try ($x as {a:$a} ?// [$a] | .)) as $y (.; .; .)"),
]
# arm pairs: (name, then, else); {F} is the fold, `1` the literal.
ARMS = [
    ("lit-then", "1", "{F}"),
    ("lit-else", "{F}", "1"),
]
CONDS = [("ca", ".a"), ("cdot", "."), ("cb", ".b")]
# `elif` chains: the fold behind the second condition, or the last arm.
ELIFS = [
    ("elif-lit-fe-lit", "if .a then 1 elif . then {F} else 2 end"),
    ("elif-fe-lit-lit", "if .a then {F} elif . then 1 else 2 end"),
    ("elif-lit-lit-fe", "if .a then 1 elif . then 2 else {F} end"),
    ("elif-lit-fe-nothing", "if .a then 1 elif .b then {F} end"),
]
CONTROL_ARMS = [("fe-then", "{F}", "1")]
WRAPPERS = [
    ("none", "{U}"),
    ("try-1", "try ({U}) catch 1"),
    ("try-dot", "try ({U}) catch ."),
    ("comma-after", "({U}, 7)"),
    ("comma-before", "(7, {U})"),
    ("opt", "({U})?"),
    ("pipe-dot", "({U}) | ."),
    ("label", "label $l | {U}"),
    ("comma-empty", "({U}, empty)"),
]
SOURCES = [".a", "(.a,.)", "(.,.a)", ".", "(.a,.a)", "(.a,.b)"]
EXTRACTS = [".a", "$i", "."]
INITS = [("init1", "."), ("initN", "(., .a)")]
CONTEXTS = [
    ("paths", "[path(. as $x | {O})?]"),
    ("path", "path(. as $x | {O})"),
    ("del", "del(. as $x | {O})"),
    ("update", "(. as $x | {O}) |= 9"),
]
DOCS = [
    "null",
    '{"a":null}',
    '{"a":{"a":null}}',
    '{"b":true}',
    "true",
    "[1]",
    '{"a":[true]}',
    '{"a":{"b":1},"b":2}',
    '{"a":false,"b":null}',
]


def build_rows(control):
    updates = []
    for (cond_name, cond), (an, then, els) in itertools.product(CONDS, CONTROL_ARMS if control else ARMS):
        updates.append((f"{cond_name}-{an}", f"if {cond} then {then} else {els} end"))
    if not control:
        updates.extend(ELIFS)
    rows = []
    for (un, u), (fn, f), (wn, w), src, ex, (inn, init), (cn, c), doc in itertools.product(
        updates, FOLDS, WRAPPERS, SOURCES, EXTRACTS, INITS, CONTEXTS, DOCS
    ):
        update = w.replace("{U}", u.replace("{F}", f))
        outer = f"foreach {src} as $i ({init}; {update}; {ex})"
        name = f"{un}/{fn}/{wn}/{src}/{ex}/{inn}/{cn}"
        rows.append((name, c.replace("{O}", outer), doc))
    return rows


def run(prefix, prog, doc, timeout=10):
    try:
        p = subprocess.run([*prefix, "-c", prog], input=doc, capture_output=True, text=True, timeout=timeout)
        return p.returncode, p.stdout
    except subprocess.TimeoutExpired:
        return "TIMEOUT", ""


def classify(oracle, got):
    (oc, oo), (gc, go) = oracle, got
    if gc == "TIMEOUT":
        return "TIMEOUT"
    if oc == gc and oo == go:
        return "MATCH"
    if oc != 0 and gc == 0:
        return "ACCEPT_WRONG"
    if oc == 0 and gc != 0:
        return "REFUSE_WRONG"
    return "DIFF"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--bin", required=True, help="succinctly build under test")
    ap.add_argument("--base", help="reference succinctly build; a lost match or new ACCEPT_WRONG against it fails")
    ap.add_argument("--jq", default="/usr/bin/jq", help="the pinned oracle (jq 1.7.1)")
    ap.add_argument("--n", type=int, default=12000, help="rows to sample (0 = the whole grid)")
    ap.add_argument("--seed", type=int, default=0, help="sampling seed: the same seed and --n pick the same rows")
    ap.add_argument("--show", type=int, default=20, help="max rows printed per section")
    ap.add_argument("--only", default="", help="restrict to rows whose name contains this (comma-separated: any of)")
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--control", action="store_true", help="fold in the `then` arm only; any output change against --base fails")
    ap.add_argument("--dump", help="write every row's classes as JSON lines to this file")
    args = ap.parse_args()
    if args.control and not args.base:
        ap.error("--control compares against --base: pass a reference build")

    rows = build_rows(args.control)
    if args.only:
        rows = [r for r in rows if any(part in r[0] for part in args.only.split(","))]
    total = len(rows)
    if args.n and args.n < total:
        rows = random.Random(args.seed).sample(rows, args.n)

    def work(row):
        name, prog, doc = row
        oracle = run([args.jq], prog, doc)
        if oracle[0] == "TIMEOUT":
            return None
        cand = run([args.bin, "jq"], prog, doc)
        base = run([args.base, "jq"], prog, doc) if args.base else None
        return (name, prog, doc, oracle, cand, base)

    with concurrent.futures.ThreadPoolExecutor(args.jobs) as ex:
        results = [r for r in ex.map(work, rows) if r]

    out = []
    for name, prog, doc, oracle, cand, base in results:
        out.append(
            {
                "name": name,
                "prog": prog,
                "doc": doc,
                "oracle": oracle,
                "cand": cand,
                "cls": classify(oracle, cand),
                "base": base,
                "bcls": classify(oracle, base) if base else None,
            }
        )
    if args.dump:
        with open(args.dump, "w") as f:
            for r in out:
                f.write(json.dumps({k: r[k] for k in ("name", "doc", "prog", "cls", "bcls")}) + "\n")

    print(f"rows={len(out)} sampled of {total} (the pinned oracle timed out on {len(rows) - len(out)})")
    print(f"candidate={dict(sorted(collections.Counter(r['cls'] for r in out).items()))}")
    if args.base:
        print(f"     base={dict(sorted(collections.Counter(r['bcls'] for r in out).items()))}")

    hung = [r for r in out if r["cls"] == "TIMEOUT"]
    sections = [("TIMEOUT", hung)]
    failed = bool(hung)
    if args.base and args.control:
        changed = [r for r in out if r["cand"] != r["base"] and r["base"][0] != "TIMEOUT"]
        print(f"control rows whose output changed against --base: {len(changed)}")
        sections.append(("CHANGED", changed))
        failed = failed or bool(changed)
    elif args.base:
        lost = [r for r in out if r["bcls"] == "MATCH" and r["cls"] != "MATCH"]
        newly = [r for r in out if r["cls"] == "ACCEPT_WRONG" and r["bcls"] != "ACCEPT_WRONG"]
        fixed = [r for r in out if r["bcls"] not in (None, "MATCH") and r["cls"] == "MATCH"]
        print(f"fixed against --base: {len(fixed)}; lost matches: {len(lost)}; new ACCEPT_WRONG: {len(newly)}")
        sections += [("LOST MATCH", lost), ("NEW ACCEPT_WRONG", newly)]
        failed = failed or bool(lost or newly)
    else:
        print(f"ACCEPT_WRONG: {sum(r['cls'] == 'ACCEPT_WRONG' for r in out)} (pass --base to judge a change)")
        sections.append(("ACCEPT_WRONG", [r for r in out if r["cls"] == "ACCEPT_WRONG"]))
    for title, section in sections:
        for r in section[: args.show]:
            print(f"{title} {r['name']} [{r['cls']}]\n   {r['prog']}\n   in={r['doc']} jq={r['oracle']} build={r['cand']} base={r['base']}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
