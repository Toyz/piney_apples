#!/usr/bin/env python3
"""Dungeon layouts - how DUNGEON::Generate builds a dungeon's floors.

    tools/dungeon.py gen  ELF A B C [--server N] [--volume N] [--index N]
                          [--save-flag] [--flag71] [--rooms] [--gims]
                                                  the dungeon three words make
    tools/dungeon.py seed ELF SEED TYPE LEVELS ROOMS [--server N] [--volume N]
                          [--word-a ID] [--rooms] [--gims]
                                                  a random dungeon from raw inputs
    tools/dungeon.py edit ELF EVENT [--index N]   a story area's edited dungeon
    tools/dungeon.py types ELF                    dungeon type per field type

Words go through tools/areas.py (WORLD_MAN::SimGenerateCode). A random
dungeon is then fully determined by dungeonSeed[index], the dungeon type
(WORLD_MAN::SetDungeonTypeFromField, 0x0019cf50), levelMax and roomMax from
dungeonData, the server and the volume - plus, because each room's dummy
objects draw from the same RNG, the room models in the dungeon's CCS file,
which are read from DATA.BIN next to the ELF. --volume defaults to the
executable's own volumeNum.

Any of the four executables works (the stripped ones through the names
piney-gen syms carries); what differs between them is read from each one's
code (see Data).
The addresses below are Infection's.

The generator (gcmn.prg, dungeon.cpp), in call order:

  WORLD_MAN::GO           0x0019f8e0  seed = dungeonSeed[game.dungeon], randcnt = 0
  DUNGEON::Generate       0x005c12a0  per floor: floorRoomNum = fieldrand(5) + roomMax
  DUNGEON::MakeFloor      0x005c0110  grow the room graph, pick the down stairs,
                                      then MakeRoom for every room
  FOOT::FOOT (first)      0x005b6840  room 0 at the map centre, one exit south
  FOOT::FOOT (next)       0x005b6ce0  a room behind an exit, 1-3 new exits
  FOOT::Move              0x005b7330  does the room behind exit k fit? mark the door
  FOOT::CheckDirection    0x005b78c0  which doors meet a neighbour's door
  ChooseRoomSize          0x005b6790  small/medium/large, 20/50/30 (vol. 1, servers 0-1)
  DUNGEON::MakeRoom       0x005ba1d0  model (by exits), Gott statue room, minimap
  DUNGEON::SetAllGim      0x005bb340  dummy objects -> gimmick slots (item boxes 80%)
  DUNGEON::CheckEntryItemBox 0x005cf800

Map cells are MAP_INFO {d, next, here}: here is the room number (15 = none),
d marks a door cell with the direction it faces (1 north/-y, 2 south/+y,
4 west/-x, 8 east/+x), next the room through that door. A room is 4, 8 or 16
cells square; one cell is 750 units.
"""

import argparse
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import areas  # noqa: E402
import eemu  # noqa: E402
import xfer  # noqa: E402
from image import Program  # noqa: E402

MAP = 80
NONE = 15
EMPTY_EXIT = 128
DIREC = (1, 2, 4, 8)            # exit k -> direction bit; MakeFloor's direc[4]
RDIREC = (2, 1, 8, 4)           # the way back; MakeFloor's rDirec[4]
SIZES = {0: (1, 4), 1: (3, 8), 2: (7, 16)}      # room size -> (door offset, cells)
SIZE_NAMES = ("small", "medium", "large")
CELL = eemu.f_from_py(750.0)
ONE = eemu.f_from_py(1.0)
PI = 0x40490FDB
HALF_PI = 0x3FC90FDB
# MakeRoom: a room turned by rotate faces the player in by this.
FACING = {0: PI, PI: 0, HALF_PI: HALF_PI | 0x80000000, HALF_PI | 0x80000000: HALF_PI}
LAKE_TYPES = (8, 9)
# DUNGEON type -> room table suffix and DungeonName index (MakeFloor's jump
# tables 0x006f8900.. and the DUNGEON constructor).
TABLE_SUFFIX = ("A", "B", "C", "4", "E0", "E1", "E2", "E4", "X", "X")
# SetAllGim's searches, in order, with the gimmick type each adds.
GIM_PATTERNS = (("OBJ_0ppi", True, 0), ("OBJ_0ppm", True, 1), ("OBJ_0ppg", False, 2),
                ("OBJ_o_fountain_l0_", False, 2), ("OBJ_0ps0", True, 3),
                ("OBJ_0ps1", True, 4), ("OBJ_0ps2", True, 5), ("OBJ_0ps3", True, 6))
GIM_NAMES = {0: "item box", 1: "magic circle", 2: "ppg/fountain", 3: "ps0", 4: "ps1",
             5: "ps2", 6: "ps3"}


class Data:
    """The gcmn tables the generator reads, and the dungeon CCS files.

    Everything whose address, length or meaning is not the same in every
    executable is read from that executable's code: the room tables and
    their row counts from MakeFloor's jump tables, symroom from MakeRoom,
    the EditDungeon count and the story dungeons' floor count from the
    DUNGEON constructor, the save flag's role from
    WORLD_MAN::SetDungeonTypeFromField."""

    def __init__(self, elf_path):
        self.elf_path = elf_path
        self.areas = areas.Data(elf_path)
        p = self.p = Program(elf_path, "gcmn")
        self.jump, self.table_va = self._room_tables()
        self.tables = {}
        for name, (va, count) in self.table_va.items():
            rows = []
            for k in range(count):
                objs, num, exit_, r = struct.unpack("<IIB3xI", p.read(va + 16 * k, 16))
                names = [p.cstr(p.u32(objs + 4 * j)).decode() for j in range(num)]
                rows.append({"exit": exit_, "num": num, "r": r, "models": names})
            self.tables[name] = rows
        # MakeRoom's local symroom[10]: the Gott statue room per type.
        self.symroom_va = self._symroom()
        self.symroom = [p.cstr(p.u32(self.symroom_va + 4 * i)).decode() for i in range(10)]
        self.dungeon_name = [self._name_list("DungeonName", i) for i in range(10)]
        self.dungeon_name2 = [self._name_list("DungeonName2", i) for i in range(10)]
        self.edit_count = self._edit_count()
        self.edit_floors = self._edit_floors()
        self.type_rule = self._type_rule()
        self._counts = {}

    # reading the code ---------------------------------------------------------
    def _code(self, name):
        """(va, words, {index: address formed}) of one function."""
        sym = self.p.symbol_named(name)
        f = self.p.function_at(sym.value)
        size = f.size if f is not None and f.value == sym.value and f.size else sym.size
        words = struct.unpack(f"<{size // 4}I", self.p.read(sym.value, size // 4 * 4))
        return sym.value, words, xfer.addresses(words, sym.value, self.p.gp)

    def _room_tables(self):
        """MakeFloor's random branch picks a ROOM_INFO table through one of
        four jump tables indexed by dungeon type (`sltiu $at, $v1, 10`):
        small, medium, large, and the large rooms' second set. Each case
        passes the table in $a3 and its row count in $t0 to
        DUNGEON::MakeRoom(int, FOOT *, ROOM_INFO *, int). Returns the
        jump tables as [4][10] table names and {name: (va, rows)}; a table
        is named by its first slot (sroom_infoA ... lroom_infoA_2), as
        Infection's symbols name them."""
        va, words, addrs = self._code("MakeFloor__7DUNGEONFv")
        make_room = self.p.symbol_named("MakeRoom__7DUNGEONFiP4FOOTP9ROOM_INFOi").value
        jumps = []
        for i, w in enumerate(words):
            if w >> 26 == 0x0B and w & 0xFFFF == 10:
                jumps.append(next(addrs[j] for j in range(i + 1, i + 8) if j in addrs))
        if len(jumps) != 4:
            raise ValueError(f"MakeFloor: {len(jumps)} room-table jump tables, expected 4")
        names, table_va = [], {}
        by_va = {}
        for n, jt in enumerate(jumps):
            row = []
            for dtype in range(10):
                k = (self.p.u32(jt + 4 * dtype) - va) // 4
                table = count = None
                while True:
                    w = words[k]
                    op, rs, rt = w >> 26, (w >> 21) & 31, (w >> 16) & 31
                    if k in addrs and rt == 7:
                        table = addrs[k]
                    if op == 0x09 and rs == 0 and rt == 8:
                        count = w & 0xFFFF
                    if op == 3:
                        if ((va + 4 * k + 4) & 0xF0000000) | ((w & 0x3FFFFFF) << 2) != make_room:
                            raise ValueError(f"MakeFloor: case at {va + 4 * k:#x} calls "
                                             "something else")
                        if k + 1 in addrs and (words[k + 1] >> 16) & 31 == 7:
                            table = addrs[k + 1]
                        if words[k + 1] >> 26 == 0x09 and (words[k + 1] >> 16) & 31 == 8:
                            count = words[k + 1] & 0xFFFF
                        break
                    k += 1
                if table not in by_va:
                    name = ("sml"[min(n, 2)] + "room_info" + TABLE_SUFFIX[dtype]
                            + ("_2" if n == 3 else ""))
                    by_va[table] = name
                    table_va[name] = (table, count)
                row.append(by_va[table])
            names.append(row)
        return names, table_va

    def room_table(self, size, dtype, second):
        """The table MakeFloor passes for a room of this size: the large
        rooms' second set unless volume 1 on server 0 or 1."""
        return self.jump[size if size < 2 else 3 if second else 2][dtype]

    def _symroom(self):
        """MakeRoom copies its symroom[10] initialiser (40 bytes: lq, lq,
        ld 32) to the stack; the address formed just before the ld."""
        _va, words, addrs = self._code("MakeRoom__7DUNGEONFiP4FOOTP9ROOM_INFOi")
        for i in sorted(addrs):
            if any(w >> 26 == 0x37 and w & 0xFFFF == 32 for w in words[i + 1:i + 5]):
                return addrs[i]
        raise ValueError("MakeRoom: no symroom initialiser")

    def _edit_count(self):
        """EditDungeon's length: the bound of the DUNGEON constructor's search
        (`sltiu` after the table's address; 88 in Infection, 90 later)."""
        _va, words, addrs = self._code("__ct__7DUNGEONFi")
        base = self.p.symbol_named("EditDungeon").value
        i = min(i for i, a in addrs.items() if a == base)
        return next(w & 0xFFFF for w in words[i:i + 32] if w >> 26 == 0x0B)

    def _edit_floors(self):
        """(floors, {event area: floors}): how many floors DUNGEON::Generate
        runs MakeRealMap over. Infection: the constant 10 in Generate's loop
        (`li $s0, 10`). From Mutation on Generate reads DUNGEON.floors
        (+0x430), which the constructor sets to 10, or 15 for event area
        125 (`li 10; sw; ...; li 125; bne; li 15; sw`)."""
        _va, words, _ = self._code("Generate__7DUNGEONFv")
        field = None
        for w in words:
            op, rs, rt = w >> 26, (w >> 21) & 31, (w >> 16) & 31
            if rt == 16 and op == 0x09 and rs == 0:
                return w & 0xFFFF, {}
            if rt == 16 and op == 0x23:
                field = w & 0xFFFF
                break
        if field is None:
            raise ValueError("Generate: no floor count")
        _va, words, _ = self._code("__ct__7DUNGEONFi")
        li, default, special, compared = {}, None, {}, None
        for w in words:
            op, rs, rt, imm = w >> 26, (w >> 21) & 31, (w >> 16) & 31, w & 0xFFFF
            if op == 0x09 and rs == 0:
                li[rt] = imm - 0x10000 if imm & 0x8000 else imm
            elif op == 0x2B and imm == field and rt in li:
                if default is None:
                    default = li[rt]
                elif compared is not None:
                    special[compared] = li[rt]
                    break
            elif op == 0x05 and default is not None and (rs in li or rt in li):
                compared = li.get(rt, li.get(rs))
            elif op == 0 and w & 0x3F not in (0x08, 0x09):
                li.pop((w >> 11) & 31, None)
            elif 0x08 <= op <= 0x0F or 0x20 <= op <= 0x27 or op == 0x37:
                li.pop(rt, None)
        return default, special

    def floors_for(self, event):
        return self.edit_floors[1].get(event, self.edit_floors[0])

    def _type_rule(self):
        """What the save flag does in WORLD_MAN::SetDungeonTypeFromField: the
        first saveData field read after the eventAreaInfo search. Infection
        reads the byte at saveData+0x6772 inside the story areas' cases and
        takes the "E" type like EVENTAREA_INFO.flag 3 ("alt"). Mutation on
        read bit 62 of the u64 at saveData+0x5ec8 before the story areas'
        cases and, when set, give a story area a random area's type, with
        the six fixed areas on their plain types ("plain").
        Returns (rule, offset, bit)."""
        _va, words, addrs = self._code("SetDungeonTypeFromField__9WORLD_MANFv")
        save = self.p.symbol_named("saveData").value
        info = self.p.symbol_named("eventAreaInfo").value
        start = min(i for i, a in addrs.items() if a == info)
        base = set()
        for i in range(start, len(words)):
            w = words[i]
            op, rs, rt, imm = w >> 26, (w >> 21) & 31, (w >> 16) & 31, w & 0xFFFF
            if op == 0x23 and addrs.get(i) == save:
                base.add(rt)
            elif op in (0x20, 0x24) and rs in base:
                return ("alt", imm, None)
            elif op == 0x37 and rs in base:
                for w2, w3 in zip(words[i + 1:i + 6], words[i + 2:i + 7]):
                    if w2 >> 26 == 0x0F and w3 & 0xFFE0003F == 0x0000003C:    # lui; dsll32
                        return ("plain", imm, 48 + (w2 & 0xFFFF).bit_length() - 1)
        raise ValueError("SetDungeonTypeFromField: no save flag")

    def dungeon_type(self, field_type, event=0, event_flag=0, save_flag=False):
        return dungeon_type(field_type, event, event_flag, save_flag, self.type_rule[0])

    def _name_list(self, sym, i):
        # DungeonName[] alternates "sd1.ccs", "sd1"; the constructor passes
        # the second of each pair to ccStream::GetCCSAdrs.
        return self.p.cstr(self.p.u32(self.p.symbol_named(sym).value + 8 * i + 4)).decode()

    def ccs_name(self, dtype, tex_type=0):
        return (self.dungeon_name2 if tex_type == 1 else self.dungeon_name)[dtype]

    def gim_counts(self, ccs_name):
        """{anime name: [count per GIM_PATTERNS entry]} for one dungeon file.

        ccAnm::GetSubstAdrs (0x00151ce0) walks the anime's index table - one
        entry per object controller (sub-chunk 0x0102) - and ccMatchIndex
        (0x00101ad0) follows ExtObj chunks to the object they copy before
        comparing names, so a room counts the controllers whose target's
        name matches. Every such object is a separate ccCoord, so
        ccSubstSearchResult::SetTbl never merges two."""
        if ccs_name in self._counts:
            return self._counts[ccs_name]
        import ccs
        import gzarc
        data_bin = os.path.join(os.path.dirname(self.elf_path), "DATA", "DATA.BIN")
        arc = gzarc.open_bytes(data_bin)
        member = next(m for m in gzarc.members(arc) if m.name.lower() == ccs_name + ".cmp")
        c = ccs.Ccs(gzarc.inflate(arc, member))
        d = c.data
        ext = {}
        animes = []
        for off, t, _n, _end in c.chunks():
            if t is None or t & 0xFFFF == 0x0005:
                break
            kind = t & 0xFFFF
            if kind == 0x0A00:
                obj, _parent, target = struct.unpack_from("<3I", d, off + 8)
                ext[obj] = target
            elif kind == 0x0700:
                animes.append(off)
        out = {}
        for off in animes:
            name = c.objects[struct.unpack_from("<I", d, off + 8)[0]][0]
            words = struct.unpack_from("<I", d, off + 16)[0]
            p, end = off + 20, off + 20 + 4 * words
            counts = [0] * len(GIM_PATTERNS)
            while p < end:
                t, n = struct.unpack_from("<II", d, p)
                if t & 0xFFFF == 0x0102:
                    obj = struct.unpack_from("<I", d, p + 8)[0]
                    for _ in range(16):
                        if obj not in ext:
                            break
                        obj = ext[obj]
                    target = c.objects[obj][0]
                    for i, (pat, wild, _) in enumerate(GIM_PATTERNS):
                        if target.startswith(pat) if wild else target == pat:
                            counts[i] += 1
                p += 8 + 4 * n
            out[name] = counts
        self._counts[ccs_name] = out
        return out


FIXED_TYPES = {120: 3, 66: 5, 91: 7, 16: 7, 18: 4, 21: 6}
PLAIN_FIXED_TYPES = {120: 3, 66: 1, 91: 3, 16: 3, 18: 0, 21: 2}


def dungeon_type(field_type, event=0, event_flag=0, save_flag=False, rule="alt"):
    """WORLD_MAN::SetDungeonTypeFromField (INF 0x0019cf50): dungeonType[0..1].

    event_flag is the EVENTAREA_INFO.flag the game looks up (the substitute
    record for areas 71 and 47 where the executable has them). save_flag is
    the save flag the function reads, and rule what it does (Data.type_rule):
    "alt" (Infection, saveData+0x6772): a story area takes the E type, as
    with flag 3; "plain" (Mutation on, saveData+0x5ec8 bit 62): a story area
    takes a random area's type, and the six fixed areas their plain type."""
    types = [0, 0]
    if event == 0 or (rule == "plain" and save_flag):
        if event in PLAIN_FIXED_TYPES:
            types[0] = PLAIN_FIXED_TYPES[event]
            return types
        types[0] = {0: 2, 1: 1, 2: 3, 3: 3, 4: 8, 5: 1, 6: 1, 7: 0, 8: 3, 9: 2, 10: 0}.get(field_type, 0)
        if field_type == 4:
            types[1] = 2
        return types
    if event in FIXED_TYPES:
        types[0] = FIXED_TYPES[event]
        return types
    if event == -1:
        return types
    alt = event_flag == 3 or (rule == "alt" and save_flag)
    table = {0: (2, 6), 1: (1, 5), 2: (3, 7), 3: (3, 7), 4: (8, 9), 5: (1, 5), 6: (1, 5),
             7: (0, 4), 8: (3, 7), 9: (2, 6), 10: (0, 4)}
    if field_type not in table:
        types[0] = 4
        return types
    types[0] = table[field_type][alt]
    if field_type == 4:
        types[1] = 6 if alt else 2
    return types


class Foot:
    """FOOT (0x60 bytes): one room while the floor grows."""

    def __init__(self):
        self.x = self.y = 0
        self.direc = 0
        self.live = 1
        self.size = 0
        self.exit_size = [EMPTY_EXIT] * 4
        self.exit_num = 0
        self.next_x = [0] * 4
        self.next_y = [0] * 4
        self.pos = (0, 0)
        self.room = 0
        self.old_direc = 0


class Floor:
    def __init__(self, level):
        self.level = level
        self.d = bytearray(MAP * MAP)           # [x * 80 + y]
        self.next = bytearray([NONE]) * (MAP * MAP)
        self.here = bytearray([NONE]) * (MAP * MAP)
        self.rooms = []
        self.up = 0
        self.down = NONE
        self.room_num = 0
        self.retries = 0
        self.start = [None, None]


class Generator:
    def __init__(self, data, seed, dtype, level_max, room_max, server=0, volume=1,
                 word_a=0, tex_type=0, field_type=0, code=0):
        self.data = data
        self.field_type = field_type        # WORLD_MAN::GetFieldType, for a lake start
        self.code = code                    # DUNGEON.code, the dungeon's index
        self.rng = areas.Rng(seed)
        self.type = dtype
        self.level_max = level_max
        self.room_max = room_max
        self.server = server
        self.volume = volume
        self.word_a = word_a
        self.ccs = data.ccs_name(dtype, tex_type)
        self.counts = data.gim_counts(self.ccs)
        self.sym_flag = self.lake_flag = 0
        self.sym_floor = self.sym_block = 0
        self.gims = []                          # gimPos: (floor, room, type)
        self.floors = []

    def rand(self, n):
        """fieldrand (0x0019c460)."""
        return self.rng.next() % n

    def choose_room_size(self):
        r = self.rand(100)
        if self.volume == 1 and self.server in (0, 1):
            return 0 if r < 20 else 2 if r < 50 else 1
        return 0 if r < 20 else 2 if r < 40 else 1

    # FOOT -------------------------------------------------------------------
    def mark_doors(self, fl, f, bits, door, size):
        """The two door cells of each side in bits, as the FOOT constructors
        and Move write them."""
        x, y = f.x, f.y
        for bit, cells in ((1, ((x + door, y), (x + door + 1, y))),
                           (2, ((x + door, y + size - 1), (x + door + 1, y + size - 1))),
                           (4, ((x, y + door), (x, y + door + 1))),
                           (8, ((x + size - 1, y + door), (x + size - 1, y + door + 1)))):
            if bits & bit:
                for cx, cy in cells:
                    fl.d[cx * MAP + cy] = bit

    def set_pos(self, f, size):
        half = size // 2
        f.pos = (eemu.f_mul(CELL, eemu.f_from_int(f.x + half)),
                 eemu.f_mul(CELL, eemu.f_from_int(f.y + half)))

    def first_foot(self, fl):
        """FOOT::FOOT(int level, MAP_INFO (*)[80][80]) - room 0."""
        f = Foot()
        f.x = f.y = 40
        self.rand(1000)                 # drawn and dropped
        f.size = self.choose_room_size()
        if not (self.volume == 1 or self.server < 2) and f.size == 2:
            if self.rand(100) >= 31:
                f.size = 1
        door, size = SIZES[f.size]
        f.x -= size // 2
        f.y -= size // 2
        self.set_pos(f, size)
        f.exit_num = 1
        if not f.direc & 2:
            f.direc = 2
            f.exit_size[1] = self.choose_room_size()
        for j in range(f.y, f.y + size):
            for i in range(f.x, f.x + size):
                fl.here[i * MAP + j] = 0
        self.mark_doors(fl, f, f.direc, door, size)
        return f

    def next_foot(self, fl, n, x, y, s, d):
        """FOOT::FOOT(level, n, xpos, ypos, s, d, RealMap) - a room behind an exit."""
        f = Foot()
        f.x, f.y, f.room, f.old_direc, f.size = x, y, n, d, s
        door, size = SIZES[s]
        for j in range(y, y + size):
            for i in range(x, x + size):
                fl.here[i * MAP + j] = n
        self.set_pos(f, size)
        while True:
            f.exit_num = self.rand(4000) // 1000 or 1
            f.exit_size = [EMPTY_EXIT] * 4
            k = f.exit_num
            while k:
                r = self.rand(400) >> 2
                if r <= 3:
                    bit = DIREC[r]
                    if not f.direc & bit:
                        f.direc |= bit
                        f.exit_size[r] = self.choose_room_size()
                        k -= 1
            if f.direc & d:
                # An exit back where we came from is no exit.
                f.exit_size[DIREC.index(d)] = EMPTY_EXIT
                f.direc -= d
                f.exit_num -= 1
                if f.exit_num == 0:
                    continue
            break
        if fl.level == self.level_max - 1:
            # The last floor only leads on to medium rooms.
            f.exit_size = [1 if e != EMPTY_EXIT else e for e in f.exit_size]
        self.mark_doors(fl, f, f.old_direc, door, size)
        return f

    def move(self, fl, f, k, flag):
        """FOOT::Move: 1 if the room behind exit k fits (and with flag, mark
        this room's door on that side)."""
        es = f.exit_size[k]
        if es == EMPTY_EXIT:
            return 0
        door, size = SIZES[f.size]
        loop = SIZES[es][1]
        ofs = (size - loop) // 2
        nx, ny = ((f.x + ofs, f.y - loop), (f.x + ofs, f.y + size),
                  (f.x - loop, f.y + ofs), (f.x + size, f.y + ofs))[k]
        f.next_x[k], f.next_y[k] = nx, ny
        if not (0 <= nx < MAP and 0 <= ny < MAP):
            return 0
        for j in range(ny, ny + loop):
            for i in range(nx, nx + loop):
                if not (i < MAP and j < MAP) or fl.here[i * MAP + j] != NONE:
                    return 0
        if flag:
            self.mark_doors(fl, f, DIREC[k], door, size)
        return 1

    def check_direction(self, fl, f):
        """FOOT::CheckDirection: the sides whose door meets another door, and
        each door cell's `next` set to the room beyond it."""
        door, size = SIZES[f.size]
        x, y = f.x, f.y
        d, nxt, here = fl.d, fl.next, fl.here
        at = lambda i, j: i * MAP + j       # noqa: E731
        out = 0
        if y - 1 > 0 and d[at(x + door, y)] and d[at(x + door, y - 1)] and d[at(x + door + 1, y - 1)]:
            nxt[at(x + door, y)] = here[at(x + door, y - 1)]
            nxt[at(x + door + 1, y)] = here[at(x + door, y - 1)]
            out |= 1
        yy = y + size
        if yy < MAP and d[at(x + door, yy - 1)] and d[at(x + door, yy)] and d[at(x + door + 1, yy)]:
            nxt[at(x + door, yy - 1)] = here[at(x + door, yy)]
            nxt[at(x + door + 1, yy - 1)] = here[at(x + door + 1, yy)]
            out |= 2
        if x - 1 > 0 and d[at(x, y + door)] and d[at(x - 1, y + door)] and d[at(x - 1, y + door + 1)]:
            nxt[at(x, y + door)] = here[at(x - 1, y + door)]
            nxt[at(x, y + door + 1)] = here[at(x - 1, y + door + 1)]
            out |= 4
        xx = x + size
        if xx < MAP and d[at(xx - 1, y + door)] and d[at(xx, y + door)] and d[at(xx, y + door + 1)]:
            nxt[at(xx - 1, y + door)] = here[at(xx, y + door)]
            nxt[at(xx - 1, y + door + 1)] = here[at(xx, y + door + 1)]
            out |= 8
        return out

    # floors -----------------------------------------------------------------
    def generate(self):
        """DUNGEON::Generate for a random dungeon."""
        levels = self.level_max
        if self.word_a == 131 and self.type not in LAKE_TYPES:
            levels = self.level_max = 4
        if self.type in LAKE_TYPES:
            levels = 1
        for level in range(levels):
            n = self.rand(5) + self.room_max
            if n >= 15:
                n = 12
            self.floors.append(self.make_floor(level, n))
        return self

    def make_floor(self, level, room_num):
        """DUNGEON::MakeFloor, random branch."""
        fl = Floor(level)
        fl.room_num = room_num
        while True:
            fl.d[:] = bytes(MAP * MAP)
            fl.next[:] = bytes([NONE]) * (MAP * MAP)
            fl.here[:] = bytes([NONE]) * (MAP * MAP)
            feet = [self.first_foot(fl)]
            if self.grow(fl, feet) and sum(f.exit_num == 1 for f in feet[:room_num]) >= 2:
                break
            fl.retries += 1
        fl.up = 0
        if level != self.level_max - 1:
            if not any(self.check_direction(fl, feet[i]) in (1, 2, 4, 8) and i != fl.up
                       for i in range(room_num)):
                raise RuntimeError(f"floor {level + 1} has no dead end for the down stairs: "
                                   "the game's loop would never end")
            found = False
            while not found:
                for i in range(room_num):
                    d = self.check_direction(fl, feet[i])
                    if d in (1, 2, 4, 8) and self.rand(100) >= 91 and fl.up != i:
                        fl.down = i
                        found = True
        for i in range(room_num):
            self.make_room(fl, i, feet[i])
        return fl

    def grow(self, fl, feet):
        """Pass over the live rooms opening exits until there are enough
        rooms; False when 2 * floorRoomNum passes were not enough."""
        target = fl.room_num
        passes = 0
        while True:
            passes += 1
            if target * 2 < passes:
                return False
            i = 0
            while i < len(feet):
                f = feet[i]
                if f.live == 1:
                    for k in range(4):
                        if f.exit_size[k] != EMPTY_EXIT and not self.move(fl, f, k, 0):
                            f.exit_size[k] = EMPTY_EXIT
                            f.direc -= DIREC[k]
                    for k in range(4):
                        if f.exit_size[k] == EMPTY_EXIT:
                            continue
                        if self.move(fl, f, k, 1) == 1:
                            f.live = 0
                            feet.append(self.next_foot(fl, len(feet), f.next_x[k], f.next_y[k],
                                                       f.exit_size[k], RDIREC[k]))
                        if len(feet) - 1 == target - 1:
                            return True
                i += 1

    def make_room(self, fl, i, f):
        """DUNGEON::MakeRoom(int, FOOT *, ROOM_INFO *, int)."""
        second = not (self.volume == 1 and self.server in (0, 1))
        table_name = self.data.room_table(f.size, self.type, second)
        table = self.data.tables[table_name]
        direc = self.check_direction(fl, f)
        if i == fl.up:
            direc |= 0x10
        if i == fl.down:
            direc |= 0x20
        room = {"index": i, "x": f.x, "y": f.y, "size": f.size, "direc": direc,
                "exits": direc & 15, "pos": f.pos, "model": None, "table": table_name}
        exits = bin(direc & 15).count("1")
        for k, row in enumerate(table):
            if row["exit"] != direc:
                continue
            r = self.rand(row["num"])
            model = row["models"][r]
            sym = False
            if not direc & 0x30 and exits == 1 and f.size == 1:
                if self.type not in LAKE_TYPES:
                    if fl.level == self.level_max - 1 and self.sym_flag == 0:
                        sym = True
                        self.sym_flag = 1
                elif self.lake_flag == 0:
                    sym = True
                    self.lake_flag = 1
                if sym:
                    model = self.data.symroom[self.type]
                    self.sym_floor, self.sym_block = fl.level, i
            if direc & 0x20:
                mini = 4
            elif direc & 0x10:
                mini = 3
            elif exits == 2 and ((direc & 3) == 3 or (direc & 12) == 12):
                mini = 5
            elif exits == 2:
                mini = 6
            else:
                mini = {3: 0, 4: 1, 1: 2}.get(exits, 128)
            room.update(k=k, r=r, model=model, rotate=row["r"], minimap=mini, sym=sym)
            # The player's start on this floor (startpos[0]) and where the
            # down stairs put him back (startpos[1]): x, y, z come from the
            # model's OBJ_0ppp dummy, except in the lake dungeons, whose up
            # room starts at the room's centre; w faces into the room.
            if direc & 0x10:
                if self.type in LAKE_TYPES:
                    fl.start[0] = [f.pos[0], f.pos[1], 0, ONE]
                    if self.field_type == 4 and self.code == 0 and fl.level == 0 and i == 0:
                        fl.start[0][3] = FACING.get(row["r"], fl.start[0][3])
                else:
                    fl.start[0] = [None, None, None, FACING.get(row["r"])]
            if direc & 0x20:
                fl.start[1] = [None, None, None, FACING.get(row["r"])]
            self.set_all_gim(fl, i, model, room)
        fl.rooms.append(room)

    def set_all_gim(self, fl, i, model, room):
        """DUNGEON::SetAllGim's RNG and gimmick slots. Item boxes take
        fieldrand(100) >= 20 each (CheckEntryItemBox with no edit data),
        the ps0-ps3 objects fieldrand(100) >= 31."""
        counts = self.counts.get(model, [0] * len(GIM_PATTERNS))
        room["gims"] = []
        for (pat, _wild, kind), n in zip(GIM_PATTERNS, counts):
            for _ in range(n):
                if kind == 0:
                    keep = self.rand(100) >= 20
                elif kind >= 3:
                    keep = self.rand(100) >= 31
                else:
                    keep = True
                room["gims"].append((pat, kind, keep))
                if keep:
                    self.gims.append((fl.level & 0xFF, i, kind))


ROOMDATA = ("floor", "index", "x", "y", "type", "size", "modelIndex", "tpFlag", "eventFlag",
            "dirc", "nFlag", "sFlag", "eFlag", "wFlag", "next0", "next1", "next2", "next3",
            "itemID")
GIMMICKDATA = ("floor", "index", "x", "y", "type", "kind", "flag", "direc")


class Edited:
    """A story area's dungeon: EditDungeon[] (INF gcmn 0x00695cd0, 88
    entries; 90 from Mutation on) names a ROOMDATA and a GIMMICKDATA table
    per (event area, dungeon index); the DUNGEON constructor copies the rooms
    into DUNGEON.roomdata and Generate runs DUNGEON::MakeRealMap
    (INF 0x005be020) for every floor: ten, or from Mutation on fifteen for
    event area 125 (Data.floors_for). No RNG decides the layout."""

    def __init__(self, data, event, index=0):
        p = data.p
        base = p.symbol_named("EditDungeon").value
        self.event, self.index = event, index
        for i in range(data.edit_count):
            ev, idx, rooms, nrooms, gims, ngims = struct.unpack("<6i", p.read(base + 24 * i, 24))
            if ev == event and idx == index:
                break
        else:
            raise KeyError(f"no EditDungeon entry for event {event} dungeon {index}")
        self.slot = i
        sym = p.symbol_at(rooms & 0xFFFFFFFF)
        self.name = sym[0].name if sym and sym[0] and not sym[1] else hex(rooms & 0xFFFFFFFF)
        self.roomdata = [dict(zip(ROOMDATA, struct.unpack("<19i", p.read(rooms + 76 * k, 76))))
                         for k in range(nrooms)]
        self.gimdata = [dict(zip(GIMMICKDATA, struct.unpack("<8i", p.read(gims + 32 * k, 32))))
                        for k in range(ngims)]
        self.floor_count = data.floors_for(event)
        self.floors = []
        for level in range(self.floor_count):
            fl = self.make_real_map(level)
            if fl.rooms:
                self.floors.append(fl)

    def make_real_map(self, level):
        """DUNGEON::MakeRealMap: each room of the floor painted with its
        index (a byte), and each door of dirc with the next0..3 it names."""
        fl = Floor(level)
        for rd in self.roomdata:
            if rd["floor"] != level:
                continue
            door, size = SIZES.get(rd["size"], (None, None))
            x, y = rd["x"], rd["y"]
            for a in range(size):
                for b in range(size):
                    fl.here[(x + a) * MAP + y + b] = rd["index"] & 0xFF
            for bit, key, cells in ((1, "next0", ((x + door, y), (x + door + 1, y))),
                                    (2, "next1", ((x + door, y + size - 1), (x + door + 1, y + size - 1))),
                                    (4, "next2", ((x, y + door), (x, y + door + 1))),
                                    (8, "next3", ((x + size - 1, y + door), (x + size - 1, y + door + 1)))):
                if rd["dirc"] & bit:
                    for cx, cy in cells:
                        fl.d[cx * MAP + cy] = bit
                        fl.next[cx * MAP + cy] = rd[key] & 0xFF
            room = {"index": rd["index"], "x": x, "y": y, "size": rd["size"], "direc": rd["dirc"],
                    "exits": rd["dirc"] & 15, "model": None, "edit": rd,
                    "gims": [g for g in self.gimdata if g["floor"] == level and g["index"] == rd["index"]]}
            fl.rooms.append(room)
            if rd["dirc"] & 0x10:
                fl.up = rd["index"]
            if rd["dirc"] & 0x20:
                fl.down = rd["index"]
        fl.room_num = len(fl.rooms)
        return fl


def snapshot(g):
    """What the generator leaves in DUNGEON, in the shape tools/test_dungeon.py
    reads it back from the game's memory: per floor the MAP_INFO grid (x-major,
    d/next/here), Up/DownRoom, and per room slot animIdx (k, r, size, table),
    rotate, pos x/y, minimap and the model; startpos w for both stairs."""
    floors = []
    for fl in g.floors:
        grid = bytearray()
        for i in range(MAP * MAP):
            grid += bytes((fl.d[i], fl.next[i], fl.here[i]))
        rooms = []
        for i in range(15):
            r = fl.rooms[i] if i < len(fl.rooms) else None
            if r is None or r["model"] is None:
                rooms.append((0, 0, 0, None, 0, 0, 0, 0, 0, None))
                continue
            rooms.append((r["k"], r["r"], r["size"], r["table"], r["rotate"], r["pos"][0],
                          r["pos"][1], r["size"], r["minimap"], r["model"]))
        start = tuple(s[3] if s else 0 for s in fl.start)
        floors.append({"map": bytes(grid), "up": fl.up, "down": fl.down, "rooms": rooms,
                       "start_w": start})
    return {"floors": floors, "sym": (g.sym_flag, g.sym_floor, g.sym_block, g.lake_flag),
            "seed": g.rng.seed, "randcnt": g.rng.count, "gims": list(g.gims),
            "floorRoomNum": g.floors[-1].room_num if g.floors else 0}


# output -----------------------------------------------------------------------

DOOR_CHARS = {1: "^", 2: "v", 4: "<", 8: ">"}


def ascii_map(fl):
    """The used part of a floor, one character per map cell: the room's
    number in base 36, a door cell as an arrow pointing out of the room."""
    cells = [(i // MAP, i % MAP) for i in range(MAP * MAP) if fl.here[i] != NONE]
    if not cells:
        return []
    x0 = min(c[0] for c in cells)
    x1 = max(c[0] for c in cells)
    y0 = min(c[1] for c in cells)
    y1 = max(c[1] for c in cells)
    lines = []
    for y in range(y0, y1 + 1):
        row = []
        for x in range(x0, x1 + 1):
            i = x * MAP + y
            if fl.here[i] == NONE:
                row.append(" ")
            elif fl.d[i]:
                row.append(DOOR_CHARS.get(fl.d[i], "#"))
            else:
                row.append("0123456789abcdefghijklmnopqrstuvwxyz"[fl.here[i]])
        lines.append("".join(row).rstrip())
    return lines, (x0, y0)


def describe(g, rooms=True, gims=False):
    if isinstance(g, Edited):
        out = [f"edited dungeon {g.name} (event {g.event}, dungeon {g.index}), "
               f"{len(g.floors)} floor(s), {len(g.gimdata)} gimmicks"]
    else:
        out = [f"type {g.type} ({g.ccs}), {len(g.floors)} floor(s), "
               f"seed after {g.rng.seed} (randcnt {g.rng.count})"]
        if g.sym_flag or g.lake_flag:
            out.append(f"Gott statue room: floor {g.sym_floor + 1}, room {g.sym_block}")
    for fl in g.floors:
        lines, (x0, y0) = ascii_map(fl)
        down = f"down stairs in room {fl.down}" if fl.down != NONE else "no down stairs"
        out.append(f"\nfloor {fl.level + 1}: {fl.room_num} rooms, up stairs in room {fl.up}, "
                   f"{down}, {fl.retries} restart(s); map from cell ({x0}, {y0})")
        out.extend("  " + line for line in lines)
        if rooms and isinstance(g, Edited):
            for r in fl.rooms:
                e = r["edit"]
                exits = "".join(c for b, c in zip(DIREC, "NSWE") if r["exits"] & b)
                flags = [n for b, n in ((0x10, "UP"), (0x20, "DOWN")) if r["direc"] & b]
                out.append(f"  room {r['index']:2}  ({r['x']:2},{r['y']:2}) {SIZE_NAMES[r['size']]:6} "
                           f"exits {exits:4} type {e['type']:2} model {e['modelIndex']} "
                           f"event {e['eventFlag']} item 0x{e['itemID'] & 0xFFFFFFFF:x} {' '.join(flags)}")
                for gm in r["gims"]:
                    out.append(f"      gimmick at ({gm['x']}, {gm['y']}) type {gm['type']} "
                               f"kind {gm['kind']} flag {gm['flag']} direc {gm['direc']}")
        elif rooms:
            for r in fl.rooms:
                flags = [n for b, n in ((0x10, "UP"), (0x20, "DOWN")) if r["direc"] & b]
                if r.get("sym"):
                    flags.append("GOTT STATUE")
                exits = "".join(c for b, c in zip(DIREC, "NSWE") if r["exits"] & b)
                boxes = sum(1 for _p, kind, keep in r.get("gims", ()) if kind == 0 and keep)
                out.append(f"  room {r['index']:2}  ({r['x']:2},{r['y']:2}) {SIZE_NAMES[r['size']]:6} "
                           f"exits {exits:4} {r['model'] or '-':14} "
                           f"rot {eemu.f_to_py(r.get('rotate', 0)):+.3f} "
                           f"minimap {r.get('minimap')} boxes {boxes} {' '.join(flags)}")
                if gims:
                    for pat, kind, keep in r.get("gims", ()):
                        out.append(f"      {pat:20} {GIM_NAMES[kind]:12} "
                                   f"{'slot kept' if keep else 'dropped'}")
    return "\n".join(out)


def from_words(data, a, b, c, server=0, volume=None, index=0, save_flag=False, flag71=False):
    area = areas.generate(data.areas, a, b, c, server, volume, flag71)
    event = data.areas.event_info(area["event"], area["volume"], flag71)
    types = data.dungeon_type(area["fieldType"], area["event"], event["flag"] if event else 0,
                              save_flag)
    wa = data.areas.find(0, a)
    return area, types[index] if index < 2 else 0, wa.ID


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("gen")
    p.add_argument("elf")
    p.add_argument("a")
    p.add_argument("b")
    p.add_argument("c")
    p.add_argument("--index", type=int, default=0, help="which of the area's dungeons")
    p.add_argument("--save-flag", "--flag6772", dest="save_flag", action="store_true",
                   help="the save flag SetDungeonTypeFromField reads (INF saveData+0x6772, "
                        "later saveData+0x5ec8 bit 62)")
    p.add_argument("--flag71", action="store_true", help="the save flag at saveData+0x5bc0 bit 62")
    p2 = sub.add_parser("seed")
    p2.add_argument("elf")
    p2.add_argument("seed", type=lambda s: int(s, 0))
    p2.add_argument("type", type=int)
    p2.add_argument("levels", type=int)
    p2.add_argument("rooms", type=int)
    p2.add_argument("--word-a", type=int, default=0)
    for q in (p, p2):
        q.add_argument("--server", type=int, default=0)
        q.add_argument("--volume", type=int, help="default: the executable's volumeNum")
        q.add_argument("--no-rooms", action="store_true")
        q.add_argument("--gims", action="store_true", help="list each room's dummy objects")
    sub.add_parser("types").add_argument("elf")
    p3 = sub.add_parser("edit")
    p3.add_argument("elf")
    p3.add_argument("event", type=int)
    p3.add_argument("--index", type=int, default=0)
    args = parser.parse_args()

    data = Data(args.elf)
    if args.cmd == "edit":
        print(describe(Edited(data, args.event, args.index)))
        return 0
    if args.cmd == "types":
        rule, offset, bit = data.type_rule
        flag = f"saveData+{offset:#x}" + (f" bit {bit}" if bit is not None else "")
        print(f"fieldType  dungeonType[0, 1]: random area  story area  story area with flag 3  "
              f"story area with {flag} ({rule})")
        for ft in range(11):
            print(f"{ft:9}  {str(data.dungeon_type(ft)):12} {str(data.dungeon_type(ft, 99, 0)):11} "
                  f"{str(data.dungeon_type(ft, 99, 3)):23} {data.dungeon_type(ft, 99, 0, True)}")
        return 0
    volume = data.areas.volume if args.volume is None else args.volume
    if args.cmd == "gen":
        def key(s):
            return int(s) if s.isdigit() else s
        area, dtype, word_a = from_words(data, key(args.a), key(args.b), key(args.c),
                                         args.server, volume, args.index, args.save_flag,
                                         args.flag71)
        print(f"{' '.join(area['words'])}: code {area['code']}, event {area['event']}, "
              f"fieldType {area['fieldType']}, dungeonSize {area['dungeonSize']} "
              f"(levelMax {area['levelMax']}, roomMax {area['roomMax']})")
        if area["event"] and not (area["fieldType"] == 4 and args.index == 0):
            print(describe(Edited(data, area["event"], args.index), not args.no_rooms))
            return 0
        seed = area["dungeonSeed"][args.index]
        g = Generator(data, seed, dtype, area["levelMax"], area["roomMax"], args.server,
                      volume, word_a, field_type=area["fieldType"], code=args.index)
    else:
        g = Generator(data, args.seed, args.type, args.levels, args.rooms, args.server,
                      volume, args.word_a)
        seed = args.seed
    print(f"dungeonSeed {seed}")
    print(describe(g.generate(), not args.no_rooms, args.gims))
    return 0


if __name__ == "__main__":
    sys.exit(main())
