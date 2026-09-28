#!/usr/bin/env python3
"""crates/piney-battle's party_chat.rs against the game's ccAI::ChatMessage*
(personal.cpp, gcmn 0x00586130-0x00592a98) run in tools/eemu.py.

Each case builds a random party world as tools/test_battle_party_ai_rs.py
does (the members, foes, their AIs, the message bus, the save's lists) and
adds what the chat lines read besides:

  - every character's base named "N<index>" (the probe names them the
    same), so #0 and #1 and the lines about others come out comparable;
  - saveData +0x220d (id 1's garbled lines), ccGame.areaPrev and
    ccGame.server;
  - each enemy's skill list (skillNum, skillList[k].skiParam, its type
    with or without the condition bit 0x20000), for
    ChatMessageAttackTarget;
  - often a member of id 11 or 12, for ChatMessageNeutral's weapon and
    level lines.

The game's chat functions run natively (the party AI harness stubs them),
with ChatMessageModify, the ccAISystem calls, CountTargetInArea,
CheckCharAttribute, checkPartyAnnihilation and the rest they call;
ccChatMsg::OpenChat is recorded and ccCheckGtHack answers 0. The same case goes to battle_probe's `chat`
request. Compared: the return value, every field of every AI (chatRequest,
atkMsgCnt, dmgMsgCnt, target, talkFlag, gDeg ... and the message queues),
the text each AI keeps (+0x24), the bus, the ccSpcChar fields, every
character's heading (dirc z: the lines that turn a member), the balloon
ChatMessageSender opened, and rand()'s state.

    python3 tools/test_battle_chat_rs.py            the unit tests
    python3 tools/test_battle_chat_rs.py bulk N     N cases of every check
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

import test_battle as TB                          # noqa: E402
import test_battle_party_ai_rs as P               # noqa: E402
import test_battle_rs as T                        # noqa: E402

NAMES = 0x01074000        # "N<index>" for each character, 16 bytes apart
SKILLS = 0x01075000       # the enemies' fake skill parameters
TEXT = volume.ai_at(0x24)  # ccAI.chatText
TOWN_LINES = (inf_va(0x006531B0), inf_va(0x00653020), inf_va(0x00653110), inf_va(0x00653390), inf_va(0x00653070), inf_va(0x006530C0), inf_va(0x00653160))


class Chat(P.Party):
    """The party AI's checks object with the chat functions running
    natively, and the chat cases."""

    def __init__(self):
        super().__init__()
        ai, m = self.pc.ai, self.pc.ai.m
        for s in ai.g.prog.functions():
            if s.name.startswith("ChatMessage") and "__4ccAIF" in s.name:
                m.hooks.pop(s.value, None)
        # Mutation's remarks (unnamed) run natively too
        for va_ in volume.remarks():
            m.hooks.pop(va_, None)
        self.opened = []

        def open_chat(mm, chat, ch, text, *_):
            from eemu import _cstr
            self.opened.append([ai.index(ch), list(_cstr(mm, text))])
            return 0
        m.hooks[ai.sym("OpenChat__9ccChatMsgFP6ccCharPc")] = open_chat
        m.hooks[ai.sym("ccCheckGtHack__Fv")] = lambda mm, *_: 0
        # ChatMessageSender's checkPartyAnnihilation runs on the party
        # manager as put (GameBattle answers a flag).
        m.hooks.pop(ai.sym("checkPartyAnnihilation__Fv"), None)

    # the case --------------------------------------------------------------------------
    def chat_case(self, rnd, fn, mangled, argf, tweak=None):
        """A case; one whose line would hang the game (a # with a code
        ChatMessageModify does not know: two lines have one) is drawn
        again."""
        import eemu
        while True:
            try:
                return self._chat_case(rnd, fn, mangled, argf, tweak)
            except eemu.Stop:
                self.scene.restore()
                self.reset()

    def _chat_case(self, rnd, fn, mangled, argf, tweak=None):
        pc, ai = self.pc, self.pc.ai
        g, m = ai.g, ai.m
        while True:
            w = pc.world(rnd)
            me = pc.me(rnd, w)
            if me is not None:
                break
        chars = w["chars"]
        # a member of id 11 or 12 now and then (Natsume, Rachel)
        if rnd.random() < 0.4 and chars[me].type & 7:
            used = {c.id for c in chars if c.type & 7}
            free = [i for i in (11, 12) if i not in used]
            if free:
                old = chars[me].id
                chars[me].id = rnd.choice(free)
                p = w["party"]
                p["ids"] = [chars[k].id if k >= 0 else -1 for k in p["members"]]
                if old in w["lists"] and not any(c.id == old for c in chars):
                    w["lists"][chars[me].id] = w["lists"].pop(old)
        if fn == "ChatMessageNeutral" and rnd.random() < 0.7:
            w["game"]["in_battle"] = 0
        if tweak:
            tweak(rnd, w, me)
        w["me"] = me
        save220d = rnd.choice((0, 0, 1))
        area_prev = rnd.choice((0, 0, 1, 2))
        server = rnd.choice((0, 1, 2, 3, 4, 5))
        tricks = []
        args, gargs = argf(rnd, w, me)
        # the world, then what the chat reads besides
        ai.put_world(w)
        for i, ch in enumerate(chars):
            va = ai.va(i)
            name = NAMES + 16 * i
            m.mem[name:name + 16] = (b"N%d" % i).ljust(16, b"\0")
            m.store(m.load(va, 4), 4, name)          # base->name
            n = 0
            if not ch.type & 7:
                k = rnd.randrange(0, 7)
                m.store(va + 0x2D4, 4, k)
                for j in range(6):
                    sp = SKILLS + 0x40 * (8 * i + j)
                    ty = rnd.choice((0, 2, 0x20002, 0x20000, 1))
                    m.mem[sp:sp + 0x40] = bytes(0x40)
                    m.store(sp + 0x2C, 4, ty)
                    m.store(va + 0x2D8 + 16 * j + 8, 4, sp if rnd.random() < 0.9 else 0)
                    if j < k and m.load(va + 0x2D8 + 16 * j + 8, 4) and ty & 0x20000:
                        n += 1
            tricks.append(n)
        # the text the AI kept from an earlier line (the sender opens it)
        text = rnd.choice((b"", b"", b"Earlier #0 line", b"x"))
        base = P.AIS + 0x400 * me + volume.ai_at(0x24)
        m.mem[base:base + 0x50] = text.ljust(0x50, b"\0")
        g.put(TB.SAVE + 0x220D, "<b", save220d)
        g.put(TB.GAME + 0x18, "<i", area_prev)
        g.put(TB.GAME + 0x1C, "<i", server)
        self.opened = []
        ai.cur = ai.name(ai.va(me))
        va = P.AIS + 0x400 * me
        ret = m.call(ai.sym(mangled), [va] + [a & 0xFFFFFFFF for a in gargs], limit=2_000_000)
        out = ai.read_world(w, P.s32(ret) if fn == "ChatMessageAttackTarget" else 0)
        out["texts"] = {str(k): list(bytes(m.mem[P.AIS + 0x400 * k + TEXT:P.AIS + 0x400 * k + TEXT + 0x50])
                                     .split(b"\0")[0]) for k in w["ais"]}
        out["dircs"] = [m.load(ai.va(i) + 0x68, 4) for i in range(len(chars))]
        out["opened"] = self.opened[-1][1] if self.opened else None
        self.scene.restore()
        a = [me] + args
        req = (f"chat {fn} {len(a)} {' '.join(str(x) for x in a)} {save220d} {area_prev} {server} "
               f"{len(tricks)} {' '.join(str(x) for x in tricks)} {len(text)} {' '.join(str(x) for x in text)} "
               f"{P.ser_world(w, self.b)}")
        return req, out, fn


def char_or_none(rnd, w, none=0.1):
    if rnd.random() < none:
        return -1
    return rnd.randrange(len(w["chars"]))


def cva(ai, i):
    return ai.va(i) if i >= 0 else 0


def make_table(c):
    ai = c.pc.ai

    def no_args(rnd, w, me):
        return [], []

    def int_arg(pool):
        def f(rnd, w, me):
            v = rnd.choice(pool(rnd))
            return [v], [v]
        return f

    def char_arg(rnd, w, me):
        t = char_or_none(rnd, w)
        return [t], [cva(ai, t)]

    def name_arg(rnd, w, me):
        t = rnd.randrange(len(w["chars"]))
        return [t], [NAMES + 16 * t]

    def attack(rnd, w, me):
        foes = [i for i, ch in enumerate(w["chars"]) if not ch.type & 7]
        t = rnd.choice(foes) if foes and rnd.random() < 0.9 else rnd.randrange(len(w["chars"]))
        mx = w["chars"][t].maxHP
        dmg = rnd.choice((-1, 0, 0, 1, 5, 9, 10, 30, mx // 100, mx // 3, mx // 3 + 1, rnd.randrange(0, 2000)))
        sid = rnd.choice((0, 1, 2, rnd.randrange(0, 304), rnd.choice(c.pc.pools.spells or [2])))
        return [t, dmg, sid], [cva(ai, t), dmg, sid]

    def damage(rnd, w, me):
        mx = w["chars"][me].maxHP
        dmg = rnd.choice((0, 1, 5, 9, 10, 30, mx // 100, mx // 3, mx // 3 + 1, rnd.randrange(0, 2000)))
        return [dmg], [dmg]

    def affect(rnd, w, me):
        kind = rnd.choice((7, 8, 16, 17, 18, 20, 1, 0))
        t = char_or_none(rnd, w, 0.05)
        n = rnd.choice((rnd.randrange(0, 1000), rnd.randrange(150, 170), 0))
        return [kind, t, n], [kind, cva(ai, t), n]

    def greeting(rnd, w, me):
        t = rnd.randrange(len(w["chars"]))
        return [t, 0], [cva(ai, t), 0]

    def line(rnd, w, me):
        tbl = rnd.choice(TOWN_LINES)
        i = rnd.randrange(0, 19)
        ptr = ai.m.load(tbl + 4 * i, 4)
        return [volume.inf_of(tbl), i], [ptr, 0, 0, 0]

    def sender(rnd, w, me):
        k = [n for n, _, _ in P.AI_LAYOUT].index("arrivalChatCnt")
        w["ais"][me][k] = rnd.choice((0, 0, 0, 1, -1, 5))
        return [], []

    plain = ["ConditionMinus", "Ghost", "CanNot", "OOM", "NoBattleModeDeny", "Accept", "DisableOcarinaDeny",
             "NoOcarinaDeny", "Victory", "UseOcarina", "HealStart", "CureStart", "ResurrectStart", "LevelDown",
             "LevelUp", "GhostCondition", "QuitPucciguso", "OpenTrapBox", "OpenTreasureBox", "TreatmentPlz",
             "WaitPlz", "WalkingTalk", "Neutral", "HealPlz", "Reencounter", "EnteredField", "EnteredTown",
             "EquipOK", "AttributeCritical", "ResurrectPlz"]
    table = {}
    for n in plain:
        fn = "ChatMessage" + n
        table[fn] = (fn, fn + "__4ccAIFv", no_args)
    table.update({
        "ChatMessageEquipNOT": ("ChatMessageEquipNOT", "ChatMessageEquipNOT__4ccAIFi",
                                int_arg(lambda r: [0, 1, 2, r.randrange(-3, 10)])),
        "ChatMessageChatCmdAccept": ("ChatMessageChatCmdAccept", "ChatMessageChatCmdAccept__4ccAIFi",
                                     int_arg(lambda r: [r.randrange(-1, 21), 0, 13, 14])),
        "ChatMessageDamage": ("ChatMessageDamage", "ChatMessageDamage__4ccAIFi", damage),
        "ChatMessageConditionModify": ("ChatMessageConditionModify", "ChatMessageConditionModify__4ccAIFi",
                                       int_arg(lambda r: [r.randrange(150, 166), r.randrange(0, 304)])),
        "ChatMessageAttributeGuard": ("ChatMessageAttributeGuard", "ChatMessageAttributeGuard__4ccAIFi",
                                      int_arg(lambda r: [r.randrange(-2, 10)])),
        "ChatMessageAttributeFollow": ("ChatMessageAttributeFollow", "ChatMessageAttributeFollow__4ccAIFi",
                                       int_arg(lambda r: [r.randrange(-2, 10)])),
        "ChatMessageGratsLevelUp": ("ChatMessageGratsLevelUp", "ChatMessageGratsLevelUp__4ccAIFP6ccChar",
                                    char_arg),
        "ChatMessagePresentOtherFellow": ("ChatMessagePresentOtherFellow",
                                          "ChatMessagePresentOtherFellow__4ccAIFP6ccChar", char_arg),
        "ChatMessageAttackTarget": ("ChatMessageAttackTarget", "ChatMessageAttackTarget__4ccAIFP6ccChar",
                                    char_arg),
        "ChatMessageDeadOtherFellow": ("ChatMessageDeadOtherFellow", "ChatMessageDeadOtherFellow__4ccAIFPc",
                                       name_arg),
        "ChatMessageAttack": ("ChatMessageAttack", "ChatMessageAttack__4ccAIFP6ccCharii", attack),
        "AffectMessages": ("AffectMessages", "AffectMessages__4ccAIFiP6ccChari", affect),
        "Greeting": ("Greeting", "Greeting__4ccAIFP6ccChari", greeting),
        "ChatMessage": ("ChatMessage", "ChatMessage__4ccAIFPcPcPcPc", line),
        "ChatMessageSender": ("ChatMessageSender", "ChatMessageSender__4ccAIFv", sender),
    })
    return table


CHECK_NAMES = []
TABLE = {}


def _entry(name):
    def f(checks, rnd):
        fn, mangled, argf = checks.chat_table[name]
        if name == "ChatMessageSender":
            # the arrival count is set by the tweak before the world is put
            return checks.chat_case(rnd, fn, mangled, lambda r, w, me: ([], []), tweak=lambda r, w, me: argf(r, w, me))
        return checks.chat_case(rnd, fn, mangled, argf)
    return f


class ChatChecks(Chat):
    def __init__(self):
        super().__init__()
        self.chat_table = make_table(self)


def _names():
    plain = ["ConditionMinus", "Ghost", "CanNot", "OOM", "NoBattleModeDeny", "Accept", "DisableOcarinaDeny",
             "NoOcarinaDeny", "Victory", "UseOcarina", "HealStart", "CureStart", "ResurrectStart", "LevelDown",
             "LevelUp", "GhostCondition", "QuitPucciguso", "OpenTrapBox", "OpenTreasureBox", "TreatmentPlz",
             "WaitPlz", "WalkingTalk", "Neutral", "HealPlz", "Reencounter", "EnteredField", "EnteredTown",
             "EquipOK", "AttributeCritical", "ResurrectPlz"]
    return (["ChatMessage" + n for n in plain]
            + ["ChatMessageEquipNOT", "ChatMessageChatCmdAccept", "ChatMessageDamage", "ChatMessageConditionModify",
               "ChatMessageAttributeGuard", "ChatMessageAttributeFollow", "ChatMessageGratsLevelUp",
               "ChatMessagePresentOtherFellow", "ChatMessageAttackTarget", "ChatMessageDeadOtherFellow",
               "ChatMessageAttack", "AffectMessages", "Greeting", "ChatMessage", "ChatMessageSender"])


CHECK_NAMES = _names()
TABLE = {name: _entry(name) for name in CHECK_NAMES}


def _test(names, seed):
    def test(self):
        for k, name in enumerate(names):
            self.check(name, seed + k)
    return test


@unittest.skipUnless(T.READY, "needs the extracted disc and cargo")
class ChatAgainstGame(T.Against):
    CASES = 120
    TABLE = TABLE
    MAKE = ChatChecks

    test_plain = _test(CHECK_NAMES[:30], 9500)
    test_with_arguments = _test(CHECK_NAMES[30:], 9540)


if __name__ == "__main__":
    T.main(CHECK_NAMES, 9500, TABLE, ChatChecks)
