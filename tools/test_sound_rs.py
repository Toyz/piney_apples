#!/usr/bin/env python3
"""crates/piney-data's sound module against tools/sound.py, tools/scei.py and
tools/adpcm.py.

    python3 tools/test_sound_rs.py fixture   the expectations, to stdout
                                             (crates/piney-data/tests/sound_fixture.txt)
    python3 tools/test_sound_rs.py           check the fixture is current and
                                             run the Rust test against it

The fixture holds only numbers and hashes, no audio: for every bank its
place in SNDDATA.BIN and an FNV-1a 64 hash of its parsed header (every VAG,
sample, sample set, program and split field, in slot order); for every VAG
its range, rate, loop flag, decoded length, loop points, clip count and an
FNV-1a 64 hash of the decoded 16-bit samples; for every .sq its chunk
offsets and MIDI blocks; and the BGM.BIN tracks, with a hash of each
one's first 256 KiB of samples. Skipped when the disc is
not extracted or cargo is missing.
"""

import os
import shutil
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402
import adpcm  # noqa: E402
import scei  # noqa: E402
import sound  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
SNDDATA = os.path.join(os.path.dirname(volume.DATA), "SNDDATA.BIN")
ISO = volume.ISO
FIXTURE = os.path.join(ROOT, "crates", "piney-data", "tests", "sound_fixture.txt")

FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
M64 = (1 << 64) - 1


def fnv(data, h=FNV_OFFSET):
    for b in data:
        h = ((h ^ b) * FNV_PRIME) & M64
    return h


def fnv_values(values):
    """FNV-1a 64 over each value as a little-endian i64."""
    return fnv(b"".join((v & M64).to_bytes(8, "little") for v in values))


def hd_values(hd):
    """The header's fields in slot order - the same list sound.rs builds."""
    out = []
    vags = {v.index: v for v in hd.vags}
    for i in range(max(vags, default=-1) + 1):
        v = vags.get(i)
        out += [i, v.offset, v.rate, v.loop, v.reserved] if v else [i, -1]
    samples = {s.index: s for s in hd.samples}
    for i in range(max(samples, default=-1) + 1):
        s = samples.get(i)
        out += [i] + [getattr(s, n) for n, _ in scei.SAMPLE_FIELDS] if s else [i, -1]
    sets = {s.index: s for s in hd.samplesets}
    for i in range(max(sets, default=-1) + 1):
        s = sets.get(i)
        out += [i, s.vel_curve, s.vel_low, s.vel_high, len(s.samples)] + s.samples if s else [i, -1]
    progs = {p.index: p for p in hd.programs}
    for i in range(max(progs, default=-1) + 1):
        p = progs.get(i)
        if p is None:
            out += [i, -1]
            continue
        out += [i] + [getattr(p, n) for n, _ in scei.PROGRAM_FIELDS]
        for sp in p.splits:
            out += [getattr(sp, n) for n, _ in scei.SPLIT_FIELDS]
    return out


def slot_counts(data, hd):
    """Slots in each table, empty ones included (scei.Hd keeps only the
    present ones)."""
    out = []
    for name in ("Vagi", "Smpl", "Sset", "Prog"):
        p = hd.chunks[name]
        out.append(0 if p == scei.NONE else len(scei._table(data, p, name)[1]))
    return out


def fixture():
    prog = sound.Program(ELF)
    snd = open(SNDDATA, "rb").read()
    banks = sound.load_banks(prog, snd)
    print("# tools/test_sound_rs.py fixture: sound.py, scei.py and adpcm.py over SNDDATA.BIN.")
    print("# bank INDEX OFFSET HD SQ1 SQ2 SQ3 BD_OFFSET BD TABLE_BD VAGI SMPL SSET PROG HD_FNV")
    print("# vag BANK SLOT START END RATE LOOPFLAG SAMPLES FRAMES LOOP_START LOOP_END CLIPPED PCM_FNV")
    print("# sq BANK OFFSET SIZE SEQU_SIZE SONG MIDI SESEQ SESONG MIDI_BLOCKS...")
    print("# bgm TRACK OFS SIZE MODE SAMPLES FNV(the first 256 KiB of 16-bit samples, as bgm --export writes them)")
    for b in banks:
        hd_bytes = snd[b.offset:b.offset + b.hd_size]
        counts = slot_counts(hd_bytes, b.hd)
        print(f"bank {b.index} {b.offset} {b.hd_size} {' '.join(str(s) for s in b.sq_sizes)} "
              f"{b.bd_offset} {b.bd_size} {b.table_bd_size} {' '.join(str(c) for c in counts)} "
              f"{fnv_values(hd_values(b.hd)):016x}")
        body = snd[b.bd_offset:b.bd_offset + b.bd_size]
        ranges = b.hd.vag_ranges()
        for v in b.hd.vags:
            a, e = ranges[v.index]
            s = adpcm.decode(body, a, e - a)
            ls, le = s.loop if s.loop else (-1, -1)
            print(f"vag {b.index} {v.index} {a} {e} {v.rate} {v.loop} {len(s.pcm)} {s.frames} "
                  f"{ls} {le} {s.clipped} {fnv(s.pcm.tobytes()):016x}")
        for off, n in b.sq_offsets:
            sq = scei.Sq(snd[off:off + n])
            ch = " ".join(str(sq.chunks[k] if sq.chunks[k] != scei.NONE else -1)
                          for k in ("Song", "Midi", "SeSequence", "SeSong"))
            print(f"sq {b.index} {off} {n} {sq.size} {ch} {' '.join(str(m) for m in sq.midi)}")
    img = sound.open_image(ISO)
    bgm = sound.read_file(img, "VOICE/BGM.BIN")
    for i, o, s, mode in sound.load_bgm(prog):
        print(f"bgm {i} {o} {s} {mode} {s // 2} {fnv(bgm[o:o + min(s, 1 << 18)]):016x}")
    voice_lines(prog, img)
    return 0


# ccEvVoiceRequest's tables: (table, first event, evVoiceFile index)
EV_TABLES = (("evVoiceDataVol1M", 0, 6), ("evVoiceDataVol1S", 50, 7),
             ("evVoiceDataVol1ME", 0, 6), ("evVoiceDataVol1SE", 50, 7))


def voice_lines(prog, img):
    """Every row of the event voice tables, and a sample of their lines
    decoded as sound.py export-voices writes them: event 1's, every 32nd,
    the shortest and the longest."""
    print("# evrow TABLE EVENT MSG OFS SIZE     (every row; -1 is no line)")
    print("# voice PATH OFS SIZE SAMPLES FNV    (FNV-1a 64 of the 16-bit samples)")
    lines = []
    for table, first, fi in EV_TABLES:
        english = table.endswith("E")
        path = sound._disc_path(prog.strings("evVoiceFile" + ("E" if english else ""))[fi])
        for i, (ptr,) in enumerate(prog.words(table)):
            if not ptr or i >= 50:
                continue
            for k, (ofs, size) in enumerate(prog.words(prog.gcmn.name_at(ptr), "<ii")):
                print(f"evrow {table} {first + i} {k} {ofs} {size}")
                if ofs >= 0:
                    lines.append((path, ofs, size, first + i))
    pick = [x for x in lines if x[3] == 1] + lines[::32]
    pick += [min(lines, key=lambda x: x[2]), max(lines, key=lambda x: x[2])]
    files = {}
    seen = set()
    for path, ofs, size, _ in pick:
        if (path, ofs) in seen:
            continue
        seen.add((path, ofs))
        if path not in files:
            files[path] = sound.read_file(img, path)
        raw = files[path][ofs:ofs + size]
        print(f"voice {path} {ofs} {size} {len(raw) // 2} {fnv(raw):016x}")


HAVE = os.path.exists(ELF) and os.path.exists(SNDDATA) and os.path.exists(ISO)


@unittest.skipUnless(HAVE, "the disc is not extracted")
class RustTest(unittest.TestCase):
    def test_fixture_is_current(self):
        import io
        import contextlib
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            fixture()
        with open(FIXTURE) as f:
            self.assertEqual(f.read(), buf.getvalue(), "run: tools/test_sound_rs.py fixture > " + FIXTURE)

    @unittest.skipUnless(shutil.which("cargo"), "no cargo")
    def test_rust(self):
        subprocess.run([shutil.which("cargo"), "test", "-q", "-p", "piney-data", "--test", "sound"],
                       cwd=ROOT, check=True)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "fixture":
        sys.exit(fixture())
    unittest.main()
