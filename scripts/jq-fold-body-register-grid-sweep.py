#!/usr/bin/env python3
"""Differential grid for a fold source that moves jq's path register inside another fold (#4505).

A `reduce`/`foreach` SOURCE inside another fold's UPDATE runs path-checked `INDEX` steps (a
navigation, or an array/object pattern) on whatever it meets. When it meets the register (`.`
or an outer `$x` bound to it) a `foreach` source does not backtrack the move, so the enclosing
fold's later emission of the accumulator raises in jq; when an enclosing `reduce` source already
moved the register onto an element, neither `.` nor `$x` is the register and the step raises for a
`reduce` source too. `jq-path-register-sweep.py` has no outer-`$x`-destructured-in-a-body operand,
so it cannot gate this: the grid here is outer fold x inner fold x source shape x write context x
document, six x three x fifty-six x seven x thirteen.

Rows are classified against jq (`/usr/bin/jq`) per build, as `jq-path-register-sweep.py` does:
MATCH, ACCEPT_WRONG (jq refused, the build answered: the dangerous one, `del`/`=`/`|=` write
through it), REFUSE_WRONG (the safe direction) and DIFF. Exit status 1 when the candidate has an
ACCEPT_WRONG outside the documented residual (`KNOWN_ACCEPT_WRONG`), a row that matched jq on
`--base` and no longer does (a lost match: a refuse-only fallback has leaked under `try` before,
#3186/#3267), or a candidate timeout. Run it against a `main` build as `--base` to see what a change moved.

Usage: scripts/jq-fold-body-register-grid-sweep.py --bin BIN [--base BIN] [--jq JQ] [--show N]
                                                   [--only SUBSTR] [--jobs N]
"""
import argparse
import collections
import concurrent.futures
import itertools
import subprocess
import sys

OUTER = [
    ("fe-lit", "foreach (1,2) as $i (.; {I}; .)"),
    ("rd-lit", "reduce (1,2) as $i (.; {I})"),
    # a navigating source leaves jq's register on the element for the whole step
    ("fe-nav", "foreach .[]? as $i (.; {I}; .)"),
    ("rd-nav", "reduce .[]? as $i (.; {I})"),
    ("fe-nava", "foreach .a? as $i (.; {I}; .)"),
    ("rd-nava", "reduce .a? as $i (.; {I})"),
]
INNER = [
    ("fe", "foreach ({S}) as $y (.; .; .)"),
    ("fe-y", "foreach ({S}) as $y (.; $y; .)"),  # EXTRACT is the element: it is the register
    ("rd", "reduce ({S}) as $y (.; .)"),
]
SOURCES = [
    ("x-pat", "$x as {a:$a} | ."),
    ("x-pat-arr", "$x as [$a] | ."),
    ("sel-x-xpat", "select($x) | $x as {a:$a} | ."),
    ("sel-x-dotpat", "select($x) | . as {a:$a} | ."),
    ("sel-1-xpat", "select(1) | $x as {a:$a} | ."),
    ("sel-x-dotpat-arr", "select($x) | . as [$a] | ."),
    ("try-xpat", "try ($x as {a:$a} | .)"),
    ("try-xpat-catch", "try ($x as {a:$a} | .) catch ."),
    ("opt-xpat", "($x as {a:$a} | .)?"),
    ("try-dotpat", "try (. as {a:$a} | .)"),
    ("try-dotpat-catch", "try (. as {a:$a} | .) catch ."),
    ("x-nav", "$x | .a"),
    ("x-nava", "$x | .a?"),
    ("x-nav-pat", "($x | .a) as {b:$b} | ."),
    ("x-idx", "$x | .[0]"),
    ("x-iter", "$x | .[]?"),
    ("first-xpat", "first($x as {a:$a} | .)"),
    ("if-xpat", "if $x then $x as {a:$a} | . else . end"),
    ("x-then-dotpat", "$x | . as {a:$a} | ."),
    ("dotpat", ". as {a:$a} | ."),
    ("sel-x-only", "select($x)"),
    ("x-only", "$x"),
    ("sel-x-nav", "select($x) | .a"),
    ("x-sel-pat", "$x | select(.) | . as {a:$a} | ."),
    ("comma-xpat", "($x as {a:$a} | .), 1"),
    ("lim-xpat", "limit(1; $x as {a:$a} | .)"),
    ("dot-nav", ".a"),
    ("dot-nava", ".a?"),
    ("dot-idx", ".[0]"),
    ("dot-iter", ".[]?"),
    ("dot-sel-nav", "select(.) | .a"),
    ("dot-try-nav", "try .a"),
    ("dot-try-pat", "try (. as [$a] | .)"),
    # a `select` that passes nothing never reaches the destructure behind it; a `?` swallows its raise
    ("sel-false-dotpat", "select(false) | . as {a:$a} | ."),
    ("sel-empty-dotpat", "select(empty) | . as {a:$a} | ."),
    ("sel-not-x-dotpat", "select($x|not) | . as {a:$a} | ."),
    ("sel-dot-opt-pat", "select(.) | (. as [$a] | .)?"),
    ("dot-opt-pat", "(. as [$a] | .)?"),
    # a `try`/`?` catches the pattern's type error (nothing moved the register) but not a success;
    # a `?//` chain retries the next alternative past the pattern
    ("try-chain-catch", "try ($x as {a:$a} ?// [$a] | .) catch ."),
    # the chain ends on its last alternative (#4585): it type-errors into the handler, or matches and
    # moves the register, or is a bare `$v` that moves nothing
    ("try-chain-catch-rev", "try ($x as [$a] ?// {a:$a} | .) catch ."),
    ("try-chain-catch-miss", "try ($x as {b:$a} ?// {a:$a} | .) catch ."),
    ("try-chain-catch-var", "try ($x as {a:$a} ?// $b | .) catch ."),
    ("try-chain-catch-3", "try ($x as {a:$a} ?// [$a] ?// $b | .) catch ."),
    ("try-chain-catch-3-arr", "try ($x as {a:$a} ?// {b:$a} ?// [$a] | .) catch ."),
    ("try-chain-catch-dot", "try (. as {a:$a} ?// [$a] | .) catch ."),
    # a last alternative of several entries that matches is answered, and a body that navigates is
    # not the register's emission
    ("try-chain-catch-multi", "try ($x as {a:$a} ?// [$a,$b] | .) catch ."),
    ("try-chain-catch-multi-obj", "try ($x as {a:$a,b:$c} ?// {a:$a,b:$c} | .) catch ."),
    ("try-chain-catch-nav", "try ($x as {a:$a} ?// [$a] | .a) catch ."),
    ("try-chain", "try ($x as {a:$a} ?// [$a] | .)"),
    ("opt-chain", "($x as {a:$a} ?// [$a] | .)?"),
    ("try-var-catch", "try ($x as $v | .) catch ."),
    ("try-empty-catch", "try ($x as {} | .) catch ."),
    ("sel-x-opt-pat", "select($x) | (. as {a:$a} | .)?"),
    ("try-arr-catch", "try ($x as [$a] | .) catch ."),
    # (#4616) a fresh value, or a `null`/boolean literal the register is not, bound to a variable and
    # destructured: the pattern's step runs on a node that is not the register
    ("fresh-var", "{a:1} as $n | $n as {b:$b} | ."),
    ("fresh-var-arr", "[1] as $n | $n as [$b] | ."),
    ("lit-null-var", "null as $n | $n as {b:$b} | ."),
    ("lit-true-var", "true as $n | $n as {b:$b} | ."),
    # (#4621) an `empty` body: the destructure's path error is caught and the handler's output makes
    # the fold emit its accumulator, which is not the register; a handler that emits nothing does not
    ("try-dotpat-empty-catch", "try (. as [$a] | empty) catch ."),
    ("try-dotpat-empty-catch-empty", "try (. as [$a] | empty) catch empty"),
    ("try-dotpat-empty", "try (. as [$a] | empty)"),
    # the outer fold's own loop variable is another value, not the register
    ("i-idx", "$i | .[0]"),
    ("i-nava", "$i | .a?"),
]
CONTEXTS = [
    ("path", "path(. as $x | {O})"),
    ("del", "del(. as $x | {O})"),
    ("update", "(. as $x | {O}) |= 9"),
    ("assign", "(. as $x | {O}) = 9"),
    # `$x` bound to a node that is not the register (#4588): a navigation, `null` included.
    ("path-nav", "path(.a as $x | {O})"),
    ("del-nav", "del(.a as $x | {O})"),
    ("update-nav", "(.a as $x | {O}) |= 9"),
]
# The ACCEPT_WRONG rows of these contexts still left (#4588): the body of a `reduce` whose source
# moved the register, where the register is on an element and a marker may be that very node, and a
# destructure of `.` behind a `select($x|not)`/`select($x)` that the markers do not decide. They count
# against a lost match against `--base` like any row, but an ACCEPT_WRONG in them does not fail the run.
RESIDUAL_CONTEXTS = {"path-nav", "del-nav", "update-nav"}
# ...and their count may not grow: the rows are not enumerated, so a new one is caught by the total.
RESIDUAL_NAV_MAX = 147
DOCS = [
    "null",
    "true",
    "[true]",
    "[null]",
    "[]",
    "[[]]",
    "[[1],[1]]",
    '{"a":true}',
    '{"a":1,"b":2}',
    '{"a":[true]}',
    '{"a":false,"b":null}',
    '{"a":{"b":1}}',
    '[{"a":1}]',
]

# The ACCEPT_WRONG rows left: the `null`-literal variable below. (A `try ... catch` or `?` around a single destructure was listed
# here until #4575, a comma beside the destructure on a `null` document until #4576, and a
# `try ... catch` around a `?//` chain of destructures until #4585.)
KNOWN_ACCEPT_WRONG = {
    # (#4616) a `null` literal bound to a variable and destructured, in the body of another fold: the
    # register's own null-ness is not known there (`NotRegister::at_fold`), so it may be the register
    ("lit-null-var", None),
}


def run(prefix, prog, doc, timeout=10):
    cmd = [*prefix, "-c", prog]
    try:
        p = subprocess.run(cmd, input=doc, capture_output=True, text=True, timeout=timeout)
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
    ap.add_argument("--base", help="reference succinctly build; a lost match against it fails")
    ap.add_argument("--jq", default="/usr/bin/jq", help="the pinned oracle (jq 1.7.1)")
    ap.add_argument("--show", type=int, default=20, help="max rows printed per section")
    ap.add_argument("--only", default="", help="restrict to rows whose name contains this")
    ap.add_argument("--jobs", type=int, default=8)
    args = ap.parse_args()

    rows = []
    for (on, o), (inn, i), (sn, s), (cn, c), doc in itertools.product(OUTER, INNER, SOURCES, CONTEXTS, DOCS):
        name = f"{on}/{inn}/{sn}/{cn}"
        if args.only and args.only not in name:
            continue
        prog = c.replace("{O}", o.replace("{I}", i.replace("{S}", s)))
        rows.append((name, sn, prog, doc))

    def work(row):
        name, sn, prog, doc = row
        oracle = run([args.jq], prog, doc)
        if oracle[0] == "TIMEOUT":
            return None
        cand = run([args.bin, "jq"], prog, doc)
        base = run([args.base, "jq"], prog, doc) if args.base else None
        return (
            name,
            sn,
            prog,
            doc,
            oracle,
            classify(oracle, cand),
            cand,
            classify(oracle, base) if base else None,
        )

    with concurrent.futures.ThreadPoolExecutor(args.jobs) as ex:
        results = [r for r in ex.map(work, rows) if r]

    totals = collections.Counter(r[5] for r in results)
    print(f"rows={len(results)} of {len(rows)} (the pinned oracle timed out on the rest)")
    print(f"candidate={dict(sorted(totals.items()))}")
    if args.base:
        print(f"           base={dict(sorted(collections.Counter(r[7] for r in results).items()))}")

    def known(r):
        return (r[1], None) in KNOWN_ACCEPT_WRONG or (r[1], r[3]) in KNOWN_ACCEPT_WRONG

    residual_nav = [
        r for r in results if r[5] == "ACCEPT_WRONG" and r[0].split("/")[3] in RESIDUAL_CONTEXTS
    ]
    fresh = [
        r
        for r in results
        if r[5] == "ACCEPT_WRONG"
        and not known(r)
        and r[0].split("/")[3] not in RESIDUAL_CONTEXTS
    ]
    residual = [r for r in results if r[5] == "ACCEPT_WRONG" and known(r)]
    lost = [r for r in results if args.base and r[7] == "MATCH" and r[5] != "MATCH"]
    hung = [r for r in results if r[5] == "TIMEOUT"]
    print(
        f"ACCEPT_WRONG outside the documented residual: {len(fresh)}; documented residual: "
        f"{len(residual)}; of a non-register `$x` (#4588): {len(residual_nav)}"
    )
    print(f"lost matches against --base: {len(lost)}; candidate timeouts: {len(hung)}")
    for title, section in (("ACCEPT_WRONG", fresh), ("LOST MATCH", lost), ("TIMEOUT", hung)):
        for name, _sn, prog, doc, oracle, klass, cand, _b in section[: args.show]:
            print(f"{title} {name} [{klass}]\n   {prog}\n   in={doc} jq={oracle} build={cand}")
    return 1 if fresh or lost or hung or len(residual_nav) > RESIDUAL_NAV_MAX else 0


sys.exit(main())
