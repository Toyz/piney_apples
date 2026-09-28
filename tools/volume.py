"""The disc a harness runs the game's code from: PINEY_VOLUME names it
(infection, mutation, outbreak or quarantine; infection by default).

    ELF, ISO, DATA   the volume's executable, disc image and DATA.BIN
    va(inf_addr)     Infection's address of a global carried to the volume:
                     the generator's carry of it (piney-gen carry), else
                     the same symbol by name (a later volume's names come
                     from the `<elf>.syms` sidecar piney-gen syms writes),
                     the same offset into it

A harness written against Infection keeps its addresses as Infection's and
passes each through va(), so that it runs on any volume whose code has the
same globals.
"""
import functools
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXECUTABLES = {"infection": "SLUS_202.67", "mutation": "SLUS_205.62", "outbreak": "SLUS_205.63",
               "quarantine": "SLUS_205.64"}
NAME = os.environ.get("PINEY_VOLUME", "infection")
if NAME not in EXECUTABLES:
    raise SystemExit("PINEY_VOLUME=%s: want one of %s" % (NAME, ", ".join(EXECUTABLES)))


def paths(name):
    """(executable, disc image, DATA.BIN) of volume `name`."""
    disc = os.path.join(ROOT, "work", name, "disc")
    return (os.path.join(disc, EXECUTABLES[name]), os.path.join(ROOT, "work", name, name + ".iso"),
            os.path.join(disc, "DATA", "DATA.BIN"))


ELF, ISO, DATA = paths(NAME)
INF_ELF = paths("infection")[0]


@functools.lru_cache(maxsize=None)
def program(path, overlay="gcmn"):
    from image import Program
    return Program(path, overlay)


CARRY_FILES = {"mutation": "mut_", "outbreak": "out", "quarantine": "qua"}
SECTIONS = {"main": 0, "gcmn": 1, "demo": 2, "desktop": 3, "toppage": 4}


@functools.lru_cache(maxsize=None)
def carry():
    """The generator's carry of Infection's globals to this volume
    (work/analysis/carry/<volume>.json, piney-gen carry): by section, the
    rows (Infection's address, its size, the volume's address), sorted."""
    import json
    rows = json.load(open(os.path.join(ROOT, "work", "analysis", "carry", CARRY_FILES[NAME] + ".json")))
    out = {}
    for section, inf, size, here in rows:
        out.setdefault(section, []).append((inf, size, here))
    for v in out.values():
        v.sort()
    return out


def carried(inf_addr, overlay):
    """The carry's place for `inf_addr`: in the overlay's rows, then main's,
    then any section's; None where no carried global holds it."""
    import bisect
    c = carry()
    order = [SECTIONS.get(overlay, 1), 0] + [k for k in c if k not in (SECTIONS.get(overlay, 1), 0)]
    for section in order:
        rows = c.get(section, [])
        i = bisect.bisect_right(rows, (inf_addr, 1 << 32, 0)) - 1
        if i >= 0:
            inf, size, here = rows[i]
            if inf <= inf_addr < inf + max(size, 1):
                return here + (inf_addr - inf)
    return None


def inf_of(here, overlay="gcmn"):
    """The reverse of va(): Infection's address of the carried global that
    holds the volume's `here` (the overlay's rows, then main's), or `here`
    itself where none does, for a harness that reports the game's pointers
    as the port names them."""
    if NAME == "infection":
        return here
    c = carry()
    for section in (SECTIONS.get(overlay, 1), 0):
        for inf, size, at in c.get(section, []):
            if at <= here < at + max(size, 1):
                return inf + (here - at)
    return here


def va(inf_addr, overlay="gcmn"):
    """Infection's `inf_addr` in this volume: where the carry put the global
    that holds it (the generator's own finding, which also tells apart the
    names that repeat), else by the symbol's carried name."""
    if NAME == "infection":
        return inf_addr
    here = carried(inf_addr, overlay)
    if here is not None:
        return here
    hit = program(INF_ELF, overlay).symbol_at(inf_addr, 0)
    if hit is None:
        raise KeyError("no Infection symbol holds 0x%08x" % inf_addr)
    sym, off = hit
    # The sidecar files a name under the overlay it was carried from, which
    # may not be the one asked for (a toppage global in main's .sbss).
    for ov in (overlay, "gcmn", "demo", "desktop", "toppage"):
        here = program(ELF, ov).symbol_named(sym.name)
        if here is not None:
            return here.value + off
    raise KeyError("%s (Infection 0x%08x) is not in %s" % (sym.name, inf_addr, NAME))


# ccSkill: from Mutation on a vector at +0x70 (where the target stood when
# the skill was asked for) moves the creator and all after it 0x10 on.
SKILL_SIZE = 0xB0 if NAME == "infection" else 0xC0


def skill_at(inf_off):
    """ccSkill's field at Infection's offset `inf_off`, in this volume."""
    return inf_off if NAME == "infection" or inf_off < 0x70 else inf_off + 0x10


def ai_at(inf_off):
    """ccAI's field at Infection's offset `inf_off`, in this volume. From
    Mutation on an int at +0x24 (the attack command's target) moves
    chatText to actSkill 4 on, and two shorts after actSkill (+0x9a,
    +0x9c) move lastMarker to noTurnRange 8 on; posOld on stays."""
    if NAME == "infection" or inf_off < 0x24 or inf_off >= 0xD0:
        return inf_off
    return inf_off + (4 if inf_off < 0x96 else 8)


def callee(caller, nth, overlay="gcmn"):
    """The `nth` (from 0) of the functions without a name that the
    volume's function `caller` calls, in the order of its calls: how a
    harness finds a function a later volume adds (piney-gen's
    `callee_table` finds their tables the same way)."""
    import struct
    p = program(ELF, overlay)
    start, end = span(caller, overlay)
    out = []
    for a in range(start, end, 4):
        w = struct.unpack("<I", p.read(a, 4))[0]
        if w >> 26 == 3:
            t = (w & 0x03FFFFFF) << 2
            hit = p.symbol_at(t, 0)
            if not (hit and hit[1] == 0):
                out.append(t)
    return out[nth]


def number():
    """The disc's `volumeNum` (1-4), as the executable holds it."""
    import struct
    p = program(ELF)
    return struct.unpack("<i", p.read(p.symbol_named("volumeNum").value, 4))[0]


def card_dir():
    """The disc's save directory on the memory card, without its leading
    `/`: `mcDirName[volumeNum - 1]` as the executable holds them."""
    import struct
    p = program(ELF)
    n = struct.unpack("<i", p.read(p.symbol_named("volumeNum").value, 4))[0]
    ptr = struct.unpack("<I", p.read(p.symbol_named("mcDirName").value + 4 * (n - 1), 4))[0]
    b = p.read(ptr, 64)
    return b[:b.index(0)].decode().lstrip("/")


# A slot file on the card: ccSaveData, and from Mutation on its extension.
SLOT_SIZE = 0x8530 if NAME == "infection" else 0x8530 + 0x854


def mail_at(inf_off):
    """A MailList_control field's offset in this volume: from Mutation on
    PhotName holds 30 photos (Infection's 27), so Tempstream (+0x7c) and
    what follows move 12 bytes on."""
    return inf_off if NAME == "infection" or inf_off < 0x7C else inf_off + 0xC


def item_list_at(save, ext, cid):
    """Character `cid`'s item list: in ccSaveData at `save`, or from
    Mutation on for characters 18-20 in the extension at `ext` (the block
    the constructor allocates, its lists first)."""
    if cid < 18:
        return save + 0x30 + 160 * cid
    return ext + 160 * (cid - 18)


# The remarks Mutation adds (piney_battle::party_chat::Remark), unnamed in
# its code: our name, the caller and which of its unnamed callees, and
# whether it takes the item's code.
REMARKS = (("ChatMessagePhysicalTolerance", "ChatMessageAttack__4ccAIFP6ccCharii", 1, False),
           ("ChatMessageMagicTolerance", "ChatMessageAttack__4ccAIFP6ccCharii", 2, False),
           ("ChatMessageUseItem", "UseItem__4ccAIFiP6ccChar", 2, True),
           ("ChatMessageUseLastItem", "UseItem__4ccAIFiP6ccChar", 1, True),
           ("ChatMessageOnlyBuff", "ChatCommandBuffPlz__4ccAIFv", 2, False),
           ("ChatMessageOnlyDebuff", "ChatCommandDeBuffPlz__4ccAIFv", 1, False))


def remarks():
    """{address: (our name, takes the item's code)} of the remark
    functions; none on Infection."""
    if NAME == "infection":
        return {}
    return {callee(c, n): (name, item) for name, c, n, item in REMARKS}


def menu_at(inf_off):
    """ccMenuCtrl's field at Infection's offset `inf_off`, in this volume.
    From Mutation on a short at +0x12c moves protect and protectCnt 2 on,
    and protectChar and all after it 4 on."""
    if NAME == "infection" or inf_off < 0x12C:
        return inf_off
    return inf_off + (2 if inf_off < 0x15C else 4)


def unused_list():
    """From Mutation on, the buff and debuff searches' candidate list (33
    words): the address the first `sw $zero` of SearchBuffUnusedFellow
    writes; None on Infection."""
    import struct
    if NAME == "infection":
        return None
    p = program(ELF)
    start, end = span("SearchBuffUnusedFellow__4ccAIFi")
    hi = None
    for a in range(start, end, 4):
        w = struct.unpack("<I", p.read(a, 4))[0]
        if w >> 16 == 0x3C01:                       # lui $at, hi
            hi = (w & 0xFFFF) << 16
        elif hi is not None and w >> 16 == 0xAC20:  # sw $zero, lo($at)
            return (hi + ((w & 0xFFFF) ^ 0x8000) - 0x8000) & 0xFFFFFFFF
    raise KeyError("no candidate list in SearchBuffUnusedFellow")


def span(name, overlay="gcmn"):
    """The volume's function `name`: (its first address, the one past its
    last), for a harness that tells callers apart by where they return to."""
    sym = program(ELF, overlay).symbol_named(name)
    if sym is None:
        raise KeyError("%s is not in %s" % (name, NAME))
    return sym.value, sym.value + sym.size
