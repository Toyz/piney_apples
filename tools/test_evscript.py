#!/usr/bin/env python3
"""tools/evscript.py on the real event scripts, and against the game's own
CheckOpen / SetCurrentOpen / Execute run in tools/eemu.py. Infection in full;
Mutation, Outbreak and Quarantine each against their own executable (through
the names piney-gen syms carries). Skipped when an executable is absent."""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402
ROOT = os.path.dirname(HERE)
ELF = volume.ELF
OTHERS = {v: os.path.join(ROOT, "work", v, "disc", e) for v, e in
          (("mutation", "SLUS_205.62"), ("outbreak", "SLUS_205.63"),
           ("quarantine", "SLUS_205.64"))}

SAVE = 0x01900000      # a scratch ccSaveData (0x8530 bytes)
SAVE_SIZE = 0x8530
INF_CHARS = ("Kite", "Mia", "Orca", "Marlo", "Sanjuro", "Nuke Usagimaru", "Balmung",
             "Moonstone", "Piros", "Wiseman", "Elk", "Natsume", "Rachel", "Gardenia",
             "Terajima Ryoko", "BlackRose", "Mistral", "Helba")


# Per disc: the scripts, the Execute / CheckOpen / SetCurrentOpen tables
# and Execute's default (the loop head), the survey (blocks, instructions,
# conditions, currents, unknown cases, kinds of op, messages, waits), and
# message 1 of its first main event (100 (N - 1) + 1) in both languages.
EXPECT = {
    "infection": (192, [(0x00355FD0, 168), (0x00355D60, 41), (0x00355D30, 12)], 0x001A8D80,
                  (1935, 13515, 2059, 1918, 0, 145, 1993, 1596),
                  (("VOICE_E/EVVOL1_E.BIN", 0x27000, 441604), "VOICE/EVVOL1.BIN")),
    "mutation": (196, [(0x00370070, 169), (0x0036FE40, 41), (0x0036FE10, 12)], 0x001BDF60,
                 (2035, 14253, 2128, 2164, 0, 146, 2074, 1704),
                 (("VOICE_E/EVVOL2_E.BIN", 972800, 198558), "VOICE/EVVOL2.BIN")),
    "outbreak": (199, [(0x00369A20, 170), (0x003697F0, 41), (0x003697C0, 12)], 0x001B42B0,
                 (2062, 14645, 2123, 2220, 1, 146, 2099, 1740),
                 (("VOICE_E/EVVOL3_E.BIN", 139264, 563200), "VOICE/EVVOL3.BIN")),
    "quarantine": (206, [(0x0025CFF0, 170), (0x0025CD90, 41), (0x0025CD60, 12)], 0x001BBAC8,
                   (2139, 14764, 2249, 2227, 0, 147, 2099, 1740),
                   (("VOICE_E/EVVOL4_E.BIN", 194560, 332832), "VOICE/EVVOL4.BIN")),
}
FIRST = 100 * (volume.number() - 1) + 1


@unittest.skipUnless(os.path.exists(ELF), "game executable not present")
class TestEvScript(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import evscript
        cls.E = evscript
        cls.ev = evscript.Events(ELF)

    def test_event_numbers(self):
        n = EXPECT[volume.NAME][0]
        self.assertEqual(len(self.ev.events), n)
        self.assertEqual(len(set(v[0] for v in self.ev.events.values())), n)
        self.assertEqual(self.ev.events[FIRST][0], "eventTblM%d01" % volume.number())
        self.assertEqual(self.ev.events[406][0], "eventTblML006")
        if volume.NAME == "infection":
            self.assertEqual(self.ev.events[1], ("eventTblM101", "MG0001 OPENING"))

    def test_names_and_extents_match_symbols(self):
        # The names are made from the event number and the extents from the
        # gaps between arrays; Infection's symbol table says the same. A
        # later disc's sidecar carries the names with Infection's sizes.
        # A later disc's sidecar names the scripts Infection has, with
        # Infection's sizes; its new ones (19, 40, 56) have no name.
        p = self.ev.p
        unnamed = 0
        for name, (va, shorts) in self.ev.bounds.items():
            at = p.symbol_at(va)
            if at is None and volume.NAME != "infection":
                unnamed += 1
                continue
            sym, off = at
            got = (sym.name, off, sym.size) if volume.NAME == "infection" else (sym.name, off)
            self.assertEqual(got, (name, 0, 2 * shorts)[:len(got)])
        self.assertEqual(unnamed, {"infection": 0, "mutation": 19, "outbreak": 40, "quarantine": 56}[volume.NAME])

    def test_message_counts_match_symbols(self):
        if volume.NAME != "infection":
            self.skipTest("only Infection's executable names the message arrays")
        p = self.ev.p
        for table in ("evMsgTbl", "evMsgTblp"):
            t = p.symbol_named(table).value
            for g in range(10):
                grp = p.u32(t + 4 * g)
                for i in range(50 if grp else 0):
                    va = p.u32(grp + 4 * i)
                    if va:
                        sym, off = p.symbol_at(va)
                        self.assertEqual(self.ev.message_count(va), (sym.size - off) // 12, va)

    def test_switches(self):
        n, tables, default, _survey, _voice = EXPECT[volume.NAME]
        self.assertEqual([(t.switch.table, t.switch.count) for t in self.ev.tables], tables)
        self.assertEqual(self.ev.execute.default, default)      # the loop head
        self.assertFalse(self.ev.execute.has(134))
        # From Mutation on opcode 99 takes an operand and 168 is a case.
        later = volume.NAME != "infection"
        self.assertEqual(self.ev.ops.args(99), int(later))
        self.assertEqual(self.ev.ops.args(168), 0 if later else None)

    def test_survey(self):
        n, _tables, _default, (blocks, insns, conds, currents, unknown, kinds, msgs, waits), _ = EXPECT[volume.NAME]
        r = self.E.survey(self.ev)
        self.assertEqual((r["scripts"], r["clean"], r["bad"]), (n, n, []))
        self.assertEqual((r["blocks"], r["insns"]), (blocks, insns))
        self.assertEqual(sum(r["conds"].values()), conds)
        self.assertEqual(sum(r["currents"].values()), currents)
        self.assertEqual(sum(r["unknown"].values()), unknown)
        self.assertEqual(len(r["ops"]), kinds)
        self.assertEqual(r["ops"][("op", 6)], msgs)       # message
        self.assertEqual(r["ops"][("op", 5)], waits)      # wait

    def test_first_story_event(self):
        if volume.NAME != "infection":
            # The header of each disc's first main event.
            text = self.E.disassemble(self.ev, self.E.script_name(FIRST)).splitlines()
            self.assertRegex(text[0], r"# eventTblM%d01 @ 0x[0-9a-f]{8}, \d+ bytes: event %d " % (
                volume.number(), FIRST))
            return
        text = self.E.disassemble(self.ev, "eventTblM101").splitlines()
        self.assertEqual(text[:12], [
            '# eventTblM101 @ 0x00317ec0, 540 bytes: event 1 "MG0001 OPENING"',
            "# 3 blocks, 3 messages",
            "open:",
            "  +0000  0001 0000                      event_done     event=0",
            "  +0004  0004 0002                      game_status    status=2",
            "block 0:",
            "  +000a  0003 0000 0000                 set phase          == 0",
            "  +0014  0071 0002                      frame_rate           rate=2",
            "  +0018  0072 0003                      overlay              num=3",
            "  +001c  000a 0002                      stream               num=2",
            "  +0020  0071 0001                      frame_rate           rate=1",
            "  +0024  000a 006a                      stream               num=106",
        ])
        # The line itself, inlined from the disc, is not spelled out here.
        self.assertRegex(text[13], r'msg=1  ; ".+" \[voice VOICE_E/EVVOL1_E\.BIN \+0x27000 4\.6s\]')
        self.assertIn('mail=4  ; "Registered yet?"', text[18])
        self.assertEqual(text[-2:], [
            "  +0214  0001 ffff                      end_event            grp=-1  ; this event",
            "  +021a  ffff                           end"])

    def test_voice(self):
        # What the game's ccEvVoiceRequest / evVoicePlay hand to sewordCmd.
        english, japanese = EXPECT[volume.NAME][4]
        self.assertEqual(self.ev.voice(FIRST, 1), english)
        self.assertEqual(self.ev.voice(FIRST, 1, english=False)[0], japanese)
        self.assertIsNone(self.ev.voice(FIRST, 1, parody=True))    # Parody Mode mutes main events

    def test_against_game_code(self):
        # Every case's operand count is documented, and every script's block
        # ends are where the game's own code stops walking it at lv 0.
        with open(os.devnull, "w", encoding="utf-8") as null:
            self.assertEqual(self.E.check(self.ev, null), 0)

    def game(self):
        g = self.ev.walker
        g.m.store(self.ev.p.symbol_named("saveData").value, 4, SAVE)
        g.m.store(self.ev.p.symbol_named("volumeNum").value, 4, 1)
        return g

    def test_flag_mode_effects(self):
        # lv 1, as ccEventFlagSet replays a volume; op 4 first so the block
        # does not set its own bit (that needs dsllv, which eemu lacks).
        g = self.game()
        g.m.mem[SAVE:SAVE + SAVE_SIZE] = bytes(SAVE_SIZE)
        pristine = bytes(g.m.mem[SAVE:SAVE + SAVE_SIZE])

        def run(code, *args):
            g.m.mem[SAVE:SAVE + SAVE_SIZE] = pristine
            va = g.load([4, code, *args, 0])
            g.call(self.ev.execute.fn, va, grp=7, ev=3, lv=1)
            after = g.m.mem[SAVE:SAVE + SAVE_SIZE]
            return {hex(i): after[i] for i in range(SAVE_SIZE) if after[i] != pristine[i]}

        self.assertEqual(run(1, 3), {"0x5517": 0x80})                 # eventFlag[3] bit 63
        self.assertEqual(run(79, 3), {"0x2220": 8, "0x222c": 8})      # partyMemberFlag, Exp
        self.assertEqual(run(84), {"0x2227": 0x80})                   # partyMemberCall bit 31
        self.assertEqual(run(96, 3, 2), {"0x7730": 2})                # spcParam[3].base.gold
        self.assertEqual(run(125, 3), {"0x2234": 8})                  # townMoveFlag
        self.assertEqual(run(138, 3, 2), {"0x64fb": 2})               # eventStatus[3] = 2
        self.assertEqual(run(140, 3, 2), {"0x64fb": 0xfe})            # eventStatus[3] -= 2
        self.assertEqual(run(149, 3, 2), {"0x77f6": 2})               # spcParam[3].friendship
        self.assertEqual(run(155, 3), {"0x8426": 3})                  # lastTown

    def test_condition_compare(self):
        # comp 0 is ==, 1 is >=, 2 is <=, the game's value on the left.
        g = self.game()
        g.m.store(SAVE + 0x64f8 + 5, 1, 7)                            # eventStatus[5] = 7

        def cond(*shorts):
            v0, _ = g.call(self.ev.checkopen.fn, g.load([*shorts, 0]), ev=-1, lv=2)
            return v0

        self.assertEqual([cond(10, 5, n, c) for n in (6, 7, 8) for c in (0, 1, 2)],
                         [0, 1, 0, 1, 1, 1, 0, 0, 1])
        self.assertEqual([cond(11, 5, 7, 9), cond(11, 5, 8, 9)], [1, 0])
        self.assertEqual([cond(39, 1, 0), cond(39, 2, 1), cond(39, 2, 2)], [1, 0, 1])

    def test_character_names(self):
        from image import Program
        demo = Program(ELF, "demo")
        tbl = demo.symbol_named("charTbl")
        # 21 characters from Mutation on (NewGame's count, volume.WALKERS).
        chars = INF_CHARS + (() if volume.NAME == "infection" else ("Tsukasa", "Subaru", "Sora"))
        rows = volume.loop_rows(demo, "NewGame__10ccSaveDataFi", "charTbl")
        names = [demo.cstr(demo.u32(tbl.value + 92 * i)).decode() for i in range(rows)]
        self.assertEqual(tuple(names), chars)
        self.assertEqual(self.ev.chars, chars)


class TestOtherVolumes(unittest.TestCase):
    """Each later volume walked against its own code. From Mutation on the
    Execute table grows (169 entries in MUT, 170 in OUT and QUA): opcode 99
    takes an operand, 168 is new, and 169 is a case only in Quarantine
    (Outbreak's entry is the default, yet its ending script already uses it)."""

    EXPECT = {   # volume: (volumeNum, Execute entries, scripts, 169 has a case)
        "mutation": (2, 169, 196, False),
        "outbreak": (3, 170, 199, False),
        "quarantine": (4, 170, 206, True),
    }

    def volumes(self):
        present = {v: p for v, p in OTHERS.items()
                   if os.path.exists(p) and os.path.exists(p + ".syms")}
        if not present:
            self.skipTest("no other volume extracted with a .syms sidecar")
        return present

    def test_volumes(self):
        import evscript as E
        for vol, path in self.volumes().items():
            with self.subTest(volume=vol):
                ev = E.Events(path)
                num, entries, scripts, has169 = self.EXPECT[vol]
                self.assertEqual(ev.volume, num)
                self.assertEqual([t.switch.count for t in ev.tables], [entries, 41, 12])
                self.assertEqual(ev.ops.args(99), 1)
                self.assertEqual(ev.ops.row(99)[0], "noise")
                self.assertEqual(ev.ops.args(168), 0)
                self.assertEqual(ev.execute.has(169), has169)
                self.assertEqual(len(ev.chars), 21)
                self.assertEqual(ev.chars[18:], ("Tsukasa", "Subaru", "Sora"))
                r = E.survey(ev)
                self.assertEqual((r["scripts"], r["clean"], r["bad"]), (scripts, scripts, []))
                # Outbreak's ending (event 314) has the opcode its table lacks.
                self.assertEqual(dict(r["unknown"]), {} if has169 or vol == "mutation"
                                 else {("op", 169): 1})
                with open(os.devnull, "w", encoding="utf-8") as null:
                    self.assertEqual(E.check(ev, null), 0)
                # Each volume's first main-story event opens on event 100 * (N - 1),
                # which ccStartEventConvert marks done when a previous save is loaded.
                first = ev.script(E.script_name(100 * (num - 1) + 1))
                self.assertEqual([(c.code, list(c.args)) for c in first.header],
                                 [(1, [100 * (num - 1)])])

    def test_voice(self):
        import evscript as E
        present = self.volumes()
        if "mutation" not in present:
            self.skipTest("Mutation not extracted")
        ev = E.Events(present["mutation"])
        v = ev.voice(102, 0)
        self.assertIsNotNone(v)
        self.assertEqual(v[0], "VOICE_E/EVVOL2_E.BIN")


if __name__ == "__main__":
    unittest.main()
