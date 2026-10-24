# Gate for the automatic "update" round. Decides whether a scheduled run should start an update,
# and on which threads, based on who PUSHED the latest change to each waiting thread.
#
# Only pushes made by the GitHub account BoogleChan (user id 345641864) count. The pusher comes from
# GitHub's repository activity log (the account that authenticated the push), not from the commit's
# author name or email, which anyone can set to anything. A change whose push can't be found in the
# log does not count: the gate fails closed.
#
# usage: GH_TOKEN=... python3 tools/testers/autoupdate.py [repo]
#   exit 0  -> prints GO plus the threads to handle (WAITING ... / NEW ...)
#   exit 1  -> nothing from BoogleChan is waiting (prints why, including waiting threads it skipped)
#   exit 2  -> couldn't check (API or git error): do not run the round
import json, os, re, subprocess, sys, urllib.request

ALLOWED_ID = 345641864           # github.com/BoogleChan (ids never change, logins can)
ALLOWED_LOGIN = 'BoogleChan'
OWNER_REPO = 'PM2CRAZY/KH2-Combat-in-KH1'
BRANCH = 'refs/heads/testing'

repo = sys.argv[1] if len(sys.argv) > 1 else '.'

def git(*a):
    return subprocess.run(['git', '-C', repo, *a], capture_output=True, text=True, check=True).stdout

def pushes():
    # GitHub's activity log: every push to testing by BoogleChan, newest first (last 100)
    tok = os.environ.get('GH_TOKEN') or os.environ.get('GITHUB_TOKEN')
    req = urllib.request.Request(
        f'https://api.github.com/repos/{OWNER_REPO}/activity?ref={BRANCH}&actor={ALLOWED_LOGIN}'
        f'&activity_type=push&per_page=100',
        headers={'Accept': 'application/vnd.github+json', **({'Authorization': f'Bearer {tok}'} if tok else {})})
    with urllib.request.urlopen(req, timeout=30) as r:
        out = json.load(r)
    if not isinstance(out, list):
        raise RuntimeError(f'activity log answered: {out}')
    return out

try:
    log = pushes()
except Exception as ex:
    print(f'CANNOT CHECK  GitHub activity log: {ex}'); sys.exit(2)

# every commit that arrived in a BoogleChan push to testing
allowed = set()
for e in log:
    a = e.get('actor') or {}
    if a.get('id') != ALLOWED_ID or a.get('login') != ALLOWED_LOGIN or e.get('ref') != BRANCH:
        continue
    before, head = e.get('before'), e.get('after')
    try:
        rng = f'{before}..{head}' if before and set(before) != {'0'} else head
        allowed.update(git('rev-list', rng).split())
    except subprocess.CalledProcessError:
        # older than the local clone's history: its commits stay out of `allowed` (fails closed),
        # and anything that recent enough to be waiting will be in the clone
        continue

def last_change(path):
    h = git('log', '-1', '--format=%H', 'HEAD', '--', path).strip()
    return h or None

r = subprocess.run([sys.executable, os.path.join(repo, 'tools/testers/waiting.py'), repo],
                   capture_output=True, text=True)
if r.returncode != 0:
    print(f'CANNOT CHECK  waiting.py failed: {r.stderr.strip()}'); sys.exit(2)

go, skip = [], []
for line in r.stdout.splitlines():
    if line.startswith('WAITING'):
        path = line.split()[1]
        src = path
    elif line.startswith('NEW'):
        src = 'docs/tester-requests.md'
    else:
        continue
    h = last_change(src)
    (go if h in allowed else skip).append((line, h))

for line, h in go:
    print(f'{line}   [pushed by {ALLOWED_LOGIN}, {h[:7]}]')
for line, h in skip:
    print(f'SKIPPED  {line}   [latest change {h[:7] if h else "?"} was not pushed by {ALLOWED_LOGIN}]')
if go:
    print('GO'); sys.exit(0)
print('nothing from BoogleChan waiting'); sys.exit(1)
