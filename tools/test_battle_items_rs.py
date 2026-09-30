#!/usr/bin/env python3
"""crates/piney-battle's items (piney_battle::item) against the game's code in tools/eemu.py.

Each check builds a random case, runs the game's function natively in the
interpreter over a scene of characters (tools/test_battle_rs.py's
GameScene), sends the same case to the battle_probe example and compares
everything: the characters (with skillID, skillStatus, noDeath, the word at
+0x140 and pauseSW), the ccSkill an item starts, the newlib rand() state,
the globals the item code writes (plw.pauseSW, dneFlag, the menu's
panelStatus and bgStatus), the save's item lists, and every call in order.
The runtime's functions are stubbed and record their arguments; their waits
(for a message, the map, the party, the Grunty ride, the book viewer) run a
random number of frames and read back as one step each, as the port gives
them.

    python3 tools/test_battle_items_rs.py           the unit tests
    python3 tools/test_battle_items_rs.py bulk N    N cases of every check

Checks (ITEMS: name -> fn(checks, rnd) returning (request, game answer,
label), checks a test_battle_rs.Checks):
  item_useful      ccCheckItemUseful (0x0057a6d0)
  skill_useful     ccCheckSkillUseful (0x0057a890)
  item_skill       ccItemSkillRequest (0x00572790), ccItemSkillRequestParam
                   (0x005727d0), ccItemSkillCompel (0x00572730)
  use_item         ccUseItemRequest (0x0057aa80), every category
  give_stat_items  ccMenuCtrl::AddSpcItem with books (0x00527950)
  ai_use_item      ccAI::UseItem (0x00589410) past its checks: the use, then
                   ccAI::ConsumeItemList on the member's list
  item_lists       ccSaveData::AddItem, DelItem, GetItemNum, GetItemSlot,
                   AddPlItem, DelPlItem, GetPlItemNum, GetPlItemSlot (main),
                   ccAI::ConsumeItemList (0x00588860)

CARGO_TARGET_DIR is honoured when locating the probe.
"""

import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import test_battle_rs as rs  # noqa: E402
import test_battle_party_ai_rs as PA  # noqa: E402
import volume  # noqa: E402
from test_battle_rs import F_ONE, NEW, fbits, rnd_pos  # noqa: E402

SAVE, MENU = 0x01010000, 0x01020000
SAVE2 = 0x01040000          # test_battle.SAVE2: the save extension (Mutation on)
TCB = 0x01033000            # the book viewer's thread block
STRS = 0x01034000           # ccKanjiStrSeparate's answers
AI_VA, CH_VA, BASE_VA = 0x01031000, 0x01000000, 0x01004000
OLD_SKILL = 0x01032400
SAVE_SIZE = 0x8530
ITEM_LIST, PL_LIST, IMP_LIST = 0x30, 0xB70, 0xCFC
ORDER = [(10, 24), (13, 3), (11, 72), (12, 34), (14, 22), (0, 82), (1, 77), (2, 97), (3, 75), (4, 74),
         (5, 76), (6, 69), (7, 68), (8, 67), (9, 68)]
ROWS = {10: 24, 11: 72, 12: 34, 13: 3, 14: 22, 15: 291}
IMPORTANT = (42, 43, 44, 45, 46, 47, 48, 49, 60, 61, 68, 69, 273, 274, 275, 276, 277, 278, 279, 280, 287, 288, 289,
             290)
FIELD_FRAME = ("CtrlAll", "ChatDisp", "Trans", "Breath")


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v & 0x80000000 else v


def normalize(raw):
    """The game's recorded calls in the port's terms: menu frames (Disp,
    Breath(1)) as ["frames", n], and each wait loop as one step."""
    out = []
    n = len(raw)

    def menu_frame(j):
        return j + 1 < n and raw[j] == ("Disp",) and raw[j + 1] == ("Breath", 1)

    def add_frames(k):
        if out and out[-1][0] == "frames":
            out[-1][1] += k
        else:
            out.append(["frames", k])

    i = 0
    while i < n:
        r = raw[i]
        k = r[0]
        if k == "Check":
            j = i
            while j < n and raw[j] != ("Check", 1):
                if raw[j] == ("Check", 0):
                    j += 1
                elif menu_frame(j):
                    j += 2
                else:
                    break
            if j < n and raw[j] == ("Check", 1):
                out.append(["WaitMessage"])
                i = j + 1
            else:
                out.append(["BadWait", "message"])
                i += 1
            continue
        if k == "ShowMap":
            zeros = frames = 0
            j = i
            while j < n:
                if raw[j] == ("ShowMap", 0):
                    zeros += 1
                    j += 1
                elif menu_frame(j):
                    frames += 1
                    j += 2
                elif raw[j] == ("ShowMap", 1):
                    j += 1
                    while menu_frame(j):
                        frames += 1
                        j += 2
                    break
                else:
                    break
            out.append(["WaitMap"] if frames == max(zeros, 20) else ["BadWait", "map", zeros, frames])
            i = j
            continue
        if k == "checkPartyAnnihilation":
            nd = r[2]
            j = i
            while j < n and raw[j][0] in ("checkPartyAnnihilation",) + FIELD_FRAME:
                j += 1
                if raw[j - 1][0] == "checkPartyAnnihilation" and raw[j - 1][1] == 0:
                    break
            out.append(["WaitParty", nd])
            i = j
            continue
        if k == "ccPuccigusoStart":
            out.append(list(r))
            j = i + 1
            while j < n and raw[j][0] in FIELD_FRAME:
                j += 1
            out.append(["WaitRide"])
            i = j
            continue
        if k == "CloseChat":
            out.append(["CloseChat"])
            j = i + 1
            if menu_frame(j):
                add_frames(1)
                j += 2
            while j < n and raw[j][0] in ("Disp", "Breath"):
                j += 1
            if j < n and raw[j][0] == "ccDeleteThread":
                out.append(["WaitBook", raw[j][1]])
                j += 1
            else:
                out.append(["BadWait", "book"])
            i = j
            continue
        if menu_frame(i):
            add_frames(1)
            i += 2
            continue
        out.append(list(r))
        i += 1
    return out


def rnd_code(rnd):
    """An item code: every category's rows, the special ones often."""
    cat = rnd.choice((10, 10, 10, 11, 11, 12, 12, 13, 13, 14, 15, 15, 15))
    if cat == 10:
        i = rnd.choice((18, 19, 20, 21, 22, 23, rnd.randrange(24)))
    elif cat == 11:
        i = rnd.randrange(72)
    elif cat == 12:
        i = rnd.choice((rnd.randrange(34), rnd.randrange(34), rnd.randrange(34, 60), 24, 25, 30, 31, 32, 33))
    elif cat == 13:
        i = rnd.choice((0, 1, 2, rnd.randrange(3, 10)))
    elif cat == 14:
        i = rnd.randrange(30)
    else:
        i = rnd.choice(IMPORTANT + IMPORTANT + (49, 49, 49, 49, rnd.randrange(291)))
    if rnd.random() < 0.02:
        return -rnd.randrange(1, 1 << 20)
    return cat << 16 | i


def rnd_slot(rnd, near=None):
    p = rnd.random()
    if near is not None and p < 0.25:
        return near
    if p < 0.4:
        return (-1, -1, 0)
    if p < 0.85:
        cat, rows = rnd.choice(ORDER)
        i = rnd.randrange(rows) if rnd.random() < 0.9 else rnd.randrange(rows, rows + 4)
        return (i, cat, rnd.choice((1, 1, 2, 5, rnd.randrange(1, 100), 99, 0, -3, 120)))
    if p < 0.92:
        return (rnd.randrange(-3, 300), 15, rnd.randrange(-5, 100))
    return (rnd.randrange(-32768, 32768), rnd.randrange(-128, 128), rnd.randrange(-128, 128))


def pack_list(slots):
    return b"".join(struct.pack("<hbb", *s) for s in slots)


class ItemRuntime:
    """The item functions' runtime stubbed over a test_battle_rs Checks
    (its GameBattle and GameScene): the hooks, and the setup and read-back
    the checks share."""

    def __init__(self, checks):
        self.checks = checks
        self.game = checks.game
        self.scene = checks.scene
        self.b, self.tb, self.data = checks.b, checks.tb, checks.data
        g = self.game
        m = g.m
        sym = g.sym
        self.names_str = {}
        for nm in ("statusUpStr", "trapDischargeStr", "showMapInfo", "installWarnStr", "epitaphStr0X"):
            self.names_str[m.load(sym(nm), 4)] = nm
        self.plw = sym("plw")
        self.party_mgr = sym("ccPartyManager")
        self.dne = sym("dneFlag")
        self.game_over = sym("compulsionGameOver")
        self.pg_ride = sym("pgRideFlag")
        self.sort_root = sym("cmndSortRoot")
        self.th_book = sym("ccThBook__FPv")
        self.str_at = STRS
        self.queues = {}
        self.book_left = None
        self.ride_left = 0
        self.rnd = None
        self.tp_va = 0
        self.in_view = {}
        rec = self.raw

        def sep(mm, s, k, *_):
            txt = f"<{self.names_str.get(s, hex(s))}:{s32(k)}>".encode() + b"\0"
            a = self.str_at
            self.str_at += 64
            mm.mem[a:a + len(txt)] = txt
            return a

        def dec2sjis(mm, v, buf, *_):
            txt = f"#{s32(v)}".encode() + b"\0"
            mm.mem[buf:buf + len(txt)] = txt
            return buf

        def text(mm, p):
            if p == 0:
                return 0
            if p in self.names_str:
                return self.names_str[p]
            return bytes(mm.mem[p:mm.mem.index(0, p)]).decode("latin-1")

        def open_info(mm, _this, a, b, c):
            rec("OpenInfo", text(mm, a), text(mm, b), text(mm, c), text(mm, mm.r[8] & 0xFFFFFFFF),
                s32(mm.r[9]), s32(mm.r[10]))
            return 0

        def queue(name, default):
            def f(mm, *_):
                q = self.queues.get(name) or []
                v = q.pop(0) if q else default
                if name == "checkPartyAnnihilation":
                    pl = mm.load(self.plw + 0x20, 4)
                    rec(name, v, (mm.load(pl + 0xE0, 1) >> 7) & 1 if pl else 0)
                else:
                    rec(name, v)
                return v
            return f

        def breath(mm, a, *_):
            rec("Breath", s32(a))
            if self.book_left is not None and mm.load(TCB + 0x18, 4):
                if self.book_left <= 0:
                    mm.store(TCB + 0x18, 4, 0)
                else:
                    self.book_left -= 1
                    mm.store(TCB + 0x18, 4, self.rnd.choice((1, 1, 2)))
            if self.ride_left > 0 and mm.load(self.pg_ride, 4):
                self.ride_left -= 1
                if self.ride_left == 0:
                    mm.store(self.pg_ride, 4, 0)
            return 0

        def pucciguso(mm, n, *_):
            rec("ccPuccigusoStart", s32(n))
            if self.rnd.random() < 0.7:
                mm.store(self.pg_ride, 4, 1)
                self.ride_left = self.rnd.randrange(1, 4)
            return 0

        def start_thread(mm, fn, pri, size, *_):
            rec("ccStartThread", "ccThBook" if fn == self.th_book else fn, s32(pri), s32(size))
            mm.mem[TCB:TCB + 0x40] = bytes(0x40)
            self.book_left = self.rnd.randrange(0, 4)
            return TCB

        def camera(mm, p, *_):
            return int(self.in_view.get(mm.load(p, 4), 0))

        self.item_hooks = {
            "effHeal__FP6ccChari": lambda mm, a, b, *_: rec("effHeal", g.name(a), s32(b)),
            "CloseMenuDisp__10ccMenuCtrlFv": lambda mm, *_: rec("CloseMenuDisp"),
            "Disp__10ccMenuCtrlFv": lambda mm, *_: rec("Disp"),
            "ccBreathThread__Fi": breath,
            "ccSeOn__Fi": lambda mm, a, *_: rec("ccSeOn", s32(a)),
            "ccSeOnNote__Fic": lambda mm, a, b, *_: rec("ccSeOnNote", s32(a), b & 0xFF),
            "ccKanjiStrSeparate__FPci": sep,
            "dec2sjis__FiPcii": dec2sjis,
            "OpenInfo__9ccMessageFPcPcPcPcii": open_info,
            "Check__9ccMessageFi": queue("Check", 1),
            "Close__9ccMessageFv": lambda mm, *_: rec("Close"),
            "CloseInstant__9ccMessageFv": lambda mm, *_: rec("CloseInstant"),
            "ccChangeCmndTarget__FP6ccChar": lambda mm, a, *_: rec("ccChangeCmndTarget", s32(a)),
            "effRemoveTrap__FPfii": lambda mm, p, a, b, *_: rec(
                "effRemoveTrap", *[mm.load(p + 4 * i, 4) for i in range(4)], s32(a), s32(b)),
            "ccSleepAllThread__Fv": lambda mm, *_: rec("ccSleepAllThread", mm.load(self.tp_va + 0x140, 4)),
            "ccWakeAllThread__Fv": lambda mm, *_: rec("ccWakeAllThread"),
            "StillOn__10ccMenuCtrlFv": lambda mm, *_: rec("StillOn"),
            "StillOff__10ccMenuCtrlFv": lambda mm, *_: rec("StillOff"),
            "ccDisableThEvent__Fv": lambda mm, *_: rec("ccDisableThEvent"),
            "CheckControlMode__9ccSpcCharFv": lambda mm, *_: self.ctrl,
            "ManualModeAI__9ccSpcCharFi": lambda mm, a, b, *_: rec("ManualModeAI", g.name(a), s32(b)),
            "CheckSysMsgID__9ccSpcCharFv": lambda mm, *_: self.sysid,
            "ccAISysMsgSend__FisUsUsUsUs": lambda mm, a, b, c, d: rec(
                "ccAISysMsgSend", s32(a), s16(b), c & 0xFFFF, d & 0xFFFF, mm.r[8] & 0xFFFF, mm.r[9] & 0xFFFF),
            "ChangeMenu__10ccMenuCtrlFi": lambda mm, a, b, *_: rec("ChangeMenu", s32(b)),
            "ShowMap__9WORLD_MANFv": queue("ShowMap", 1),
            "CtrlAll__11ccDamUprStrFv": lambda mm, *_: rec("CtrlAll"),
            "Disp__9ccChatMsgFi": lambda mm, a, b, *_: rec("ChatDisp", s32(b)),
            "Trans__8ccSpriteFv": lambda mm, *_: rec("Trans"),
            "checkPartyAnnihilation__Fv": queue("checkPartyAnnihilation", 0),
            "ccClearConditionAllEnemy__Fv": lambda mm, *_: rec("ccClearConditionAllEnemy"),
            "ccPuccigusoStart__Fi": pucciguso,
            "ccEpitaphMsg__FPPci": lambda mm, a, b, *_: rec("ccEpitaphMsg", volume.inf_of(a), s32(b)),
            "ccStartThread__FPFPv_vii": start_thread,
            "CloseChat__9ccChatMsgFv": lambda mm, *_: rec("CloseChat"),
            "ccDeleteThread__FP6ccTscb": lambda mm, a, *_: rec("ccDeleteThread", s32(mm.load(a + 0x14, 4))),
            "ccCheckCameraDeg__FPfs": camera,
        }
        self.item_hooks = {sym(k): v for k, v in self.item_hooks.items()}
        # Mutation's remarks after an AI's use (unnamed in its code)
        for va_, (short, item) in volume.remarks().items():
            self.item_hooks[va_] = (lambda short: lambda mm, ai, a, *_: rec(short, s32(a)))(short)

    def raw(self, *r):
        self.game.calls.append(r)
        return 0

    # the game side ---------------------------------------------------------------------
    def hooked(self, fn):
        m = self.game.m
        saved = {a: m.hooks.get(a) for a in self.item_hooks}
        m.hooks.update(self.item_hooks)
        try:
            return fn()
        finally:
            for a, h in saved.items():
                if h is None:
                    m.hooks.pop(a, None)
                else:
                    m.hooks[a] = h

    def ensure_pc(self, rnd, chars, pcs, enes):
        """At least one party member in the scene (listed, usually)."""
        if not any(c.type & 7 for c in chars):
            b, tb, data = self.b, self.tb, self.data
            pc = tb.rnd_pc(b, data, rnd)
            pc.type = rnd.choice((7, 6, 4, 2, 1))
            for k in ("cond", "pos_p", "width", "ai", "party_flag", "skill_id", "skill_status", "anm_flag"):
                if hasattr(chars[0], k) and k != "cond":
                    setattr(pc, k, getattr(chars[0], k))
            pc.cond[0] = 0
            chars[0] = pc
            if 0 in enes:
                enes.remove(0)
            if rnd.random() > 0.05:
                pcs.insert(0, 0)
        return [i for i, c in enumerate(chars) if c.type & 7]

    def scene_setup(self, rnd, chars, pcs, enes, running, cp):
        g = self.game
        m = g.m
        self.scene.put(chars, pcs, enes)
        m.store(self.scene.xnote, 4, 0)
        for a in (self.scene.skill_top, self.scene.skill_tail, self.scene.skill_num):
            m.store(a, 4, 0)
        if running:
            m.mem[OLD_SKILL:OLD_SKILL + volume.SKILL_SIZE] = bytes(volume.SKILL_SIZE)
            m.store(OLD_SKILL, 4, 1)
            m.store(OLD_SKILL + volume.skill_at(0x70), 4, self.scene.va(cp))
            m.store(self.scene.skill_top, 4, OLD_SKILL)
            m.store(self.scene.skill_tail, 4, OLD_SKILL)
        self.scene.new_at = NEW
        self.str_at = STRS
        g.calls = []

    def read_chars(self, chars):
        m = self.game.m
        out = self.scene.read(chars)
        vas = [self.scene.va(i) for i in range(len(out))]
        for i, st in enumerate(out):
            va = vas[i]
            e0 = m.load(va + 0xE0, 1)
            tc = m.load(va + 0x78, 4)
            st["extra"] = [s16(m.load(va + 0x7C, 2)), s16(m.load(va + 0x7E, 2)), (e0 >> 7) & 1,
                           m.load(va + 0x140, 4), e0 & 1, vas.index(tc) if tc in vas else -1]
        return out

    def read_skill(self):
        m = self.game.m
        if m.load(self.scene.skill_num, 4) == 0:
            return None
        sk = m.load(self.scene.skill_tail, 4)
        b12, b13 = m.load(sk + 12, 1), m.load(sk + 13, 1)
        return {"sid": s32(m.load(sk, 4)), "stype": b12 & 0xF, "param": s32(m.load(sk + 0x2C, 4)),
                "ac": s16(m.load(sk + 0x54, 2)), "modify": (b12 >> 6) & 1, "hold": b13 & 1,
                "compel": (b13 >> 1) & 1, "creator": self.game.name(m.load(sk + volume.skill_at(0x70), 4)),
                "target": self.game.name(m.load(sk + volume.skill_at(0x74), 4))}

    def ser_scene(self, chars, pcs, enes):
        return self.checks.ser_scene(chars, pcs, enes)[len("scene "):]

    def rnd_scene(self, rnd):
        return self.checks.rnd_scene(rnd)

    def put_skill(self, sk, va):
        self.checks.put_skill(sk, va)

    def rnd_env(self, rnd, chars):
        """The player, the party and the globals ccUseItemRequest reads."""
        pcs_all = [i for i, c in enumerate(chars) if c.type & 7]
        player = rnd.choice(pcs_all)
        party = [rnd.choice(pcs_all + [-1]) for _ in range(3)]
        return {"player": player, "party": party, "pause": rnd.choice((0, 1)), "plw": rnd.choice((0, 1)),
                "dne": rnd.choice((0, 0, 1, rnd.randrange(-5, 5))), "parody": rnd.choice((0, 1)),
                "ctrl": rnd.choice((0, 0, 1, 2)), "sysid": rnd.randrange(0x10000), "running": rnd.choice((0, 1))}

    def ser_env(self, env):
        return " ".join(str(v) for v in [env["player"]] + env["party"] + [
            env["pause"], env["plw"], env["dne"], env["parody"], env["ctrl"], env["sysid"], env["running"]])

    def put_env(self, rnd, env):
        g = self.game
        m = g.m
        va = self.scene.va
        pl = va(env["player"])
        m.store(pl + 0xE0, 1, m.load(pl + 0xE0, 1) & ~1 | env["pause"])
        m.store(self.plw, 1, m.load(self.plw, 1) & ~1 | env["plw"])
        m.store(self.plw + 0x20, 4, pl)
        for n in range(3):
            m.store(self.party_mgr + 4 * n, 4, va(env["party"][n]) if env["party"][n] >= 0 else 0)
        m.store(self.dne, 4, env["dne"] & 0xFFFFFFFF)
        m.store(self.game_over, 4, 0)
        m.store(self.pg_ride, 4, 0)
        m.store(SAVE + 0x842B, 1, env["parody"])
        m.store(MENU + 0xC, 2, 0)
        m.store(MENU + 0xE, 2, 0)
        self.ctrl, self.sysid = env["ctrl"], env["sysid"]
        self.rnd = rnd
        self.book_left = None
        self.ride_left = 0
        self.queues = {"Check": [0] * rnd.choice((0, 0, 1, 3)) + [1],
                       "ShowMap": [0] * rnd.choice((0, 5, 19, 20, 21, 30)) + [1],
                       "checkPartyAnnihilation": [1] * rnd.choice((0, 0, 1, 2)) + [0]}

    @staticmethod
    def near_caps(rnd, pc):
        """A book's target with its stats, HP and SP at and around the caps."""
        pc.elm = [rnd.choice((990, 980, 970, 989, 999, 1000, 979, 32767, 32760, 9, 10, 0, -5, v)) for v in pc.elm]
        pc.HP = rnd.choice((pc.HP, 9969, 9970, 9989, 9990, 9999, 32767, -32768))
        pc.SP = rnd.choice((pc.SP, 984, 985, 994, 995, 999, 32767, -32768))
        pc.p_maxHP = rnd.choice((pc.p_maxHP, 32767, 32740, -32768))
        pc.p_maxSP = rnd.choice((pc.p_maxSP, 32767, 32760, -32768))

    def use_scene(self, rnd):
        chars, pcs, enes = self.rnd_scene(rnd)
        self.ensure_pc(rnd, chars, pcs, enes)
        if rnd.random() < 0.3:
            o = self.tb.rnd_other(self.b, rnd)
            o.ent_root = rnd.choice((1, 0, rnd.getrandbits(32)))
            o.pos_p, o.width = rnd_pos(rnd), 0
            chars.append(o)
        for c in chars:
            c.no_death = rnd.random() < 0.3
        return chars, pcs, enes

    def finish(self, chars, env):
        g = self.game
        m = g.m
        return {"calls": normalize(g.calls), "chars": self.read_chars(chars), "skill": self.read_skill(),
                "rand": g.rand_now(), "env": [m.load(self.plw, 1) & 1, s32(m.load(self.dne, 4))],
                "menu": [s16(m.load(MENU + 0xC, 2)), s16(m.load(MENU + 0xE, 2))]}


def runtime(checks):
    """The item runtime over a Checks, made once and kept on it."""
    it = getattr(checks, "item_runtime", None)
    if it is None:
        it = checks.item_runtime = ItemRuntime(checks)
    return it


def useful(checks, rnd):
    it = runtime(checks)
    cat = rnd.choice((10, 11, 12, 13, 14, 15, 15, 15, rnd.randrange(-3, 20)))
    i = rnd.choice(IMPORTANT + (0, 1, 2, rnd.randrange(400), rnd.randrange(-5, 5)))
    g = it.game
    ret = s32(g.m.call(g.sym("ccCheckItemUseful__Fii"), (cat & 0xFFFFFFFF, i & 0xFFFFFFFF)))
    return f"useful {cat} {i}", ret, "useful"


def skill_useful(checks, rnd):
    it = runtime(checks)
    g = it.game
    m = g.m
    chars, pcs, enes = it.rnd_scene(rnd)
    for c in chars:
        if hasattr(c, "PPcount"):
            c.PPcount = rnd.choice((0, 0, rnd.randrange(1, 300)))
        c.cond[0] = rnd.choice((0, 0, 0, 1, 2, 5, 5))
    sid = rnd.choice((2, 3, 4, 5, 180, 180, rnd.randrange(1, 304), rnd.randrange(1, 304), rnd.randrange(-2, 1)))
    patch = None
    if 1 <= sid < 304:
        sk = dict(it.data.skills[sid])
        sk["attr"] = list(sk["attr"])
        sk["targetType"] = rnd.choice((sk["targetType"], 1, 2, 3, 4, 6, 7, 0x20, 0x40, 0x60, 0x80, 0xE0, 0x61))
        sk["triggerRange_bits"] = fbits(rnd.choice((sk["triggerRange"], 0.0, 10.0, rnd.uniform(0, 80))))
        sk["targetRange_bits"] = fbits(sk["targetRange"])
        it.put_skill(sk, sk["va"])
        patch = f"patch {sid} {rs.ser_skill(sk)}"
    members = [i for i in range(len(chars)) if chars[i].type & 7]
    party = [rnd.choice(members + [-1]) if members else -1 for _ in range(3)]
    order = rnd.sample(range(len(chars)), rnd.randrange(0, len(chars) + 1))
    dist = {i: fbits(rnd.choice((rnd.uniform(0, 90), 0.0, 20.0))) for i in order}
    view = {i: rnd.random() < 0.8 for i in order}
    it.scene.put(chars, pcs, enes)
    va = it.scene.va
    it.in_view = {}
    for i in range(len(chars)):
        m.store(va(i) + 0x40, 4, i + 1)          # pos.x names the character for the camera stub
        it.in_view[i + 1] = view.get(i, False)
    for n in range(3):
        m.store(it.party_mgr + 4 * n, 4, va(party[n]) if party[n] >= 0 else 0)
    m.store(it.sort_root, 4, va(order[0]) if order else 0)
    for a, b_ in zip(order, order[1:] + [None]):
        m.store(va(a) + 0xC0, 4, va(b_) if b_ is not None else 0)
        m.store(va(a) + 0xC4, 4, dist[a])
    off = rnd.choice((0, 0, 0, 1))             # eventStatus[40]: from Mutation on, no Data Drain target
    m.store(SAVE + 0x64F8 + 40, 1, off)
    ret = it.hooked(lambda: g.m.call(g.sym("ccCheckSkillUseful__Fi"), (sid & 0xFFFFFFFF,)))
    it.scene.restore()
    if patch:
        g.restore(it.data.skills[sid]["va"], 0x38)
    req = (f"skilluseful {it.ser_scene(chars, pcs, enes)} {' '.join(map(str, party))} {len(order)} "
           + " ".join(f"{i} {dist[i]} {int(view[i])}" for i in order) + f" {sid} {off}")
    return ([patch, req, f"restore {sid}"] if patch else req), s32(ret), "skill_useful"


def item_skill(checks, rnd):
    it = runtime(checks)
    g = it.game
    m = g.m
    chars, pcs, enes = it.rnd_scene(rnd)
    cp, tp = rnd.randrange(len(chars)), rnd.randrange(len(chars))
    kind = rnd.choice((0, 1, 2))
    sid = rnd.choice((rnd.randrange(150, 304), rnd.randrange(150, 193), 295, -1, 0, 1, rnd.randrange(304)))
    param = rnd.choice((800, rnd.randrange(-5, 2000)))
    flag = rnd.choice((0, 0, 1, 2, -1))
    running = rnd.choice((0, 1))
    state = rnd.getrandbits(64)
    it.scene_setup(rnd, chars, pcs, enes, running, cp)
    g.set_rand(state)
    va = it.scene.va
    fn, args = {0: ("ccItemSkillRequest__FP6ccCharP6ccCharii", (va(cp), va(tp), sid, flag)),
                1: ("ccItemSkillRequestParam__FP6ccCharP6ccChariii", (va(cp), va(tp), sid, param, flag)),
                2: ("ccItemSkillCompel__FP6ccCharP6ccCharii", (va(cp), va(tp), sid, flag))}[kind]
    it.hooked(lambda: m.call(g.sym(fn), tuple(a & 0xFFFFFFFF for a in args)))
    out = {"calls": [list(c) for c in g.calls], "chars": it.read_chars(chars), "skill": it.read_skill(),
           "rand": g.rand_now()}
    it.scene.restore()
    req = (f"itemskill {it.ser_scene(chars, pcs, enes)} {kind} {cp} {tp} {sid} {param} {flag} {running} "
           f"{state}")
    return req, out, "item_skill"


def use_item(checks, rnd):
    it = runtime(checks)
    g = it.game
    m = g.m
    chars, pcs, enes = it.use_scene(rnd)
    env = it.rnd_env(rnd, chars)
    code = rnd_code(rnd)
    pcs_all = [i for i, c in enumerate(chars) if c.type & 7]
    cp = env["player"] if rnd.random() < 0.7 else rnd.randrange(len(chars))
    tp = rnd.randrange(len(chars))
    if code >> 16 == 12:
        foes = [i for i, c in enumerate(chars) if hasattr(c, "row")]
        tp = (env["player"] if rnd.random() < 0.6 else rnd.choice(foes) if foes and rnd.random() < 0.4
              else rnd.choice(pcs_all))
        if chars[tp].type & 7:
            it.near_caps(rnd, chars[tp])
    elif code >> 16 == 13 and code & 0xFFFF == 0 and rnd.random() < 0.6 and len(chars) > 1:
        tp = len(chars) - 1
    pn = rnd.choice((rnd.randrange(0, 9), rnd.randrange(-3, 12), -1, 0, 8, 9))
    pos = [fbits(rnd.uniform(-50, 50)) for _ in range(3)] + [F_ONE]
    state = rnd.getrandbits(64)
    it.scene_setup(rnd, chars, pcs, enes, env["running"], cp)
    it.put_env(rnd, env)
    va = it.scene.va
    it.tp_va = va(tp)
    for k in range(4):
        m.store(va(tp) + 0x40 + 4 * k, 4, pos[k])
    g.set_rand(state)
    it.hooked(lambda: m.call(g.sym("ccUseItemRequest__FP6ccCharP6ccCharii"),
                               (va(cp), va(tp), code & 0xFFFFFFFF, pn & 0xFFFFFFFF)))
    out = it.finish(chars, env)
    it.scene.restore()
    req = (f"useitem {it.ser_scene(chars, pcs, enes)} {it.ser_env(env)} {cp} {tp} {code} {pn} "
           f"{' '.join(map(str, pos))} {state}")
    return req, out, f"use_item {code >> 16}" if code >= 0 else "use_item -"


def give(checks, rnd):
    it = runtime(checks)
    g = it.game
    m = g.m
    chars, pcs, enes = it.use_scene(rnd)
    env = it.rnd_env(rnd, chars)
    pcs_all = [i for i, c in enumerate(chars) if c.type & 7]
    ch = env["player"] if rnd.random() < 0.5 else rnd.choice(pcs_all)
    i = rnd.choice((rnd.randrange(34), rnd.randrange(34), rnd.randrange(34, 50)))
    num = rnd.choice((1, 1, 2, 3, 0, -1))
    state = rnd.getrandbits(64)
    it.scene_setup(rnd, chars, pcs, enes, env["running"], ch)
    it.put_env(rnd, env)
    va = it.scene.va
    it.tp_va = va(ch)
    g.set_rand(state)
    ret = it.hooked(lambda: m.call(g.sym("AddSpcItem__10ccMenuCtrlFP6ccChariiii"),
                                     (MENU, va(ch), 12, i, num & 0xFFFFFFFF, 0)))
    out = it.finish(chars, env)
    out["ret"] = s32(ret)
    it.scene.restore()
    req = f"give {it.ser_scene(chars, pcs, enes)} {it.ser_env(env)} {ch} {i} {num} {state}"
    return req, out, "give"


def ai_use(checks, rnd):
    it = runtime(checks)
    """ccAI::UseItem with its checks met (the member awake and free, from
    Mutation on able to act, the item in its list, the target listed and
    up, or the item a
    Resurrect): the use and the member's list after."""
    g = it.game
    m = g.m
    chars, pcs, enes = it.use_scene(rnd)
    env = it.rnd_env(rnd, chars)
    pcs_all = [i for i, c in enumerate(chars) if c.type & 7]
    ch = rnd.choice(pcs_all)
    for k in (1, 9, 10, 11, 14):
        chars[ch].cond[k] = 0
    # free to act (Mutation's CheckAction(7)): no skill running
    chars[ch].skill_id = chars[ch].skill_status = 0
    if not pcs and not enes:
        pcs.append(ch)
    tp = rnd.choice(pcs + enes)
    code = rnd_code(rnd)
    while code < 0:
        code = rnd_code(rnd)
    if rnd.random() < 0.5:
        code = rnd.choice((10 << 16 | rnd.randrange(24), 11 << 16 | rnd.randrange(72), 10 << 16 | 5))
    cat, i = code >> 16, code & 0xFFFF
    if chars[tp].cond[0] >= 2 and code != (10 << 16 | 5):
        chars[tp].cond[0] = rnd.choice((0, 1))
    sl = [rnd_slot(rnd) for _ in range(40)]
    k = rnd.randrange(40)
    sl[k] = (i, cat, rnd.randrange(1, 6))
    for j in range(k):
        if sl[j][0] == i and sl[j][1] == cat:
            sl[j] = (-1, -1, 0)
    if k < 39 and rnd.random() < 0.3:
        sl[rnd.randrange(k + 1, 40)] = (i, cat, rnd.randrange(-2, 99))
    own = pack_list(sl)
    state = rnd.getrandbits(64)
    it.scene_setup(rnd, chars, pcs, enes, env["running"], ch)
    it.put_env(rnd, env)
    va = it.scene.va
    it.tp_va = va(tp)
    # ccSpcChar's vtable: Mutation's UseItem asks CheckAction(7) through it
    c = chars[ch]
    m.store(va(ch) + 0x1F4, 4, PA.VT_PLAYER if c.type & 1 else PA.VT_FELLOW if c.type & 4 else PA.VT_SPC)
    cid = chars[ch].id
    la = volume.item_list_at(SAVE, SAVE2, cid)
    m.mem[la:la + 160] = own
    m.mem[AI_VA:AI_VA + 0x100] = bytes(0x100)
    m.store(AI_VA + 0x18, 4, va(ch))
    m.store(AI_VA + volume.ai_at(0x76), 2, rnd.choice((99, -1, 3)))
    m.store(AI_VA + 1, 1, rnd.randrange(256))
    m.store(va(tp) + 0x4C, 4, F_ONE)            # pos as the port's Char starts it: (0, 0, 0, 1)
    g.set_rand(state)
    it.hooked(lambda: m.call(g.sym("UseItem__4ccAIFiP6ccChar"), (AI_VA, code, va(tp))))
    out = it.finish(chars, env)
    out["list"] = bytes(m.mem[la:la + 160]).hex()
    it.scene.restore()
    req = (f"aiuse {it.ser_scene(chars, pcs, enes)} {it.ser_env(env)} {ch} {tp} {code} {own.hex()} "
           f"{state}")
    return req, out, f"ai_use {cat}"


def inventory(checks, rnd):
    it = runtime(checks)
    g = it.game
    m = g.m
    op = rnd.choice(("add", "add", "add", "del", "del", "num", "slot", "addpl", "addpl", "delpl", "numpl",
                     "slotpl", "consume", "consume"))
    ch = rnd.randrange(18)
    lists = []
    for _ in range(18):
        full = rnd.random() < 0.1
        sl = []
        for _ in range(40):
            s = rnd_slot(rnd, sl[-1] if sl and rnd.random() < 0.1 else None)
            if full and s[0] < 0:
                s = (rnd.randrange(24), 10, rnd.randrange(1, 99))
            sl.append(s)
        lists.append(sl)
    pl = [rnd_slot(rnd) for _ in range(99)]
    if rnd.random() < 0.1:
        pl = [s if s[0] >= 0 else (rnd.randrange(72), 11, 3) for s in pl]
    imp = bytes(rnd.choice((0, 0, 1, 5, 99, rnd.randrange(256))) for _ in range(320))
    own = lists[ch] if not op.endswith("pl") else pl
    pick = [s for s in own if s[0] >= 0]
    if pick and rnd.random() < 0.6:
        i, cat, _ = rnd.choice(pick)
    else:
        cat = rnd.choice((10, 11, 12, 13, 14, 15, 15, 0, 5, 9, rnd.randrange(-2, 20)))
        i = rnd.randrange(ROWS.get(cat, 90) + 3) if rnd.random() < 0.95 else rnd.randrange(-3, 400)
    if cat == 15 and ch == 0 and not (0 <= i < 320):
        i = rnd.randrange(320)
    num = rnd.choice((1, 1, 1, 2, 5, 99, 0, -1, 150))
    m.mem[SAVE:SAVE + SAVE_SIZE] = bytes(SAVE_SIZE)
    for k in range(18):
        m.mem[SAVE + ITEM_LIST + 160 * k:SAVE + ITEM_LIST + 160 * (k + 1)] = pack_list(lists[k])
    m.mem[SAVE + PL_LIST:SAVE + PL_LIST + 396] = pack_list(pl)
    m.mem[SAVE + IMP_LIST:SAVE + IMP_LIST + 320] = imp
    before = bytes(m.mem[SAVE:SAVE + SAVE_SIZE])
    name = {"add": ("AddItem__10ccSaveDataFiiii", (SAVE, ch, cat, i, num)),
            "del": ("DelItem__10ccSaveDataFiiii", (SAVE, ch, cat, i, num)),
            "num": ("GetItemNum__10ccSaveDataFiii", (SAVE, ch, cat, i)),
            "slot": ("GetItemSlot__10ccSaveDataFi", (SAVE, ch)),
            "addpl": ("AddPlItem__10ccSaveDataFiii", (SAVE, cat, i, num)),
            "delpl": ("DelPlItem__10ccSaveDataFiii", (SAVE, cat, i, num)),
            "numpl": ("GetPlItemNum__10ccSaveDataFii", (SAVE, cat, i)),
            "slotpl": ("GetPlItemSlot__10ccSaveDataFv", (SAVE,)),
            "consume": ("ConsumeItemList__4ccAIFi", (AI_VA, (cat << 16) | (i & 0xFFFF)))}[op]
    if op == "consume":
        m.store(AI_VA + 24, 4, CH_VA)
        m.store(CH_VA, 4, BASE_VA)
        m.store(BASE_VA + 12, 2, ch)
    ret = s32(m.call(g.sym(name[0]), tuple(a & 0xFFFFFFFF for a in name[1])))
    if op in ("add", "del", "addpl", "delpl"):
        ret = 0
    after = bytes(m.mem[SAVE:SAVE + SAVE_SIZE])
    la = ITEM_LIST + 160 * ch
    mine = [(la, la + 160), (PL_LIST, PL_LIST + 396), (IMP_LIST, IMP_LIST + 320)]
    other = bytearray(after)
    for a, b_ in mine:
        other[a:b_] = before[a:b_]
    out = {"ret": ret, "list": after[la:la + 160].hex(), "pl": after[PL_LIST:PL_LIST + 396].hex(),
           "imp": after[IMP_LIST:IMP_LIST + 320].hex()}
    if bytes(other) != before:
        out["other"] = "changed"
    req = (f"inv {op} {ch} {cat} {i} {num} {before[la:la + 160].hex()} {before[PL_LIST:PL_LIST + 396].hex()} "
           f"{before[IMP_LIST:IMP_LIST + 320].hex()}")
    return req, out, f"inventory {op}"


ITEMS = {"item_useful": useful, "skill_useful": skill_useful, "item_skill": item_skill, "use_item": use_item,
         "give_stat_items": give, "ai_use_item": ai_use, "item_lists": inventory}


@unittest.skipUnless(rs.READY, "needs the extracted disc and cargo")
class ItemsAgainstGame(rs.Against):
    TABLE = ITEMS

    def test_useful(self):
        self.check("item_useful", 9000)
        self.check("skill_useful", 9001)

    def test_item_skill(self):
        self.check("item_skill", 9002)

    def test_use_item(self):
        self.check("use_item", 9003)

    def test_give_and_ai(self):
        self.check("give_stat_items", 9004)
        self.check("ai_use_item", 9005)

    def test_item_lists(self):
        self.check("item_lists", 9006)


if __name__ == "__main__":
    rs.main(list(ITEMS), 9000, ITEMS)
