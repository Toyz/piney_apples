"""Tests for tools/adpcm.py, tools/wav.py, tools/scei.py and tools/sound.py.
The decoder and the WAV writer are checked on hand-made data; the rest against
PINEY_VOLUME's disc (volume.py; Infection by default), skipped when its
executable (with DATA/SNDDATA.BIN and the overlays) or its image is absent:
the sound banks, the voice files as the volume's own code names them
(sound.load_voices: the Spc tables, ccVoiceRequest's and ccVoicePgFood's
cases, ccEvVoiceRequest's), the BGM and a stream's PCM.
Run: PINEY_VOLUME=<volume> python3 tools/test_sound_rt.py"""

import array
import collections
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import adpcm  # noqa: E402
import scei  # noqa: E402
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
import wav  # noqa: E402

ELF, ISO = volume.ELF, volume.ISO
DISC = os.path.dirname(ELF)
SNDDATA = os.path.join(DISC, "DATA", "SNDDATA.BIN")
HAVE_ELF = all(os.path.exists(p) for p in (ELF, SNDDATA)) and os.path.exists(os.path.join(DISC, "DATA", "GCMN.PRG"))

# Per volume: the banks' header tables (vags, samples, sets, programs) and
# the VAGs by (loop byte, end flags, one-shot tail), as each SNDDATA.BIN has
# them; the voice files the code names (how many on the disc, those
# absent); the BGM tracks, the file's length and what
# pads it past the last; how str7100's PCM is read (a voice language from
# Outbreak on).
SND = {
    "infection": ({"vags": 1013, "samples": 1402, "sets": 1398, "programs": 926},
                  {(0, 1, True): 699, (1, 3, None): 314}),
    "mutation": ({"vags": 1018, "samples": 1407, "sets": 1403, "programs": 927},
                 {(0, 1, True): 704, (1, 3, None): 314}),
    "outbreak": ({"vags": 1018, "samples": 1407, "sets": 1403, "programs": 927},
                 {(0, 1, True): 704, (1, 3, None): 314}),
    "quarantine": ({"vags": 1011, "samples": 1400, "sets": 1396, "programs": 934},
                   {(0, 1, True): 699, (1, 3, None): 312}),
}
VOICES = {
    "infection": (54, {"VOICE/BOSSTALK.BIN", "VOICE_E/BOSSTALK.BIN", "VOICE/EVVOL2S.BIN"}),
    "mutation": (57, {"VOICE/BOSSTALK.BIN", "VOICE_E/BOSSTALK.BIN"}),
    "outbreak": (64, {"VOICE2_E/SPC18_E.BIN", "VOICE2_E/SPC19_E.BIN", "VOICE2_E/SPC20_E.BIN"}),
    "quarantine": (66, {"VOICE/BOSSTALK.BIN", "VOICE_E/BSTALK_E.BIN", "VOICE_E/MIAE.BIN"}),
}
PADDED = ("VOICE/INU.BIN", "VOICE_E/FOOD_E.BIN", "VOICE/PGUSO.BIN")
LATER_BGM = [(0, 0, 16941056, 2), (1, 16941056, 46073124, 0), (2, 63014912, 36812964, 0),
             (3, 99829760, 856064, 0), (4, 100685824, 1118208, 0), (5, 101804032, 7022592, 2),
             (6, 108826624, 1207296, 0)]
BGM = {
    "infection": ([(0, 0, 16941056, 2), (1, 16941056, 46073124, 0)], 63014912, {0xFF}),
    "mutation": (LATER_BGM + [(7, 110034944, 20386816, 2)], 130422784, {0xFF}),
    "outbreak": (LATER_BGM + [(7, 110034944, 17889280, 2)], 127924224, set()),
    "quarantine": (LATER_BGM + [(7, 110034944, 17889280, 2)], 127924224, set()),
}
PCM_LANGS = {"infection": (None,), "mutation": (None,), "outbreak": (0, 1), "quarantine": (0, 1)}
# The lines the code can reach, on the disc, and the lines on the disc no
# reachable row names (path, first byte, end of its data): a row of -1 where
# a line stands (MUT event 101 msg 2, OUT 204 msg 2, both languages); a row
# past the event's messages (QUA side 355 row 25: 26 rows, 25 messages); a
# row turned to another line (QUA 315 en row 27 plays 0x4cf9000, event
# 314's); a line between two events' that no row names (EVVOL3S, OUT and
# QUA, between side events 262 and 263).
LINES = {
    "infection": (8208, set()),
    "mutation": (8212, {("VOICE/EVVOL2.BIN", 0x170800, 0x177F80), ("VOICE_E/EVVOL2_E.BIN", 0x11E000, 0x125400)}),
    "outbreak": (8726, {("VOICE/EVVOL3.BIN", 0x11D0000, 0x121C320), ("VOICE_E/EVVOL3_E.BIN", 0xE91800, 0xECBA42),
                        ("VOICE/EVVOL3S.BIN", 0x3F83800, 0x3FF4260)}),
    "quarantine": (9421, {("VOICE/EVVOL3S.BIN", 0x3F83800, 0x3FF4260), ("VOICE/EVVOL4S.BIN", 0x1371000, 0x139C1FF),
                          ("VOICE_E/EVVOL4_E.BIN", 0x5421000, 0x54501BE)}),
}


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


@unittest.skipUnless(HAVE_ELF, "needs the volume's executable and DATA/")
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
        self.assertEqual(dict(c), dict(SND[volume.NAME][0], bad=0))

    def test_vagi_loop_flag_matches_the_adpcm(self):
        # Scan flag bytes only (decoding them all takes seconds): the Vagi loop
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
        self.assertEqual(dict(kinds), SND[volume.NAME][1])

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


@unittest.skipUnless(HAVE_ELF and os.path.exists(ISO), "needs the volume's executable and image")
class DiscAudioTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import sound
        cls.sound = sound
        cls.prog = sound.Program(ELF)
        cls.img = sound.open_image(ISO)
        cls.files = sound.load_voices(cls.prog)

    def test_voice_files_named(self):
        """The files the code names, on the disc and not."""
        absent = {path for path in self.files if self.sound.find(self.img, path) is None}
        self.assertEqual((len(self.files) - len(absent), absent), VOICES[volume.NAME])

    def test_voice_tables_cover_the_files(self):
        """The lines the code reaches tile each file on the disc: sector
        aligned, in order, only 0xFF padding between them, but for the
        lines no reachable row names (LINES), each a run of data from a
        sector's start."""
        on_disc = 0
        per_file = {}
        unread = set()
        for path, vf in self.files.items():
            e = self.sound.find(self.img, path)
            if e is None:
                continue
            lines = vf.lines()
            on_disc += len(lines)
            per_file[path] = len(lines)
            end = 0
            raw = None
            for (o, s), _ in lines:
                self.assertEqual(o % 2048, 0)
                self.assertEqual(s % 2, 0)
                self.assertGreaterEqual(o, end)
                if o - end >= 2048:
                    raw = raw or self.sound.read_file(self.img, path)
                    data = [i for i in range(end, o) if raw[i] != 0xFF]
                    self.assertEqual(data[0] % 2048, 0, path)
                    unread.add((path, data[0], data[-1] + 1))
                end = o + s
            self.assertLess(e.size - end, 2048)
        self.assertEqual((on_disc, unread), LINES[volume.NAME])
        if volume.NAME == "infection":
            self.assertEqual(per_file["VOICE_E/SPC00_E.BIN"], 172)
            self.assertEqual(per_file["VOICE/PGUSO.BIN"], 294)
            self.assertEqual(per_file["VOICE_E/EVVOL1_E.BIN"], 489)
        else:
            # Mia's ten lines (rows 0-9, her alternatives' reach); Quarantine's
            # disc has no MIAE.BIN (VOICES).
            self.assertEqual(per_file.get("VOICE_E/MIAE.BIN"), None if volume.NAME == "quarantine" else 10)

    def test_padding_is_ff(self):
        for path in PADDED:
            raw = self.sound.read_file(self.img, path)
            end = 0
            for (o, s), _ in self.files[path].lines():
                self.assertEqual(set(raw[end:o]) - {0xFF}, set(), path)
                end = o + s

    def test_bgm(self):
        tracks = self.sound.load_bgm(self.prog)
        want, size, pad = BGM[volume.NAME]
        self.assertEqual(tracks, want)
        raw = self.sound.read_file(self.img, "VOICE/BGM.BIN")
        self.assertEqual(len(raw), size)
        self.assertEqual(set(raw[max(o + s for _, o, s, _ in tracks):]), pad)

    def test_stream_pcm(self):
        import ccs
        import gzarc
        data = self.sound.read_file(self.img, "STREAM/STRCMNE.BIN")
        # Outbreak's and Quarantine's archives call every member tmp.ccs.
        raw = next(r for r in (gzarc.inflate(data, m) for m in gzarc.members(data))
                   if ccs.Ccs(r).name == "str7100")
        for lang in PCM_LANGS[volume.NAME]:
            head, blocks, frames = self.sound.stream_pcm(raw, lang)
            want = {"id": 0, "type": 0, "bits": 16, "track": 1, "blocks": 64, "words": 256}
            self.assertEqual(head, want if lang is None else dict(want, lang=lang))
            self.assertEqual((len(blocks), frames), (1406, 225))
            # Each block is two 256-sample runs: the seam between them jumps,
            # the step from one block's first half into the next's does not.
            a = [array.array("h", b) for b in blocks]
            seam = sum(abs(x[256] - x[255]) for x in a) / len(a)
            onward = sum(abs(y[0] - x[255]) for x, y in zip(a, a[1:])) / (len(a) - 1)
            self.assertGreater(seam, 4 * onward)
            st = self.sound.blocks_to_stereo(blocks[:2])
            self.assertEqual(st[0:4].tolist(), [a[0][0], a[0][256], a[0][1], a[0][257]])
            self.assertEqual(st[512:514].tolist(), [a[1][0], a[1][256]])


if __name__ == "__main__":
    unittest.main()
