#!/usr/bin/env python3
"""crates/piney-world/src/foe.rs (the enemies' and the magic portal's
drawing) against the game's own code run in the EE interpreter (the Rust
machine, tools/test_anim.py's machine_class). The foe_probe example answers
for the port.

  looks   what each row loads: ccAddRequestFileListEntry (gcmn 0x0042f5a0)
          and ccEntryCtrl::initEntryCCS (0x0042feb0) run natively over the
          row (ccStream::GetCCSAdrs recording the files), then the race's
          constructor through ccEntryRaceTbl (ccEnemy::ccEnemy, initEnemy,
          initEnemyCCS, ccEntryChangeCLUT, the race's own part) with
          GetChunkAdrsF, ccClump::Init, ccClump::ChangeClut and ccAnm::SetAnm
          recording: the file and clump of the main model, the palette swaps
          (material, clut) in order, the animation it plays, a middle boss's
          second file and animation. The world the constructors ask is
          tools/test_battle_spawn_rs.py's harness.
  pose    an enemy's clump posed at a dispEnemy matrix: one of the row's
          animations compiled and played by the game (GameAnim:
          ConvLCNum2ALCNum, InitAnmCtrlWork, _AnimateForward) onto one
          ccObj per node under its ExtObj parent and the anm, the anm's
          matrix set as dispEnemy sets it (+0x40, matCalcSW), then
          ccObj::Draw (main 0x0013f220) of each clump node at the anm's
          transparency: the world matrix ccCoord::_SetLWMatrix composes and
          the alpha ccModel::Draw is handed, for every node, against
          foe::Model::worlds and node_alphas.
  circle  the portal's CMP_xmagcir0 posed by ANM_xmagcir1 or 2 the same
          way, the anm placed by ccCoord::SetMatrix_PosRotZYX (main
          0x001382f0), against foe::Circle::root and the node matrices.
  draw    ccChar::Draw (gcmn 0x0056b1c0) natively on a character of random
          state (condition.dead, conditionNum, the affect flash and the
          condition tint, setTransparency, transDist, hitAttribute) with the
          camera, player and area at random (ccGetCameraTransparency native,
          ccTransPosW2P the field's frame, ccAnm::Draw and the lights
          recorded): the blend set, the transparency, whether it draws, the
          shade, the shadow's alpha and length, and the affect members after,
          against foe::char_draw_parts.

    python3 tools/test_foe_rs.py            the unit tests
    python3 tools/test_foe_rs.py bulk N     N cases of pose, circle and draw

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import math
import os
import random
import shutil
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
ISO = volume.ISO
TARGET = os.environ.get("CARGO_TARGET_DIR") or os.path.join(ROOT, "target")
EXAMPLE = os.path.join(TARGET, "release", "examples", "foe_probe")
READY = os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo") is not None

ONE = 0x3F800000
ENEMY_TBL, ROW = inf_va(0x005F1E70), 0x1C0
RACE_TBL = inf_va(0x005F1D60)

# The first field's and dungeon's rows (field 14), some of their neighbours
# (the gold goblins' rows 154-157 with ccEnemyG's second palette, a row of
# each race's first form), and middle bosses of the ported races.
LOOK_ROWS = [67, 130, 151, 185, 189, 219, 268, 66, 129, 131, 154, 155, 156, 157, 184, 187, 218, 267, 72, 76, 164,
             165, 224, 225]
# The rows whose clumps the pose check moves (field 14's).
POSE_ROWS = [67, 130, 151, 185, 189, 219, 268]
# Matrix elements: the game's VU0 products truncate, the port's f32 ones
# round, and its keyed rotations come from tools/anim.py's double arithmetic
# (test_world_rs.py's KiteWeapons bounds, over as many nodes).
ROT_TOL, POS_TOL, ALPHA_TOL = 1e-4, 1e-2, 1e-6


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def f32(bits):
    return struct.unpack("<f", struct.pack("<I", bits & 0xFFFFFFFF))[0]


def hexs(*v):
    return " ".join("%x" % (x & 0xFFFFFFFF) for x in v)


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example", "foe_probe"],
                   cwd=ROOT, check=True)


def ask(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True, cwd=ROOT)
    out = [json.loads(line) for line in p.stdout.splitlines()]
    if p.returncode or len(out) != len(lines):
        raise RuntimeError(f"foe_probe failed after {len(out)} answers\n{p.stderr[-2000:]}")
    return out


def cstr(m, a, n=64):
    return bytes(m.mem[a:a + n]).split(b"\0")[0].decode("latin-1")


# --- looks: the loading and the race constructors -----------------------------------

STREAMS, CHUNK_BASE = 0x01C00000, 0x01C10000


class Looks:
    """The spawn harness's machine with the loading and the model calls
    recorded."""

    def __init__(self):
        import test_battle_rs as rs
        import test_battle_spawn_rs as sp
        self.sp = sp
        self.h = h = sp.harness(rs.Checks())
        self.m, self.sym = h.m, h.sym
        self.streams, self.chunks, self.names = {}, {}, {}
        self.rec = []
        me = self

        def ccs_adrs(mm, name, *_):
            n = cstr(mm, name)
            if n not in me.streams:
                a = STREAMS + 16 * (len(me.streams) + 1)
                me.streams[n] = a
                me.names[a] = n
            me.rec.append(("ccs", n))
            return me.streams[n]

        def chunk(mm, stream, name, *_):
            key = (me.names.get(stream, hex(stream)), cstr(mm, name))
            if key not in me.chunks:
                me.chunks[key] = CHUNK_BASE + 16 * (len(me.chunks) + 1)
            me.rec.append(("chunk", key))
            return me.chunks[key]

        def chunk_name(a):
            return next((k for k, v in me.chunks.items() if v == a), (None, hex(a)))

        def clump_init(mm, clump, ch, *_):
            me.rec.append(("clump", clump, chunk_name(ch)))
            return 0

        def change_clut(mm, clump, clut, mat, *_):
            me.rec.append(("clut", clump, chunk_name(clut), chunk_name(mat)))
            return 0

        def strchr(mm, s, c, *_):
            c &= 0xFF
            while True:
                b = mm.load(s, 1)
                if b == c:
                    return s
                if b == 0:
                    return 0
                s += 1

        def set_anm(mm, anm, ch, *_):
            me.rec.append(("anm", anm, chunk_name(ch)))
            mm.store(anm + 0xAC, 4, ch or 1)
            return 0

        self.hooks = {
            "GetCCSAdrs__8ccStreamFPCc": ccs_adrs,
            "GetChunkAdrsF__8ccStreamFPCci": chunk,
            "Init__7ccClumpFP12ccClumpChunk": clump_init,
            "ChangeClut__7ccClumpFP11ccClutChunkP15ccMaterialChunk": change_clut,
            "SetAnm__5ccAnmFP10ccAnmChunkUi": set_anm,
            "ccAddFileListOne__FP10ccFileList": lambda mm, fl, *_: me.rec.append(
                ("load", mm.load(fl, 4), cstr(mm, mm.load(fl + 4, 4)))) or 0,
            "ccAddRtpcWeapon__Fi": lambda mm, *_: 0,
            # newlib's strchr is MMI code the interpreter does not run.
            "strchr": strchr,
        }
        # ccEntryChangeCLUT runs natively (the spawn harness stubs it).
        self.native = ["ccEntryChangeCLUT__FP7ccEntryP7ccClump"]

    def install(self):
        m, sym = self.m, self.sym
        saved = {}
        for n, f in self.hooks.items():
            a = sym(n)
            saved[a] = m.hooks.get(a)
            m.hooks[a] = f
        for n in self.native:
            a = sym(n)
            saved[a] = m.hooks.pop(a, None)
        return saved

    def uninstall(self, saved):
        for a, f in saved.items():
            if f is None:
                self.m.hooks.pop(a, None)
            else:
                self.m.hooks[a] = f

    def look(self, row, rnd):
        """What the game loads and makes for row: the answer foe_probe's
        `look` gives."""
        h, m, sp = self.h, self.m, self.sp
        race = next(r for r in range(22) if h.race_first[r] <= row < h.race_first[r] + h.race_num[r])
        sc = h.rnd_scene(rnd, ne=0, nc=0, ng=0, nn=0)
        ent = h.rnd_ent(rnd, sc, 0, row)
        sc["attr"] = sc["attr"] or [inf_va(0x00304050)]
        saved = self.install()
        try:
            h.put_scene(sc)
            h.put_ent(sp.EP, ent)
            for i in range(303):
                m.store(ENEMY_TBL + ROW * i + 0x68, 4, 1 if i == row else 0)
            self.rec = []
            h.call("ccAddRequestFileListEntry__Fv")
            h.call("initEntryCCS__11ccEntryCtrlFv", (sp.CTRL,))
            loading = list(self.rec)
            entry = ENEMY_TBL + ROW * row + 0x68
            ccsc, ccsc2 = m.load(entry + 0x10, 4), m.load(entry + 0x14, 4)
            m.store(entry + 0xC, 4, sp.EP)
            self.rec = []
            h.call(m.load(RACE_TBL + 12 * race, 4), (entry,))
            made = list(self.rec)
            obj = h.addrs[-1] if h.addrs else 0
            anm, anm2 = m.load(obj + 0xD4, 4), m.load(obj + 0x1BC, 4)
            clump = m.load(obj + 0xD0, 4)
        finally:
            h.restore()
            self.uninstall(saved)
        file = self.names.get(ccsc)
        clumps = [r[2] for r in made if r[0] == "clump" and r[1] == clump]
        anims = [r[2][1] for r in made if r[0] == "anm" and r[1] == anm]
        anims2 = [r[2][1] for r in made if r[0] == "anm" and anm2 and r[1] == anm2]
        swaps = [[r[3][1], r[2][1]] for r in made if r[0] == "clut" and r[1] == clump]
        # Every chunk the constructor asked of the main model's file came
        # from its ccsc; the second clump from ccsc2.
        bad = [r for r in made if r[0] == "clump" and r[2][0] not in (file, self.names.get(ccsc2))]
        return {"file": file, "clump": clumps[0][1] if clumps and clumps[0][0] == file else clumps,
                "swaps": swaps, "anim": anims[-1] if anims else None,
                "second": self.names.get(ccsc2) if ccsc2 else None, "anim2": anims2[-1] if anims2 else None,
                "loads": [r[2] for r in loading if r[0] == "load"], "bad": bad}


def check_looks(rows=LOOK_ROWS, seed=1):
    lk = Looks()
    rnd = random.Random(seed)
    game = [lk.look(r, rnd) for r in rows]
    port = ask([f"look {r:x}" for r in rows])
    bad = []
    for r, g, p in zip(rows, game, port):
        want = {k: g[k] for k in ("file", "clump", "swaps", "anim", "second", "anim2")}
        got = {k: p.get(k) for k in want}
        # The files loaded: the fileList (and a middle boss's 'X' file).
        loads = [n.split(".")[0].lower() for n in g["loads"]]
        if got != want or g["bad"] or loads[:1] != [g["file"]]:
            bad.append((r, want, got, g["loads"], g["bad"]))
    return rows, game, bad


# --- pose and circle: clumps posed by the game's animation code -----------------------

class Poser:
    """A scene file's clump in the game's own animation code (as
    tools/test_world_rs.py's KiteWeapons does Kite's): one of the file's
    animations compiled and played by the game onto one ccObj per object,
    each under its ExtObj parent and the first under the anm; every clump
    node that the animation drives given a stand-in model, and drawn by
    ccObj::Draw, ccModel::Draw catching the matrix and alpha."""

    def __init__(self, stem, clump):
        import test_anim
        import test_world_rs as tw
        from image import Program
        self.ta = test_anim
        g = self.g = test_anim.GameAnim()
        prog = Program(ELF, "gcmn")
        m = tw.weapon_machine_class()(prog)
        m.hooks.update(g.m.hooks)
        g.prog, g.m = prog, m
        self.m, self.sym = m, g.sym
        m.store(g.sym("ccSys"), 4, test_anim.SYS)
        m.store(test_anim.SYS + 604, 4, test_anim.SCRATCH)
        c = self.c = test_anim.member(stem + ".cmp")
        names = [o[0] for o in c.objects]
        ch = [n for n in c.chunks() if n[1] is not None and n[1] & 0xFFFF == 0x0900
              and names[struct.unpack_from("<I", c.data, n[0] + 8)[0]] == clump][0][0]
        count = struct.unpack_from("<H", c.data, ch + 12)[0]
        self.clump = list(struct.unpack_from("<%dI" % count, c.data, ch + 16))
        # Obj chunks: each object's model, when that has something to draw
        # (a Model chunk with mmats).
        mmats, self.model_of = {}, {}
        for off, t, n, end in c.chunks():
            if t is not None and t & 0xFFFF == 0x0800:
                mdl, _, _, num = struct.unpack_from("<IIHH", c.data, off + 8)
                mmats[mdl] = num
        for off, t, n, end in c.chunks():
            if t is not None and t & 0xFFFF == 0x0100:
                o, _, mdl = struct.unpack_from("<3I", c.data, off + 8)
                self.model_of[o] = mdl if mmats.get(mdl) else 0
        self.anims, ext = {}, []
        _, anims = test_anim.animes(c)
        by_off = dict(anims)
        for off, t, n, end in c.chunks():
            if t is None:
                break
            if t & 0xFFFF == 0x0A00:
                ext.append(struct.unpack_from("<3I", c.data, off + 8))
            elif t & 0xFFFF == 0x0700:
                self.anims[c.objects[struct.unpack_from("<I", c.data, off + 8)[0]][0]] = (off, by_off[off], ext)
                ext = []
        self.drawn = []

        def model_draw(mm, model, param, *a):
            obj = mm.load(param + 144, 4)
            self.drawn.append((obj, [mm.load(obj + 4 * i, 4) for i in range(16)], mm.load(param + 288, 4)))
            return 0
        m.hooks[self.sym("Draw__7ccModelFP16ccDrawModelParam")] = model_draw

    def setup(self, name, speed):
        g, m, ta = self.g, self.m, self.ta
        off, a, ext = self.anims[name]
        self.a, self.time, self.speed = a, 0, speed
        self.table = g.load(self.c.data, off)
        self.coords, _, _ = g.playback(self.table, lambda o: self.c.objects[o][0], speed)
        parent = {o: p for o, p, t in ext}
        target = {o: t for o, p, t in ext}
        for o, co in self.coords.items():
            p = parent.get(o, 0)
            m.store(co + 128, 4, self.coords[p] if p else ta.ANM)
        # The clump's nodes: the coords of the objects driving them.
        self.node_of = {}
        for o, co in self.coords.items():
            t = target.get(o, o)
            if t in self.clump and t not in self.node_of.values():
                self.node_of[co] = t
        for co in self.node_of:
            model = g.malloc(m, 0x40)
            m.call(self.sym("SetModel__5ccObjFP7ccModel"), (co, model))
        m.store(inf_va(0x003788D8), 4, g.malloc(m, 0x100))
        m.store(inf_va(0x003788C4), 4, g.malloc(m, 0x200))
        self.pv = g.malloc(m, 32)
        # The clump's nodes this animation does not drive: none has a model
        # to draw.
        return sorted(o for o in set(self.clump) - set(self.node_of.values()) if self.model_of.get(o))

    def frame(self, alpha, matrix=None, pos=None, dirc=None):
        """One _AnimateForward, the anm placed (a dispEnemy matrix, or
        SetMatrix_PosRotZYX(pos, dirc)), then ccObj::Draw of each node in
        the animation's order. Returns (the time posed or None, {node
        object: (matrix bits, alpha bits)}, the anm's matrix)."""
        g, m, ta = self.g, self.m, self.ta
        got_time, got_end = g.forward(self.speed)
        f = self.a.forward(self.time, self.speed)
        self.time = f.time
        assert (got_time, got_end) == (f.time, int(f.ended)), (got_time, got_end, f)
        if f.pose_at is None:
            return None, {}, None
        if matrix is not None:
            for k, v in enumerate(matrix):
                m.store(ta.ANM + 64 + 4 * k, 4, v)
            m.store(ta.ANM + 141, 1, 1)
        else:
            m.mem[self.pv:self.pv + 32] = struct.pack("<8I", *(list(pos[:3]) + [ONE] + list(dirc[:3]) + [0]))
            m.call(self.sym("SetMatrix_PosRotZYX__7ccCoordFPfPf"), (ta.ANM, self.pv, self.pv + 16))
        m.store(ta.ANM + 136, 4, alpha)
        self.drawn = []
        for obj, _ in self.table:
            co = self.coords.get(obj)
            if co is None or co not in self.node_of:
                continue
            m.f[12] = alpha
            m.call(self.sym("Draw__5ccObjFf"), (co,), limit=5_000_000)
        out = {self.node_of[co]: (mat, tp) for co, mat, tp in self.drawn}
        return f.pose_at, out, [m.load(ta.ANM + 64 + 4 * k, 4) for k in range(16)]


def rnd_matrix(rnd):
    """A dispEnemy-like matrix: a facing (sometimes tipped), a place on a
    field."""
    rz = rnd.uniform(-math.pi, math.pi)
    rx, ry = (rnd.uniform(-0.4, 0.4), rnd.uniform(-0.4, 0.4)) if rnd.random() < 0.3 else (0.0, 0.0)
    cx, sx, cy, sy, cz, sz = math.cos(rx), math.sin(rx), math.cos(ry), math.sin(ry), math.cos(rz), math.sin(rz)
    # Rx Ry Rz, columns.
    r = [[cy * cz, cx * sz + sx * sy * cz, sx * sz - cx * sy * cz],
         [-cy * sz, cx * cz - sx * sy * sz, sx * cz + cx * sy * sz],
         [sy, -sx * cy, cx * cy]]
    t = [rnd.uniform(0, 48000), rnd.uniform(0, 48000), rnd.uniform(-300, 900)]
    return [fb(v) for v in r[0] + [0.0] + r[1] + [0.0] + r[2] + [0.0] + t + [1.0]]


def compare_nodes(game, port, names, what):
    """(mismatches, worst rotation, worst position, first difference)."""
    bad, worst, first = 0, [0.0, 0.0, 0.0], None
    by_name = {n: (mat, tp) for n, mat, tp in port}
    for node, (mat, tp) in game.items():
        p = by_name.get(names[node])
        if p is None:
            bad += 1
            first = first or f"{what}: {names[node]} not in the port's nodes"
            continue
        pm, ptp = p
        d_rot = max(abs(f32(mat[k]) - f32(pm[k])) for k in range(12) if k % 4 != 3)
        # Positions to within a few of their own float steps as well.
        d_pos = max(abs(f32(mat[k]) - f32(pm[k])) - 1e-6 * abs(f32(mat[k])) for k in (12, 13, 14))
        d_tp = abs(f32(tp) - f32(ptp))
        exact = [mat[k] for k in (3, 7, 11, 15)] == [pm[k] for k in (3, 7, 11, 15)]
        worst = [max(worst[0], d_rot), max(worst[1], d_pos), max(worst[2], d_tp)]
        if d_rot > ROT_TOL or d_pos > POS_TOL or d_tp > ALPHA_TOL or not exact:
            bad += 1
            FAILS.append((what, names[node], d_rot, d_pos, d_tp))
            if first is None:
                first = (f"{what} {names[node]}: rotation {d_rot:.3g} position {d_pos:.3g} alpha {d_tp:.3g}\n"
                         f" game {[round(f32(x), 4) for x in mat]} tp {f32(tp)}\n"
                         f" port {[round(f32(x), 4) for x in pm]} tp {f32(ptp)}")
    return bad, worst, first


# Every node that failed, for a look after a run.
FAILS = []


def pose_cases(n, seed):
    """n frames of the field-14 rows' clumps: (row, animation, frames...)."""
    rnd = random.Random(seed)
    per = max(1, n // len(POSE_ROWS))
    lines, games = [], []
    unposed = {}
    for row in POSE_ROWS:
        stem = ask([f"look {row:x}"])[0]["file"]
        p = Poser(stem, "CMP_trall")
        names = [o[0] for o in p.c.objects]
        clips = sorted(k for k in p.anims)
        done = 0
        while done < per:
            name = rnd.choice(clips)
            speed = rnd.choice((256, 256, rnd.randrange(0x40, 0x300)))
            missing = p.setup(name, speed)
            unposed[(stem, name)] = [names[o] for o in missing]
            for _ in range(rnd.randrange(1, 40)):
                mat = rnd_matrix(rnd)
                alpha = rnd.choice((ONE, ONE, fb(rnd.random())))
                posed, drawn, _ = p.frame(alpha, matrix=mat)
                if posed is None or rnd.random() < 0.5:
                    continue
                drawn = {k: (v[0], v[1]) for k, v in drawn.items()}
                lines.append(f"pose {row:x} {name} {posed:x} {hexs(*mat)}")
                games.append((f"row {row} {name} at {posed}", drawn, names, alpha))
                done += 1
                if done >= per:
                    break
    return lines, games, unposed


def run_pose(n, seed=11):
    lines, games, unposed = pose_cases(n, seed)
    port = ask(lines)
    return tally(games, port), unposed


def tally(games, port):
    bad, worst, first, nodes = 0, [0.0, 0.0, 0.0], None, 0
    for (what, drawn, names, alpha), p in zip(games, port):
        if "error" in p:
            bad += 1
            first = first or f"{what}: {p['error']}"
            continue
        # The port's node transparency times the anm's, as ccObj::Draw takes it.
        pnodes = [(nm, mat, fb(f32(tp) * f32(alpha))) for nm, mat, tp in p["nodes"]]
        b, w, f = compare_nodes(drawn, pnodes, names, what)
        bad += b
        nodes += len(drawn)
        worst = [max(a, c) for a, c in zip(worst, w)]
        first = first or f
    return {"cases": len(games), "nodes": nodes, "bad": bad, "worst": worst, "first": first}


def run_circle(n, seed=12):
    rnd = random.Random(seed)
    p = Poser("xmagcir", "CMP_xmagcir0")
    names = [o[0] for o in p.c.objects]
    lines, games, roots = [], [], []
    while len(games) < n:
        name = rnd.choice(("ANM_xmagcir1", "ANM_xmagcir2"))
        p.setup(name, rnd.choice((256, 256, rnd.randrange(0x80, 0x200))))
        for _ in range(rnd.randrange(1, 90)):
            pos = [fb(rnd.uniform(0, 48000)), fb(rnd.uniform(0, 48000)), fb(rnd.uniform(-200, 1200))]
            dirc = [0, 0, fb(rnd.uniform(-math.pi, math.pi))] if rnd.random() < 0.8 else \
                [fb(rnd.uniform(-1, 1)), fb(rnd.uniform(-1, 1)), fb(rnd.uniform(-math.pi, math.pi))]
            alpha = rnd.choice((ONE, fb(rnd.random())))
            posed, drawn, anm = p.frame(alpha, pos=pos, dirc=dirc)
            if posed is None or rnd.random() < 0.6:
                continue
            lines.append(f"circle {name} {posed:x} {hexs(*pos)} {hexs(*dirc)}")
            games.append((f"{name} at {posed}", drawn, names, alpha))
            roots.append(anm)
            if len(games) >= n:
                break
    port = ask(lines)
    root_bad = sum(1 for r, q in zip(roots, port) if q.get("root") != r)
    t = tally(games, port)
    t["root_bad"] = root_bad
    return t


# --- draw: ccChar::Draw ----------------------------------------------------------------

class CharDraw:
    """ccChar::Draw on a character laid out as chara.cpp's ccChar (DWARF):
    base +0, condition.dead +8, conditionNum +0x30, pos +0x40, hitAttribute
    +0x80, transparency +0x88, setTransparency +0x8c, transDist +0x90, the
    affect members +0xa6..+0xb8, anm +0xd4."""

    SYS, SCR, CH, BASE, ANM, ENV, GAME, CAM, WM = (0x01000000, 0x01010000, 0x01100000, 0x01100400, 0x01100800,
                                                   0x01100C00, 0x01101000, 0x01101400, 0x01101800)
    ACTIVE_CAM, GAME_P, DRAWENV, WORLDMAN, CCSYS, EFFECT_SW = (inf_va(0x0037896C), inf_va(0x003789CC), inf_va(0x003788C4), inf_va(0x00378A7C),
                                                               inf_va(0x003788E0), inf_va(0x00378CE4))

    def __init__(self):
        import test_anim
        from image import Program
        prog = Program(ELF, "gcmn")
        m = self.m = test_anim.machine_class()(prog)
        self.sym = lambda n: prog.symbol_named(n).value     # noqa: E731
        sym = self.sym
        m.store(self.CCSYS, 4, self.SYS)
        m.store(self.SYS + 604, 4, self.SCR)
        m.store(self.GAME_P, 4, self.GAME)
        m.store(self.ACTIVE_CAM, 4, self.CAM)
        m.store(self.DRAWENV, 4, self.ENV)
        m.store(self.WORLDMAN, 4, self.WM)
        self.ev = []
        me = self

        def w2p(mm, out, inp, *_):
            import eemu
            v = [mm.load(inp + 4 * k, 4) for k in range(4)]
            r = [eemu.f_sub(v[0], me.player[0]), eemu.f_sub(v[1], me.player[1]), v[2], ONE]
            for k in range(4):
                mm.store(out + 4 * k, 4, r[k])
            return 0

        def anm_draw(mm, anm, *_):
            e = me.ENV
            me.ev.append(("draw", mm.load(anm + 0x88, 4), mm.load(e + 0xA4, 1), mm.load(e + 0xA0, 4)))
            return 0

        def fog_blend(mm, env, *a):
            me.ev.append(("blend", mm.f[12], a[0] & 0xFFFFFF))
            mm.store(env + 0xB0, 4, mm.f[12])
            mm.store(env + 0xB4, 4, a[0] & 0xFFFFFF)
            return 0

        m.hooks[sym("ccTransPosW2P__FPfPf")] = w2p
        m.hooks[sym("Draw__5ccAnmFv")] = anm_draw
        m.hooks[sym("SetFogBlend__9ccDrawEnvFfUi")] = fog_blend
        m.hooks[sym("SetActiveLayer__9WORLD_MANFi")] = lambda mm, w, n, *a: me.ev.append(("layer", n)) or 0
        m.hooks[sym("SleepDistantLight__9WORLD_MANFv")] = lambda mm, *a: me.ev.append(("sleep",)) or 0
        m.hooks[sym("AwakeDistantLight__9WORLD_MANFv")] = lambda mm, *a: me.ev.append(("awake",)) or 0
        m.hooks[sym("checkCameraType__Fv")] = lambda mm, *a: 1 if me.eye else 3

    def run(self, c):
        m = self.m
        m.mem[self.CH:self.CH + 0x100] = bytes(0x100)
        m.mem[self.ENV:self.ENV + 0xE0] = bytes(0xE0)
        m.store(self.ENV + 0xA4, 1, 128)
        m.store(self.CH, 4, self.BASE)
        m.store(self.BASE + 0x18, 4, c["h"])
        m.store(self.BASE + 0x1C, 4, c["w"])
        m.store(self.CH + 8, 2, c["dead"] & 0xFFFF)
        m.store(self.CH + 0x30, 4, c["num"] & 0xFFFFFFFF)
        for k in range(3):
            m.store(self.CH + 0x40 + 4 * k, 4, c["pos"][k])
        m.store(self.CH + 0x4C, 4, ONE)
        m.store(self.CH + 0x80, 4, c["attr"])
        m.store(self.CH + 0x8C, 4, c["set"])
        m.store(self.CH + 0x90, 4, c["td"])
        m.store(self.CH + 0xA8, 2, c["cnt"] & 0xFFFF)
        m.store(self.CH + 0xAA, 2, c["rate"] & 0xFFFF)
        m.store(self.CH + 0xAC, 4, c["color"])
        m.store(self.CH + 0xB4, 2, c["ccnt"] & 0xFFFF)
        m.store(self.CH + 0xB6, 2, c["crate"] & 0xFFFF)
        m.store(self.CH + 0xB8, 4, c["ccolor"])
        m.store(self.CH + 0xD4, 4, self.ANM)
        m.store(self.GAME + 0x14, 4, 0 if c["town"] else 1)
        for k in range(3):
            m.store(self.CAM + 4 * k, 4, c["cam"][k])
        m.store(self.CAM + 12, 4, ONE)
        m.store(self.CAM + 0x5A, 2, c["deg1"] & 0xFFFF)
        m.store(self.EFFECT_SW, 1, c["sw"])
        self.player, self.eye = c["player"], c["eye"]
        self.ev = []
        drawn = m.call(self.sym("Draw__6ccCharFv"), (self.CH,), limit=5_000_000)
        s16 = lambda v: v - 0x10000 if v & 0x8000 else v    # noqa: E731
        blends = [e for e in self.ev if e[0] == "blend"]
        draws = [e for e in self.ev if e[0] == "draw"]
        out = {
            "blend": [blends[-1][1], blends[-1][2]] if blends else [0, 0],
            "transparency": m.load(self.CH + 0x88, 4),
            "drawn": int(bool(drawn)),
            "shaded": int(("sleep",) in self.ev),
            "affect": [s16(m.load(self.CH + 0xA8, 2)), s16(m.load(self.CH + 0xAA, 2)), m.load(self.CH + 0xAC, 4),
                       s16(m.load(self.CH + 0xB4, 2)), s16(m.load(self.CH + 0xB6, 2)), m.load(self.CH + 0xB8, 4)],
            "layers": [e[1] for e in self.ev if e[0] == "layer"],
        }
        if draws:
            out["shadow_alpha"], out["shadow_length"] = draws[0][2], draws[0][3]
            out["anm"] = draws[0][1]
        return out


def rnd_draw_case(rnd):
    player = [rnd.uniform(4000, 44000), rnd.uniform(4000, 44000), rnd.uniform(-100, 600)]
    pos = [player[0] + rnd.uniform(-9000, 9000), player[1] + rnd.uniform(-9000, 9000), rnd.uniform(-100, 600)]
    if rnd.random() < 0.4:
        cam = [pos[0] + rnd.uniform(-300, 300), pos[1] + rnd.uniform(-300, 300), pos[2] + rnd.uniform(-50, 400)]
    else:
        cam = [player[0] + rnd.uniform(-1500, 1500), player[1] + rnd.uniform(-1500, 1500),
               player[2] + rnd.uniform(0, 900)]
    h, w = rnd.choice(((180.0, 60.0), (350.0, 100.0), (200.0, 60.0), (350.0, 60.0),
                       (rnd.uniform(0, 500), rnd.uniform(0, 200))))
    return {
        "dead": rnd.choice((0, 0, 0, 1, 2, 3, 4, 5, rnd.randrange(-3, 9))),
        "num": rnd.choice((-1, -1, 0, 1, 2, 3, 4, 5, 6, 7, rnd.randrange(-5, 20))),
        "sw": rnd.choice((0, 1, 1)),
        # The flash's and the tint's counters round their edges (0, 60).
        "cnt": rnd.choice((-4, -8, -2, -1, 1, 2, rnd.randrange(-40, 40))),
        "rate": rnd.choice((0, 0, 1, 2, 4, 8, rnd.randrange(-20, 90), rnd.randrange(1, 12))),
        "color": rnd.getrandbits(24),
        "ccnt": rnd.choice((12, -2, -1, 1, 2, 0, rnd.randrange(-30, 30))),
        "crate": rnd.choice((0, 1, 2, 12, 48, 49, 50, 58, 59, 60, 61, 62, rnd.randrange(-20, 80))),
        "ccolor": rnd.choice((0, rnd.getrandbits(24))),
        "pos": [fb(v) for v in pos], "h": fb(h), "w": fb(w),
        "set": rnd.choice((ONE, ONE, fb(rnd.random()), 0, fb(0.04))),
        "td": rnd.choice((1, 1, 1, 1, 0)),
        "attr": rnd.choice((0, 0x40000, rnd.getrandbits(32), rnd.getrandbits(32) & ~0x40000)),
        "player": [fb(v) for v in player], "cam": [fb(v) for v in cam],
        "deg1": rnd.choice((1512, 7301, rnd.randrange(-2000, 9000), rnd.randrange(0, 8192))),
        "eye": rnd.random() < 0.1, "town": rnd.random() < 0.2,
    }


def draw_line(c):
    return ("draw " + hexs(c["dead"], c["num"], c["sw"], c["cnt"], c["rate"], c["color"], c["ccnt"], c["crate"],
                           c["ccolor"], *c["pos"], c["w"], c["h"], c["set"], c["td"], c["attr"], *c["player"],
                           *c["cam"], c["deg1"], int(c["eye"]), int(c["town"])))


def run_draw(n, seed=13):
    rnd = random.Random(seed)
    cd = CharDraw()
    cases = [rnd_draw_case(rnd) for _ in range(n)]
    games = [cd.run(c) for c in cases]
    port = ask([draw_line(c) for c in cases])
    bad, first, drawn, blended = 0, None, 0, 0
    for c, g, p in zip(cases, games, port):
        want = {k: g[k] for k in ("blend", "transparency", "drawn", "shaded", "affect")}
        got = {k: p[k] for k in want}
        if g["drawn"]:
            drawn += 1
            want["shadow"] = [g["shadow_alpha"], g["shadow_length"]]
            got["shadow"] = [p["shadow_alpha"], p["shadow_length"]]
            if g["anm"] != g["transparency"]:
                want["anm"] = g["anm"]
                got["anm"] = p["transparency"]
        if g["blend"][0]:
            blended += 1
        if g["layers"] != [5, 0]:
            want["layers"], got["layers"] = g["layers"], [5, 0]
        if want != got:
            bad += 1
            first = first or f"{draw_line(c)}\n game {want}\n port {got}"
    return {"cases": n, "drawn": drawn, "blended": blended, "bad": bad, "first": first}


@unittest.skipUnless(READY, "needs the extracted disc and cargo")
class FoeAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def test_looks(self):
        rows, game, bad = check_looks()
        for r, g in zip(rows, game):
            print(f"look {r}: {g['file']} {g['clump']} swaps {g['swaps']} anim {g['anim']} second {g['second']} "
                  f"{g['anim2']}")
        self.assertEqual(bad, [], bad[:2])

    def test_pose(self):
        t, unposed = run_pose(140)
        print(f"pose: {t['cases']} frames, {t['nodes']} nodes, worst rotation {t['worst'][0]:.3g} position "
              f"{t['worst'][1]:.3g} alpha {t['worst'][2]:.3g}, {t['bad']} mismatches")
        # The one node with a model that no animation of its file drives:
        # the Sword of Chaos's OBJ_trall (MDL_trall, rigid and unlit), which
        # stays at the anm's matrix (not checked here).
        missing = {k[0]: v for k, v in unposed.items() if v}
        self.assertEqual(missing, {"eks1": ["OBJ_trall"]})
        self.assertEqual(t["bad"], 0, t["first"])

    def test_circle(self):
        t = run_circle(120)
        print(f"circle: {t['cases']} frames, {t['nodes']} nodes, worst rotation {t['worst'][0]:.3g} position "
              f"{t['worst'][1]:.3g} alpha {t['worst'][2]:.3g}, {t['bad']} mismatches, {t['root_bad']} roots differ")
        self.assertEqual((t["bad"], t["root_bad"]), (0, 0), t["first"])

    def test_draw(self):
        t = run_draw(1000)
        print(f"draw: {t['cases']} cases ({t['drawn']} drawn, {t['blended']} blended), {t['bad']} mismatches")
        self.assertEqual(t["bad"], 0, t["first"])


def bulk(n):
    build()
    rows, game, bad = check_looks()
    print(f"looks: {len(rows)} rows, {len(bad)} mismatches" + (f"; first {bad[0]}" if bad else ""))
    t, unposed = run_pose(n)
    missing = {k: v for k, v in unposed.items() if v}
    print(f"pose: {t['cases']} frames, {t['nodes']} nodes, worst {t['worst']}, {t['bad']} mismatches"
          + (f"\n  {t['first']}" if t["first"] else "") + (f"\n  nodes no animation drives: {missing}" if missing
                                                           else ""))
    t = run_circle(n)
    print(f"circle: {t['cases']} frames, {t['nodes']} nodes, worst {t['worst']}, {t['bad']} mismatches, "
          f"{t['root_bad']} roots" + (f"\n  {t['first']}" if t["first"] else ""))
    t = run_draw(n)
    print(f"draw: {t['cases']} cases ({t['drawn']} drawn, {t['blended']} blended), {t['bad']} mismatches"
          + (f"\n  {t['first']}" if t["first"] else ""))


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "bulk":
        bulk(int(sys.argv[2]) if len(sys.argv) > 2 else 500)
    else:
        unittest.main()
