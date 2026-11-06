# Writes crates/kh1-hook/src/win/epic_map.in: every St(0x...) in the hook's source, Steam RVA ->
# Epic RVA, from the player's own two exes.
#
# usage: python3 tools/epic/map.py <steam exe> <epic exe> [--check]
#   Runs match.py's matching (cached next to this script's output in /tmp), then resolves each
#   address: code by its function (identical code, or the same prefix up to the address), globals by
#   the operands of matched code that reach them, else by a byte search near the expected place.
#   Steam addresses the old Epic -> Steam table already pairs are taken from it first.
#   Unresolved addresses are listed and the table is not written.
import sys, os, re, json, subprocess
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
from resolve import Resolver

repo = os.path.dirname(os.path.dirname(here))
src = os.path.join(repo, 'crates', 'kh1-hook', 'src')
steam_exe, epic_exe = sys.argv[1], sys.argv[2]
cache = '/tmp/kh1-epic-match.json'
if not os.path.exists(cache):
    subprocess.run([sys.executable, os.path.join(here, 'match.py'), steam_exe, epic_exe, cache], check=True)
r = Resolver(cache, steam_exe, epic_exe)

old = {}
for e, s in re.findall(r'\(0x([0-9a-fA-F]+),\s*0x([0-9a-fA-F]+)\)', open(os.path.join(src, 'win', 'steam_map.in')).read()):
    old[int(s, 16)] = int(e, 16)

# Found by hand (each checked against both exes, see docs/kh1-epic-addresses.md):
MANUAL = {
    # KH1's settings: Steam's `lea rax, [0x1404D8614]` getter 0x12AF20 is Epic's 0x12AC50, which
    # returns 0x1422DB014; every field Epic reads is 4 less than Steam's (KH1's own reads after a
    # call of the getter, 170 sites: 0x16 -> 0x12, 0x1E -> 0x1A, ...), so Steam's base maps to 4
    # before Epic's.
    0x4D8614: 0x22DB010,
    # Volume Mute's fade reads (Steam's 0xDDD5B site -0x10, the same function's next reads).
    0xDDD9C: 0xDDD8C,
    0xDDDDD: 0xDDDCD,
    # KH1's GS register writes ALPHA / CLAMP: byte-identical triplets, the GS writes beside them
    # (0x386630, 0x386660, 0x386960) all 0x2360 lower on Epic.
    0x386060: 0x383D00,
    0x386210: 0x383EB0,
    # The Abilities page window's update pointer: the only place Epic holds Epic's 0x14030F410.
    0x52ADB0: 0x52B7D0,
    # The Config labels: their 32 bytes are only at Epic 0x547100 (its list definition + 0x70 as
    # on Steam).
    0x546410: 0x547100,
    # Drive orbs' fourth table read: the reads beside it (0x2AC474, ...) are all 0x2190 lower on Epic,
    # and Epic's 0x2AA30B has the same instruction (its table's own place in it differs).
    0x2AC49B: 0x2AA30B,
    # Frame Advance's put-back block ends where Steam has 0x142866D18: no code reaches it; the globals
    # around it moved +0xA00 (below) and +0xA10 (above). The lower one is used, so the block copied
    # back is never longer than it should be. Assumption (exe).
    0x2866D18: 0x2867718,
}

want = {}
for dp, _, fs in os.walk(src):
    for f in fs:
        if f.endswith('.rs'):
            p = os.path.join(dp, f)
            for n, line in enumerate(open(p, encoding='utf-8'), 1):
                for m in re.finditer(r'\bSt\(0x([0-9A-Fa-f_]+)\)', line):
                    want.setdefault(int(m.group(1).replace('_', ''), 16), f'{os.path.relpath(p, src)}:{n}')

out, bad = [], []
for s in sorted(want):
    if s in MANUAL:
        out.append((s, MANUAL[s], 'by hand'))
        continue
    if s in old:
        out.append((s, old[s], 'old table'))
        continue
    e, how = r.resolve(s)
    if e is None: bad.append((s, how, want[s]))
    else: out.append((s, e, how))
for s, how, at in bad:
    print(f'unresolved {s:#x} ({how}) at {at}')
print(f'{len(want)} addresses, {len(out)} resolved, {len(bad)} not')
if bad and '--force' not in sys.argv: sys.exit(1)
if '--check' in sys.argv: sys.exit(0)
with open(os.path.join(src, 'win', 'epic_map.in'), 'w') as f:
    f.write('// Steam RVA -> Epic RVA for every St(0x...) in the source (tools/epic/map.py; see addr.rs).\n[\n')
    for s, e, how in out:
        f.write(f'    (0x{s:07X}, 0x{e:07X}),\n')
    f.write(']\n')
