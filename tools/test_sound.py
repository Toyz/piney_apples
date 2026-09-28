#!/usr/bin/env python3
"""Tests for tools/adpcm.py, tools/wav.py, tools/scei.py and tools/sound.py.
The decoder and the WAV writer are checked on hand-made data; the rest against
the retail .hack//Infection disc, skipped when work/infection/disc/SLUS_202.67
(with DATA/SNDDATA.BIN and the overlays) or work/infection/infection.iso is
absent.
Run: python3 tools/test_sound.py"""

import array
import collections
import os
import tempfile
import unittest

import adpcm
import scei
import wav

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "work", "infection")
ELF = os.path.join(ROOT, "disc", "SLUS_202.67")
SNDDATA = os.path.join(ROOT, "disc", "DATA", "SNDDATA.BIN")
ISO = os.path.join(ROOT, "infection.iso")
HAVE_ELF = all(os.path.exists(p) for p in (ELF, SNDDATA)) and os.path.exists(
    os.path.join(ROOT, "disc", "DATA", "GCMN.PRG"))


def frame(shift, filt, flags, nibbles):
    """A 16-byte PS-ADPCM frame from 28 signed nibbles, low nibble first."""
    assert len(nibbles) == 28
    body = bytes((a & 15) | (b & 15) << 4 for a, b in zip(nibbles[0::2], nibbles[1::2]))
    return bytes((shift | filt << 4, flags)) + body


class AdpcmTest(unittest.TestCase):
    def test_filter0_shift12_is_the_nibbles(self):
        nib = [1, 2, -1, -8, 7, 0] + [3] * 22
        s = adpcm.decode(frame(12, 0, adpcm.END, nib))
        self.assertEqual(list(s.pcm), nib)
        self.assertEqual((s.frames, s.loop), (1, None))

    def test_filter1_by_hand(self):
        # shift 0: the nibble is worth nibble << 12. Filter 1 adds 60/64 of s1,
        # rounded by +32 then >> 6: 4096, (4096*60+32)>>6 = 3840, 3600, 3375.
        s = adpcm.decode(frame(0, 1, adpcm.END, [1] + [0] * 27))
        self.assertEqual(list(s.pcm[:4]), [4096, 3840, 3600, 3375])

    def test_filter2_negative_nibble_and_clamp(self):
        # nibble -1, shift 4 -> -4096 >> 4 = -256; filter 2 is (115, -52).
        s = adpcm.decode(frame(4, 2, adpcm.END, [-1, -1, -1] + [0] * 25))
        a = -256
        b = -256 + ((a * 115 + 32) >> 6)
        c = -256 + ((b * 115 + a * -52 + 32) >> 6)
        self.assertEqual(list(s.pcm[:3]), [a, b, c])
        # 7 << 12 = 28672 twice with filter 4 overflows and clamps.
        s = adpcm.decode(frame(0, 4, adpcm.END, [7, 7] + [0] * 26))
        self.assertEqual(list(s.pcm[:2]), [28672, 32767])
        self.assertEqual(s.clipped, s.pcm.tolist().count(32767) + s.pcm.tolist().count(-32768))

    def test_shift_13_acts_as_9(self):
        a = adpcm.decode(frame(13, 0, adpcm.END, [5] * 28)).pcm
        b = adpcm.decode(frame(9, 0, adpcm.END, [5] * 28)).pcm
        self.assertEqual(a, b)
        self.assertEqual(a[0], (5 << 12) >> 9)

    def test_one_shot_stops_at_the_end_frame(self):
        # Every one-shot on the disc ends with a 0x01 frame, then a 0x07 frame
        # of 0x77 nibbles that is never played.
        data = (frame(12, 0, 0, [1] * 28) + frame(12, 0, adpcm.END, [2] * 28)
                + bytes.fromhex("0007") + b"\x77" * 14)
        s = adpcm.decode(data)
        self.assertEqual((len(s.pcm), s.frames, s.loop), (56, 2, None))
        self.assertEqual(s.flags, {0, 1})

    def test_loop_points(self):
        data = (frame(12, 0, 0, [0] * 28)
                + frame(12, 0, adpcm.LOOP_START | adpcm.REPEAT, [1] * 28)
                + frame(12, 0, adpcm.REPEAT, [2] * 28)
                + frame(12, 0, adpcm.END | adpcm.REPEAT, [3] * 28))
        s = adpcm.decode(data)
        self.assertEqual((len(s.pcm), s.frames, s.loop), (112, 4, (28, 112)))

    def test_limit_and_history_carry_across_frames(self):
        data = frame(0, 1, 0, [1] + [0] * 27) + frame(0, 1, adpcm.END, [0] * 28)
        s = adpcm.decode(data)
        # sample 28 continues the decay: s1 = pcm[27], s2 = pcm[26]
        self.assertEqual(s.pcm[28], (s.pcm[27] * 60 + 32) >> 6)
        self.assertEqual(len(adpcm.decode(data, 0, 16).pcm), 28)
        self.assertEqual(len(adpcm.decode_frames(data)), 56)


class WavTest(unittest.TestCase):
    def test_round_trip_with_loop(self):
        pcm = array.array("h", [0, 1, -1, 32767, -32768, 5])
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "t.wav")
            wav.write(p, pcm, 22050, 1, (2, 6))
            rate, ch, data, loop = wav.read(p)
        self.assertEqual((rate, ch, loop), (22050, 1, (2, 6)))
        self.assertEqual(array.array("h", data).tolist(), pcm.tolist())

    def test_stereo_frames_only(self):
        with self.assertRaises(ValueError):
            wav.encode(b"\0\0\0\0\0\0", 48000, 2)


@unittest.skipUnless(HAVE_ELF, "needs work/infection/disc/SLUS_202.67 and DATA/")
class SndDataTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import sound
        cls.sound = sound
        cls.prog = sound.Program(ELF)
        with open(SNDDATA, "rb") as f:
            cls.snd = f.read()
        cls.banks = sound.load_banks(cls.prog, cls.snd)

    def test_banks_tile_the_file(self):
        self.assertEqual(len(self.banks), 79)
        end = 0
        for b in self.banks:
            self.assertEqual(b.offset, end, b.name)     # every bank starts where the last ended
            self.assertEqual(b.hd.hd_size, b.hd_size)
            end = (b.end + 2047) // 2048 * 2048
        self.assertEqual(end, len(self.snd))

    def test_one_table_row_has_the_wrong_bd_size(self):
        wrong = [(b.name, b.table_bd_size, b.bd_size) for b in self.banks
                 if b.table_bd_size != b.bd_size]
        self.assertEqual(wrong, [("typeI[2]", 0x5CBF0, 0x4EC30)])

    def test_header_tables_hang_together(self):
        c = collections.Counter()
        for b in self.banks:
            hd = b.hd
            vags = {v.index for v in hd.vags}
            samples = {s.index for s in hd.samples}
            sets = {s.index for s in hd.samplesets}
            c["vags"] += len(vags)
            c["samples"] += len(samples)
            c["sets"] += len(sets)
            c["programs"] += len(hd.programs)
            c["bad"] += sum(s.vag not in vags for s in hd.samples)
            c["bad"] += sum(x not in samples for s in hd.samplesets for x in s.samples)
            c["bad"] += sum(sp.sample_set not in sets for p in hd.programs for sp in p.splits)
            c["bad"] += sum((p.split_offset, p.split_size) != (0x24, 0x14) for p in hd.programs)
            offs = [v.offset for v in hd.vags]
            c["bad"] += offs != sorted(set(offs))
        self.assertEqual(dict(c), {"vags": 1013, "samples": 1402, "sets": 1398,
                                   "programs": 926, "bad": 0})

    def test_vagi_loop_flag_matches_the_adpcm(self):
        # Scan flag bytes only (decoding all 1013 takes seconds): the Vagi loop
        # byte is 1 exactly when the sample ends with end+repeat, and every
        # one-shot is followed by the same 0x07 frame.
        kinds = collections.Counter()
        tail = bytes.fromhex("0007") + b"\x77" * 14
        for b in self.banks:
            body = self.snd[b.bd_offset:b.bd_offset + b.bd_size]
            for v in b.hd.vags:
                a, e = b.hd.vag_ranges()[v.index]
                self.assertEqual(body[a:a + 16], bytes(16))
                p = a
                while not body[p + 1] & adpcm.END:
                    p += 16
                end_flags = body[p + 1]
                rest = body[p + 16:e]
                kinds[(v.loop, end_flags, rest == tail if rest else None)] += 1
        self.assertEqual(dict(kinds), {(0, 1, True): 699, (1, 3, None): 314})

    def test_decode_one_bank(self):
        b = self.banks[1]      # sqDataTitle[0]
        body = self.snd[b.bd_offset:b.bd_offset + b.bd_size]
        total = 0
        for v in b.hd.vags:
            a, e = b.hd.vag_ranges()[v.index]
            s = adpcm.decode(body, a, e - a)
            self.assertEqual(s.reserved_filters, 0)
            self.assertEqual(s.loop is not None, bool(v.loop))
            total += len(s.pcm)
        self.assertEqual(len(b.hd.vags), 8)
        self.assertGreater(total, 0)

    def test_sequences(self):
        n = 0
        for b in self.banks:
            for off, size in b.sq_offsets:
                sq = scei.Sq(self.snd[off:off + size])
                self.assertEqual(sq.size, size)
                n += 1
        self.assertEqual(n, 150)

    def test_jukebox_titles(self):
        titles = [t for b in self.banks for t in b.titles]
        self.assertEqual(len(titles), 51)
        self.assertIn("BGM 06 (Piros' Theme)", self.banks[35].titles)


@unittest.skipUnless(HAVE_ELF and os.path.exists(ISO), "needs the executable and infection.iso")
class DiscAudioTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import sound
        cls.sound = sound
        cls.prog = sound.Program(ELF)
        cls.img = sound.open_image(ISO)
        cls.files = sound.load_voices(cls.prog)

    def test_voice_tables_cover_the_files(self):
        on_disc = 0
        per_file = {}
        for path, vf in self.files.items():
            e = self.sound.find(self.img, path)
            lines = vf.lines()
            if e is None:
                self.assertIn("BOSSTALK", path)     # volume 1 ships without it
                continue
            on_disc += len(lines)
            per_file[path] = len(lines)
            end = 0
            for (o, s), _ in lines:
                self.assertEqual(o % 2048, 0)
                self.assertEqual(s % 2, 0)
                self.assertGreaterEqual(o, end)
                self.assertLess(o - end, 2048)      # only sector padding between
                end = o + s
            self.assertLess(e.size - end, 2048)
        self.assertEqual(len(per_file), 54)
        self.assertEqual(on_disc, 8208)
        self.assertEqual(per_file["VOICE_E/SPC00_E.BIN"], 172)
        self.assertEqual(per_file["VOICE/PGUSO.BIN"], 294)
        self.assertEqual(per_file["VOICE_E/EVVOL1_E.BIN"], 489)

    def test_padding_is_ff(self):
        for path in ("VOICE_E/SPC00_E.BIN", "VOICE/INU.BIN", "VOICE_E/FOOD_E.BIN"):
            raw = self.sound.read_file(self.img, path)
            end = 0
            for (o, s), _ in self.files[path].lines():
                self.assertEqual(set(raw[end:o]) - {0xFF}, set(), path)
                end = o + s

    def test_bgm(self):
        tracks = self.sound.load_bgm(self.prog)
        self.assertEqual(tracks, [(0, 0, 16941056, 2), (1, 16941056, 46073124, 0)])
        raw = self.sound.read_file(self.img, "VOICE/BGM.BIN")
        self.assertEqual(len(raw), 63014912)
        self.assertEqual(set(raw[16941056 + 46073124:]), {0xFF})

    def test_stream_pcm(self):
        import gzarc
        data = self.sound.read_file(self.img, "STREAM/STRCMNE.BIN")
        m = next(m for m in gzarc.members(data) if m.name == "str7100.tmp")
        head, blocks, frames = self.sound.stream_pcm(gzarc.inflate(data, m))
        self.assertEqual(head, {"id": 0, "type": 0, "bits": 16, "track": 1,
                                "blocks": 64, "words": 256})
        self.assertEqual((len(blocks), frames), (1406, 225))
        # Each block is two 256-sample runs: the seam between them jumps, the
        # step from one block's first half into the next block's does not.
        a = [array.array("h", b) for b in blocks]
        seam = sum(abs(x[256] - x[255]) for x in a) / len(a)
        onward = sum(abs(y[0] - x[255]) for x, y in zip(a, a[1:])) / (len(a) - 1)
        self.assertGreater(seam, 4 * onward)
        st = self.sound.blocks_to_stereo(blocks[:2])
        self.assertEqual(st[0:4].tolist(), [a[0][0], a[0][256], a[0][1], a[0][257]])
        self.assertEqual(st[512:514].tolist(), [a[1][0], a[1][256]])


if __name__ == "__main__":
    unittest.main()
