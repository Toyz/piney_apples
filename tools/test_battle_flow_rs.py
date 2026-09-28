#!/usr/bin/env python3
"""crates/piney-battle's skill life and normal attack against the game's
code in tools/eemu.py, as tools/test_battle_rs.py does (its scene, hooks
and runner):

  skill_main            ccSkill::Main (0x005731d0), one frame, with the
                        systems it calls: ConditionModifySystem, Healing-
                        System, RecoverySystem, the spell systems' choice
  note_affect           ccSkillCheckNote / ccSkill::NoteEventAffect
                        (0x00573a10, 0x005738e0)
  skill_check           ccSkillCheck (0x005723e0)
  attack_note           ccPlayer::CheckNote, ccFellow::CheckNote
                        (0x0059c300, 0x0041e1e0)
  condition_adjustment  ccSpcChar::ConditionAdjustment (0x0059ec70)
  level_absent          ccSpcCheckLevelUp (0x005a1630)
  area_item             ccMenuCtrl::AreaItem (0x00544c00)

    python3 tools/test_battle_flow_rs.py            the unit tests
    python3 tools/test_battle_flow_rs.py bulk N     N cases of every check
"""

import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import test_battle_rs as T  # noqa: E402
import volume  # noqa: E402

SKO = 0x01033000        # the ccSkill under test
ANM = 0x01034000        # a stand-in ccAnm
NOTE = 0x01034400


def flow_hooks(checks):
    if getattr(checks, "_flow_hooked", False):
        return
    checks._flow_hooked = True
    T.affect_hooks(checks)
    g = checks.game
    m = g.m
    rec = g.record
    sym = g.sym
    m.hooks[sym("_AnimateForward__5ccAnmFUi")] = lambda mm, *_: checks.anim_done
    m.hooks[sym("NoteProcess__5ccAnmFv")] = lambda mm, *_: 0
    m.hooks[sym("ccSeOn3D__FiPf")] = lambda mm, a, *_: rec("ccSeOn3D", a)
    m.hooks[sym("effPhysicalSkillHitShockWave__FPfi")] = lambda mm, *_: rec("shockwave")
    m.hooks[sym("ccSeSetParamSPC__FUiP6ccChar")] = lambda mm, a, b_, *_: rec("sound", a)
    for k, name in enumerate(("FallSystem", "TornadoSystem", "ConvergenceSystem", "UpheavalSystem",
                              "SummonsSystem")):
        m.hooks[sym(f"{name}__7ccSkillFv")] = (lambda k: lambda mm, *_: rec("spell", k))(k)


def rnd_run(checks, rnd, chars):
    """A ccSkill of a random kind on the scene's characters."""
    sid = rnd.choice((1, 1, 5, rnd.randrange(6, 150), rnd.randrange(150, 156), 295, rnd.randrange(156, 193),
                      rnd.randrange(178, 181), rnd.randrange(193, 295), 0, rnd.randrange(0, 304)))
    n = len(chars)
    run = {
        "id": sid, "stype": rnd.choice((0, 0, 1, 2)), "status": rnd.choice((0,) * 8 + (1, -1)),
        "modify": rnd.choice((0, 0, 1)), "report": rnd.choice((0, 1)), "hold": rnd.choice((0, 0, 1)),
        "force": rnd.choice((0, 0, 1)), "level": rnd.randrange(0, 5),
        "count": rnd.choice((0, 0, 1, 74, 75, 76, 449, 450, rnd.randrange(0, 600))),
        "type": rnd.choice((checks.data.skills[sid]["type"],) * 3 + (rnd.getrandbits(19),)),
        "target_type": rnd.choice((6, 7, 0x20, 0x60, 0x80, 0)), "param": rnd.choice((0, 800, rnd.randrange(-5, 3000))),
        "pos": T.rnd_pos(rnd), "target_pos": T.rnd_pos(rnd), "ac": rnd.choice((0, 0, 1, -1)),
        "creator": rnd.choice(tuple(range(n)) + (-1,)), "target": rnd.choice(tuple(range(n)) + (-1,)),
        "anm": rnd.choice((0, 0, 1)),
    }
    if rnd.random() < 0.2:
        # a skill on its own caster: a buff, heal or cure on itself
        run["target"] = run["creator"]
    return run


def ser_run(r):
    v = [r["id"], r["stype"], r["status"], r["modify"], r["report"], r["hold"], r["force"], r["level"], r["count"],
         r["type"], r["target_type"], r["param"]] + r["pos"] + r["target_pos"] + [r["ac"], r["creator"], r["target"],
                                                                                  r["anm"]]
    return " ".join(map(str, v))


def put_run(checks, r):
    g = checks.game
    m = g.m
    va = checks.scene.va
    m.mem[SKO:SKO + volume.SKILL_SIZE] = bytes(volume.SKILL_SIZE)
    m.store(SKO, 4, r["id"])
    m.store(SKO + 8, 4, checks.data.skills[r["id"]]["va"])
    m.store(SKO + 12, 1, (r["stype"] & 15) | (r["status"] & 3) << 4 | r["modify"] << 6 | r["report"] << 7)
    m.store(SKO + 13, 1, r["hold"] | r["force"] << 1)
    m.store(SKO + 0x10, 4, r["level"])
    g.put(SKO + 0x14, "<h", r["count"])
    m.store(SKO + 0x24, 4, r["type"])
    m.store(SKO + 0x28, 4, r["target_type"])
    m.store(SKO + 0x2C, 4, r["param"])
    m.mem[SKO + 0x30:SKO + 0x40] = struct.pack("<4I", *r["pos"])
    m.store(SKO + 0x50, 4, 0x43480000)
    g.put(SKO + 0x54, "<h", r["ac"])
    m.mem[SKO + 0x60:SKO + 0x70] = struct.pack("<4I", *r["target_pos"])
    m.store(SKO + volume.skill_at(0x70), 4, va(r["creator"]) if r["creator"] >= 0 else 0)
    m.store(SKO + volume.skill_at(0x74), 4, va(r["target"]) if r["target"] >= 0 else 0)
    m.mem[ANM:ANM + 0x110] = bytes(0x110)
    m.store(ANM + 0xAC, 4, 1)
    m.store(SKO + volume.skill_at(0x80), 4, ANM if r["anm"] else 0)


def read_run(checks):
    g = checks.game
    m = g.m
    b12 = m.load(SKO + 12, 1)
    st = (b12 >> 4) & 3
    st = st - 4 if st & 2 else st
    c = m.load(SKO + volume.skill_at(0x70), 4)
    t = m.load(SKO + volume.skill_at(0x74), 4)
    return ([st, (b12 >> 6) & 1, m.load(SKO + 13, 1) & 1, (m.load(SKO + 13, 1) >> 1) & 1, g.shorts(SKO + 0x14, 1)[0],
             g.shorts(SKO + 0x54, 1)[0], (c - T.SCN) // 0x1000 if c else -1, (t - T.SCN) // 0x1000 if t else -1]
            + list(struct.unpack("<4I", bytes(m.mem[SKO + 0x30:SKO + 0x40])))
            + list(struct.unpack("<4I", bytes(m.mem[SKO + 0x60:SKO + 0x70]))))


def split_calls(chars, calls):
    idx = {f"c{i}": i for i in range(len(chars))}
    started = [idx[c[1]] for c in calls if c[0] == "effSkillStart"]
    resisted = [idx[c[3]] for c in calls if c[0] == "ccEntryFlyFontNew"]
    spell = next((c[1] for c in calls if c[0] == "spell"), -1)
    heal = int(any(c[0] == "ccSeOn3D" for c in calls))
    rest = [c for c in calls if c[0] not in ("effSkillStart", "ccEntryFlyFontNew", "spell", "ccSeOn3D")]
    return started, resisted, spell, heal, rest


def check_skill_main(checks, rnd, note=False):
    """ccSkill::Main (0x005731d0) one frame, or NoteEventAffect
    (0x005738e0) with a note kind."""
    flow_hooks(checks)
    g = checks.game
    m = g.m
    chars, pcs, enes = checks.rnd_scene(rnd)
    for c in chars:
        c.pos = T.rnd_pos(rnd)
        c.anm_flag = rnd.choice((0, 1, 1))
    r = rnd_run(checks, rnd, chars)
    patch = checks.patched(rnd, r["id"])
    env = checks.tb.rnd_env(checks.b, rnd)
    checks.anim_done = rnd.choice((0, 0, 1))
    ann = rnd.choice((0, 0, 1))
    kind = rnd.choice((0, 1, 1, 2, 3, 4, 5, 6, 7))
    state = rnd.getrandbits(64)
    checks.scene.put(chars, pcs, enes)
    for i, c in enumerate(chars):
        m.mem[checks.scene.va(i) + 0x40:checks.scene.va(i) + 0x50] = struct.pack("<4I", *c.pos)
    put_run(checks, r)
    g.put_env(env)
    g.annihilated = ann
    g.set_rand(state)
    g.calls = []
    if note:
        m.call(g.sym("NoteEventAffect__7ccSkillFUi"), (SKO, kind))
        ret = 0
    else:
        ret = checks.b.s32(m.call(g.sym("Main__7ccSkillFv"), (SKO,)))
        ret = ret - (1 << 32) if ret >= 1 << 31 else ret
    started, resisted, spell, heal, rest = split_calls(chars, list(g.calls))
    out = {"ret": ret, "rand": g.rand_now(), "calls": rest, "chars": checks.scene.read(chars), "run": read_run(checks),
           "started": started, "resisted": resisted, "spell": spell, "heal": heal}
    checks.scene.restore()
    g.restore(checks.data.skills[r["id"]]["va"], 0x38)
    op = "noteaffect" if note else "skillmain"
    tail = f"{kind}" if note else f"{checks.anim_done} {ann}"
    me = r["creator"] if r["creator"] >= 0 else 0
    req = (f"{checks.ser_scene(chars, pcs, enes)} {op} {me} {ser_run(r)} {T.ser_env(env)} {tail} {state}")
    return [patch, req, f"restore {r['id']}"], out, op


def check_note_affect(checks, rnd):
    return check_skill_main(checks, rnd, note=True)


def check_skill_check(checks, rnd):
    """ccSkillCheck (0x005723e0) over a list of running skills."""
    flow_hooks(checks)
    g = checks.game
    m = g.m
    chars, pcs, enes = checks.rnd_scene(rnd)
    me = rnd.randrange(len(chars))
    runs = []
    for _ in range(rnd.randrange(0, 4)):
        r = rnd_run(checks, rnd, chars)
        r["creator"] = me if rnd.random() < 0.8 else rnd.randrange(len(chars))
        runs.append(r)
    act = rnd.choice((0, 17, 18, 5))
    checks.scene.put(chars, pcs, enes)
    top = g.sym("SkillEntryTop")
    prev = 0
    base = 0x01035000
    for k, r in enumerate(runs):
        a = base + 0x100 * k
        m.mem[a:a + volume.SKILL_SIZE] = bytes(volume.SKILL_SIZE)
        m.store(a, 4, r["id"])
        m.store(a + 12, 1, (r["stype"] & 15) | (r["status"] & 3) << 4 | r["modify"] << 6 | r["report"] << 7)
        m.store(a + volume.skill_at(0x70), 4, checks.scene.va(r["creator"]))
        if prev:
            m.store(prev + 4, 4, a)
        else:
            m.store(top, 4, a)
        prev = a
    if not runs:
        m.store(top, 4, 0)
    plw = g.sym("plw")
    m.store(plw + 0x20, 4, 0x01036000)
    g.put(0x01036000 + 0xEE, "<h", act)
    fn = g.sym("ccSkillCheck__FP6ccChar")
    hook = m.hooks.pop(fn, None)
    ret = checks.b.s32(m.call(fn, (checks.scene.va(me),)))
    if hook is not None:
        m.hooks[fn] = hook
    m.store(top, 4, 0)
    checks.scene.restore()
    # The probe's skillcheck makes every run the character's own: give it
    # the ones that are (the others are skipped by the game too).
    own = [r for r in runs if r["creator"] == me]
    req = (f"{checks.ser_scene(chars, pcs, enes)} skillcheck {me} {len(own)} "
           f"{' '.join(ser_run(r) for r in own)} {act}")
    return req, {"ret": ret, "calls": [], "chars": checks.scene.read(chars)}, "skillcheck"


def check_attack_note(checks, rnd):
    """The normal attack's hit: ccPlayer::CheckNote (0x0059c300) or
    ccFellow::CheckNote (0x0041e1e0) on note 0x8005."""
    flow_hooks(checks)
    g = checks.game
    m = g.m
    chars, pcs, enes = checks.rnd_scene(rnd)
    for c in chars:
        c.pos = T.rnd_pos(rnd)
        c.skill_id = rnd.choice((1, 1, 5, rnd.randrange(0, 304)))
        c.HP = rnd.choice((c.HP, 0, 1))
    me = rnd.randrange(len(chars))
    tgt = rnd.choice(tuple(range(len(chars))) + (-1,))
    player = rnd.random() < 0.5
    env = checks.tb.rnd_env(checks.b, rnd)
    state = rnd.getrandbits(64)
    last = rnd.randrange(-5, 1000)
    arms = T.fbits(rnd.choice((0.0, 10.0, 40.0, rnd.uniform(0, 100))))
    dist_tg = T.fbits(rnd.uniform(0, 100))
    checks.scene.put(chars, pcs, enes)
    for i, c in enumerate(chars):
        m.mem[checks.scene.va(i) + 0x40:checks.scene.va(i) + 0x50] = struct.pack("<4I", *c.pos)
    a = checks.scene.va(me)
    m.store(a + 0x78, 4, checks.scene.va(tgt) if tgt >= 0 else 0)
    g.put_env(env)
    g.set_rand(state)
    note = NOTE
    m.mem[note:note + 16] = struct.pack("<IIII", 0, 0x8005, 0, 0)
    g.calls = []
    if player:
        m.store(a + 0x238, 4, last & 0xFFFFFFFF)
        m.call(g.sym("CheckNote__8ccPlayerFP9ccAnmNote"), (a, note))
    else:
        AIV = 0x01031000
        prm = 0x01031800
        m.store(a + 0x128, 4, AIV)
        m.store(AIV + 0x14, 4, prm)
        m.store(prm + 0xC, 4, arms)
        m.store(AIV + 0xA8, 4, dist_tg)
        m.call(g.sym("CheckNote__8ccFellowFP9ccAnmNote"), (a, note))
    out = {"ret": 0, "rand": g.rand_now(), "calls": list(g.calls), "chars": checks.scene.read(chars)}
    if player:
        out["dist"] = checks.b.s32(m.load(a + 0x238, 4))
        req = (f"{checks.ser_scene(chars, pcs, enes)} playernote {me} {last} {tgt} {T.ser_env(env)} {state}")
    else:
        # the probe names the AI "ai" when the character has one; here it
        # always has (the note reads its range through it)
        chars[me].ai = 1
        req = (f"{checks.ser_scene(chars, pcs, enes)} fellownote {me} {arms} {dist_tg} {tgt} {T.ser_env(env)} "
               f"{state}")
    checks.scene.restore()
    return req, out, "attack_note"


def check_condition_adjustment(checks, rnd):
    """ccSpcChar::ConditionAdjustment (0x0059ec70) on one party member."""
    flow_hooks(checks)
    g = checks.game
    m = g.m
    chars, pcs, enes = checks.rnd_scene(rnd)
    pcs_idx = [i for i, c in enumerate(chars) if c.type & 7]
    if not pcs_idx:
        chars.append(checks.tb.rnd_pc(checks.b, checks.data, rnd))
        pcs_idx = [len(chars) - 1]
        pcs.append(pcs_idx[0])
    me = rnd.choice(pcs_idx)
    c = chars[me]
    c.cond[0] = rnd.choice((0, 1, 2, 3, 4, 5, 5, 2))
    c.act_num = rnd.choice((0, 2, 9, 10, rnd.randrange(0, 20)))
    c.spc_flags = rnd.getrandbits(32) & ~(0x80 | 7 << 14)
    c.cnt = rnd.randrange(0, 100)
    c.cloak = rnd.choice((0, F_ONE_BITS))
    checks.scene.put(chars, pcs, enes)
    a = checks.scene.va(me)
    m.store(a + 0xE0, 4, m.load(a + 0xE0, 4) | c.spc_flags)
    g.put(a + 0xEE, "<hh", c.act_num, 7)
    m.store(a + 0x114, 4, c.cnt)
    m.store(a + 0x110, 4, c.cloak)
    g.calls = []
    m.call(g.sym("ConditionAdjustment__9ccSpcCharFv"), (a,))
    out = {"ret": 0, "calls": list(g.calls), "chars": checks.scene.read(chars),
           "spc": [g.shorts(a + 0xEE, 1)[0], g.shorts(a + 0xF0, 1)[0], m.load(a + 0xE0, 4) & ~(0x80 | 7 << 14),
                   checks.b.s32(m.load(a + 0x114, 4)), m.load(a + 0x110, 4)]}
    checks.scene.restore()
    # actNumOld starts at 7 in the game's memory: give the port the same
    req = f"{checks.ser_scene(chars, pcs, enes)} condadj {me} 7"
    return req, out, "condition_adjustment"


F_ONE_BITS = T.F_ONE


def check_level_absent(checks, rnd):
    """ccSpcCheckLevelUp (0x005a1630): members away from the party level up
    in the save."""
    b, tb, data = checks.b, checks.tb, checks.data
    g = checks.game
    m = g.m
    n = g.members
    area = rnd.choice((0, 1, 1, 2))
    flags, exp_flags = rnd.getrandbits(n), rnd.getrandbits(n)
    ids = [rnd.choice((-1, rnd.randrange(0, n))) for _ in range(3)]
    members = {}
    for i in range(n):
        pc = tb.rnd_pc(b, data, rnd)
        pc.id = rnd.choice((i, i, rnd.randrange(0, n)))
        pc.exp = rnd.choice((0, 999, 1000, 1999, 2500, 30000, rnd.randrange(0, 32768)))
        pc.level = rnd.choice((1, 50, 97, 98, 99, rnd.randrange(1, 100)))
        members[i] = pc
    save = checks.tb.SAVE
    m.mem[save:save + 0x8500] = bytes(0x8500)
    for i, pc in members.items():
        g.put_spc(pc, g.spc[i])
    m.store(save + 0x2220, 4, flags)
    m.store(save + 0x222C, 4, exp_flags)
    m.store(checks.tb.GAME + 0x14, 4, area)
    g.party = {}
    for k, i in enumerate(ids):
        if i >= 0 and i not in g.party:
            g.party[i] = k
    m.call(g.sym("ccSpcCheckLevelUp__Fv"), ())
    got = {}
    for i in members:
        va = g.spc[i]
        got[str(i)] = (list(g.shorts(va + 14, 2)) + list(g.shorts(va + 0x24, 2)) + list(g.shorts(va + 0x28, 16)))
    g.party = {}
    req = (f"levelabsent {area} {flags} {exp_flags} {ids[0]} {ids[1]} {ids[2]} {len(members)} "
           + " ".join(f"{i} {T.ser_char(pc, b)}" for i, pc in members.items()))
    return req, got, "level_absent"


def check_area_item(checks, rnd):
    """ccMenuCtrl::AreaItem (0x00544c00): the item a box, trap or idol
    holds."""
    g = checks.game
    m = g.m
    item = rnd.choice((-1, -1, -1, 0x0B0003, rnd.randrange(0, 0x100000)))
    kind = rnd.choice((0, 1, 2, 3, 4, 5, 6, -1))
    server = rnd.randrange(0, 5)
    attr = rnd.randrange(0, 6)
    floor = rnd.randrange(0, 10)
    field = rnd.choice((0, 1))
    ev_level = rnd.randrange(0, 60)
    level = rnd.randrange(1, 5)
    off = rnd.randrange(0, 20)
    base = ev_level if field else 25 * (level - 1) + 10 + off
    state = rnd.getrandbits(64)
    GAMEV = 0x01021000
    WM = 0x01037000
    INFO = 0x01037400
    EVI = 0x01037800
    m.store(GAMEV + 0x1C, 4, server)
    m.store(GAMEV + 0x24, 4, field)
    m.store(GAMEV + 0x2C, 4, floor)
    m.store(g.sym("worldman"), 4, WM)
    m.store(WM + 0x158, 4, INFO)
    m.store(INFO + 0x20, 4, level)
    m.store(INFO + 0x28, 4, off)
    m.store(EVI + 0x1C, 4, ev_level)
    m.hooks[g.sym("GetFieldAttrb__9WORLD_MANFv")] = lambda mm, *_: attr
    m.hooks[g.sym("GetEventAreaInfo__9WORLD_MANFv")] = lambda mm, *_: EVI
    g.set_rand(state)
    ret = checks.b.s32(m.call(g.sym("AreaItem__10ccMenuCtrlFii"), (0x01020000, item & 0xFFFFFFFF, kind & 0xFFFFFFFF)))
    req = f"areaitem {item} {kind} {server} {attr} {floor} {base} {state}"
    return req, {"ret": ret, "rand": g.rand_now()}, "area_item"


FLOW = {"skill_main": check_skill_main, "note_affect": check_note_affect, "skill_check": check_skill_check,
        "attack_note": check_attack_note, "condition_adjustment": check_condition_adjustment,
        "level_absent": check_level_absent, "area_item": check_area_item}


@unittest.skipUnless(T.READY, "needs the extracted disc and cargo")
class SkillFlowAgainstGame(T.Against):
    TABLE = FLOW

    def test_skill_main(self):
        self.check("skill_main", 7000)

    def test_note_affect(self):
        self.check("note_affect", 7001)

    def test_skill_check(self):
        self.check("skill_check", 7002)

    def test_attack_note(self):
        self.check("attack_note", 7003)

    def test_condition_adjustment(self):
        self.check("condition_adjustment", 7004)

    def test_rewards(self):
        self.check("level_absent", 7005)
        self.check("area_item", 7006)


if __name__ == "__main__":
    T.main(list(FLOW), 7000, FLOW)
