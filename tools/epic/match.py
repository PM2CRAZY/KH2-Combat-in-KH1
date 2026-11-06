# Matches the Steam KH1 exe to the Epic KH1 exe, function by function, and pairs the globals
# their RIP-relative operands reach. Both are the player's own files; nothing of theirs is written
# to the repo, only RVAs.
#
# usage: python3 tools/epic/match.py <steam exe> <epic exe> <out.json>
#   out.json: {"funcs": [[steam_begin, steam_end, epic_begin], ...],
#              "data": [[steam_rva, epic_rva, votes], ...]}
import sys, json, hashlib, collections
import pefile, capstone
from capstone import x86

def load(path):
    pe = pefile.PE(path, fast_load=True)
    pe.parse_data_directories(directories=[pefile.DIRECTORY_ENTRY['IMAGE_DIRECTORY_ENTRY_EXCEPTION']])
    img = pe.get_memory_mapped_image()
    funcs = sorted({(e.struct.BeginAddress, e.struct.EndAddress) for e in pe.DIRECTORY_ENTRY_EXCEPTION})
    secs = [(s.Name.rstrip(b'\0').decode(), s.VirtualAddress, s.VirtualAddress + max(s.Misc_VirtualSize, s.SizeOfRawData)) for s in pe.sections]
    return img, funcs, secs

def analyse(img, funcs):
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
    md.detail = True
    out = {}
    for b, e in funcs:
        code = img[b:e]
        sig = bytearray(code)
        refs = []  # (offset in func, kind, target rva)
        for i in md.disasm(code, b):
            off = i.address - b
            # RIP-relative memory operand
            if i.disp_size and any(op.type == x86.X86_OP_MEM and op.mem.base == x86.X86_REG_RIP for op in i.operands):
                d0 = off + i.disp_offset
                sig[d0:d0 + i.disp_size] = b'\0' * i.disp_size
                for op in i.operands:
                    if op.type == x86.X86_OP_MEM and op.mem.base == x86.X86_REG_RIP:
                        refs.append((off, 'd', i.address + i.size + op.mem.disp))
            # relative branch / call
            if i.imm_size and (i.group(capstone.CS_GRP_JUMP) or i.group(capstone.CS_GRP_CALL)) and i.operands and i.operands[0].type == x86.X86_OP_IMM:
                t = i.operands[0].imm
                if not (b <= t < e):
                    i0 = off + i.imm_offset
                    sig[i0:i0 + i.imm_size] = b'\0' * i.imm_size
                    refs.append((off, 'c', t))
        out[b] = (e, hashlib.sha1(bytes(sig)).hexdigest(), refs, bytes(sig))
    return out

def main():
    simg, sf, ssec = load(sys.argv[1])
    eimg, ef, esec = load(sys.argv[2])
    S, E = analyse(simg, sf), analyse(eimg, ef)
    by_s, by_e = collections.defaultdict(list), collections.defaultdict(list)
    for b, (e, h, *_) in S.items(): by_s[h].append(b)
    for b, (e, h, *_) in E.items(): by_e[h].append(b)
    m = {}
    for h, l in by_s.items():
        if len(l) == 1 and len(by_e.get(h, [])) == 1:
            m[l[0]] = by_e[h][0]
    # Propagate through calls of matched functions (same hash required).
    changed = True
    while changed:
        changed = False
        for sb, eb in list(m.items()):
            sr, er = S[sb][2], E[eb][2]
            if len(sr) != len(er): continue
            for (so, sk, st), (eo, ek, et) in zip(sr, er):
                if sk == 'c' and so == eo and st in S and et in E and st not in m and S[st][1] == E[et][1]:
                    m[st] = et; changed = True
    # Loose pairs: a matched function's call at the same place reaches functions whose code differs
    # (a few bytes or a length); kept with how far from the start their code agrees.
    loose = {}
    changed = True
    while changed:
        changed = False
        for sb, eb in list(m.items()) + list(loose.items()):
            sr, er = S[sb][2], E[eb][2]
            for (so, sk, st), (eo, ek, et) in zip(sr, er):
                if so != eo or sk != ek: break
                if sk == 'c' and st in S and et in E and st not in m and st not in loose:
                    loose[st] = et; changed = True
    same = {}
    for sb, eb in loose.items():
        a, b = S[sb][3], E[eb][3]
        n = 0
        while n < min(len(a), len(b)) and a[n] == b[n]: n += 1
        same[sb] = n
    # Globals: votes from matched functions' operands.
    votes = collections.defaultdict(collections.Counter)
    for sb, eb in list(m.items()) + list(loose.items()):
        sr, er = S[sb][2], E[eb][2]
        lim = same.get(sb, 1 << 30)
        for (so, sk, st), (eo, ek, et) in zip(sr, er):
            if so != eo or so >= lim: break
            if sk == 'd': votes[st][et] += 1
    data = []
    for st, c in sorted(votes.items()):
        (et, n), = c.most_common(1)
        data.append([st, et, n, sum(c.values())])
    funcs = [[sb, S[sb][0], eb] for sb, eb in sorted(m.items())]
    funcs += [[sb, sb + same[sb], eb] for sb, eb in sorted(loose.items()) if same[sb] > 0]
    funcs.sort()
    json.dump({"funcs": funcs, "data": data, "steam_sections": ssec, "epic_sections": esec}, open(sys.argv[3], 'w'))
    print(f"functions: steam {len(S)}, epic {len(E)}, matched {len(m)}; loose {len(loose)}; globals paired {len(data)}")

main()
