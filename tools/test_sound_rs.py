#!/usr/bin/env python3
"""crates/piney-data's sound module against tools/sound.py, tools/scei.py and
tools/adpcm.py.

    python3 tools/test_sound_rs.py fixture   the expectations, to stdout
                                             (crates/piney-data/tests/sound_fixture.txt;
                                             PINEY_VOLUME's: sound_fixture_mut.txt ...)
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
# Infection's sound_fixture.txt, a later volume's sound_fixture_mut.txt and so on.
SUFFIX = {"infection": "", "mutation": "_mut", "outbreak": "_out", "quarantine": "_qua"}[volume.NAME]
FIXTURE = os.path.join(ROOT, "crates", "piney-data", "tests", f"sound_fixture{SUFFIX}.txt")

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


def voice_lines(prog, img):
    """Every row of ccEvVoiceRequest's tables as the volume's code picks
    them (sound.event_tables), and a sample of their lines decoded as
    sound.py export-voices writes them: the first voiced event's, every
    32nd, the shortest and the longest. Rows past an array's end are the
    words the game reads there (Outbreak's EVVOL3SE event 266 array is one
    row in .sdata at 0x003862f8); only lines within their file are decoded."""
    print("# evrow SECTION KIND LANG EVENT MSG OFS SIZE   (every row; -1 is no line)")
    print("# voice PATH OFS SIZE SAMPLES FNV    (FNV-1a 64 of the 16-bit samples)")
    lines = []
    files = {"jp": prog.strings("evVoiceFile"), "en": prog.strings("evVoiceFileE")}
    sizes = {}
    for section, kind, lang, _start, events in sound.event_tables(prog):
        file = 6 + 3 * section + sound.EV_KINDS.index(kind)
        path = sound._disc_path(files[lang][file]) if file < len(files[lang]) else None
        first = 100 * section + sound.EV_FIRST[kind]
        for i, rows in enumerate(events):
            for k, (ofs, size) in enumerate(rows or ()):
                print(f"evrow {section} {kind} {lang} {first + i} {k} {ofs} {size}")
                if path not in sizes:
                    e = path and sound.find(img, path)
                    sizes[path] = e.size if e else 0
                if 0 <= ofs and ofs + size <= sizes[path]:
                    lines.append((path, ofs, size, first + i))
    pick = [x for x in lines if x[3] == min(x[3] for x in lines)] + lines[::32]
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

    def test_event_tables_from_the_code(self):
        """sound.event_tables reads ccEvVoiceRequest's code. On Infection,
        the one disc with its own names, every table it finds starts at an
        evVoiceData symbol and every array with rows is a whole VOICE_DATA
        object (the placeholder tables of the other volumes run into other
        data, which holds no row). On every disc its own hundred's main and
        side tables have rows in both languages."""
        prog = sound.Program(ELF)
        g = prog.gcmn
        tables = sound.event_tables(prog)
        if volume.number() == 1:
            for _section, _kind, _lang, start, events in tables:
                sym, off = g.symbol_at(start)
                self.assertEqual((off, sym.name[:11]), (0, "evVoiceData"), hex(start))
                for i, rows in enumerate(events):
                    if rows:
                        arr, off = g.symbol_at(g.u32(start + 4 * i))
                        self.assertEqual((off, arr.size), (0, 8 * len(rows)), (sym.name, i))
        own = {(kind, lang) for section, kind, lang, _, events in tables
               if section == volume.number() - 1 and any(events)}
        self.assertTrue({("main", "jp"), ("main", "en"), ("side", "jp"), ("side", "en")} <= own, own)

    def test_script_messages_voice_the_rows(self):
        """Every message every event script can open, in both languages and
        Parody Mode: the line the game's ccEvVoiceRequest and evVoicePlay
        hand SEWORDS (tools/evscript.py runs them) is the table row
        sound.event_tables reads, and no message is past its event's array
        (the rows the tables keep past an array's end are never asked)."""
        import evscript
        ev = evscript.Events(ELF)
        rows = {}
        for section, kind, lang, _start, events in sound.event_tables(sound.Program(ELF)):
            first = 100 * section + sound.EV_FIRST[kind]
            for i, r in enumerate(events):
                rows[(kind, lang, first + i)] = r
        voiced = 0
        for event in sorted(ev.events):
            for parody in (False, True):
                kind = "side" if event % 100 >= 50 else "parody" if parody else "main"
                for k in range(len(ev.messages(event, parody))):
                    for english in (False, True):
                        r = rows.get((kind, "en" if english else "jp", event))
                        self.assertFalse(r is not None and k >= len(r), (event, k, english, parody))
                        game = ev.voice(event, k, english, parody)
                        if game is not None:
                            self.assertEqual(r and r[k], game[1:], (event, k, english, parody))
                            voiced += 1
        self.assertGreater(voiced, 1000)

    @unittest.skipUnless(shutil.which("cargo"), "no cargo")
    def test_rust(self):
        subprocess.run([shutil.which("cargo"), "test", "-q", "-p", "piney-data", "--test", "sound"],
                       cwd=ROOT, check=True)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "fixture":
        sys.exit(fixture())
    unittest.main()
