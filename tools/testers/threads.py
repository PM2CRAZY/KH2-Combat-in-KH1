# Tester requests, one file per thread: testers/<slug>/<slug>.md. Rebuilds each thread file's message list and
# block ids (Obsidian: a "^mN" line after each message, linked as [..](#^mN)), and the index in docs/tester-requests.md.
# usage: threads.py <repo> [split]   (split: move the threads in tester-requests.md's Requests section into files)
import re,sys,os,glob
repo=sys.argv[1]; split=len(sys.argv)>2 and sys.argv[2]=='split'
idx=os.path.join(repo,'docs/tester-requests.md')
def slug(t):
    t=t.lower().strip(); t=re.sub(r"[^\w\- ]","",t); return t.replace(' ','-')
HDR=re.compile(r'^> \[!(note|tip)\]\s*(.*)$')
def messages(lines):
    """[(start, end, kind, who, first)] for non-empty boxes; end = index after the box."""
    out=[]; i=0
    while i<len(lines):
        m=HDR.match(lines[i])
        if not m: i+=1; continue
        j=i+1; body=[]
        while j<len(lines):
            l=lines[j]
            if HDR.match(l) or l.startswith('## ') or l.strip()=='---' or re.match(r'^\^m\d+\s*$',l) or l.startswith('<a id='): break
            if l.startswith('>'): body.append(l)
            elif l.strip(): break
            j+=1
        # trim trailing blank lines from the box's end
        k=j
        while k>i+1 and not lines[k-1].strip(): k-=1
        text=[b.lstrip('>').strip() for b in body if b.lstrip('>').strip()]
        if text:
            who='Assistant' if m.group(1)=='tip' else (m.group(2).replace('Tester','').strip() or 'Tester')
            first=re.sub(r'[\[\]`*|#^]','',text[0])[:60]
            out.append((i,k,m.group(1),who,first))
        i=max(j,i+1)
    return out
def rebuild_thread(path):
    s=open(path).read()
    s=re.sub(r'\n\^m\d+[ \t]*\n','\n',s)
    s=re.sub(r'<a id="[^"]*"></a>\n','',s)
    title=re.search(r'^# (.+)$',s,re.M).group(1).strip()
    head,sep,body=s.partition('\n---\n')
    lines=body.split('\n')
    ms=messages(lines)
    for n,(a,b,kind,who,first) in reversed(list(enumerate(ms,1))):
        lines[b:b]=['',f'^m{n}','']
    body='\n'.join(lines)
    body=re.sub(r'\n{4,}','\n\n\n',body)
    items='\n'.join(f'- [{n}. {who}: {first}](#^m{n})' for n,(a,b,kind,who,first) in enumerate(ms,1))
    rel=os.path.relpath(idx,os.path.dirname(path)).replace(os.sep,'/')
    head=f'# {title}\n\n[Back to the tester requests]({rel})\n\n## Messages\n\n{items}\n'
    open(path,'w').write(head+sep+body)
    return title,[(n,who,first) for n,(a,b,k,who,first) in enumerate(ms,1)]
s=open(idx).read()
if split:
    req=s.index('\n## Requests')
    head,body=s[:req],s[req:]
    parts=body.split('\n---\n')
    keep=[parts[0]]
    for part in parts[1:]:
        m=re.search(r'^## (.+)$',part,re.M)
        if not m: keep.append(part); continue
        title=m.group(1).strip(); sl=slug(title)
        d=os.path.join(repo,'testers',sl); os.makedirs(d,exist_ok=True)
        text=re.sub(r'<a id="[^"]*"></a>\n','',part)
        text=text[text.index('## '+title)+len('## '+title):]
        open(os.path.join(d,sl+'.md'),'w').write(f'# {title}\n\n---\n'+text.rstrip()+'\n')
    s=head+keep[0].rstrip()+'\n'
    open(idx,'w').write(s)
threads=[]
for path in sorted(glob.glob(os.path.join(repo,'testers/*/*.md'))):
    if os.path.basename(path).lower()=='readme.md': continue
    if os.path.basename(path)[:-3]!=os.path.basename(os.path.dirname(path)): continue
    threads.append((path,)+rebuild_thread(path))
s=open(idx).read()
lines=['## Contents','']
for path,title,ms in threads:
    rel=os.path.relpath(path,os.path.dirname(idx)).replace(os.sep,'/')
    lines.append(f'- [{title}]({rel})')
    for n,who,first in ms:
        lines.append(f'  - [{n}. {who}: {first}]({rel}#^m{n})')
s=re.sub(r'## Contents\n\n(?:- .*\n|  - .*\n)*\n?','\n'.join(lines)+'\n\n',s,count=1)
open(idx,'w').write(s)
print('\n'.join(f'{t}: {len(m)} messages' for p,t,m in threads))
