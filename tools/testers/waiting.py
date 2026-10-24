# Lists the tester threads that are waiting on a reply: the last message is a tester's box, or
# there is text after the assistant's last box that isn't in a box (testers sometimes write plain
# text). Also lists new threads still in docs/tester-requests.md's Requests section.
# usage: python3 tools/testers/waiting.py [repo]
import re, sys, os, glob
repo = sys.argv[1] if len(sys.argv) > 1 else '.'
HDR = re.compile(r'^> \[!(note|tip)\]')
waiting = []
for path in sorted(glob.glob(os.path.join(repo, 'testers/*/*.md'))):
    if os.path.basename(path)[:-3] != os.path.basename(os.path.dirname(path)):
        continue
    lines = open(path, encoding='utf-8').read().split('\n')
    last = None
    for i, l in enumerate(lines):
        m = HDR.match(l)
        if m:
            # skip empty boxes (no text in them)
            j = i + 1; text = False
            while j < len(lines) and lines[j].startswith('>'):
                if lines[j].lstrip('>').strip(): text = True
                j += 1
            if text: last = (i, m.group(1))
    if last is None:
        continue
    tail = lines[last[0] + 1:]
    k = 0
    while k < len(tail) and (tail[k].startswith('>') or not tail[k].strip() or re.match(r'^\^m\d+\s*$', tail[k])):
        k += 1
    plain = any(t.strip() and not t.startswith('>') and not re.match(r'^\^m\d+\s*$', t) for t in tail[k:])
    if last[1] == 'note':
        waiting.append((path, 'tester box'))
    elif plain:
        waiting.append((path, 'plain text after the assistant'))
idx = os.path.join(repo, 'docs/tester-requests.md')
s = open(idx, encoding='utf-8').read()
req = s[s.index('\n## Requests'):]
new = [t for t in re.findall(r'^## (.+)$', req, re.M) if t.strip() != 'Requests']
for p, why in waiting:
    print(f'WAITING  {os.path.relpath(p, repo)}  ({why})')
for t in new:
    print(f'NEW      {t}  (in the Requests section: move it to its own folder first)')
if not waiting and not new:
    print('nothing waiting')
