#!/usr/bin/env python3
"""crates/piney-data's ccSaveData::Init (save/init.rs) against the game's own
Init__10ccSaveDataFi (INF SLUS_202.67:0x001743d0) run in eemu (the Rust
eemu_rs when it is built into tools/, as test_anim.machine_class picks).

Compared, every byte of the 0x8530-byte ccSaveData:

  - Init(0) and Init(1) on a zeroed record, one of 0xff bytes and random
    ones: what Init writes and, as much, what it leaves alone. ccSnd points
    at a ccSound whose mainVol, bgmVol, seVol and outputMode are the
    constructor's (256, 256, 256, 1) or random words, so Init(0)'s copy of
    them (as shorts) is checked too, and that Init(1) does not read them;
  - the boot as ccThMother runs it (boot_save): ccSound's constructor
    (0x00180ed0, which leaves 256, 256, 256, 1) on a zeroed ccSound with
    ccSnd pointing at it, ccSaveData's constructor (0x00174320: the clear
    of random bytes and Init(0)), then Init(1), against SaveData::boot;
  - the repair of the port's own earlier saves (SaveData::repair_port_save,
    not the game's): the game's boot save and the random ones pass through
    it unchanged, and a boot save with the old port's signature (ok and
    cancel the only buttons, Init's -1 lists zero) comes back as the boot.

Init's one piece of text, timeIdolRankDefStr, is read by the probe from the
disc (InitText::from_disc) and by the game from its own image.

InitLaterVolumes does the same for Mutation's, Outbreak's and Quarantine's
own Init on their discs, every byte of ccSaveData and of the 0x854-byte
extension their accessors write (the extension's pointer, the $gp global
after saveData, set to a buffer of the same pattern as the record).

ConvGameLaterVolumes runs the later volumes' CONVERT carry
(ccStartEventConvert, ConvGame) on random records against the port's.

boot_save and fresh_save are the starting save of the harnesses that
compare the port's desktop, title, board, field UI and events with the
game's.

Skipped when the disc is not extracted or cargo is missing.
"""

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
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "save_probe")
NEWGAME_EXAMPLE = os.path.join(os.path.dirname(EXAMPLE), "newgame_probe")

SIZE = 0x8530
SAVEDATA, GAME_P, CCSYS = inf_va(0x003789D8), inf_va(0x003789CC), inf_va(0x003788E0)    # gp globals
GAMEOBJ, SYSOBJ = 0x01840000, 0x01850000
CCSND = inf_va(0x00378A08)          # ccSound *ccSnd (gp global)
SND = 0x01800000            # the ccSound it points at (0x150 bytes)
HEAP = 0x01860000           # what the constructors allocate (Mutation's the extension)
SAVE = 0x01810000
BOOT_LEVELS = (256, 256, 256, 1)

# The ranges Init writes with -1 that the port's earlier saves left zero,
# and the buttons from +0x8404.
OLD_PORT_ZERO = [(0x0030, 0xb40), (0x0b70, 0x18c), (0x1ec4, 0x2d0), (0x5278, 0x280), (0x50ac, 0x190),
                 (0x65dc, 0x190), (0x6548, 0x80), (0x8404, 0x16)]


_BOOT = {}
_BOOT_EXT = {}


def boot_save(elf=ELF):
    """The ccSaveData ccThMother builds at boot, made by the game's own code
    in eemu: ccSound's constructor on the ccSound ccSnd points at, then
    ccSaveData's constructor (the clear and Init(0)) and Init(1). The other
    harnesses start from it (with the name the port's fresh save and
    NewGame give, `fresh_save`), so that what they compare starts from a
    save the game has. Made once per executable."""
    if elf not in _BOOT:
        from image import Program
        from test_anim import machine_class
        p = Program(elf)
        m = machine_class()(p)
        m.mem[SND:SND + 0x150] = bytes(0x150)
        m.store(p.symbol_named("ccSnd").value, 4, SND)
        # Mutation on, the constructor makes an object of its own (new).
        heap = [HEAP]

        def alloc(mm, n, *_):
            a = heap[0]
            heap[0] = (a + n + 15) & ~15
            mm.mem[a:a + n] = bytes(n)
            return a
        for name in ("__nw__FUi", "ccMalloc__FUi"):
            m.hooks[p.symbol_named(name).value] = alloc
        m.call(p.symbol_named("__ct__7ccSoundFv").value, [SND])
        m.mem[SAVE:SAVE + SIZE] = bytes(random.Random(5).randrange(256) for _ in range(SIZE))
        m.call(p.symbol_named("__ct__10ccSaveDataFv").value, [SAVE])
        m.call(p.symbol_named("Init__10ccSaveDataFi").value, [SAVE, 1])
        _BOOT[elf] = bytes(m.mem[SAVE:SAVE + SIZE])
        # A later volume's extension, where the constructor put it.
        ext_at = next((g for e, _, g in LATER.values() if elf.endswith(e)), None)
        if ext_at:
            ptr = m.load(ext_at, 4)
            _BOOT_EXT[elf] = bytes(m.mem[ptr:ptr + EXT_SIZE])
    return _BOOT[elf]


def boot_extension(elf=ELF):
    """A later volume's boot extension (boot_save's), empty for Infection."""
    boot_save(elf)
    return _BOOT_EXT.get(elf, b"")


def extension_pointer(elf=ELF):
    """The $gp global holding a later volume's extension pointer, None for
    Infection."""
    return next((g for e, _, g in LATER.values() if elf.endswith(e)), None)


def fresh_slot(elf=ELF):
    """fresh_save with, from Mutation on, the boot extension after it: what
    a slot file holds."""
    return bytearray(fresh_save(elf)) + bytearray(boot_extension(elf))


def place_save(m, at, data, elf=ELF):
    """A slot's bytes into a machine: the record at `at`, and from Mutation
    on the extension right after it, with the $gp pointer set to it."""
    m.mem[at:at + SIZE] = bytes(data[:SIZE])
    ptr = extension_pointer(elf)
    if ptr is not None:
        m.mem[at + SIZE:at + SIZE + EXT_SIZE] = bytes(data[SIZE:SIZE + EXT_SIZE]).ljust(EXT_SIZE, b"\0")
        m.store(ptr, 4, at + SIZE)


def fresh_save(elf=ELF):
    """SaveState::fresh_with's ccSaveData: the boot's (boot_save), the player
    named Kite as NewGame names him from charTbl[0]."""
    b = bytearray(boot_save(elf))
    b[0:4] = b"Kite"
    return b


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-data", "--example",
                    "save_probe"], cwd=ROOT, check=True)
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-fieldui", "--example",
                    "newgame_probe"], cwd=ROOT, check=True)


def game_new_game(parody):
    """A new game's save as the game makes it on the way into The World:
    the boot's (boot_save); ccSetupDemo's NewGame(1); parodyFlag when the
    title's Parody is chosen; New Game's NewGame(0) (DEMO.PRG in, its
    charTbl); then ccSetupNewGame's part with GCMN.PRG in: InitSpcParam,
    newGameFlag = 1, SetSpcBaseMsg. What NewGame does outside the save
    (the display offset, the sound settings, the camera type) is
    answered with nothing."""
    from image import Program
    from test_anim import machine_class
    Machine = machine_class()

    def machine(overlay):
        p = Program(ELF, overlay)
        m = Machine(p)
        m.mem[GAMEOBJ:GAMEOBJ + 0x100] = bytes(0x100)
        m.mem[SYSOBJ:SYSOBJ + 0x400] = bytes(0x400)
        m.store(SAVEDATA, 4, SAVE)
        m.store(GAME_P, 4, GAMEOBJ)
        m.store(CCSYS, 4, SYSOBJ)
        for name in ("SetDisplayOffset__8ccSystemFii", "SetSoundEnv__10ccSaveDataFv", "setCameraCtrlType__Fi"):
            m.hooks[p.symbol_named(name).value] = lambda mm, *a: 0
        return p, m

    p, m = machine("demo")
    m.mem[SAVE:SAVE + SIZE] = boot_save(ELF)
    m.call(p.symbol_named("NewGame__10ccSaveDataFi").value, [SAVE, 1])
    if parody:
        m.store(SAVE + 0x842B, 1, 1)
    m.call(p.symbol_named("NewGame__10ccSaveDataFi").value, [SAVE, 0])
    save = bytes(m.mem[SAVE:SAVE + SIZE])
    p, m = machine("gcmn")
    m.mem[SAVE:SAVE + SIZE] = save
    m.call(p.symbol_named("InitSpcParam__10ccSaveDataFv").value, [SAVE])
    m.store(SAVE + 0x6770, 1, 1)
    m.call(p.symbol_named("SetSpcBaseMsg__10ccSaveDataFv").value, [SAVE])
    return bytes(m.mem[SAVE:SAVE + SIZE])


def ask(lines, iso=ISO):
    p = subprocess.run([EXAMPLE, iso], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True,
                       cwd=ROOT)
    return p.stdout.splitlines()


# The later volumes: their executable, disc, and the $gp global holding the
# extension's pointer (docs/formats/save.md, "The extension").
LATER = {
    "MUT": ("mutation/disc/SLUS_205.62", "mutation/mutation.iso", 0x0038B95C),
    "OUT": ("outbreak/disc/SLUS_205.63", "outbreak/outbreak.iso", 0x00386CF0),
    "QUA": ("quarantine/disc/SLUS_205.64", "quarantine/quarantine.iso", 0x002795F8),
}
EXT_SIZE = 0x854
EXT = 0x01830000


def first_difference(a, b):
    for i, (x, y) in enumerate(zip(a, b)):
        if x != y:
            return f"+0x{i:04x}: game {x:02x}, port {y:02x}"
    return "lengths differ" if len(a) != len(b) else "same"


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class InitAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from image import Program
        from test_anim import machine_class
        build()
        cls.p = Program(ELF)
        cls.Machine = machine_class()

    def sym(self, name):
        return self.p.symbol_named(name).value

    def machine(self, levels):
        m = self.Machine(self.p)
        m.mem[SND:SND + 0x150] = bytes(0x150)
        for i, v in enumerate(levels):
            m.store(SND + 4 * i, 4, v & 0xFFFFFFFF)
        m.store(CCSND, 4, SND)
        return m

    def game_init(self, flag, levels, data):
        m = self.machine(levels)
        m.mem[SAVE:SAVE + SIZE] = data
        m.call(self.sym("Init__10ccSaveDataFi"), [SAVE, flag])
        return bytes(m.mem[SAVE:SAVE + SIZE])

    def game_boot(self):
        m = self.machine((0, 0, 0, 0))
        m.call(self.sym("__ct__7ccSoundFv"), [SND])
        self.assertEqual([m.load(SND + 4 * i, 4) for i in range(4)], list(BOOT_LEVELS))
        return boot_save()

    @unittest.skipUnless(volume.NAME == "infection", "InitLaterVolumes runs the later discs' Init")
    def test_init(self):
        rng = random.Random(1)
        bases = [bytes(SIZE), bytes([0xFF]) * SIZE] + [bytes(rng.randrange(256) for _ in range(SIZE))
                                                      for _ in range(4)]
        cases = []
        for flag in (0, 1):
            for k, base in enumerate(bases):
                levels = BOOT_LEVELS if k % 2 == 0 else tuple(rng.randrange(-0x80000000, 0x80000000)
                                                              for _ in range(4))
                cases.append((flag, levels, base))
        got = ask([f"init {f} {' '.join(map(str, lv))} {b.hex()}" for f, lv, b in cases])
        self.assertEqual(len(got), len(cases))
        for i, ((flag, levels, base), g) in enumerate(zip(cases, got)):
            want = self.game_init(flag, levels, base)
            port = bytes.fromhex(g)
            self.assertEqual(want, port, f"case {i} Init({flag}): {first_difference(want, port)}")

    def test_boot(self):
        want = self.game_boot() + boot_extension()
        port = bytes.fromhex(ask(["boot"])[0])
        self.assertEqual(want, port, first_difference(want, port))
        # What the port's fresh saves were missing: the bag's empty entries,
        # the buttons, fountainRecord cut by protectArea's loop.
        self.assertEqual(want[0x30:0x34], bytes([0xFF, 0xFF, 0xFF, 0]))
        self.assertEqual(want[0x8404:0x8408], bytes([0x40, 0, 0x10, 0]))
        self.assertEqual(want[0x65dc + 4 * 26:0x65dc + 4 * 28], bytes(4) + bytes([0xFF]) * 4)

    def test_repair(self):
        boot = self.game_boot()
        rng = random.Random(2)
        saves = [boot] + [bytes(rng.randrange(256) for _ in range(SIZE)) for _ in range(3)]
        old = bytearray(boot)
        for at, n in OLD_PORT_ZERO:
            old[at:at + n] = bytes(n)
        old[0x840e:0x8412] = bytes([0x40, 0, 0x20, 0])
        got = ask([f"repair {s.hex()}" for s in saves] + [f"repair {bytes(old).hex()}"])
        for s, g in zip(saves, got):
            changed, data = g.split()
            self.assertEqual((changed, bytes.fromhex(data)), ("0", s))
        changed, data = got[-1].split()
        self.assertEqual(changed, "1")
        self.assertEqual(bytes.fromhex(data), boot, first_difference(boot, bytes.fromhex(data)))


@unittest.skipUnless(shutil.which("cargo"), "needs cargo")
class InitLaterVolumes(unittest.TestCase):
    """The later volumes' Init (MUT 0x00175740) against the port's, with
    the extension."""

    @classmethod
    def setUpClass(cls):
        from test_anim import machine_class
        build()
        cls.Machine = machine_class()

    def game_init(self, p, gp_ext, flag, levels, data):
        m = self.Machine(p)
        m.mem[SND:SND + 0x150] = bytes(0x150)
        for i, v in enumerate(levels):
            m.store(SND + 4 * i, 4, v & 0xFFFFFFFF)
        m.store(p.symbol_named("ccSnd").value, 4, SND)
        m.mem[SAVE:SAVE + SIZE] = data[:SIZE]
        m.mem[EXT:EXT + EXT_SIZE] = data[SIZE:]
        m.store(gp_ext, 4, EXT)
        m.call(p.symbol_named("Init__10ccSaveDataFi").value, [SAVE, flag])
        return bytes(m.mem[SAVE:SAVE + SIZE]) + bytes(m.mem[EXT:EXT + EXT_SIZE])

    def test_init(self):
        from image import Program
        for vol, (elf, iso, gp_ext) in LATER.items():
            elf, iso = os.path.join(ROOT, "work", elf), os.path.join(ROOT, "work", iso)
            if not (os.path.exists(elf) and os.path.exists(iso)):
                continue
            p = Program(elf)
            rng = random.Random(3)
            full = SIZE + EXT_SIZE
            bases = [bytes(full), bytes([0xFF]) * full] + [bytes(rng.randrange(256) for _ in range(full))
                                                          for _ in range(2)]
            cases = []
            for flag in (0, 1):
                for k, base in enumerate(bases):
                    levels = BOOT_LEVELS if k % 2 == 0 else tuple(rng.randrange(-0x80000000, 0x80000000)
                                                                  for _ in range(4))
                    cases.append((flag, levels, base))
            got = ask([f"init {f} {' '.join(map(str, lv))} {b.hex()}" for f, lv, b in cases], iso)
            self.assertEqual(len(got), len(cases))
            for i, ((flag, levels, base), g) in enumerate(zip(cases, got)):
                want = self.game_init(p, gp_ext, flag, levels, base)
                port = bytes.fromhex(g)
                self.assertEqual(want, port, f"{vol} case {i} Init({flag}): {first_difference(want, port)}")


@unittest.skipUnless(shutil.which("cargo"), "needs cargo")
class NewGameLaterVolumes(unittest.TestCase):
    """The title's part of a new game on the later volumes: their boot save
    (the constructor, Init(1)), then NewGame(1), parodyFlag, NewGame(0)
    with DEMO.PRG in (MUT 0x00176270: 21 characters, the trade lists and
    records past Infection's in the extension, the volume's Parody level
    and default words, important item 5), against the port's, every byte
    of ccSaveData and the extension."""

    @classmethod
    def setUpClass(cls):
        from test_anim import machine_class
        build()
        cls.Machine = machine_class()

    def game(self, elf, gp_ext, parody):
        from image import Program

        def machine(overlay):
            p = Program(elf, overlay)
            m = self.Machine(p)
            m.store(gp_ext, 4, EXT)
            nw = p.symbol_named("__nw__FUi")
            m.hooks[nw.value] = lambda mm, *a: EXT
            for name in ("SetDisplayOffset__8ccSystemFii", "SetSoundEnv__10ccSaveDataFv", "setCameraCtrlType__Fi"):
                m.hooks[p.symbol_named(name).value] = lambda mm, *a: 0
            return p, m

        p, m = machine(None)
        m.mem[SND:SND + 0x150] = bytes(0x150)
        m.store(p.symbol_named("ccSnd").value, 4, SND)
        m.call(p.symbol_named("__ct__7ccSoundFv").value, [SND])
        m.mem[SAVE:SAVE + SIZE] = bytes(random.Random(5).randrange(256) for _ in range(SIZE))
        m.mem[EXT:EXT + EXT_SIZE] = bytes(random.Random(6).randrange(256) for _ in range(EXT_SIZE))
        m.call(p.symbol_named("__ct__10ccSaveDataFv").value, [SAVE])
        m.call(p.symbol_named("Init__10ccSaveDataFi").value, [SAVE, 1])
        record = bytes(m.mem[SAVE:SAVE + SIZE]) + bytes(m.mem[EXT:EXT + EXT_SIZE])
        p, m = machine("demo")
        m.mem[SAVE:SAVE + SIZE] = record[:SIZE]
        m.mem[EXT:EXT + EXT_SIZE] = record[SIZE:]
        new_game = p.symbol_named("NewGame__10ccSaveDataFi").value
        m.call(new_game, [SAVE, 1])
        if parody:
            m.store(SAVE + 0x842B, 1, 1)
        m.call(new_game, [SAVE, 0])
        return bytes(m.mem[SAVE:SAVE + SIZE]) + bytes(m.mem[EXT:EXT + EXT_SIZE])

    def test_new_game(self):
        for vol, (elf, iso, gp_ext) in LATER.items():
            elf, iso = os.path.join(ROOT, "work", elf), os.path.join(ROOT, "work", iso)
            if not (os.path.exists(elf) and os.path.exists(iso)):
                continue
            p = subprocess.run([NEWGAME_EXAMPLE, iso], input="newgame 0\nnewgame 1\n", capture_output=True,
                               text=True, check=True, cwd=ROOT)
            got = p.stdout.split()
            for parody in (0, 1):
                want = bytearray(self.game(elf, gp_ext, parody))
                port = bytearray(bytes.fromhex(got[parody]))
                # The name pointers: the game's buffers are its own; the
                # port's are the carried addresses of the same ones.
                self.assertEqual(bytes(want), bytes(port), f"{vol} parody {parody}: {first_difference(want, port)}")


@unittest.skipUnless(shutil.which("cargo"), "needs cargo")
class ConvGameLaterVolumes(unittest.TestCase):
    """CONVERT's carry on the later volumes (issue #55): ccStartEventConvert
    (MUT 0x001cac50) then ConvGame (MUT 0x001767e0, OUT 0x00176020, QUA
    0x00175f80) with DEMO.PRG in, on random records and extensions with
    random party members, against the port's piney_demo::newgame, every byte
    of ccSaveData and the extension."""

    @classmethod
    def setUpClass(cls):
        from test_anim import machine_class
        build()
        cls.Machine = machine_class()

    def game(self, elf, gp_ext, record):
        from image import Program
        p = Program(elf, "demo")
        m = self.Machine(p)
        m.store(gp_ext, 4, EXT)
        m.store(p.symbol_named("saveData").value, 4, SAVE)
        for name in ("SetDisplayOffset__8ccSystemFii", "SetSoundEnv__10ccSaveDataFv", "setCameraCtrlType__Fi"):
            m.hooks[p.symbol_named(name).value] = lambda mm, *a: 0
        m.mem[SAVE:SAVE + SIZE] = record[:SIZE]
        m.mem[EXT:EXT + EXT_SIZE] = record[SIZE:]
        m.call(p.symbol_named("ccStartEventConvert__Fv").value, [])
        m.call(p.symbol_named("ConvGame__10ccSaveDataFv").value, [SAVE])
        return bytes(m.mem[SAVE:SAVE + SIZE]) + bytes(m.mem[EXT:EXT + EXT_SIZE])

    def test_conv_game(self):
        for vol, (elf, iso, gp_ext) in LATER.items():
            elf, iso = os.path.join(ROOT, "work", elf), os.path.join(ROOT, "work", iso)
            if not (os.path.exists(elf) and os.path.exists(iso)):
                continue
            rng = random.Random(55)
            records = []
            for k in range(6):
                b = bytearray(rng.randrange(256) for _ in range(SIZE + EXT_SIZE))
                # partyMemberFlag: none, all, or some of the characters.
                members = (0, 0xFFFFFFFF, rng.getrandbits(32))[k % 3]
                struct.pack_into("<I", b, 0x2220, members)
                records.append(bytes(b))
            p = subprocess.run([NEWGAME_EXAMPLE, iso], input="".join(f"convgame {r.hex()}\n" for r in records),
                               capture_output=True, text=True, check=True, cwd=ROOT)
            got = p.stdout.split()
            self.assertEqual(len(got), len(records))
            for i, (r, g) in enumerate(zip(records, got)):
                want = self.game(elf, gp_ext, r)
                port = bytes.fromhex(g)
                self.assertEqual(want, port, f"{vol} record {i}: {first_difference(want, port)}")


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class NewGameAgainstGame(unittest.TestCase):
    """The save `--mode world` enters The World with (the probe builds it
    with the runtime's own new_game_save and FieldUi::setup_new_game)
    against game_new_game's, every byte, with and without Parody; then what
    InitSpcParam gave Kite: skills, equipment, items, key items."""

    @classmethod
    def setUpClass(cls):
        build()

    def ask(self, parody):
        p = subprocess.run([NEWGAME_EXAMPLE, ISO], input=f"save {parody}\n", capture_output=True, text=True,
                           check=True, cwd=ROOT)
        return bytes.fromhex(p.stdout.split()[0])

    def test_new_game_save(self):
        for parody in (0, 1):
            want = game_new_game(parody)
            got = self.ask(parody)
            self.assertEqual(want, got, f"parody {parody}: {first_difference(want, got)}")
            kite = struct.unpack_from("<20h", want, 0x1EC4)
            self.assertNotEqual(kite[0], -1, "InitSpcParam gave Kite no skills")
            self.assertNotEqual(want[0x30:0x34], bytes([0xFF, 0xFF, 0xFF, 0]), "no starting items")
            self.assertEqual(want[0x6770], 1)


if __name__ == "__main__":
    unittest.main()
