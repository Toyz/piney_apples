#!/usr/bin/env python3
"""crates/piney-battle's Data Drain against the game's
ccMenuCtrl::DataDrainMenu (0x00532ae0) in tools/eemu.py, as
tools/test_battle_rs.py does: each step run natively with the menu's
presentation (fades, noise, messages, streams, effects) stubbed.

  drain         step 0 after the movie, then step 1: the infection, the
                drops, the targets leaving, the side-effect roll
  side_effect   step 10: the side effect on Kite and the party
  evolution     step 20: the drain count and the bracelet's growth

    python3 tools/test_battle_drain_rs.py           the unit tests
    python3 tools/test_battle_drain_rs.py bulk N    N cases of every check
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc
import test_battle_rs as T  # noqa: E402

MENU = 0x01020000
NOIZ = 0x01038100
FADE = 0x01038300
LAYER = 0x01038400
GLAYER = 0x01038500
STREAM = 0x01038600
THREADS = 0x01039000


def drain_hooks(checks):
    if getattr(checks, "_drain_hooked", False):
        return
    checks._drain_hooked = True
    T.affect_hooks(checks)
    g = checks.game
    m = g.m
    sym = g.sym
    rec = g.record
    nop = lambda mm, *_: 0  # noqa: E731
    checks.threads = []

    def start_thread(mm, fn, *_):
        a = THREADS + 0x100 * len(checks.threads)
        mm.mem[a:a + 0x100] = bytes(0x100)
        checks.threads.append(a)
        return a

    def breath(mm, *_):
        # The stream thread ends as soon as the menu waits for it.
        for a in checks.threads:
            mm.store(a + 0x14, 4, 0xFFFFFFFF)
        return 0

    hooks = {
        "EntryFade__8ccScFadeFiiiffff": lambda mm, *_: 1,
        "CheckFade__8ccScFadeFi": lambda mm, *_: 0,
        "DeleteFade__8ccScFadeFi": nop,
        "EntryFlash__8ccScFadeFiiffff": nop,
        "SetNoizRn__6ccNoizFi": nop, "SetNoizBs__6ccNoizFi": nop, "SetNoizBr__6ccNoizFi": nop,
        "SetNoiz__6ccNoizFiii": nop,
        "Disp__10ccMenuCtrlFv": nop,
        "ccBreathThread__Fi": breath,
        "fontOnFlip__Fv": nop, "fontOffFlip__Fv": nop,
        "OnFlipExcept__7ccLayerFv": nop, "OffFlipExcept__7ccLayerFv": nop,
        "ccStartThread__FPFPv_vii": start_thread,
        "ccDeleteThread__FP6ccTscb": nop,
        "ccGetStreamAdrs__Fv": lambda mm, *_: STREAM,
        "ccGetStreamFrame__Fv": nop,
        "ccGetStreamNum__Fv": lambda mm, *_: 1,
        "ccCheckObjectSize__FP6ccChar": lambda mm, *_: 1,
        "ccSleepAllThread__Fv": nop, "ccWakeAllThread__Fv": nop,
        "OpenInfo__9ccMessageFPcPcPcPcii": nop,
        "Check__9ccMessageFi": lambda mm, *_: 1,
        "Close__9ccMessageFv": nop,
        "ChangeInfo__9ccMessageFPcPcPcPcii": nop,
        "ccKanjiStrSeparate__FPci": lambda mm, a, *_: a,
        "ccSeOn__Fi": nop,
        "effHeal__FP6ccChari": lambda mm, a, b_, *_: rec("effHeal", a, b_),
        "effSkillStartEffect__FP6ccCharii": lambda mm, a, b_, c, *_: rec("effSkillStartEffect", a, b_, c),
        "ccEntryFlyFontNewMiss__FPfP6ccChar": lambda mm, a, b_, *_: rec("miss", b_),
        "ccEntryFlyFontNewExp__FiiPfP6ccChar": lambda mm, a, b_, c, d: rec("exp", a, checks.b.s32(b_), d),
        "ccEntryFlyFontNewLevelDown__FiPfP6ccChar": lambda mm, a, b_, c, *_: rec("leveldown", a, c),
        "ccDeleteCmnd__FP6ccChar": lambda mm, a, *_: rec("ccDeleteCmnd", a),
        "ccGetItemName__Fii": lambda mm, *_: 0x01038700,
        "AddItem__10ccSaveDataFiiii": lambda mm, a, b_, c, d: rec("AddItem", b_, c, d, mm.r[8]),
    }
    for name, fn in hooks.items():
        m.hooks[sym(name)] = fn
    checks.cmnd_target = sym("cmndTarget")
    checks.plw = sym("plw")
    checks.volume = sym("volumeNum")


def put_menu(checks, step):
    g = checks.game
    m = g.m
    m.mem[MENU:MENU + 0x300] = bytes(0x300)
    g.put(MENU + 6, "<h", 0)
    g.put(MENU + 0x1A, "<hh", step, 0)
    m.store(MENU + 0xB8, 4, FADE)
    m.store(MENU + 0xE8, 4, NOIZ)
    m.store(NOIZ + 0x10, 4, NOIZ + 0x80)
    m.store(MENU + 0x58, 4, LAYER)
    m.store(checks.tb.GAME + 0x80, 4, GLAYER)
    m.mem[STREAM:STREAM + 0x200] = bytes(0x200)
    g.put(STREAM + 0x17E, "<H", 8)
    checks.threads = []


def check_drain(checks, rnd):
    """Step 0 from the movie's end, falling into step 1."""
    drain_hooks(checks)
    g = checks.game
    m = g.m
    chars, pcs, enes = checks.rnd_scene(rnd)
    pcs_idx = [i for i, c in enumerate(chars) if c.type & 7]
    foes = [i for i, c in enumerate(chars) if not c.type & 7]
    if not pcs_idx or not foes:
        extra = checks.tb.rnd_pc(checks.b, checks.data, rnd)
        chars.append(extra)
        pcs.append(len(chars) - 1)
        pcs_idx = [len(chars) - 1]
        extra2 = checks.tb.rnd_foe(checks.b, checks.data, rnd, "enemy")
        chars.append(extra2)
        enes.append(len(chars) - 1)
        foes = [len(chars) - 1]
    kite = rnd.choice(pcs_idx)
    target = rnd.choice(foes + [rnd.randrange(len(chars))])
    for i in foes:
        if hasattr(chars[i], "PPcount"):
            chars[i].PPcount = rnd.choice((0, 0, 300, 5))
    sid = rnd.choice((2, 3, 4, 5, 3, 5))
    erosion = rnd.choice((0, 10, 50, 99, 100, rnd.randrange(0, 101)))
    state = rnd.getrandbits(64)
    put_menu(checks, 0)
    checks.scene.put(chars, pcs, enes)
    va = checks.scene.va
    m.store(checks.cmnd_target, 4, va(target))
    m.store(checks.plw + 0x20, 4, va(kite))
    m.store(MENU + volume.menu_at(0x238), 4, sid)
    g.put(checks.tb.SAVE + 0x676E, "<h", erosion)
    patch = checks.patched(rnd, sid)
    g.set_rand(state)
    g.calls = []
    m.call(g.sym("DataDrainMenu__10ccMenuCtrlFv"), (MENU,))
    calls = [c for c in g.calls if c[0] != "ccDeleteCmnd"]
    out = {"ret": 0, "rand": g.rand_now(), "calls": calls, "chars": checks.scene.read(chars),
           "erosion": g.shorts(checks.tb.SAVE + 0x676E, 1)[0],
           "drops": [checks.b.s32(m.load(MENU + volume.menu_at(0x1AC) + 4 * k, 4)) for k in range(17)],
           "count": m.load(MENU + volume.menu_at(0x1F0), 4), "remove": int(any(c[0] == "ccDeleteCmnd" for c in g.calls)),
           "side": int(g.shorts(MENU + 0x1A, 1)[0] == 10)}
    checks.scene.restore()
    g.restore(checks.data.skills[sid]["va"], 0x38)
    req = f"{checks.ser_scene(chars, pcs, enes)} drain {kite} {target} {sid} {erosion} {state}"
    return [patch, req, f"restore {sid}"], out, "drain"


def side_starts(calls, idx):
    """Step 10's effects and numbers in the order the game starts them, each
    [kind, who, value]: 0 effHeal(ch, 1), 1 the MISS, 2
    effSkillStartEffect(ch, 0, 1), 3 ccEntryFlyFontNewExp(23, value, pos,
    ch), 4 effAfterDrain(ch, 0). A start with other arguments is kept as
    [-1, ...] so it shows as a difference."""
    out = []
    for c in calls:
        if c[0] == "effHeal":
            out += [0, idx[c[1]], 0] if c[2] == 1 else [-1, idx[c[1]], c[2]]
        elif c[0] == "miss":
            out += [1, idx[c[1]], 0]
        elif c[0] == "effSkillStartEffect":
            out += [2, idx[c[1]], 0] if (c[2], c[3]) == (0, 1) else [-1, idx[c[1]], c[2]]
        elif c[0] == "exp":
            out += [3, idx[c[3]], c[2]] if c[1] == 23 else [-1, idx[c[3]], c[1]]
        elif c[0] == "effAfterDrain":
            out += [4, idx[c[1]], 0] if c[2] == 0 else [-1, idx[c[1]], c[2]]
    return out


def check_side_effect(checks, rnd):
    """Step 10: the side effect."""
    drain_hooks(checks)
    b, tb = checks.b, checks.tb
    g = checks.game
    m = g.m
    chars, pcs, enes = checks.rnd_scene(rnd)
    pcs_idx = [i for i, c in enumerate(chars) if c.type & 7]
    if not pcs_idx:
        chars.append(tb.rnd_pc(b, checks.data, rnd))
        pcs.append(len(chars) - 1)
        pcs_idx = [len(chars) - 1]
    kite = rnd.choice(pcs_idx)
    chars[kite].exp = rnd.choice((0, 100, 199, 500, 999, rnd.randrange(0, 1000)))
    chars[kite].level = rnd.choice((1, 1, 2, 30))
    members = [kite] + [rnd.choice(pcs_idx + [-1]) for _ in range(2)]
    members = [x if x not in members[:k] else -1 for k, x in enumerate(members)]
    for c in chars:
        c.real[14] = rnd.choice((0, rnd.randrange(0, 1100)))
        c.real[15] = rnd.choice((0, rnd.randrange(0, 1100)))
    erosion = rnd.choice((0, 24, 25, 50, 75, 99, 100, rnd.randrange(0, 101)))
    items = []
    for _ in range(40):
        if rnd.random() < 0.3:
            items.append((-1, -1, 0))
        else:
            items.append((rnd.randrange(0, 300), rnd.choice((10, 11, 12, 13, 14, 15, 15, 6, 0)),
                          rnd.randrange(1, 10)))
    state = rnd.getrandbits(64)
    put_menu(checks, 10)
    checks.scene.put(chars, pcs, enes)
    va = checks.scene.va
    m.store(checks.plw + 0x20, 4, va(kite))
    for k in range(3):
        m.store(g.sym("ccPartyManager") + 4 * k, 4, va(members[k]) if members[k] >= 0 else 0)
    save = tb.SAVE
    g.put(save + 0x676E, "<h", erosion)
    for i, (iid, cat, num) in enumerate(items):
        g.put(save + 0x30 + 4 * i, "<hbb", iid, cat, num)
    g.set_rand(state)
    g.calls = []
    m.call(g.sym("DataDrainMenu__10ccMenuCtrlFv"), (MENU,))
    idx = {f"c{i}": i for i in range(len(chars))}
    fx_id = g.shorts(inf_va(0x0072EFE0) + 0x10, 1)[0]
    level_down = g.shorts(inf_va(0x0072EFE0) + 0xC, 1)[0]
    lost = checks.b.s32(m.load(MENU + volume.menu_at(0x23C), 4))
    exp_calls = [c for c in g.calls if c[0] == "exp"]
    out = {"ret": 0, "rand": g.rand_now(), "calls": [], "chars": checks.scene.read(chars),
           "fx": [fx_id, level_down, lost if fx_id == 29 else -1, -exp_calls[0][2] if exp_calls else 0],
           "items": [v for i in range(40) for v in g.get(save + 0x30 + 4 * i, "<hbb")],
           "missed": [idx[c[1]] for c in g.calls if c[0] == "miss"],
           "shown": [idx[c[1]] for c in g.calls if c[0] == "effSkillStartEffect"],
           "healed": [idx[c[1]] for c in g.calls if c[0] == "effHeal"],
           "starts": side_starts(g.calls, idx)}
    checks.scene.restore()
    for k in range(3):
        m.store(g.sym("ccPartyManager") + 4 * k, 4, 0)
    req = (f"{checks.ser_scene(chars, pcs, enes)} drainfx {kite} {members[0]} {members[1]} {members[2]} {erosion} "
           + " ".join(f"{a} {b_} {c}" for a, b_, c in items) + f" {state}")
    return req, out, "side_effect"


def check_evolution(checks, rnd):
    """Step 20: the drain count and the bracelet's growth."""
    drain_hooks(checks)
    g = checks.game
    m = g.m
    count = rnd.choice((0, 9, 10, 19, 20, 79, 80, 159, 160, 239, 240, 9999, 10000, rnd.randrange(0, 300)))
    evo = rnd.choice((0, 1, 7, 8, 9, 10, 11, rnd.randrange(0, 12)))
    parody = rnd.choice((0, 0, 1))
    volume = rnd.choice((1, 2, 3, 4))
    put_menu(checks, 20)
    save = checks.tb.SAVE
    g.put(save + 0x685E, "<hh", count, evo)
    m.store(save + 0x842B, 1, parody)
    m.store(checks.volume, 4, volume)
    g.calls = []
    m.call(g.sym("DataDrainMenu__10ccMenuCtrlFv"), (MENU,))
    adds = [c for c in g.calls if c[0] == "AddItem"]
    new_count, new_evo = g.shorts(save + 0x685E, 2)
    out = {"count": new_count, "evo": new_evo, "stage": new_evo if new_evo != evo else 0,
           "item": (adds[0][2] << 16 | adds[0][3]) if adds else -1}
    m.store(checks.volume, 4, 1)
    return f"evolution {count} {evo} {parody} {volume}", out, "evolution"


DRAIN = {"drain": check_drain, "side_effect": check_side_effect, "evolution": check_evolution}


@unittest.skipUnless(T.READY, "needs the extracted disc and cargo")
class DataDrainAgainstGame(T.Against):
    TABLE = DRAIN

    def test_drain(self):
        self.check("drain", 8000)

    def test_side_effect(self):
        self.check("side_effect", 8001)

    def test_evolution(self):
        self.check("evolution", 8002)


if __name__ == "__main__":
    T.main(list(DRAIN), 8000, DRAIN)
