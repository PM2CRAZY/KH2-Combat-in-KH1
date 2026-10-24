# Makes one version: applies an optional patch, adds the AGENTS.md stage, bumps VERSION in
# crates/kh1-hook/src/lib.rs, type-checks kh1-hook for Windows and commits.
# usage: python3 tools/testers/version.py <number> "<title>" "<stage body>" "<commit msg>" [patch]
#   e.g. version.py 477 "KH2's guard on KH1's guard state" "- **...** The tester (...). ..." "native guard"
# Env: KH_TRAILER = the commit's attribution lines (optional), CARGO_TARGET_DIR (optional).
import sys, subprocess, re, datetime, os
ver = int(sys.argv[1]); title, body, msg = sys.argv[2], sys.argv[3], sys.argv[4]
patch = sys.argv[5] if len(sys.argv) > 5 else None
if patch:
    r = subprocess.run(['git', 'apply', '--3way', patch], capture_output=True, text=True)
    if r.returncode: print(r.stderr); sys.exit(1)
p = 'AGENTS.md'; s = open(p, encoding='utf-8').read()
s = f'### {datetime.date.today()}: Stage {ver} (testing branch: {title})\n' + body.rstrip() + '\n\n' + s
open(p, 'w', encoding='utf-8').write(s)
p = 'crates/kh1-hook/src/lib.rs'; s = open(p, encoding='utf-8').read()
s, n = re.subn(r'pub const VERSION: &str = "t\d+";', f'pub const VERSION: &str = "t{ver}";', s)
assert n == 1, 'VERSION line not found'
open(p, 'w', encoding='utf-8').write(s)
env = dict(os.environ, RUSTFLAGS='--cfg windows -A explicit_builtin_cfgs_in_flags')
chk = subprocess.run(['cargo', 'check', '-q', '-p', 'kh1-hook'], capture_output=True, text=True, env=env)
errs = [l for l in chk.stderr.split('\n') if l.startswith('error')]
if errs: print(chk.stderr[-4000:]); sys.exit(2)
subprocess.run(['git', 'add', '-A'], check=True)
trailer = os.environ.get('KH_TRAILER', '').strip()
subprocess.run(['git', 'commit', '-q', '-m', f't{ver} (testing): {msg}' + (f'\n\n{trailer}' if trailer else '')], check=True)
print('ok', ver)
