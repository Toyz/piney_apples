#!/usr/bin/env python3
"""The .hack//Infection audio: the SNDDATA.BIN sound banks, the VOICE and
VOICE_E voice files, VOICE/BGM.BIN, and the PCM track of the STREAM cutscenes.

    tools/sound.py banks         ELF                         every bank, who loads it
    tools/sound.py bank          ELF N [--programs] [--samples]   one bank's header
    tools/sound.py export-banks  ELF OUTDIR [--bank N]       every VAG as WAV
    tools/sound.py voices        ELF IMAGE [--file PATH]     voice lines per file
    tools/sound.py export-voices ELF IMAGE OUTDIR [--file PATH] [--limit N]
    tools/sound.py bgm           ELF IMAGE [--export OUTDIR] the two BGM.BIN tracks
    tools/sound.py stream        IMAGE ARCHIVE MEMBER OUT.wav   a cutscene's PCM
    tools/sound.py streams       IMAGE                       PCM in every cutscene

ELF is SLUS_202.67 with the overlays in DATA/ next to it (as tools/image.py
wants) and DATA/SNDDATA.BIN there too; IMAGE is the disc image, from which
the voice and stream files are read in place. ARCHIVE is STR1, STR1E,
STRCMN or STRCMNE.

Nothing on the disc indexes these files; the executable does:

- SNDDATA.BIN is 79 banks back to back. `commseTbl` (u32 ofs, hd size, bd
  size) is the common sound-effect bank; every other bank is a row of an
  SQ_LOAD table (int ofs, hdSize, sqSize1..3, bdSize; -1 unused) - typeA ..
  typeO and piroshi via sqDataField, sqDataDungeon, sqDataTown, sqDataTitle,
  sqDataDesktop, sqDataToppage, sqDataEvent, sqDataStream. At `ofs` sit the
  .hd, then up to three .sq, and the .bd starts at the next 2048-byte
  boundary (SNDBASE.IRX ccSQDataLoadCD, module offset 0x20b0). The bd size to
  trust is the Head chunk's: one table row (typeI[2]) carries another bank's.
- Voice files are headerless mono 16-bit PCM, one line per VOICE_DATA
  (int ofs, int siz; bytes; -1 = none) row, lines 2048-aligned, 0xff between.
- BGM.BIN is two stereo 16-bit PCM tracks, bgmWavTbl.
- Cutscene PCM rides in Pcm (0x2200) and F_Pcm (0x2201) chunks as 1024-byte
  blocks, each 256 left samples then 256 right.

All PCM goes to the SPU2 through SEWORDS.IRX's block transfer to a core's
sound-data input, which runs at 48 kHz; that is the rate written here. The
ADPCM samples carry their own rate in the bank.
"""

import argparse
import array
import bisect
import collections
import pathlib
import struct
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import adpcm  # noqa: E402
import image  # noqa: E402
import scei  # noqa: E402
import wav  # noqa: E402

SECTOR = 2048
PCM_RATE = 48000        # SPU2 core sound-data input
PCM_BLOCK = 1024        # 256 samples left, then 256 right

# SQ_LOAD tables (snddtbl.cpp), in the order sqDataField and the scene code
# reach them.
SQ_TABLES = ("typeA", "typeC", "typeE", "typeG", "typeI", "typeK", "typeM", "typeO",
             "piroshi", "sqDataDungeon", "sqDataTown", "sqDataTitle", "sqDataDesktop",
             "sqDataToppage", "sqDataEvent", "sqDataStream")

# WaveData.category -> SQ_LOAD table, from the jump table at 0x0034e130 in
# ccSndChangeData (0x001834e0). 3 goes through sqDataField[fieldType].
CATEGORY_TABLE = {1: "sqDataDesktop", 2: "sqDataTown", 3: "sqDataField",
                  4: "sqDataDungeon", 5: "sqDataEvent", 6: "sqDataStream",
                  7: "sqDataTitle"}

# The functions that pick a field voice: each case loads its tables' bases
# (lui/addiu) and stores `voiceFile` (ccVoiceRequest INF 0x0017eeb0,
# ccVoicePgFood 0x001800e0). SPCnn is voiceData[nn] with spcVoiceData[nn],
# from skillVoicePlay (0x0017e350); the event voices are event_tables'.
FIELD_REQUESTS = ("ccVoiceRequest__Fii", "ccVoicePgFood__Fi")
# From Mutation on a case of party, Spc talk or present voices also asks
# for Mia's English lines, `voiceFile` 20 (evVoiceFileE[20], MIAE.BIN).
MIA_FILE = 20


def _disc_path(name):
    """'cdrom0:\\VOICE_E\\FOUNT_E.BIN' -> 'VOICE_E/FOUNT_E.BIN'"""
    return name.split(":", 1)[-1].lstrip("\\").replace("\\", "/")


class Program:
    """An executable with gcmn loaded (the voice tables) and desktop on the
    side (the jukebox titles). A later volume's carries Infection's names
    with Infection's sizes, so its tables are sized by its own layout
    against Infection's (`rows`), as piney-gen's placement::sound does."""

    def __init__(self, elf):
        import volume
        self.elf = elf
        self.gcmn = image.Program(elf, "gcmn")
        later = pathlib.Path(elf).resolve() != pathlib.Path(volume.INF_ELF).resolve()
        self.ref = image.Program(volume.INF_ELF, "gcmn") if later else None

    def sym(self, name):
        s = self.gcmn.symbol_named(name)
        if s is None:
            raise KeyError(name)
        return s

    def rows(self, name, row):
        """How many `row`-byte rows the table has: its symbol's size, on a
        later volume as volume.carried_rows lays it out."""
        import volume
        s = self.sym(name)
        return s.size // row if self.ref is None else volume.carried_rows(self.gcmn, self.ref, name, row)

    def words(self, name, fmt="<I"):
        s = self.sym(name)
        n = struct.calcsize(fmt)
        return [struct.unpack_from(fmt, self.gcmn.read(s.value + n * i, n))
                for i in range(self.rows(name, n))]

    def strings(self, name):
        return [self.gcmn.cstr(p).decode("latin-1") if p else None
                for (p,) in self.words(name)]

    def snddata_path(self):
        return pathlib.Path(self.elf).resolve().parent / "DATA" / "SNDDATA.BIN"


# -- SNDDATA.BIN --------------------------------------------------------------

class Bank:
    __slots__ = ("index", "offset", "hd_size", "sq_sizes", "bd_size", "table_bd_size",
                 "refs", "titles", "hd")

    def __init__(self, offset, hd_size, sq_sizes, bd_size):
        self.index = None
        self.offset = offset
        self.hd_size = hd_size
        self.sq_sizes = sq_sizes
        self.table_bd_size = bd_size    # what the loading table says
        self.bd_size = bd_size          # what the Head chunk says, once read
        self.refs = []
        self.titles = []
        self.hd = None

    @property
    def sq_offsets(self):
        out = []
        p = self.offset + self.hd_size
        for n in self.sq_sizes:
            if n:
                out.append((p, n))
            p += n
        return out

    @property
    def bd_offset(self):
        # ccSQDataLoadCD: sectNum + (ofs + hdSize + sqSize + 2047) >> 11
        return (self.offset + self.hd_size + sum(self.sq_sizes) + SECTOR - 1) // SECTOR * SECTOR

    @property
    def end(self):
        return self.bd_offset + self.bd_size

    @property
    def name(self):
        return self.refs[0] if self.refs else f"bank{self.index}"


def load_banks(prog, snddata=None):
    """Every bank the executable can load, in file order, with the Head chunk
    read from SNDDATA.BIN when it is given (bytes)."""
    banks = {}
    ofs, hd, bd = (w for (w,) in prog.words("commseTbl")[:3])     # its padding grew on Outbreak
    banks[ofs] = Bank(ofs, hd, (0, 0, 0), bd)
    banks[ofs].refs.append("commseTbl")
    for t in SQ_TABLES:
        for row, (o, hd, s1, s2, s3, bd) in enumerate(prog.words(t, "<6i")):
            if o < 0 or hd <= 0:
                continue    # -1 rows are unused, and typeE/typeI end in a zero row
            b = banks.get(o)
            if b is None:
                b = banks[o] = Bank(o, hd, (s1, s2, s3), bd)
            elif (b.hd_size, b.sq_sizes) != (hd, (s1, s2, s3)):
                raise ValueError(f"{t}[{row}] disagrees with {b.refs[0]} about bank 0x{o:x}")
            b.refs.append(f"{t}[{row}]")
    out = sorted(banks.values(), key=lambda b: b.offset)
    for i, b in enumerate(out):
        b.index = i
        if snddata is not None:
            b.hd = scei.Hd(snddata[b.offset:b.offset + b.hd_size])
            b.bd_size = b.hd.bd_size
    _attach_titles(prog, out)
    return out


def _attach_titles(prog, banks):
    """The desktop jukebox's Wave[] (wavetbl.cpp): title, category, fieldType,
    bgNum, comment - the track names of the sequenced music."""
    try:
        desk = image.Program(prog.elf, "desktop")
    except (OSError, ValueError):
        return
    wave = desk.symbol_named("Wave")
    field = [desk.name_at(p) for (p,) in prog.words("sqDataField")]
    at = {t: prog.sym(t).value for t in SQ_TABLES}
    by_offset = {b.offset: b for b in banks}
    for i in range(wave.size // 24):
        title, cat, ftype, bgnum, comment, _ = struct.unpack_from(
            "<IiiiIi", desk.read(wave.value + 24 * i, 24))
        table = CATEGORY_TABLE.get(cat)
        if table is None:
            continue
        if table == "sqDataField":
            table = field[ftype]
        o = struct.unpack_from("<i", prog.gcmn.read(at[table] + 24 * bgnum, 4))[0]
        name = desk.cstr(title).decode("latin-1")
        note = desk.cstr(comment).decode("cp932", "replace") if comment else ""
        if o in by_offset:
            by_offset[o].titles.append(f"{name} ({note})" if note else name)


def export_bank(bank, snddata, outdir):
    """Write every VAG of `bank` as vag_NNN.wav under outdir, at its Vagi rate
    with the ADPCM's loop as a smpl chunk; return a list of (Vag, samples,
    loop, frames, problem)."""
    outdir.mkdir(parents=True, exist_ok=True)
    body = snddata[bank.bd_offset:bank.bd_offset + bank.bd_size]
    ranges = bank.hd.vag_ranges()
    rows = []
    for v in bank.hd.vags:
        a, b = ranges[v.index]
        s = adpcm.decode(body, a, b - a)
        problem = None
        if (s.loop is not None) != bool(v.loop):
            problem = f"Vagi loop={v.loop}, data loop={s.loop}"
        wav.write(outdir / f"vag_{v.index:03d}.wav", s.pcm, v.rate, 1, s.loop)
        rows.append((v, len(s.pcm), s.loop, s.frames, problem))
    return rows


# -- voice files ----------------------------------------------------------------

class VoiceFile:
    __slots__ = ("path", "language", "tables")

    def __init__(self, path, language):
        self.path = path            # disc path, VOICE/... or VOICE_E/...
        self.language = language    # "J" or "E"
        self.tables = []            # [(table name, [(row, ofs, siz)])]

    def lines(self):
        """Distinct (ofs, siz) lines, file order, with every table row naming each."""
        seen = {}
        for name, rows in self.tables:
            for row, o, s in rows:
                seen.setdefault((o, s), []).append(f"{name}[{row}]")
        return sorted(seen.items())


def _voice_stores(p, fn, voice_file):
    """[(voiceFile value, [(table base, reached rows or None)] built since
    the last store)] of function `fn` of Program `p`, in code order. A base
    built after the call to a talkNum getter is an alternative for the
    messages the two `slti` before it bound, its row the message plus the
    `addiu` after the call (piney-gen's voice_case): its reached rows are
    range(first + add, end + add)."""
    import mips
    f = p.symbol_named(fn)
    insns = [mips.decode(p.u32(va), va) for va in range(f.value, f.value + f.size, 4)]
    built = mips.track_lui(insns, p.gp)
    li, pending, out = {}, [], []
    slti, add = [], None
    for i in insns:
        w = i.word
        op, rs, rt = w >> 26, (w >> 21) & 31, (w >> 16) & 31
        imm = ((w & 0xFFFF) ^ 0x8000) - 0x8000
        if op == 0x09 and rs == 0:
            li[rt] = imm
        elif i.pc in built and i.mnemonic == "addiu":
            reached = None
            if add is not None:
                first, end = slti[-2:]
                reached = range(first + add, end + add)
                add = None
            pending.append((built[i.pc], reached))
        elif op == 0x09 and rs not in (0, rt) and add is not None:
            add = imm
        elif op == 0x0A:
            slti.append(imm)
        elif op == 0x03 and any(p.u32(i.target + 4 * k) >> 26 == 0x20 and p.u32(i.target + 4 * k) & 0xFFFF == 0x220C
                                for k in range(16)):
            add = 0     # a talkNum getter (lb +0x220c)
        elif op == 0x2B and rs == 28 and (p.gp + imm) & 0xFFFFFFFF == voice_file:
            out.append((0 if rt == 0 else li[rt], pending))
            pending, slti = [], []
    return out


def field_tables(prog):
    """[(voiceFile, language, table address, reached rows or None)] of the
    field voices as the volume's own code picks them (FIELD_REQUESTS). A
    case's store follows its Japanese then English base; from Mutation on a
    case may store three times, its Japanese table, Mia's (MIA_FILE,
    English: the rows its alternative reaches) and its English table."""
    out = []
    voice_file = prog.sym("voiceFile").value
    for fn in FIELD_REQUESTS:
        singles = collections.Counter()
        for file, bases in _voice_stores(prog.gcmn, fn, voice_file):
            if len(bases) == 2:
                out += [(file, "J", *bases[0]), (file, "E", *bases[1])]
                continue
            assert len(bases) == 1, (fn, file, bases)
            lang = "E" if file == MIA_FILE or singles[file] else "J"
            assert (bases[0][1] is not None) == (file == MIA_FILE), (fn, file, bases)
            singles[file] += 1
            out.append((file, lang, *bases[0]))
    return out


def voice_rows(prog, va, starts):
    """A voice table's rows (index, ofs, siz) that are lines: from `va`
    to the next table's start (`starts`, sorted), ending early at a row
    that is neither a line (2048-aligned, a size) nor -1, -1."""
    i = bisect.bisect_right(starts, va)
    end = starts[i] if i < len(starts) else va + 8 * 512
    rows = []
    for k in range((end - va) // 8):
        o, s = struct.unpack("<ii", prog.gcmn.read(va + 8 * k, 8))
        if not ((o >= 0 and o % SECTOR == 0 and s > 0) or (o, s) == (-1, -1)):
            break
        if o >= 0:
            rows.append((k, o, s))
    return rows


def event_messages(prog):
    """messages(kind, event): how many messages event `event` has in
    evMsgTbl (Parody Mode's evMsgTblp for the parody tables; evscript),
    with or without a script (Infection's side event 63 has none but a
    message and its line). ccEvVoiceRequest(event, msg) asks for row `msg`,
    so an event's rows past its messages are never read (Outbreak's
    EVVOL3SE event 266 array is one row in .sdata at 0x003862f8, the words
    after it other data)."""
    import evscript
    ev = evscript.Events(prog.elf)
    return lambda kind, event: len(ev.messages(event, kind == "parody"))


def load_voices(prog):
    """{disc path: VoiceFile} for every voice file the code's tables name:
    the Spc lines (voiceData / spcVoiceData), the field voices
    (field_tables; Mia's the rows its alternative reaches), each table's
    rows running to the next table's start, and the event voices
    (event_tables), each event's rows those of its messages
    (event_messages)."""
    spc = {lang: [(prog.gcmn.cstr(n).decode("latin-1") if n else None, t)
                  for (n,), (t,) in zip(prog.words("spcVoiceData" + sfx), prog.words("voiceData" + sfx))]
           for lang, sfx in (("J", ""), ("E", "E"))}
    field = field_tables(prog)
    events = event_tables(prog)
    arrays = {prog.gcmn.u32(t + 4 * k) for *_, t, ev in events for k, rows in enumerate(ev) if rows}
    starts = sorted({t for v in spc.values() for _, t in v if t} | {va for _, _, va, _ in field} |
                    {t for *_, t, _ in events} | arrays)
    paths = {lang: prog.strings("evVoiceFile" + sfx) for lang, sfx in (("J", ""), ("E", "E"))}
    files = {}

    def add(path, lang, va, rows=None):
        path = _disc_path(path)
        vf = files.setdefault(path, VoiceFile(path, lang))
        name = prog.gcmn.name_at(va) or f"0x{va:08x}"
        vf.tables.append((name, voice_rows(prog, va, starts) if rows is None else rows))

    for lang, rows in spc.items():
        for path, t in rows:
            if path and t:
                add(path, lang, t)
    for file, lang, va, reached in field:
        rows = voice_rows(prog, va, starts)
        add(paths[lang][file], lang, va, rows if reached is None else [r for r in rows if r[0] in reached])
    messages = event_messages(prog)
    for section, kind, lang, t, ev in events:
        file = 6 + 3 * section + EV_KINDS.index(kind)
        lang = "J" if lang == "jp" else "E"
        first = 100 * section + EV_FIRST[kind]
        for k, rows in enumerate(ev):
            if rows:
                a = prog.gcmn.u32(t + 4 * k)
                reach = rows[:messages(kind, first + k)]
                add(paths[lang][file], lang, a, [(i, o, s) for i, (o, s) in enumerate(reach) if o >= 0])
    return files


def open_image(path):
    import iso
    return iso.Iso(path)


def find(img, path):
    try:
        return img.find(path)
    except KeyError:
        return None


def read_file(img, path):
    e = find(img, path)
    if e is None:
        return None
    return img.read(e.lba, e.size)


def _safe(name):
    return name.replace("/", "_").replace("[", "_").replace("]", "")


# -- BGM.BIN ----------------------------------------------------------------------

def load_bgm(prog):
    """[(index, ofs, siz, mode)]: bgmWavTbl and bgmParam's mode word, row by
    row as wavPlay (INF 0x0017e6e0, MUT 0x00181750) indexes both by the
    track, the tracks with a size. Mode & 0xf nonzero loops. Infection has
    two tracks (bgmWavTbl in gcmn); from Mutation on both tables are in main
    with eight rows (MUT 0x0031e840 and 0x0031cce0)."""
    rows = zip(prog.words("bgmWavTbl", "<ii"), prog.words("bgmParam", "<iI"))
    return [(i, o, s, mode) for i, ((o, s), (mode, _)) in enumerate(rows) if s > 0]


# ccEvVoiceRequest's tables: kind by `voiceFile` (6 + 3 section main, + 1
# side, + 2 Parody Mode's) and each kind's first event in its hundred.
EV_KINDS = ("main", "side", "parody")
EV_FIRST = {"main": 0, "side": 50, "parody": 0}


def event_tables(prog):
    """ccEvVoiceRequest's tables as the volume's own code picks them, by
    `event / 100`: [(section, kind, lang, start, [rows or None per event])].
    Every lui/addiu base before a `voiceFile` store is that file's, Japanese
    first, English (saveData.voice) second; a table is its base plus 4 times
    its first event and runs to the next table (at most 50 events), ending
    early at a word that is neither NULL nor an address. An event's rows
    run to the next array or table (at most 128) and end at the first that
    is neither a line (2048-aligned, a size) nor -1, -1."""
    p = prog.gcmn
    f = prog.sym("ccEvVoiceRequest__Fii")
    voice_file = prog.sym("voiceFile").value
    hi, pending, li, stores = {}, [], None, []
    for k in range(f.size // 4):
        w = p.u32(f.value + 4 * k)
        op, rs, rt = w >> 26, (w >> 21) & 31, (w >> 16) & 31
        imm = ((w & 0xFFFF) ^ 0x8000) - 0x8000
        if op == 0x0F:
            hi[rt] = (w & 0xFFFF) << 16
        elif op == 0x09 and rs == 0:
            li = imm
        elif op == 0x09 and rs == rt and rs in hi:
            pending.append((hi.pop(rs) + imm) & 0xFFFFFFFF)
        elif op == 0x2B and rs == 28 and (p.gp + imm) & 0xFFFFFFFF == voice_file:
            stores.append((li, pending))
            pending = []
    tables = []
    for file, bases in stores:
        section, kind = divmod(file - 6, 3)
        first = 100 * section + EV_FIRST[EV_KINDS[kind]]
        for lang, base in zip(("jp", "en"), bases):
            tables.append((section, EV_KINDS[kind], lang, (base + 4 * first) & 0xFFFFFFFF))
    starts = sorted({t[3] for t in tables})

    def bound(edges, a):
        i = bisect.bisect_right(edges, a)
        return edges[i] if i < len(edges) else None

    entries = {}
    for t in starts:
        b = bound(starts, t)
        out = []
        for i in range(50 if b is None else min((b - t) // 4, 50)):
            a = p.u32(t + 4 * i)
            if a and not p.mapped(a):
                break
            out.append(a)
        entries[t] = out
    edges = sorted({a for e in entries.values() for a in e if a} | set(starts))
    result = []
    for section, kind, lang, t in tables:
        events = []
        for a in entries[t]:
            if not a:
                events.append(None)
                continue
            b = bound(edges, a)
            rows = []
            for i in range(1 if b is None else min((b - a) // 8, 128)):
                ofs, siz = struct.unpack("<ii", p.read(a + 8 * i, 8))
                if not ((ofs >= 0 and ofs % SECTOR == 0 and siz > 0) or (ofs, siz) == (-1, -1)):
                    break
                rows.append((ofs, siz))
            events.append(rows or None)
        result.append((section, kind, lang, t, events))
    return result


# -- cutscene PCM ---------------------------------------------------------------

def stream_pcm(data, lang=None):
    """(CCSTRM_PCM header dict, [1024-byte blocks], frames) out of an inflated
    CCSF stream file, in the order ccStream::Decode_Pcm (0x0014ddb0) and
    DecodeF_Pcm (0x0014e4a0) hand blocks to ccPcmSound. With `lang`, as
    Outbreak's and Quarantine's decoders read them (Decode_Pcm OUT
    0x0014cab0, DecodeF_Pcm 0x0014d240): each Pcm chunk and F_Pcm record
    carries a voice language (the Pcm chunk's byte after the track; an
    F_Pcm record's u8 lang, then u32 blocks and words), and only `lang`'s
    are played."""
    import ccs
    c = ccs.Ccs(data)
    head = None
    blocks = []
    frames = 0
    for off, t, n, end in c.chunks():
        if t is None:
            break
        kind = t & 0xFFFF
        if kind == 0x2200:
            my_id, typ, bits, track, voice, num, size = struct.unpack_from("<IBBBBII", data, off + 8)
            if lang is not None and voice != lang:
                continue
            head = {"id": my_id, "type": typ, "bits": bits, "track": track,
                    "blocks": num, "words": size}
            if lang is not None:
                head["lang"] = voice
            p = off + 8 + 16
        elif kind == 0x2201 and lang is None:
            my_id, num, size = struct.unpack_from("<IHxxI", data, off + 8)
            p = off + 8 + 12
        elif kind == 0x2201:
            my_id, voice, num, size = struct.unpack_from("<IBxxxII", data, off + 8)
            if voice != lang:
                continue
            p = off + 8 + 16
        else:
            if kind == 0xFF01 and struct.unpack_from("<i", data, off + 8)[0] >= 0:
                frames += 1
            continue
        if size * 4 != PCM_BLOCK:
            raise ValueError(f"PCM block of {size} words at 0x{off:x}")
        for i in range(num):
            blocks.append(data[p + i * PCM_BLOCK:p + (i + 1) * PCM_BLOCK])
    return head, blocks, frames


def blocks_to_stereo(blocks):
    """Interleave SPU2 input blocks into LRLR frames. SEWORDS.IRX copies the
    blocks untouched into the buffer it hands sceSdBlockTrans (CcspcmBuffCopy,
    0x3ed0), and that buffer is 512 bytes of left then 512 of right per 1024 -
    the layout its own stereo converter writes (bgm_r2s.s, 0x3df0-0x3e28)."""
    out = array.array("h", bytes(4 * 256 * len(blocks)))
    for i, b in enumerate(blocks):
        a = array.array("h", b)
        if sys.byteorder != "little":
            a.byteswap()
        base = i * 512
        out[base:base + 512:2] = a[:256]
        out[base + 1:base + 512:2] = a[256:]
    return out


# -- CLI --------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("banks")
    p.add_argument("elf")
    p = sub.add_parser("bank")
    p.add_argument("elf")
    p.add_argument("n", type=int)
    p.add_argument("--programs", action="store_true")
    p.add_argument("--samples", action="store_true")
    p = sub.add_parser("export-banks")
    p.add_argument("elf")
    p.add_argument("outdir")
    p.add_argument("--bank", type=int)
    p = sub.add_parser("voices")
    p.add_argument("elf")
    p.add_argument("image")
    p.add_argument("--file")
    p = sub.add_parser("export-voices")
    p.add_argument("elf")
    p.add_argument("image")
    p.add_argument("outdir")
    p.add_argument("--file")
    p.add_argument("--limit", type=int)
    p = sub.add_parser("bgm")
    p.add_argument("elf")
    p.add_argument("image")
    p.add_argument("--export")
    p = sub.add_parser("stream")
    p.add_argument("image")
    p.add_argument("archive")
    p.add_argument("member")
    p.add_argument("out")
    p = sub.add_parser("streams")
    p.add_argument("image")
    args = parser.parse_args()

    if args.cmd in ("stream", "streams"):
        return _cmd_streams(args)
    prog = Program(args.elf)
    if args.cmd in ("banks", "bank", "export-banks"):
        return _cmd_banks(args, prog)
    if args.cmd == "bgm":
        return _cmd_bgm(args, prog)
    return _cmd_voices(args, prog)


def _cmd_banks(args, prog):
    snd = prog.snddata_path().read_bytes()
    banks = load_banks(prog, snd)
    if args.cmd == "banks":
        print(f"{'#':>2} {'offset':>9} {'hd':>6} {'sq':>6} {'bd@':>9} {'bd':>8} "
              f"{'vag':>4} {'prog':>4}  loaded by")
        for b in banks:
            note = ""
            if b.bd_size != b.table_bd_size:
                note = f"  [table bd size 0x{b.table_bd_size:x}, Head says 0x{b.bd_size:x}]"
            print(f"{b.index:2d} 0x{b.offset:07x} {b.hd_size:6d} {sum(b.sq_sizes):6d} "
                  f"0x{b.bd_offset:07x} {b.bd_size:8d} {len(b.hd.vags):4d} "
                  f"{len(b.hd.programs):4d}  {', '.join(b.refs)}{note}")
            for t in b.titles:
                print(f"{'':44}jukebox: {t}")
        end = (banks[-1].end + SECTOR - 1) // SECTOR * SECTOR
        print(f"# {len(banks)} banks, {sum(len(b.hd.vags) for b in banks)} VAGs; "
              f"the last ends at 0x{end:x}, the file is 0x{len(snd):x} bytes")
        return 0
    if args.cmd == "bank":
        b = banks[args.n]
        print(f"bank {b.index}: 0x{b.offset:x}, hd {b.hd_size}, sq {b.sq_offsets}, "
              f"bd 0x{b.bd_offset:x} + {b.bd_size}; {', '.join(b.refs)}")
        for off, n in b.sq_offsets:
            sq = scei.Sq(snd[off:off + n])
            print(f"  sq at 0x{off:x}: {sq.size} bytes, {len(sq.midi)} midi blocks")
        for line in scei.describe(b.hd, args.programs, args.samples):
            print(line)
        return 0
    out = pathlib.Path(args.outdir)
    bad = total = 0
    for b in banks:
        if args.bank is not None and b.index != args.bank:
            continue
        rows = export_bank(b, snd, out / f"bank{b.index:02d}_{_safe(b.name)}")
        total += len(rows)
        for v, n, loop, frames, problem in rows:
            if problem:
                bad += 1
                print(f"bank {b.index} vag {v.index}: {problem}")
        print(f"bank {b.index:2d} {b.name}: {len(rows)} VAGs")
    print(f"# {total} VAGs written under {out}, {bad} loop disagreements")
    return 0


def _cmd_voices(args, prog):
    img = open_image(args.image)
    files = load_voices(prog)
    grand = 0
    for path in sorted(files):
        if args.file and path.upper() != args.file.upper():
            continue
        vf = files[path]
        e = find(img, path)
        lines = vf.lines()
        secs = sum(s for (o, s), _ in lines) / 2 / PCM_RATE
        if e:
            grand += len(lines)
        state = f"{e.size} bytes" if e else "NOT ON DISC"
        print(f"{path:24s} {state:>16}  {len(lines):4d} lines  {secs:8.1f} s  "
              f"tables: {', '.join(n for n, _ in vf.tables)}")
        if args.cmd == "voices" and args.file:
            for (o, s), names in lines:
                print(f"    0x{o:08x} {s:8d}  {s / 2 / PCM_RATE:6.2f} s  {', '.join(names)}")
        if args.cmd == "export-voices" and e:
            raw = img.read(e.lba, e.size)
            out = pathlib.Path(args.outdir) / path.replace("/", "_").rsplit(".", 1)[0]
            out.mkdir(parents=True, exist_ok=True)
            for (o, s), names in lines[:args.limit]:
                wav.write(out / f"{o // SECTOR:06d}_{_safe(names[0])}.wav",
                          raw[o:o + s], PCM_RATE, 1)
    print(f"# {grand} distinct lines in the files on the disc")
    return 0


def _cmd_bgm(args, prog):
    img = open_image(args.image)
    raw = read_file(img, "VOICE/BGM.BIN")
    for i, o, s, mode in load_bgm(prog):
        print(f"track {i}: 0x{o:08x} {s:9d} bytes  {s / 4 / PCM_RATE:7.2f} s stereo  "
              f"mode {mode} ({'loops' if mode & 0xF else 'once'})")
        if args.export:
            out = pathlib.Path(args.export)
            out.mkdir(parents=True, exist_ok=True)
            wav.write(out / f"bgm_{i}.wav", raw[o:o + s], PCM_RATE, 2)
    return 0


def _cmd_streams(args):
    img = open_image(args.image)
    import gzarc
    if args.cmd == "stream":
        data = read_file(img, f"STREAM/{args.archive.upper()}.BIN")
        want = args.member.lower()
        for m in gzarc.members(data):
            if m.name.lower() in (want, want + ".tmp"):
                head, blocks, frames = stream_pcm(gzarc.inflate(data, m))
                break
        else:
            print(f"no member {args.member} in {args.archive}")
            return 1
        if head is None:
            print(f"{args.member} has no Pcm chunk")
            return 1
        wav.write(args.out, blocks_to_stereo(blocks), PCM_RATE, 2)
        n = len(blocks) * 256
        print(f"{head}; {len(blocks)} blocks, {n} frames of stereo, {n / PCM_RATE:.2f} s, "
              f"{frames} stream frames, {n / max(frames, 1):.1f} samples per frame")
        return 0
    total = collections.Counter()
    for arc in ("STRCMN", "STRCMNE", "STR1", "STR1E"):
        data = read_file(img, f"STREAM/{arc}.BIN")
        for m in gzarc.members(data):
            head, blocks, frames = stream_pcm(gzarc.inflate(data, m))
            if head is None:
                continue
            n = len(blocks) * 256
            total["files"] += 1
            total["blocks"] += len(blocks)
            print(f"{arc}::{m.name:16s} {frames:5d} frames {len(blocks):6d} blocks "
                  f"{n / PCM_RATE:7.2f} s  {n / max(frames, 1):7.1f} samples/frame  "
                  f"bits {head['bits']} track {head['track']} prefill {head['blocks']}")
    print(f"# {total['files']} cutscenes with PCM, {total['blocks']} blocks, "
          f"{total['blocks'] * 256 / PCM_RATE:.1f} s")
    return 0


if __name__ == "__main__":
    sys.exit(main())
