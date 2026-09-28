#!/usr/bin/env python3
"""tools/dungeon.py against the game's own DUNGEON::Generate.

The real Generate (INF gcmn.prg 0x005c12a0) runs in tools/eemu.py over a
scratch DUNGEON, WORLD_MAN and ccGame, with the global RNG seeded as
WORLD_MAN::GO seeds it. Everything that only loads or draws is stubbed:
operator new is a bump allocator, ccStream::GetChunkAdrsF hands back a token
for the model name, ccAnm::SetAnm records which model a room got,
SetRoom/DeleteRoom and the VU0 maths do nothing. ccAnm::GetSubstAdrs, the one
stub that feeds the generator, answers from dungeon.Data.gim_counts - the
dummy objects read from the room models in DATA.BIN - so what is checked is
every decision the game code makes given those counts. Every field Generate
writes is compared with dungeon.snapshot(). Infection in full; the other
three volumes, when extracted with a .syms sidecar, each against its own
executable and DATA.BIN. Skipped when the game files are not present.
"""

import os
import random
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
DATA_BIN = volume.DATA
OTHERS = [os.path.join(ROOT, "work", v, "disc", e) for v, e in
          (("mutation", "SLUS_205.62"), ("outbreak", "SLUS_205.63"),
           ("quarantine", "SLUS_205.64"))]

DG, GIMPOS, WM, GAME, SAVE = 0x01000000, 0x01100000, 0x01200000, 0x01202000, 0x01300000
HEAP, HEAP_END = 0x01400000, 0x01800000
ARRAYS = 0x01800000         # the arrays the later volumes' DUNGEON allocates

# DUNGEON's layout, by the executable's volumeNum. "inline" arrays sit in
# DUNGEON at that offset; "pointer" ones are allocated by the constructor,
# (offset of the pointer, bytes per floor); "startpos" is the (up/down,
# floor) stride of float startpos[..][..][4].
LAYOUTS = {
    # Infection: the DWARF of dungeon.cpp (docs/engine/dungeon.md).
    1: {"size": 0xD3910, "floors": None,
        "inline": {"realmap": 0x430, "room": 0x2F230, "rotate": 0x2F488, "pos": 0x2F6E0,
                   "animidx": 0x30040, "startpos": 0x30590, "minimap": 0x306D0,
                   "roomdata": 0x30894, "uproom": 0xD3538, "downroom": 0xD3560},
        "pointer": {}, "startpos": (0xA0, 16)},
}
# Mutation, Outbreak and Quarantine: read from each one's code, the same in
# all three (MUT/OUT/QUA addresses).
LATER = {
    # WORLD_MAN::GO: operator new(3296) before DUNGEON::DUNGEON
    # (0x001b5c34 / 0x001abc58 / 0x001b3468).
    "size": 0xCE0,
    # DUNGEON::DUNGEON (0x005e31e0 / 0x005e1950 / 0x004d43b0) stores 10, or 15
    # for event area 125; Generate reads it as its floor count
    # (0x005ec2e4 / 0x005eadf4 / 0x004dd854).
    "floors": 0x430,
    # ccAnm *room[15][15]: MakeRoom's `sw $v0, 1080($v1)` (0x005e580c /
    # 0x005e40f4 / 0x004d6b54); UpRoom[15] and DownRoom[15]: MakeFloor's
    # `sw 2232` / `sw 2292` (0x005eb260, 0x005eb2cc / 0x005e9d10, 0x005e9d80 /
    # 0x004dc770, 0x004dc7e0) and the constructor's loop setting both to 15.
    "inline": {"room": 0x438, "uproom": 0x8B8, "downroom": 0x8F4},
    # The constructor's new[] calls, in order, each size a multiple of the
    # floor count at +0x430 (0x005e3258-0x005e3340 / 0x005e19c0-0x005e1aa8 /
    # 0x004d4420-0x004d4508); what each holds from its use in MakeFloor,
    # MakeRoom, MakeRealMap and SetAllGim.
    "pointer": {"realmap": (0x434, 0x4B00), "animidx": (0x7BC, 120), "minimap": (0x7C0, 45),
                "roomdata": (0x7C4, 15 * 76), "smallmap": (0x7C8, 0x10000),
                "startpos": (0x930, 32), "rotate": (0x7CC, 60), "pos": (0x7D0, 240)},
    # startpos[floors][2][4]: MakeRoom's `sll 5` per floor, +16 for the down stairs.
    "startpos": (16, 32),
}
LAYOUTS.update({2: LATER, 3: LATER, 4: LATER})


class GameDungeon:
    """DUNGEON::Generate run in the interpreter, read back like snapshot()."""

    def __init__(self, data):
        import eemu
        self.eemu = eemu
        self.data = data
        self.prog = data.p
        self.layout = LAYOUTS[data.areas.volume]
        self.m = eemu.Machine(self.prog)
        sym = self.prog.symbol_named
        self.sym = lambda n: sym(n).value        # noqa: E731
        self.table_at = {va: name for name, (va, _n) in data.table_va.items()}
        m = self.m
        stubs = {
            "__nw__FUi": self.new, "__nwa__FUi": self.new,
            "__dl__FPv": self.nop, "__dla__FPv": self.nop,
            "__ct__5ccAnmFv": lambda m, a0, *_: a0,
            "GetChunkAdrsF__8ccStreamFPCci": self.get_chunk,
            "SetAnm__5ccAnmFP10ccAnmChunkUi": self.set_anm,
            "_AnimateForward__5ccAnmFUi": self.nop,
            "SetMatrix_PosRotZYX__7ccCoordFPfPf": self.nop,
            "_SetLWMatrix__7ccCoordFv": self.nop,
            "GetSubstAdrs__5ccAnmFPCcP19ccSubstSearchResult": self.get_subst,
            "GetSubstAdrsF__5ccAnmFPCcb": self.get_subst_f,
            "sceVu0ApplyMatrix": self.nop, "sceVu0AddVector": self.nop,
            "sceVu0SubVector": self.nop, "atan2": self.nop, "atan2f": self.nop,
            "fptodp": self.nop, "dptofp": self.nop,
            "DeleteRoom__7DUNGEONFii": self.nop, "SetRoom__7DUNGEONFii": self.nop,
        }
        for name, fn in stubs.items():
            m.hooks[self.sym(name)] = fn

    # stubs ---------------------------------------------------------------
    @staticmethod
    def nop(m, *_):
        return 0

    def new(self, m, n, *_):
        a = self.heap
        self.heap = (a + n + 15) & ~15
        if self.heap > HEAP_END:
            raise self.eemu.Stop("scratch heap exhausted")
        m.mem[a:a + n] = bytes(n)
        return a

    def get_chunk(self, m, ccs, name, *_):
        token = self.new(m, 16)
        self.chunks[token] = m.mem[name:m.mem.index(0, name)].decode()
        return token

    def set_anm(self, m, anm, chunk, *_):
        m.store(anm + 0x94, 4, chunk)           # ccAnm.anmChunk
        return 0

    def get_subst(self, m, anm, pattern, result, *_):
        pat = m.mem[pattern:m.mem.index(0, pattern)].decode()
        model = self.chunks[m.load(anm + 0x94, 4)]
        counts = self.counts.get(model, [0] * 8)
        for (name, wild, _), n in zip(self.gim_patterns, counts):
            if (pat == name + "*") if wild else (pat == name):
                tbl = m.load(result, 4)
                num = m.load(result + 4, 2)
                for _ in range(n):
                    m.store(tbl + 8 * num, 4, self.new(m, 0x90))
                    m.store(tbl + 8 * num + 4, 2, 0x0100)
                    num += 1
                m.store(result + 4, 2, num)
        return 0

    def get_subst_f(self, m, *_):
        return self.new(m, 0x90)

    # run -----------------------------------------------------------------
    def reset(self, floors=10):
        """A zeroed DUNGEON with its arrays, as far as Generate reads them;
        self.at maps each array to its address."""
        m, lay = self.m, self.layout
        m.mem[DG:DG + lay["size"]] = bytes(lay["size"])
        self.at = {k: DG + off for k, off in lay["inline"].items()}
        a = ARRAYS
        for k, (off, per) in lay["pointer"].items():
            m.mem[a:a + per * floors] = bytes(per * floors)
            m.store(DG + off, 4, a)
            self.at[k] = a
            a = (a + per * floors + 15) & ~15
        if lay["floors"] is not None:
            m.store(DG + lay["floors"], 4, floors)

    def run(self, seed, dtype, level_max, room_max, server=0, volume=1, word_a=0, field_type=0):
        import dungeon
        self.gim_patterns = dungeon.GIM_PATTERNS
        self.counts = self.data.gim_counts(self.data.ccs_name(dtype))
        m = self.m
        self.heap = HEAP
        self.chunks = {}
        self.reset()
        m.mem[GIMPOS:GIMPOS + 750 * 0x30] = bytes(750 * 0x30)
        for i in range(750):
            m.store(GIMPOS + 0x30 * i + 0x22, 1, 0xFF)
        for i in range(10):
            m.store(self.at["uproom"] + 4 * i, 4, 15)
            m.store(self.at["downroom"] + 4 * i, 4, 15)
        m.store(DG + 0x10, 4, dtype)
        m.store(DG + 0x4c, 4, 1)
        m.store(DG + 0x424, 4, GIMPOS)
        m.mem[WM:WM + 0x4e0] = bytes(0x4e0)
        m.store(WM + 0x138, 4, level_max)
        m.store(WM + 0x13c, 4, room_max)
        m.store(WM + 0x14c, 4, word_a)
        m.store(WM + 0x10, 4, field_type)
        m.store(GAME + 0x1c, 4, server)
        m.store(self.sym("worldman"), 4, WM)
        m.store(self.sym("game"), 4, GAME)
        m.store(self.sym("saveData"), 4, SAVE)
        m.store(self.sym("volumeNum"), 4, volume)
        m.store(self.sym("seed"), 4, seed)
        m.store(self.sym("randcnt"), 4, 0)
        m.call(self.sym("Generate__7DUNGEONFv"), (DG,), limit=200_000_000)
        return self.read(dtype, m.load(WM + 0x138, 4))

    def real_maps(self, rooms, floors=10):
        """DUNGEON::MakeRealMap for every floor over the given ROOMDATA rows
        (raw 76-byte records), after the constructor's initialisation: every
        row of roomdata[15 * floors] on floor `floors`, every cell
        {0, 15, 15}."""
        m = self.m
        self.reset(floors)
        rd, grid = self.at["roomdata"], self.at["realmap"]
        for i in range(15 * floors):
            m.store(rd + 76 * i, 4, floors)
        for i, raw in enumerate(rooms):
            m.mem[rd + 76 * i:rd + 76 * i + 76] = raw
        m.mem[grid:grid + floors * 0x4b00] = bytes((0, 15, 15)) * (floors * 6400)
        for lv in range(floors):
            m.store(DG + 4, 4, lv)
            m.call(self.sym("MakeRealMap__7DUNGEONFv"), (DG,))
        return [bytes(m.mem[grid + 0x4b00 * lv:grid + 0x4b00 * (lv + 1)]) for lv in range(floors)]

    def read(self, dtype, level_max):
        m, at = self.m, self.at
        per_side, per_floor = self.layout["startpos"]
        levels = 1 if dtype in (8, 9) else level_max
        floors = []
        for lv in range(levels):
            grid = bytes(m.mem[at["realmap"] + 0x4b00 * lv:at["realmap"] + 0x4b00 * (lv + 1)])
            rooms = []
            for i in range(15):
                slot = lv * 15 + i
                anm = m.load(at["room"] + 4 * slot, 4)
                k, r, size, _pad, info = struct.unpack(
                    "<bbBBI", bytes(m.mem[at["animidx"] + 8 * slot:at["animidx"] + 8 * slot + 8]))
                mm = bytes(m.mem[at["minimap"] + 3 * slot:at["minimap"] + 3 * slot + 3])
                if not anm:
                    rooms.append((k, r, size, None, 0, 0, 0, mm[0], mm[1], None))
                    continue
                rooms.append((k, r, size, self.table_at.get(info, hex(info)),
                              m.load(at["rotate"] + 4 * slot, 4), m.load(at["pos"] + 16 * slot, 4),
                              m.load(at["pos"] + 16 * slot + 4, 4), mm[0], mm[1],
                              self.chunks[m.load(anm + 0x94, 4)]))
                if mm[2]:
                    rooms[-1] += ("minimap flag", mm[2])
            start = tuple(m.load(at["startpos"] + per_side * s + per_floor * lv + 12, 4)
                          for s in (0, 1))
            floors.append({"map": grid, "up": m.load(at["uproom"] + 4 * lv, 4),
                           "down": m.load(at["downroom"] + 4 * lv, 4), "rooms": rooms,
                           "start_w": start})
        gims = []
        for i in range(750):
            e = GIMPOS + 0x30 * i
            if m.load(e + 0x22, 1) == 0xFF:
                break
            gims.append((m.load(e + 0x20, 1), m.load(e + 0x21, 1), m.load(e + 0x22, 1)))
        return {"floors": floors,
                "sym": tuple(m.load(DG + o, 4) for o in (0x30, 0x34, 0x38, 0x44)),
                "seed": m.load(self.sym("seed"), 4), "randcnt": m.load(self.sym("randcnt"), 4),
                "gims": gims, "floorRoomNum": m.load(DG + 0x18, 4)}


def compare(py, game):
    """[(where, python, game)] for every difference."""
    out = []
    for k in ("sym", "seed", "randcnt", "gims", "floorRoomNum"):
        if py[k] != game[k]:
            out.append((k, py[k], game[k]))
    if len(py["floors"]) != len(game["floors"]):
        out.append(("floors", len(py["floors"]), len(game["floors"])))
    for lv, (a, b) in enumerate(zip(py["floors"], game["floors"])):
        for k in ("up", "down", "start_w"):
            if a[k] != b[k]:
                out.append((f"floor {lv} {k}", a[k], b[k]))
        if a["map"] != b["map"]:
            diff = [i for i in range(len(a["map"])) if a["map"][i] != b["map"][i]]
            i = diff[0]
            out.append((f"floor {lv} map, {len(diff)} bytes, first at x={i // 240} "
                        f"y={i % 240 // 3} field {i % 3}", a["map"][i], b["map"][i]))
        for i, (ra, rb) in enumerate(zip(a["rooms"], b["rooms"])):
            if ra != rb:
                out.append((f"floor {lv} room {i}", ra, rb))
    return out


def check(test, dungeon, data, game, seed, dtype, level_max, room_max, server=0, volume=1,
          word_a=0, field_type=0):
    """One random dungeon, Python against the game."""
    py = dungeon.snapshot(dungeon.Generator(data, seed, dtype, level_max, room_max, server,
                                            volume, word_a, field_type=field_type).generate())
    got = game.run(seed, dtype, level_max, room_max, server, volume, word_a, field_type)
    test.assertEqual(compare(py, got), [], f"{data.elf_path}: seed {seed} type {dtype} "
                                           f"levels {level_max} rooms {room_max} "
                                           f"server {server} volume {volume}")


def check_edited(test, dungeon, data, game, slots):
    """MakeRealMap over the given EditDungeon entries against dungeon.Edited."""
    p = data.p
    base = p.symbol_named("EditDungeon").value
    for i in slots:
        ev, idx, rooms, n = struct.unpack("<4i", p.read(base + 24 * i, 16))
        ed = dungeon.Edited(data, ev, idx)
        got = game.real_maps([p.read(rooms + 76 * k, 76) for k in range(n)], ed.floor_count)
        for lv in range(ed.floor_count):
            fl = ed.make_real_map(lv)
            grid = b"".join(bytes((fl.d[c], fl.next[c], fl.here[c])) for c in range(6400))
            test.assertEqual(grid, got[lv], f"{data.elf_path}: EditDungeon[{i}] event {ev} "
                                            f"floor {lv}")


def check_types(test, data, path, events, flag71s=(False,)):
    """dungeon.Data.dungeon_type against WORLD_MAN::SetDungeonTypeFromField,
    with EVENTAREA_INFO.flag 0 and 3 and the save flag the function reads
    (Data.type_rule) clear and set. Where the game reads a substitute record
    (areas 71 and 47, see tools/areas.py) the flag it sees is the
    substitute's, not the one poked into the table."""
    import eemu
    from image import Program
    prog = Program(path)
    m = eemu.Machine(prog)
    info = prog.symbol_named("eventAreaInfo").value
    fn = prog.symbol_named("SetDungeonTypeFromField__9WORLD_MANFv").value
    m.store(prog.symbol_named("saveData").value, 4, SAVE)
    _rule, offset, bit = data.type_rule
    for ev in events:
        rec = next((i for i in range(len(data.areas.events)) if m.load(info + 0x54 * i, 4) == ev),
                   None)
        for flag71 in flag71s:
            m.store(SAVE + 0x5bc0, 8, 1 << 62 if flag71 else 0)
            seen = data.areas.event_info(ev, data.areas.volume, flag71)
            substitute = seen is not None and seen is data.areas.substitutes.get(ev)
            for flag in (0, 3):
                for save in (0, 1):
                    if rec is not None:
                        m.store(info + 0x54 * rec + 0x24, 4, flag)
                    if bit is None:
                        m.store(SAVE + offset, 1, save)
                    else:
                        m.store(SAVE + offset, 8, save << bit)
                    for ft in range(12):
                        m.mem[WM:WM + 0x4e0] = bytes(0x4e0)
                        m.store(WM + 0x120, 4, ev)
                        m.store(WM + 0x10, 4, ft)
                        m.call(fn, (WM,))
                        got = [m.load(WM + 0x34, 4), m.load(WM + 0x38, 4)]
                        want = data.dungeon_type(ft, ev, seen["flag"] if substitute else flag,
                                                 bool(save))
                        test.assertEqual(want, got, (path, ev, flag71, flag, save, ft))


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(DATA_BIN), "game files not present")
class TestDungeon(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import dungeon
        cls.dungeon = dungeon
        cls.data = dungeon.Data(ELF)
        cls.game = GameDungeon(cls.data)

    def check(self, seed, dtype, level_max, room_max, server=0, volume=1, word_a=0, field_type=0):
        check(self, self.dungeon, self.data, self.game, seed, dtype, level_max, room_max, server,
              volume, word_a, field_type)

    def test_random_dungeons_match_game(self):
        rnd = random.Random(11)
        for _ in range(12):
            size = rnd.randrange(1, 11)
            level_max, room_max = self.data.areas.dungeon[size]
            self.check(rnd.getrandbits(32), rnd.randrange(10), level_max, room_max)

    def test_servers_volumes_match_game(self):
        rnd = random.Random(12)
        for server, volume in ((2, 1), (3, 2), (0, 2), (4, 4)):
            self.check(rnd.getrandbits(32), rnd.randrange(8), 3, 7, server, volume)

    def test_edited_maps_match_game(self):
        """MakeRealMap over every EditDungeon entry against dungeon.Edited."""
        check_edited(self, self.dungeon, self.data, self.game, range(88))

    def test_dungeon_types_match_game(self):
        """dungeon.dungeon_type against WORLD_MAN::SetDungeonTypeFromField."""
        check_types(self, self.data, ELF, [0, 120, 66, 91, 16, 18, 21, 14, 17, 47])

    def test_first_story_dungeon(self):
        ed = self.dungeon.Edited(self.data, 14, 0)
        self.assertEqual(ed.name, "D0001_room")
        self.assertEqual([len(f.rooms) for f in ed.floors], [5, 5])
        self.assertEqual((ed.floors[0].up, ed.floors[0].down), (0, 4))

    def test_word_131_match_game(self):
        self.check(0x1234567, 1, 2, 5, word_a=131)
        self.check(0x7654321, 8, 2, 5, word_a=131)

    def test_lake_start_match_game(self):
        """Field type 4 leads straight into a lake dungeon (types 8, 9),
        whose first room faces the player in."""
        self.check(0x2468ACE, 8, 3, 7, field_type=4)
        self.check(0x1357BDF, 9, 3, 7, field_type=4)

    def test_read_from_the_code(self):
        """What dungeon.Data reads from the code is what Infection's
        symbols say: every room table's address and length, symroom,
        EditDungeon's 88 entries, ten floors, the byte at saveData+0x6772."""
        p = self.data.p
        for name, (va, rows) in self.data.table_va.items():
            sym = p.symbol_named(name)
            self.assertEqual((va, rows), (sym.value, sym.size // 16), name)
        self.assertEqual(len(self.data.table_va), 31)
        self.assertEqual(self.data.symroom_va, inf_va(0x00696550))
        self.assertEqual(self.data.edit_count, p.symbol_named("EditDungeon").size // 24)
        self.assertEqual(self.data.edit_floors, (10, {}))
        self.assertEqual(self.data.type_rule, ("alt", 0x6772, None))


class TestOtherVolumes(unittest.TestCase):
    """Mutation, Outbreak and Quarantine, each against its own Generate,
    MakeRealMap and SetDungeonTypeFromField. From Mutation on DUNGEON
    allocates its per-floor arrays (LATER), EditDungeon has 90 entries,
    event area 125's dungeon has 15 floors, and the save flag that
    SetDungeonTypeFromField reads moved and changed meaning."""

    def test_volumes(self):
        import dungeon
        present = [p for p in OTHERS if os.path.exists(p) and os.path.exists(p + ".syms")]
        if not present:
            self.skipTest("no other volume extracted with a .syms sidecar")
        for path in present:
            with self.subTest(path=path):
                data = dungeon.Data(path)
                vol = OTHERS.index(path) + 2
                self.assertEqual(data.areas.volume, vol)
                self.assertEqual(len(data.table_va), 31)
                self.assertEqual((data.edit_count, data.edit_floors, data.type_rule),
                                 (90, (10, {125: 15}), ("plain", 0x5EC8, 62)))
                game = GameDungeon(data)
                rnd = random.Random(20 + vol)
                for server, volume in ((rnd.randrange(2), vol),
                                       (rnd.randrange(2, 5), 2 + (vol - 1) % 3)):
                    size = rnd.randrange(1, 11)
                    level_max, room_max = data.areas.dungeon[size]
                    check(self, dungeon, data, game, rnd.getrandbits(32), rnd.randrange(10),
                          level_max, room_max, server, volume)
                check(self, dungeon, data, game, rnd.getrandbits(32), rnd.randrange(8), 2, 5,
                      rnd.randrange(5), vol, word_a=131)
                check(self, dungeon, data, game, rnd.getrandbits(32), 8 + vol % 2, 3, 7,
                      rnd.randrange(5), vol, field_type=4)
                base = data.p.symbol_named("EditDungeon").value
                special = [i for i in range(data.edit_count)
                           if data.p.u32(base + 24 * i) in data.edit_floors[1]]
                slots = sorted(set(range(vol - 2, data.edit_count, 7)) | set(special)
                               | {data.edit_count - 2, data.edit_count - 1})
                check_edited(self, dungeon, data, game, slots)
                check_types(self, data, path, [0, 120, 66, 91, 16, 18, 21, 14, 17, 47, 71, -1],
                            (False, True))


if __name__ == "__main__":
    unittest.main()
