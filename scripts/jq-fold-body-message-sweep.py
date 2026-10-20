#!/usr/bin/env python3
"""Message-aware differential over the fold-body grid's rows (#4933).

`jq-fold-body-register-grid-sweep.py` classifies a row by stdout and exit code, so a refusal worded as the
end of the path where jq names the step (`with result <acc>` vs `near attempt to access element`) is a
MATCH there. This reuses that script's rows and adds the first line of stderr (minus its location prefix):
MATCH is the same stdout, exit code and message; MSG the same stdout and exit code with a different one.
Exit status 1 on a row that matched `--base` and no longer does, or one that newly accepts where jq refuses.

Usage: scripts/jq-fold-body-message-sweep.py --bin BIN --base BIN --only SUBSTR[,SUBSTR...]
                                             [--shard I/N] [--dump FILE] [--jobs N] [--show N]
It execs the tables of `jq-fold-body-register-grid-sweep.py` next to it.
"""
import argparse, collections, concurrent.futures, itertools, json, re, subprocess, sys
src = open(__import__('os').path.join(__import__('os').path.dirname(__import__('os').path.abspath(__file__)), 'jq-fold-body-register-grid-sweep.py')).read().replace('sys.exit(main())', '')
g = {'__name__': 'grid'}
exec(compile(src, 'grid', 'exec'), g)
def run(prefix, prog, doc):
    try:
        p = subprocess.run([*prefix, '-c', prog], input=doc, capture_output=True, text=True, timeout=10)
    except subprocess.TimeoutExpired:
        return ('TIMEOUT', '', '')
    err = p.stderr.strip().split('\n')[0] if p.stderr.strip() else ''
    err = re.sub(r'^jq: error \(at [^)]*\): ', '', err)
    return (p.returncode, p.stdout, err)
def cls(o, c):
    if c[0] == 'TIMEOUT': return 'TIMEOUT'
    if o[:2] == c[:2]: return 'MATCH' if o[2] == c[2] else 'MSG'
    if o[0] != 0 and c[0] == 0: return 'ACCEPT_WRONG'
    if o[0] == 0 and c[0] != 0: return 'REFUSE_WRONG'
    return 'DIFF'
ap = argparse.ArgumentParser()
ap.add_argument('--bin', required=True); ap.add_argument('--base', required=True)
ap.add_argument('--only', default=''); ap.add_argument('--shard', default='0/1')
ap.add_argument('--dump'); ap.add_argument('--jobs', type=int, default=10); ap.add_argument('--show', type=int, default=10)
a = ap.parse_args()
si, sn = map(int, a.shard.split('/'))
rows = []
k = 0
for (on, o), (inn, i), (sn_, s), (cn, c), doc in itertools.product(g['OUTER'], g['INNER'], g['SOURCES'], g['CONTEXTS'], g['DOCS']):
    name = f'{on}/{inn}/{sn_}/{cn}'
    if a.only and not any(p in name for p in a.only.split(',')): continue
    k += 1
    if ((k * 2654435761) >> 7) % sn != si: continue
    prog = c.replace('{O}', o.replace('{I}', i.replace('{S}', s)))
    rows.append((name, prog, doc))
def work(r):
    name, prog, doc = r
    o = run(['/usr/bin/jq'], prog, doc)
    if o[0] == 'TIMEOUT': return None
    return (name, prog, doc, o, cls(o, run([a.bin, 'jq'], prog, doc)), cls(o, run([a.base, 'jq'], prog, doc)))
with concurrent.futures.ThreadPoolExecutor(a.jobs) as ex:
    res = [r for r in ex.map(work, rows) if r]
print('rows', len(res), 'cand', dict(sorted(collections.Counter(r[4] for r in res).items())), 'base', dict(sorted(collections.Counter(r[5] for r in res).items())))
lost = [r for r in res if r[5] == 'MATCH' and r[4] != 'MATCH']
newacc = [r for r in res if r[4] == 'ACCEPT_WRONG' and r[5] != 'ACCEPT_WRONG']
gained = [r for r in res if r[5] != 'MATCH' and r[4] == 'MATCH']
print(f'lost={len(lost)} newly_accept_wrong={len(newacc)} gained={len(gained)}')
if a.dump:
    with open(a.dump, 'w') as out:
        for r in res: out.write(json.dumps({'name': r[0], 'prog': r[1], 'doc': r[2], 'oracle': r[3], 'cand': r[4], 'base': r[5]}) + '\n')
for t, sec in (('LOST', lost), ('NEWACC', newacc)):
    for r in sec[:a.show]: print(t, r[0], r[4], '\n  ', r[1], '\n   in', r[2], 'jq', r[3])
sys.exit(1 if lost or newacc else 0)
