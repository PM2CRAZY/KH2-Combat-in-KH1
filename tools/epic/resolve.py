# Steam RVA -> Epic RVA from match.py's output.
import json, bisect, re
import pefile, capstone
from capstone import x86

class Resolver:
    def __init__(self, path, steam_exe=None, epic_exe=None):
        j = json.load(open(path))
        self.simg = pefile.PE(steam_exe, fast_load=True).get_memory_mapped_image() if steam_exe else None
        self.eimg = pefile.PE(epic_exe, fast_load=True).get_memory_mapped_image() if epic_exe else None
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64); self.md.detail = True
        self.funcs = sorted(j['funcs'])
        self.fb = [f[0] for f in self.funcs]
        self.data = {d[0]: (d[1], d[2], d[3]) for d in j['data']}
        self.dk = sorted(self.data)
        self.ssec = j['steam_sections']

    def section(self, rva):
        for n, a, b in self.ssec:
            if a <= rva < b: return n
        return None

    def resolve(self, rva):
        """(epic rva, how) or (None, why)."""
        i = bisect.bisect_right(self.fb, rva) - 1
        if i >= 0 and self.funcs[i][0] <= rva < self.funcs[i][1]:
            sb, se, eb = self.funcs[i]
            return eb + rva - sb, 'code'
        if rva in self.data:
            e, n, t = self.data[rva]
            if n * 2 > t: return e, f'data({n}/{t})'
        sec = self.section(rva)
        if sec == '.text':
            # Code is never placed by its neighbours: its own bytes are searched for.
            if self.simg is None: return None, 'code in an unmatched function'
            return self.search(rva, None, None)
        j = bisect.bisect_left(self.dk, rva)
        lo = next((self.dk[k] for k in range(j - 1, max(j - 200, -1), -1) if self.data[self.dk[k]][1] * 2 > self.data[self.dk[k]][2]), None)
        hi = next((self.dk[k] for k in range(j, min(j + 200, len(self.dk))) if self.data[self.dk[k]][1] * 2 > self.data[self.dk[k]][2]), None)
        if lo is not None and hi is not None and self.section(lo) == sec == self.section(hi):
            dl, dh = self.data[lo][0] - lo, self.data[hi][0] - hi
            if dl == dh: return rva + dl, 'between'
            if sec == '.text' and self.simg is not None: return self.search(rva, lo, hi)
            return None, f'neighbours disagree ({lo:#x}{dl:+#x}, {hi:#x}{dh:+#x})'
        if sec == '.text' and self.simg is not None:
            return self.search(rva, lo, hi)
        if sec == '.text':
            return None, 'code in an unmatched function'
        return None, 'no anchor'

    def pattern(self, rva, n=48):
        """Steam bytes at rva as a regex, relative operands left open."""
        code = self.simg[rva:rva + n + 16]
        parts, got = [], 0
        for i in self.md.disasm(code, rva):
            b = bytearray(i.bytes); mask = [True] * len(b)
            if i.disp_size == 4 and any(op.type == x86.X86_OP_MEM for op in i.operands):
                for k in range(i.disp_offset, i.disp_offset + i.disp_size): mask[k] = False
            if i.imm_size and (i.group(capstone.CS_GRP_JUMP) or i.group(capstone.CS_GRP_CALL)) and i.size >= 5:
                for k in range(i.imm_offset, i.imm_offset + i.imm_size): mask[k] = False
            parts += [re.escape(bytes([x])) if m else b'.' for x, m in zip(b, mask)]
            got += i.size
            if got >= n: break
        return b''.join(parts), got

    def search(self, rva, lo, hi):
        pat, got = self.pattern(rva)
        if got < 16: return None, 'too little code to search'
        deltas = {self.data[a][0] - a for a in (lo, hi) if a is not None}
        i = bisect.bisect_right(self.fb, rva) - 1
        for k in range(max(0, i - 3), min(len(self.funcs), i + 4)):
            deltas.add(self.funcs[k][2] - self.funcs[k][0])
        found = set()
        rx = re.compile(pat, re.S)
        for d in deltas:
            a = max(0, rva + d - 0x8000)
            for mt in rx.finditer(self.eimg, a, rva + d + 0x8000):
                found.add(mt.start())
        if len(found) == 1: return found.pop(), f'searched ({got} bytes)'
        # Several copies of the same code: the one at a neighbouring function's exact shift.
        near = [self.funcs[k][2] - self.funcs[k][0] for k in (i, i + 1) if 0 <= k < len(self.funcs)]
        pick = {f for f in found if f - rva in near}
        if len(pick) == 1: return pick.pop(), f'searched ({got} bytes, {len(found)} copies, the neighbours\' shift)'
        return None, f'search found {len(found)}'
