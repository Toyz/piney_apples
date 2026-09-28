#!/usr/bin/env python3
"""crates/piney-effect's skill effects against the game's own code run in
eemu: what shows as a skill starts, a physical skill's shock wave, heals
and cures, a box opened and a trap removed, the stat changes' effects and
the resistant shield.

  - effSkillStart (main 0x001d35f0 by skill id, 0x001d3650 by
    ccSkillParam), effSkillStartEffect (0x001d39d0): the start's sparks,
    the controller -12 / -13 and, through it, effSkillExecSummonsCircle /
    effSkillExecCircle (85, 86) at its count 7; effSkillExecRing,
    effSkillExecForceRing, effSkillExecForceRing2 (82, 83, 84);
  - effPhysicalSkillHitShockWave (0x001d2790): the smoke and radiate
    pieces by element, the waves 79, 80, 81, the shake and the sound;
  - effHeal (0x001ccd20), effHealSkill (0x001cccf0): the ring 130 and the
    controller -19; effCure, effSanity, effResurrect (0x001cef50,
    0x001cf0d0, 0x001cedd0): the controllers -2, -3, -1; effOpenBox
    (0x001ced40); effRemoveTrap (0x001d0980) and its controller -6;
  - effAbilityUp, effAbilityDown (0x001d7380, 0x001d7630): the controllers
    -20, -21, -22, ended (their endFlag, as ~ccConditionEffect does) at a
    set or random frame;
  - effResistantShield (gcmn 0x00501ab0, 0x00501cb0) and
    ccResistantShieldElement::Main (gcmn 0x00501970) on the element
    manager;
  - the dungeon objects': effVirusCrystal (0x001cf420) and its controller
    -4; effCrushBarrel, effCrushEgg, effCrushPot, effCrushCorpse
    (0x001d0300-0x001d0660) with the fragments (x 0, and x not 0 for the
    CLUT swap); effOpenTrapBox (0x001d0780) and its controller -5;
    effStatueOfGod (0x001d0e60), its syncSW a character's +0x1e0 (the
    idol's effsw), set 1 at the start and 0 some frames on;
  - the ids whose ccEffect::Main cases the port once lacked: effIceRock
    (0x001ceaa0; the rocks 15-18, an IceBreak's twelve throws together)
    over a flat land (ccLandHitCheck2) at a random height; and, made by
    ccNewEffect and set up as a creator would since nothing in the game
    makes them, 0 (ANM_x300) as effMeteoFireBall2 sets up a meteor, 113
    (CMP_x042) with lives about its 5 and 15, and -11..-8.

The machine is test_effect_spell_rs's SpellMachine (main and GCMN.PRG in
eemu, the effect files served from DATA.BIN, ccEffectCtrl::Main and
ccEffectElementManager::Main each frame, characters on the command lists
or not, down or not, rand() and genrand() seeded as the probe seeds them;
the draws, sounds, shakes, generators with every field their starters set,
CLUT swaps and every element member by DWARF recorded), with a ccAnm's
transparency left 1 by its constructor (as ccCoord's would: the shield's
draw reads it) and the characters' posP (+0x50) and affectPerson (+0x98)
set. spell_probe runs the port on the same requests. Random worlds,
characters of every type (PC, boss, enemy), moving, leaving the lists,
going down; each group's sweep (every value worth a start once: all 304
skills, every element, every level, condition and shield case), then
random skills, elements, levels, textures and sizes. Each
start's answer (what it returned, its events and generators, both random
generators' states) and each frame (every live slot, element and
ccEffect2, the draws in order, events, generators, rand and genrand) are
compared.

    python3 tools/test_effect_skill_rs.py            the unit tests
    python3 tools/test_effect_skill_rs.py bulk N [GROUPS [nosweep]]
                                                     each group's sweeps and N
                                                     random cases

Skipped when the disc is not extracted or cargo is missing.
"""

import os
import random
import shutil
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import test_effect_rs as base  # noqa: E402
import test_effect_spell_rs as spell  # noqa: E402
from test_effect_rs import EFFWORK, ELF, ISO, ONE, fb, hexs  # noqa: E402

SKILL_TBL = spell.SKILL_TBL
EFFECT_STARTERS = {
    "skillstart": "effSkillStart__FP6ccChariii",
    "skillstartp": "effSkillStart__FP6ccCharP12ccSkillParamii",
    "skillstarteffect": "effSkillStartEffect__FP6ccCharii",
    "shock": "effPhysicalSkillHitShockWave__FPfi",
    "heal": "effHeal__FP6ccChari",
    "healskill": "effHealSkill__FP6ccChari",
    "cure": "effCure__FP6ccChar",
    "sanity": "effSanity__FP6ccChar",
    "resurrect": "effResurrect__FP6ccChar",
    "openbox": "effOpenBox__FPf",
    "removetrap": "effRemoveTrap__FPfii",
    "abilityup": "effAbilityUp__FP6ccChari",
    "abilitydown": "effAbilityDown__FP6ccChari",
    "shield": "effResistantShield__FP6ccCharii",
    "virus": "effVirusCrystal__FPf",
    "crushbarrel": "effCrushBarrel__FPfi",
    "crushegg": "effCrushEgg__FPfi",
    "crushpot": "effCrushPot__FPfi",
    "crushcorpse": "effCrushCorpse__FPfi",
    "opentrap": "effOpenTrapBox__FPfii",
    "statue": "effStatueOfGod__FPfPi",
    "icerock": "effIceRock__FPfPfffi",
    "spawn": "ccNewEffect__Fi",
}
# The starters taking a position first.
POS_FIRST = ("shock", "openbox", "removetrap", "virus", "crushbarrel", "crushegg", "crushpot", "crushcorpse",
             "opentrap", "statue")
# The idol's effsw: ccGimIdol +0x1e0, which effStatueOfGod's generator
# reads as its syncSW.
EFFSW = 0x1E0
# Answers not compared: effHeal's (and so effHealSkill's) is always 0 in
# the game (the port answers its controller), effOpenBox's, the crushes'
# and effStatueOfGod's always 1.
VOID = {"heal", "healskill", "openbox", "crushbarrel", "crushegg", "crushpot", "crushcorpse", "statue"}


class SkillFxMachine(spell.SpellMachine):
    """SpellMachine with a ccAnm's transparency as its constructor leaves
    it, the characters' posP and affectPerson, and the starters."""

    def __init__(self, seed=1, gseed=4352):
        super().__init__(seed=seed, gseed=gseed, area=1)
        self.m.hooks[self.sym("__ct__5ccAnmFv")] = self.anm_ctor
        self.vecbuf = self.malloc(self.m, 0x10)
        self.vecbuf2 = self.malloc(self.m, 0x10)

    @staticmethod
    def anm_ctor(m, a, *r):
        # ccCoord::ccCoord (hooked out here) sets localtp and worldtp 1.
        m.store(a + 0x84, 4, ONE)
        m.store(a + 0x88, 4, ONE)
        return a

    def set_pos_p(self, cid, v):
        self.vec(self.chars[cid] + 0x50, v)

    def set_affect(self, cid, by):
        self.m.store(self.chars[cid] + 0x98, 4, 0 if by is None else self.chars[by])

    def set_effsw(self, cid, v):
        self.m.store(self.chars[cid] + EFFSW, 4, v & 0xFFFFFFFF)

    def end_slot(self, k):
        a = EFFWORK + 0xC0 * k + 0x60
        self.m.store(a, 1, self.m.load(a, 1) | 8)

    def set_field(self, slot, key, *v):
        """The base's fields, and posT, param and flags."""
        a = EFFWORK + 0xC0 * slot
        if key == "posT":
            self.vec(a + 0x50, list(v))
        elif key in ("param", "flags"):
            self.m.store(a + (0x64 if key == "param" else 0x68), 4, v[0] & 0xFFFFFFFF)
        else:
            super().set_field(slot, key, *v)

    def start(self, name, args):
        """A starter with the probe's arguments (characters by id, vectors
        as bits): its answer, events, generators and the generators' states."""
        self.events, self.gens = [], []
        ch = lambda c: self.chars[c]           # noqa: E731

        def vec(x, y, z, buf=None):
            buf = buf or self.vecbuf
            self.vec(buf, [x, y, z, ONE])
            return buf
        a = list(args)
        fargs = ()
        if name == "icerock":
            # effIceRock(p, r, v, s, n): v and s in f12, f13.
            call = [vec(*a[:3]), vec(*a[3:6], buf=self.vecbuf2), a[8]]
            fargs = (a[6], a[7])
        elif name == "spawn":
            call = a
        elif name == "statue":
            call = [vec(*a[:3]), ch(a[3]) + EFFSW]
        elif name in POS_FIRST:
            call = [vec(*a[:3])] + a[3:]
        elif name == "skillstartp":
            call = [ch(a[0]), SKILL_TBL + 0x38 * a[1]] + a[2:]
        else:
            call = [ch(a[0])] + a[1:]
        r = self.call(EFFECT_STARTERS[name], *[v & 0xFFFFFFFF for v in call], fargs=fargs) & 0xFFFFFFFF
        if name == "shield":
            ret = -1 if r == 0 else self.element_slot(r, -1)
        elif EFFWORK <= r < EFFWORK + 0xC0 * 500:
            ret = (r - EFFWORK) // 0xC0
        else:
            ret = -1
        return {"ret": None if name in VOID else ret, "events": self.events, "gens": self.gens,
                "rand": self.rand.s, "genrand": self.genrand.count}


class Run:
    """One case sent to both sides: the world, the starts and the frames."""

    def __init__(self, seed, gseed):
        self.sm = SkillFxMachine(seed=seed, gseed=gseed)
        self.lines = ["reset %x %x 1 0 0" % (seed, gseed)]
        self.want = []
        self.chars = {}
        self.player = (0.0, 0.0, 0.0)

    def set_player(self, p):
        self.player = p
        self.sm.set_player(p)
        self.lines.append("player " + hexs(*map(fb, p)))

    def camera(self, eye, view):
        self.sm.set_camera(eye, eye, view)
        self.lines.append("camera " + hexs(*map(fb, list(eye) + list(eye) + list(view))))

    def char(self, cid, pos, height, width, dirc=(0.0, 0.0, 0.0), listed=True, dead=0, ctype=0x20, size=0):
        c = dict(pos=tuple(pos), height=height, width=width, dirc=tuple(dirc), listed=listed, dead=dead, ctype=ctype,
                 size=size)
        if cid in self.chars:
            self.sm.move_char(cid, c["pos"])
            self.sm.set_state(cid, listed, dead, size)
        else:
            self.sm.add_spell_char(cid, c["pos"], height, width, c["dirc"], listed, dead, ctype, size)
        self.chars[cid] = c
        self.lines.append(spell.Case.char_line(cid, c))
        pp = [fb(pos[0] - self.player[0]), fb(pos[1] - self.player[1]), fb(pos[2]), ONE]
        self.sm.set_pos_p(cid, pp)
        self.lines.append("posp %x " % cid + hexs(*pp))

    def affect(self, cid, by):
        self.sm.set_affect(cid, by)
        self.lines.append("affect %x %x" % (cid, 0xFFFFFFFF if by is None else by))

    def ground(self, z):
        """The flat land ccLandHitCheck2 meets, at height z."""
        self.sm.ground = fb(z)
        self.lines.append("ground %x" % fb(z))

    def set(self, k, key, *v):
        self.sm.set_field(k, key, *v)
        self.lines.append("set %x %s %s" % (k, key, hexs(*v)))

    def start(self, name, *args):
        self.lines.append(name + " " + hexs(*args))
        w = self.sm.start(name, args)
        self.want.append(("start %s" % name, w))
        return w["ret"]

    def end_slot(self, k):
        self.sm.end_slot(k)
        self.lines.append("endflag %x" % k)

    def effsw(self, cid, v):
        self.sm.set_effsw(cid, v)
        self.lines.append("effsw %x %x" % (cid, v & 0xFFFFFFFF))

    def frame(self):
        self.lines.append("frame")
        fr = self.sm.frame()
        self.want.append(("frame", fr))
        return fr

    def check(self, what):
        """(frames compared, the first difference or None)."""
        got = [g for g in spell.ask(self.lines) if g]
        if len(got) != len(self.want):
            return 0, "%s: %d answers against %d" % (what, len(self.want), len(got))
        frames = 0
        for k, ((kind, w), g) in enumerate(zip(self.want, got)):
            if w.get("ret", 0) is None:
                g["ret"] = None
            d = spell.compare(w, g, "%s step %d (%s)" % (what, k, kind))
            if d:
                return frames, d
            frames += kind == "frame"
            STATS["starts"] = STATS.get("starts", 0) + (kind != "frame")
            for key in ("draws", "gens", "events", "slots", "elements"):
                STATS[key] = STATS.get(key, 0) + len(w.get(key) or [])
            IDS.update(x["id"] for x in w.get("slots") or [])
        return frames, None


STATS = {}
IDS = set()


def world(run, rng, n=4, types=None):
    """A field about a player, a PC (0) and others of every type (or
    `types`), a camera near them."""
    px, py = rng.uniform(-20000, 20000), rng.uniform(-20000, 20000)
    run.set_player((px, py, rng.uniform(-100, 300)))
    tz = rng.uniform(-50, 200)
    for cid in range(n):
        if types:
            ctype = types[cid]
        else:
            ctype = 0x7 if cid == 0 else rng.choice([0x1, 0x7, 0x20, 0x40, 0x80, 0x80, 0xA0, 0x100, 0x0])
        run.char(cid, (px + rng.uniform(-600, 600), py + rng.uniform(-600, 600), tz + rng.uniform(-30, 30)),
                 rng.choice([160.0, 120.0, rng.uniform(40, 500)]), rng.uniform(10, 200),
                 (0.0, 0.0, rng.uniform(-3.14, 3.14)), ctype=ctype, size=rng.choice([0, 1, 2, 3, 4]))
    for cid in range(n):
        run.affect(cid, rng.choice([None] + [c for c in range(n) if c != cid]))
    new_camera(run, rng)


def new_camera(run, rng):
    px, py, pz = run.player
    ex, ey = px + rng.uniform(-1500, 1500), py + rng.uniform(-1500, 1500)
    if rng.random() < 0.1:
        ex, ey = px + rng.uniform(-8000, 8000), py + rng.uniform(-8000, 8000)
    view = (px + rng.uniform(-200, 200), py + rng.uniform(-200, 200), pz)
    if rng.random() < 0.1:
        view = (2 * ex - px, 2 * ey - py, pz)
    run.camera((ex, ey, pz + rng.uniform(100, 900)), view)


def jitter(run, rng, moving):
    for cid, c in list(run.chars.items()):
        if moving and rng.random() < 0.4:
            p = c["pos"]
            run.char(cid, (p[0] + rng.uniform(-40, 40), p[1] + rng.uniform(-40, 40), p[2] + rng.uniform(-10, 10)),
                     c["height"], c["width"], c["dirc"], c["listed"], c["dead"], c["ctype"], c["size"])
        if rng.random() < 0.004:
            run.char(cid, c["pos"], c["height"], c["width"], c["dirc"], not c["listed"], c["dead"], c["ctype"],
                     c["size"])
        if rng.random() < 0.003:
            run.char(cid, c["pos"], c["height"], c["width"], c["dirc"], c["listed"], rng.choice([0, 1, 2, 3]),
                     c["ctype"], c["size"])
    if rng.random() < 0.05:
        new_camera(run, rng)


def random_pos(run, rng):
    px, py, pz = run.player
    return [fb(px + rng.uniform(-900, 900)), fb(py + rng.uniform(-900, 900)), fb(pz + rng.uniform(-50, 150))]


ATTRS = [0, 4, 8, 16, 32, 64, 128, 0x14, 0x44, 3, 0x105]


def random_spec(run, rng, group):
    """One random start of the group's starters: (name, args, frames
    until its endFlag or None)."""
    chs = list(run.chars)
    c = rng.choice(chs)
    if group == "start":
        k = rng.random()
        a, b = rng.choice([0, 0, 1]), rng.choice([0, 1])
        sid = rng.choice([1, rng.randrange(0, 304), rng.randrange(150, 300), rng.randrange(2, 150)])
        if k < 0.45:
            return "skillstart", [c, sid, a, b], None
        if k < 0.8:
            return "skillstartp", [c, sid, a, b], None
        return "skillstarteffect", [c, rng.choice(ATTRS), b], None
    if group == "shock":
        return "shock", random_pos(run, rng) + [rng.choice(ATTRS)], None
    if group == "heal":
        k = rng.randrange(7)
        if k == 0:
            return "heal", [c, rng.choice([1, 1, rng.randrange(-3, 20)]) & 0xFFFFFFFF], None
        if k == 1:
            return "healskill", [c, rng.choice([150, 151, 152, 153, 154, 155, 295, rng.randrange(140, 170)])], None
        if k in (2, 3, 4):
            return ["cure", "sanity", "resurrect"][k - 2], [c], None
        if k == 5:
            return "openbox", random_pos(run, rng), None
        ta, tb = rng.choice([(-1, -1), (103, 121), (rng.randrange(0, 245), rng.randrange(0, 245)), (-1, 77)])
        return "removetrap", random_pos(run, rng) + [ta & 0xFFFFFFFF, tb & 0xFFFFFFFF], None
    if group == "ability":
        num = rng.choice([rng.randrange(0, 19), rng.randrange(20, 35), rng.randrange(0, 40)])
        return "abilityup" if rng.random() < 0.5 else "abilitydown", [c, num], rng.randrange(20, 200)
    if group == "gimmick":
        k = rng.randrange(4)
        if k == 0:
            return "virus", random_pos(run, rng), None
        if k == 1:
            return rng.choice(CRUSHES), random_pos(run, rng) + [rng.choice([0, 0, 1, 2])], None
        if k == 2:
            return "opentrap", random_pos(run, rng) + [rng.choice([0, 1, 1, 3]), rng.choice([0, 3, 4])], None
        return "statue", random_pos(run, rng) + [c], None
    if group == "shield":
        c = rng.choice([k for k in chs if run.chars[k]["ctype"] & 0xE0] or chs)
        return "shield", [c, rng.choice([-1, 0, 1, 2, 5]) & 0xFFFFFFFF,
                          rng.choice([-1, -1, 0, 1, 2, 3, 4]) & 0xFFFFFFFF], None
    if group == "ids":
        k = rng.random()
        if k < 0.4:
            return ice_rock(run, rng)
        if k < 0.7:
            return meteor_anm(run, rng)
        if k < 0.9:
            return held_wave(run, rng)
        return no_code(run, rng)
    raise KeyError(group)


# The ids whose ccEffect::Main cases the port once lacked: the ice rocks
# 15-18 (effIceRock), and 0, 113 and -11..-8, which no function of any
# volume makes: spawned by ccNewEffect and set up as a creator would.
ICE_BREAK = [(50.0, 0.2, 10), (25.0, 1.0, 5), (10.0, 5.0, 1)]
DIRS = [0.0, 1.5707963705062866, -3.1415927410125732, -1.5707963705062866]


def ice_rock(run, rng, vsn=None, d=None, p=None):
    """effIceRock(p, r, v, s, n) as ccBossEffIceBreak::Draw calls it (100
    above a point, turned about z, one of its three sizes) or at random."""
    if p is None:
        p = random_pos(run, rng)
        p[2] = fb(fu(p[2]) + 100.0)
    if d is None:
        d = rng.choice(DIRS) if rng.random() < 0.6 else rng.uniform(-3.2, 3.2)
    r = [0, 0, fb(d)]
    if rng.random() < 0.2:
        r = [fb(rng.uniform(-3.2, 3.2)) for _ in range(3)]
    if vsn is None:
        vsn = rng.choice(ICE_BREAK) if rng.random() < 0.6 else (rng.uniform(0, 80), rng.uniform(0.1, 6),
                                                                 rng.randrange(0, 12))
    v, s, n = vsn
    return "icerock", p + r + [fb(v), fb(s), n], None


fu = spell.fu


def meteor_anm(run, rng, atr=None, falling=True):
    """Id 0 (ANM_x300) as effMeteoFireBall2 sets up its meteors: life 300,
    param the element, offset above the target, posT the target's place, a
    turn, falling at 20 from pos to temp[0]."""
    c = rng.choice(list(run.chars))
    t = run.chars[c]["pos"]
    if atr is None:
        atr = rng.choice([4, 16, 64, 128, 8, 32, 0x14, 0x44, 0x84, 3, 0])
    off = [rng.uniform(-400, 400), rng.uniform(-400, 400), t[2] + rng.uniform(150, 900)]
    land = rng.choice([t[2], fu(run.sm.ground), t[2] - rng.uniform(0, 80), t[2] + rng.uniform(0, 60)])
    rot = [fb(rng.uniform(-3.2, 3.2)) for _ in range(3)] + [0]
    sets = [("life", 300), ("param", atr), ("offset", *[fb(x) for x in off], ONE), ("target", c),
            ("posT", *[fb(x) for x in t], ONE), ("rot", *rot), ("speed", 0, 0, fb(-20.0), 0),
            ("pos", fb(t[0] + off[0]), fb(t[1] + off[1]), fb(off[2]), ONE), ("flags", int(falling)),
            ("temp", fb(land), 0, 0, 0)]
    return "spawnset", [0, sets], None


def held_wave(run, rng, life=None):
    """Id 113 (CMP_x042) somewhere, with a life (-1 ended at random)."""
    if life is None:
        life = rng.choice([0, 1, 4, 5, 6, 14, 15, 16, 19, 20, 21, 30, 45, rng.randrange(0, 90), -1])
    p = random_pos(run, rng) + [ONE]
    rot = [fb(rng.uniform(-3.2, 3.2)) for _ in range(3)] + [0]
    sets = [("life", life & 0xFFFF), ("pos", *p), ("rot", *rot)]
    return "spawnset", [113, sets], rng.randrange(10, 80) if life < 0 else None


def no_code(run, rng, eid=None):
    """Ids -11..-8 (no object; their case stores to address 0)."""
    eid = rng.randrange(-11, -7) if eid is None else eid
    life = rng.choice([0, 3, 20, rng.randrange(0, 60), -1])
    sets = [("life", life & 0xFFFF), ("pos", *random_pos(run, rng), ONE)]
    return "spawnset", [eid, sets], rng.randrange(5, 40) if life < 0 else None


CRUSHES = ("crushbarrel", "crushegg", "crushpot", "crushcorpse")

# The shield sweep's characters: a PC, two enemies, a boss, an enemy and
# boss at once.
SHIELD_TYPES = [0x7, 0x20, 0x40, 0x80, 0xA0]


def sweep_specs(group):
    """Every value of the group's starters worth one start: (name, args
    with CHAR for a random character, frames until its endFlag or None),
    by chunks of a case each."""
    C = "CHAR"
    if group == "start":
        specs = [("skillstart" if sid % 2 else "skillstartp", [C, sid, int(sid % 3 == 0), (sid // 2) % 2], None)
                 for sid in range(304)]
        specs += [("skillstarteffect", [C, attr, b], None) for attr in ATTRS for b in (0, 1)]
        return [specs[k:k + 30] for k in range(0, len(specs), 30)]
    if group == "shock":
        specs = [("shock", ["POS", attr], None) for attr in ATTRS]
        return [specs[k:k + 6] for k in range(0, len(specs), 6)]
    if group == "heal":
        specs = [("heal", [C, n & 0xFFFFFFFF], None) for n in range(-8, 17)]
        specs += [("healskill", [C, sid], None) for sid in list(range(140, 171)) + [295]]
        specs += [(name, [C], None) for name in ("cure", "sanity", "resurrect") for _ in range(2)]
        specs += [("openbox", ["POS"], None)] * 2
        specs += [("removetrap", ["POS", a & 0xFFFFFFFF, b & 0xFFFFFFFF], None)
                  for a, b in ((-1, -1), (103, 121), (0, 0), (244, 1), (-1, 77), (55, -1))]
        return [specs[k:k + 24] for k in range(0, len(specs), 24)]
    if group == "ability":
        specs = [(name, [C, num], 30 + 7 * num) for num in range(41) for name in ("abilityup", "abilitydown")]
        return [specs[k:k + 21] for k in range(0, len(specs), 21)]
    if group == "gimmick":
        specs = [("virus", ["POS"], None)] * 2
        specs += [(name, ["POS", x], None) for name in CRUSHES for x in (0, 1)]
        specs += [("opentrap", ["POS", kind, trap], None) for kind in (0, 1) for trap in (0, 3, 4)]
        specs += [("statue", ["POS", C], None)] * 2
        return [specs[k:k + 6] for k in range(0, len(specs), 6)]
    if group == "shield":
        specs = [("shield", [c, m & 0xFFFFFFFF, n & 0xFFFFFFFF], None)
                 for c in range(1, 5) for m in (-1, 0, 1, 2) for n in (-1, 0, 1, 3, 4)]
        return [specs[k:k + 20] for k in range(0, len(specs), 20)]
    if group == "ids":
        # An IceBreak's twelve throws; each element's meteor 0, falling and
        # not; 113 at lives around its 5 and 15; each of -11..-8.
        at = {}

        def burst(r, g, vsn, d):
            if r not in at:
                at[r] = random_pos(r, g)
                at[r][2] = fb(fu(at[r][2]) + 100.0)
            return ice_rock(r, g, vsn, d, list(at[r]))
        chunks = [[lambda r, g, vsn=vsn, d=d: burst(r, g, vsn, d) for d in DIRS for vsn in ICE_BREAK]]
        chunks.append([lambda r, g, a=a: meteor_anm(r, g, a) for a in (4, 16, 64, 128, 8, 0)]
                      + [lambda r, g: meteor_anm(r, g, 16, False)])
        chunks.append([lambda r, g, n=n: held_wave(r, g, n) for n in (0, 1, 4, 5, 6, 15, 19, 20, 21, 40, -1)])
        chunks.append([lambda r, g, e=e: no_code(r, g, e) for e in range(-11, -7)])
        return chunks
    raise KeyError(group)


def fill(run, rng, args):
    out = []
    for a in args:
        if a == "CHAR":
            out.append(rng.choice(list(run.chars)))
        elif a == "POS":
            out.extend(random_pos(run, rng))
        else:
            out.append(a)
    return out


def play(run, rng, plan, moving):
    """The starts of `plan` {frame: [spec]}, then frames until everything
    has ended."""
    ends, offs = {}, {}
    last = max(plan) if plan else 0
    for f in range(600):
        for spec in plan.get(f, []):
            name, args, end = spec(run, rng) if callable(spec) else spec
            args = fill(run, rng, args)
            if name == "statue":
                run.effsw(args[3], 1)
                offs.setdefault(f + rng.randrange(5, 120), []).append(args[3])
                last = max(last, f + 120)
            if name == "spawnset":
                # ccNewEffect(id), then the fields as a creator sets them.
                eid, sets = args
                ret = run.start("spawn", eid & 0xFFFFFFFF)
                for key, *v in sets if ret >= 0 else ():
                    run.set(ret, key, *v)
            else:
                ret = run.start(name, *args)
            if end is not None and ret is not None and ret >= 0:
                ends.setdefault(f + end, []).append(ret)
                last = max(last, f + end)
        for k in ends.pop(f, []):
            run.end_slot(k)
        # An idol open 100 frames: its effsw off, the glow's generator
        # stopping.
        for cid in offs.pop(f, []):
            run.effsw(cid, 0)
        jitter(run, rng, moving)
        fr = run.frame()
        if f > last and not fr["slots"] and not fr["elements"]:
            break


def run_case(group, seed):
    """A random case: 1-4 random starts in the first 40 frames."""
    rng = random.Random(seed * 7919 + GROUPS.index(group))
    run = Run(rng.randrange(1, 1 << 31), rng.randrange(1, 1 << 31))
    world(run, rng, rng.randrange(2, 6))
    if group == "ids":
        ground(run, rng)
    plan = {}
    for _ in range(rng.randrange(1, 5)):
        plan.setdefault(rng.randrange(0, 40), []).append(lambda r, g, grp=group: random_spec(r, g, grp))
    play(run, rng, plan, rng.random() < 0.6)
    return run.check("%s seed %d" % (group, seed))


def run_sweep(group, k, chunk):
    """A sweep case: the chunk's starts, a few a frame, in a random world."""
    rng = random.Random(1000003 * (k + 1) + GROUPS.index(group))
    run = Run(rng.randrange(1, 1 << 31), rng.randrange(1, 1 << 31))
    if group == "shield":
        world(run, rng, len(SHIELD_TYPES), SHIELD_TYPES)
    else:
        world(run, rng, rng.randrange(2, 6))
    # The ids' chunks start together (an IceBreak's twelve in one Draw).
    per = 3
    if group == "ids":
        ground(run, rng)
        per = len(chunk)
    plan = {}
    for i, spec in enumerate(chunk):
        plan.setdefault(i // per, []).append(spec)
    play(run, rng, plan, rng.random() < 0.6)
    return run.check("%s sweep %d" % (group, k))


def ground(run, rng):
    """The land at, under or over the player's feet, or at 0."""
    pz = run.player[2]
    run.ground(rng.choice([pz, pz - rng.uniform(0, 60), pz + rng.uniform(0, 40), 0.0]))


GROUPS = ("start", "shock", "heal", "ability", "shield", "gimmick", "ids")


def run_group(group, n, first=1, sweeps=True):
    """The group's sweeps, then n random cases: (frames, the first
    difference or None)."""
    STATS.clear()
    IDS.clear()
    total = 0
    cases = [lambda k=k, c=c: run_sweep(group, k, c) for k, c in enumerate(sweep_specs(group))] if sweeps else []
    cases += [lambda seed=seed: run_case(group, seed) for seed in range(first, first + n)]
    for case in cases:
        frames, d = case()
        total += frames
        STATS["cases"] = STATS.get("cases", 0) + 1
        if d:
            return total, d
    return total, None


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class SkillEffectsAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        base.build("spell_probe")

    def check(self, group, n):
        total, d = run_group(group, n)
        print("%s: %d cases, %d frames" % (group, n, total), dict(STATS), "ids", sorted(IDS), file=sys.stderr)
        self.assertIsNone(d)
        self.assertGreater(total, 0)

    def test_skill_start(self):
        self.check("start", 30)

    def test_shock_wave(self):
        self.check("shock", 20)

    def test_heals_and_cures(self):
        self.check("heal", 30)

    def test_ability(self):
        self.check("ability", 20)

    def test_resistant_shield(self):
        self.check("shield", 30)

    def test_gimmicks(self):
        self.check("gimmick", 30)

    def test_ice_rocks_and_unmade_ids(self):
        self.check("ids", 30)


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        base.build("spell_probe")
        for g in (sys.argv[3].split(",") if len(sys.argv) > 3 else GROUPS):
            total, d = run_group(g, int(sys.argv[2]), first=1000, sweeps=len(sys.argv) < 5)
            print(g, "frames", total, dict(STATS), "ids", sorted(IDS), "mismatch" if d else "ok", d or "", flush=True)
    else:
        unittest.main()
