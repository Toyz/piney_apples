#!/usr/bin/env python3
"""Sony's SCEI sound formats as the PS2 hardware synthesizer (MODHSYN.IRX) and
MIDI sequencer (MODMIDI.IRX) take them: the .hd bank header, whose samples
live in a separate .bd body of PS-ADPCM, and the .sq sequence file.

    tools/scei.py hd FILE [--offset N] [--programs] [--samples]   a bank header
    tools/scei.py sq FILE [--offset N]                           a sequence file

Every chunk starts with the creator and type as little-endian u32s, so the
bytes read backwards: "IECSsreV" is SCEI/Vers.

    u32 creator 'SCEI'   u32 type   u32 chunk size   ...

.hd: Vers (16 bytes), then Head (64 bytes), which gives the .hd and .bd sizes
and the offsets of four chunks, each a count and a table of offsets relative
to the chunk (0xffffffff = unused slot):

    Vagi  u32 max index, u32 offs[max+1] -> u32 bd offset, u16 rate,
          u8 loop (1 = the ADPCM loops), u8 0xff
    Smpl  u32 max index, u32 offs[max+1] -> 42-byte sample: u16 vag index,
          velocity/key ranges, base note (+0x0b), pan, volume, ADSR1/2 (+0x12)
    Sset  u32 max index, u32 offs[max+1] -> u8 velocity curve, u8 vel low,
          u8 vel high, u8 n, u16 sample[n]
    Prog  u32 max index, u32 offs[max+1] -> 36-byte program (split table
          offset, split count, split size, volume, pan, ...) then 20-byte
          splits: u16 sample set, u8 key low, u8 key crossfade, u8 key high, ...

.sq: Vers, then Sequ (u32 file size, u32 song, midi, se-sequence, se-song
chunk offsets), then Midi (u32 max index, u32 offs[max+1]).

Field names after the ones on the Head, Vagi, Sset and split tables are the
SDK's; the fields this tool relies on are checked against every bank on the
disc by tools/test_sound.py. Also a library: `Hd(data)`, `Sq(data)`.
"""

import argparse
import pathlib
import struct
import sys

NONE = 0xFFFFFFFF

SAMPLE_FIELDS = (
    ("vag", "H"), ("vel_low", "B"), ("vel_crossfade", "B"), ("vel_high", "B"),
    ("vel_follow_pitch", "b"), ("vel_follow_pitch_center", "B"),
    ("vel_follow_pitch_curve", "B"), ("vel_follow_amp", "b"),
    ("vel_follow_amp_center", "B"), ("vel_follow_amp_curve", "B"),
    ("base_note", "B"), ("detune", "b"), ("pan", "b"), ("group", "B"),
    ("priority", "B"), ("volume", "B"), ("reserved", "B"),
    ("adsr1", "H"), ("adsr2", "H"),
    ("kf_ar", "b"), ("kf_ar_center", "B"), ("kf_dr", "b"), ("kf_dr_center", "B"),
    ("kf_sr", "b"), ("kf_sr_center", "B"), ("kf_rr", "b"), ("kf_rr_center", "B"),
    ("kf_sl", "b"), ("kf_sl_center", "B"),
    ("pitch_lfo_delay", "H"), ("pitch_lfo_fade", "H"),
    ("amp_lfo_delay", "H"), ("amp_lfo_fade", "H"),
    ("lfo_attr", "B"), ("spu_attr", "B"),
)
SAMPLE_FMT = "<" + "".join(t for _, t in SAMPLE_FIELDS)
SAMPLE_SIZE = struct.calcsize(SAMPLE_FMT)   # 42

PROGRAM_FIELDS = (
    ("split_offset", "I"), ("n_split", "B"), ("split_size", "B"), ("volume", "B"),
    ("pan", "b"), ("transpose", "b"), ("detune", "b"), ("key_follow_pan", "b"),
    ("key_follow_pan_center", "B"), ("attr", "B"), ("reserved", "B"),
    ("lfo_wave", "B"), ("lfo_wave2", "B"), ("lfo_phase", "B"), ("lfo_phase2", "B"),
    ("lfo_random", "B"), ("lfo_random2", "B"), ("lfo_cycle", "H"), ("lfo_cycle2", "H"),
    ("lfo_pitch_depth", "h"), ("lfo_pitch_depth2", "h"),
    ("lfo_midi_pitch_depth", "h"), ("lfo_midi_pitch_depth2", "h"),
    ("lfo_amp_depth", "b"), ("lfo_amp_depth2", "b"),
    ("lfo_midi_amp_depth", "b"), ("lfo_midi_amp_depth2", "b"),
)
PROGRAM_FMT = "<" + "".join(t for _, t in PROGRAM_FIELDS)
PROGRAM_SIZE = struct.calcsize(PROGRAM_FMT)   # 36

SPLIT_FIELDS = (
    ("sample_set", "H"), ("key_low", "B"), ("key_crossfade", "B"), ("key_high", "B"),
    ("number", "B"), ("bend_low", "H"), ("bend_high", "H"),
    ("key_follow_pitch", "b"), ("key_follow_pitch_center", "B"),
    ("key_follow_amp", "b"), ("key_follow_amp_center", "B"),
    ("key_follow_pan", "b"), ("key_follow_pan_center", "B"),
    ("volume", "B"), ("pan", "b"), ("transpose", "b"), ("detune", "b"),
)
SPLIT_FMT = "<" + "".join(t for _, t in SPLIT_FIELDS)
SPLIT_SIZE = struct.calcsize(SPLIT_FMT)   # 20


class Record:
    """A parsed struct whose fields are attributes; `index` is its table slot."""

    def __init__(self, index, names, values):
        self.index = index
        for n, v in zip(names, values):
            setattr(self, n, v)

    def __repr__(self):
        return f"<{self.__class__.__name__} {self.index} {vars(self)}>"


class Vag(Record):
    pass


class SampleParam(Record):
    pass


class SampleSet(Record):
    pass


class Program(Record):
    pass


class Split(Record):
    pass


def chunk_at(data, p):
    """(creator, type, size) with the four-character codes the right way round."""
    creator, kind, size = struct.unpack_from("<4s4sI", data, p)
    return creator[::-1].decode("latin-1"), kind[::-1].decode("latin-1"), size


def _table(data, p, want):
    creator, kind, size = chunk_at(data, p)
    if (creator, kind) != ("SCEI", want):
        raise ValueError(f"expected SCEI/{want} at 0x{p:x}, found {creator}/{kind}")
    top = struct.unpack_from("<I", data, p + 12)[0]
    offs = struct.unpack_from(f"<{top + 1}I", data, p + 16)
    return size, [None if o == NONE else p + o for o in offs]


class Hd:
    def __init__(self, data):
        self.data = data
        c, k, _ = chunk_at(data, 0)
        if (c, k) != ("SCEI", "Vers"):
            raise ValueError(f"not an SCEI .hd: starts {c}/{k}")
        self.version = (data[14], data[15])
        c, k, _ = chunk_at(data, 16)
        if (c, k) != ("SCEI", "Head"):
            raise ValueError(f"no Head chunk: {c}/{k}")
        (self.hd_size, self.bd_size, prog, sset, smpl, vagi,
         self.se_timbre) = struct.unpack_from("<7I", data, 16 + 12)
        self.chunks = {"Prog": prog, "Sset": sset, "Smpl": smpl, "Vagi": vagi}

        self.vags = []
        if vagi != NONE:
            _, offs = _table(data, vagi, "Vagi")
            for i, o in enumerate(offs):
                if o is not None:
                    self.vags.append(Vag(i, ("offset", "rate", "loop", "reserved"),
                                         struct.unpack_from("<IHBB", data, o)))
        self.samples = []
        if smpl != NONE:
            _, offs = _table(data, smpl, "Smpl")
            for i, o in enumerate(offs):
                if o is not None:
                    self.samples.append(SampleParam(
                        i, [n for n, _ in SAMPLE_FIELDS], struct.unpack_from(SAMPLE_FMT, data, o)))
        self.samplesets = []
        if sset != NONE:
            _, offs = _table(data, sset, "Sset")
            for i, o in enumerate(offs):
                if o is not None:
                    curve, lo, hi, n = struct.unpack_from("<4B", data, o)
                    self.samplesets.append(SampleSet(
                        i, ("vel_curve", "vel_low", "vel_high", "samples"),
                        (curve, lo, hi, list(struct.unpack_from(f"<{n}H", data, o + 4)))))
        self.programs = []
        if prog != NONE:
            _, offs = _table(data, prog, "Prog")
            for i, o in enumerate(offs):
                if o is None:
                    continue
                pr = Program(i, [n for n, _ in PROGRAM_FIELDS],
                             struct.unpack_from(PROGRAM_FMT, data, o))
                pr.splits = [Split(j, [n for n, _ in SPLIT_FIELDS],
                                   struct.unpack_from(SPLIT_FMT, data,
                                                      o + pr.split_offset + j * pr.split_size))
                             for j in range(pr.n_split)]
                self.programs.append(pr)

    def vag_by_index(self):
        return {v.index: v for v in self.vags}

    def vag_ranges(self):
        """{vag index: (bd offset, bd end)}: each VAG runs to the next one's
        start, the last to the end of the body."""
        order = sorted(self.vags, key=lambda v: v.offset)
        out = {}
        for a, b in zip(order, order[1:] + [None]):
            out[a.index] = (a.offset, b.offset if b else self.bd_size)
        return out


class Sq:
    def __init__(self, data):
        self.data = data
        c, k, _ = chunk_at(data, 0)
        if (c, k) != ("SCEI", "Vers"):
            raise ValueError(f"not an SCEI .sq: starts {c}/{k}")
        c, k, _ = chunk_at(data, 16)
        if (c, k) != ("SCEI", "Sequ"):
            raise ValueError(f"no Sequ chunk: {c}/{k}")
        self.size, song, midi, se_seq, se_song = struct.unpack_from("<5I", data, 16 + 12)
        self.chunks = {"Song": song, "Midi": midi, "SeSequence": se_seq, "SeSong": se_song}
        self.midi = []
        if midi != NONE:
            _, offs = _table(data, midi, "Midi")
            self.midi = [o for o in offs if o is not None]


def describe(hd, programs=False, samples=False):
    """Lines of text: the header, every VAG, and optionally samples and programs."""
    yield (f"hd {hd.hd_size} bytes, bd {hd.bd_size} bytes, version {hd.version}, "
           f"{len(hd.vags)} vags, {len(hd.samples)} samples, "
           f"{len(hd.samplesets)} sample sets, {len(hd.programs)} programs")
    ranges = hd.vag_ranges()
    for v in hd.vags:
        a, b = ranges[v.index]
        yield (f"  vag {v.index:3d}  bd 0x{a:06x}-0x{b:06x}  {v.rate:5d} Hz  "
               f"{'loop' if v.loop else 'one-shot'}")
    if samples:
        for s in hd.samples:
            yield (f"  sample {s.index:3d}  vag {s.vag:3d}  vel {s.vel_low}-{s.vel_high}  "
                   f"base note {s.base_note}  pan {s.pan}  vol {s.volume}  "
                   f"adsr 0x{s.adsr1:04x} 0x{s.adsr2:04x}")
    if programs:
        sets = {x.index: x for x in hd.samplesets}
        for pr in hd.programs:
            yield f"  program {pr.index:3d}  vol {pr.volume}  pan {pr.pan}  {pr.n_split} splits"
            for sp in pr.splits:
                ss = sets.get(sp.sample_set)
                yield (f"      keys {sp.key_low:3d}-{sp.key_high:3d}  set {sp.sample_set:3d} "
                       f"-> samples {ss.samples if ss else '?'}")


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("hd")
    p.add_argument("file")
    p.add_argument("--offset", type=lambda s: int(s, 0), default=0)
    p.add_argument("--programs", action="store_true")
    p.add_argument("--samples", action="store_true")
    p = sub.add_parser("sq")
    p.add_argument("file")
    p.add_argument("--offset", type=lambda s: int(s, 0), default=0)
    args = parser.parse_args()

    data = pathlib.Path(args.file).read_bytes()[args.offset:]
    if args.cmd == "sq":
        sq = Sq(data)
        print(f"sq {sq.size} bytes, chunks "
              + ", ".join(f"{k}=0x{v:x}" for k, v in sq.chunks.items() if v != NONE)
              + f", {len(sq.midi)} midi blocks")
        return 0
    for line in describe(Hd(data), args.programs, args.samples):
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
