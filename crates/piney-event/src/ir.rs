//! Our own instruction set: a typed form of an event script.
//!
//! A script is a list of open conditions, then up to [`MAX_BLOCKS`] blocks.
//! Each block has persistent precondition settings ([`Tag`]), conditions
//! ([`Cond`]) and a body of instructions ([`Op`]). There are no jumps: the
//! block is the unit of control flow, and a block that has run is marked in
//! the event's flag word so it does not run again (unless it contains
//! [`Op::Repeatable`]).
//!
//! Nothing here depends on the game's bytecode: there are no opcode numbers
//! and no byte offsets. The names and the order of the fields are those of
//! the game's operand structs (`tools/evscript.py`), because that order is
//! how the reference pages describe them; [`crate::official`] maps them to
//! and from the game's `short` arrays. Message text lives in a separate
//! table ([`Message`]), which instructions refer to by index.
//!
//! Every field is a signed 16-bit value, as in the game, except comparisons,
//! which are a [`Cmp`]. The text form of all of this is in [`crate::text`].

use std::fmt;

/// The walk loop's bound (`eventSub`, `ccEventFlagSet`): blocks past this
/// many are never reached, and block `b`'s run bit is bit `b` of the event's
/// flag word, below the done (62) and closed (63) bits.
pub const MAX_BLOCKS: usize = 62;

/// How a condition compares the game's value with the operand: `Eq` is
/// `==`, `Ge` is game value `>=` operand, `Le` is game value `<=` operand.
/// Any other value makes the condition fail in the game (it is kept so a
/// script round-trips exactly).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cmp {
    Eq,
    Ge,
    Le,
    Other(i16),
}

impl Cmp {
    pub fn from_short(v: i16) -> Cmp {
        match v {
            0 => Cmp::Eq,
            1 => Cmp::Ge,
            2 => Cmp::Le,
            v => Cmp::Other(v),
        }
    }
    pub fn to_short(self) -> i16 {
        match self {
            Cmp::Eq => 0,
            Cmp::Ge => 1,
            Cmp::Le => 2,
            Cmp::Other(v) => v,
        }
    }
    /// `game (cmp) operand`; `None` for [`Cmp::Other`], which never passes.
    pub fn test<T: PartialOrd>(self, game: T, operand: T) -> Option<bool> {
        match self {
            Cmp::Eq => Some(game == operand),
            Cmp::Ge => Some(game >= operand),
            Cmp::Le => Some(game <= operand),
            Cmp::Other(_) => None,
        }
    }
}

/// A field of an instruction: how it becomes a `short` and how it is written.
pub trait Operand: Copy {
    fn from_short(v: i16) -> Self;
    fn to_short(self) -> i16;
    fn write_text(self, out: &mut String);
    fn read_text(s: &str) -> Option<Self>;
}

impl Operand for i16 {
    fn from_short(v: i16) -> Self {
        v
    }
    fn to_short(self) -> i16 {
        self
    }
    fn write_text(self, out: &mut String) {
        out.push_str(&self.to_string());
    }
    fn read_text(s: &str) -> Option<Self> {
        parse_int(s).and_then(|v| i16::try_from(v).ok())
    }
}

impl Operand for Cmp {
    fn from_short(v: i16) -> Self {
        Cmp::from_short(v)
    }
    fn to_short(self) -> i16 {
        Cmp::to_short(self)
    }
    fn write_text(self, out: &mut String) {
        match self {
            Cmp::Eq => out.push_str("eq"),
            Cmp::Ge => out.push_str("ge"),
            Cmp::Le => out.push_str("le"),
            Cmp::Other(v) => out.push_str(&v.to_string()),
        }
    }
    fn read_text(s: &str) -> Option<Self> {
        match s {
            "eq" => Some(Cmp::Eq),
            "ge" => Some(Cmp::Ge),
            "le" => Some(Cmp::Le),
            _ => i16::read_text(s).map(Cmp::from_short),
        }
    }
}

/// A decimal or `0x` hexadecimal integer, optionally negative.
pub(crate) fn parse_int(s: &str) -> Option<i64> {
    let (neg, body) = match s.strip_prefix('-') {
        Some(b) => (true, b),
        None => (false, s),
    };
    let v = if let Some(h) = body.strip_prefix("0x") {
        i64::from_str_radix(h, 16).ok()?
    } else {
        if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        body.parse::<i64>().ok()?
    };
    Some(if neg { -v } else { v })
}

macro_rules! field_name {
    ($f:ident) => {
        stringify!($f)
    };
    ($f:ident, $n:literal) => {
        $n
    };
}

/// One instruction family: the enum with named fields, a field-less `Kind`
/// enum, names for the text form, and conversion from and to operand lists.
macro_rules! instruction_set {
    (
        $(#[$emeta:meta])*
        $Enum:ident, $Kind:ident {
            $(
                $(#[doc = $doc:literal])*
                $Variant:ident = $name:literal { $( $field:ident $(= $fname:literal)? : $ty:ty ),* $(,)? }
            ),* $(,)?
        }
    ) => {
        $(#[$emeta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $Enum {
            $( $(#[doc = $doc])* $Variant { $( $field: $ty ),* } ),*
        }

        /// Which variant, without its fields.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $Kind {
            $( $Variant ),*
        }

        impl $Kind {
            pub const ALL: &'static [$Kind] = &[ $( $Kind::$Variant ),* ];

            /// The name used in the text form.
            pub fn name(self) -> &'static str {
                match self { $( $Kind::$Variant => $name ),* }
            }

            pub fn from_name(s: &str) -> Option<Self> {
                match s { $( $name => Some($Kind::$Variant), )* _ => None }
            }

            /// Field names, in operand order.
            pub fn fields(self) -> &'static [&'static str] {
                match self { $( $Kind::$Variant => &[ $( field_name!($field $(, $fname)?) ),* ] ),* }
            }

            pub fn arity(self) -> usize {
                self.fields().len()
            }

            /// The instruction from its operands, in field order.
            pub fn build(self, args: &[i16]) -> Option<$Enum> {
                if args.len() != self.arity() {
                    return None;
                }
                #[allow(unused_mut, unused_variables)]
                let mut it = args.iter().copied();
                Some(match self {
                    $( $Kind::$Variant => $Enum::$Variant {
                        $( $field: <$ty as Operand>::from_short(it.next()?) ),*
                    } ),*
                })
            }

            /// The instruction from `name=value` text fields, in any order.
            pub(crate) fn build_text(self, get: &mut dyn FnMut(&'static str) -> Option<String>) -> Result<$Enum, String> {
                Ok(match self {
                    $( $Kind::$Variant => $Enum::$Variant {
                        $( $field: {
                            let n = field_name!($field $(, $fname)?);
                            let v = get(n).ok_or_else(|| format!("{}: missing {}=", $name, n))?;
                            <$ty as Operand>::read_text(&v)
                                .ok_or_else(|| format!("{}: bad value {}={}", $name, n, v))?
                        } ),*
                    } ),*
                })
            }
        }

        impl $Enum {
            pub fn kind(&self) -> $Kind {
                match self { $( $Enum::$Variant { .. } => $Kind::$Variant ),* }
            }

            pub fn name(&self) -> &'static str {
                self.kind().name()
            }

            /// The operands, in field order.
            pub fn args(&self) -> Vec<i16> {
                match *self {
                    $( $Enum::$Variant { $( $field ),* } => vec![ $( Operand::to_short($field) ),* ] ),*
                }
            }

            /// ` name=value` for each field.
            pub(crate) fn write_fields(&self, out: &mut String) {
                match *self {
                    $( $Enum::$Variant { $( $field ),* } => {
                        $(
                            out.push(' ');
                            out.push_str(field_name!($field $(, $fname)?));
                            out.push('=');
                            Operand::write_text($field, out);
                        )*
                    } ),*
                }
            }
        }

        impl fmt::Display for $Enum {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut s = String::from(self.name());
                self.write_fields(&mut s);
                f.write_str(&s)
            }
        }
    };
}

instruction_set! {
    /// A precondition setting (`-2 TAG ...` in the game's form). It stays in
    /// force for the rest of the event walk, is tested before each later
    /// block's own conditions, and replaces the earlier setting of the same
    /// kind. `phase`, `game_status`, `block_done` and the two status tags
    /// each keep one setting; the scene tags share one.
    Tag, TagKind {
        /// This event's block bit `num`.
        BlockDone = "block_done" { num: i16 },
        /// `eventMng.enablePhase` against `phase`.
        Phase = "phase" { phase: i16, comp: Cmp },
        /// `game.status` (the mode); every status but 5 also clears the scene setting.
        GameStatus = "game_status" { status: i16 },
        /// The room (floor, block) of event point `num`; nothing if no point has that number.
        InPoint = "in_point" { num: i16 },
        /// The whole scene; -1 in any field but `area` matches anything.
        Scene = "scene" { area: i16, town: i16, field: i16, dungeon: i16, floor: i16, block: i16 },
        /// Area 0, this town.
        InTown = "in_town" { town: i16 },
        /// Area 1, this town and field.
        InField = "in_field" { town: i16, field: i16 },
        /// Area 2, this town, field and dungeon.
        InDungeon = "in_dungeon" { town: i16, field: i16, dungeon: i16 },
        /// `saveData.eventStatus[index]` against `num`.
        Status = "status" { index: i16, num: i16, comp: Cmp },
        /// `lo <= saveData.eventStatus[index] <= hi`.
        StatusRange = "status_range" { index: i16, lo: i16, hi: i16 },
    }
}

instruction_set! {
    /// A condition. A list passes when every condition passes; the game
    /// stops evaluating a list at the first failure.
    Cond, CondKind {
        /// Event `event` is done (flag bit 62).
        EventDone = "event_done" { event: i16 },
        /// This event's block bit `num`.
        BlockDone = "block_done" { num: i16 },
        /// `eventMng.enablePhase` against `phase`.
        Phase = "phase" { phase: i16, comp: Cmp },
        /// `game.status == status`.
        GameStatus = "game_status" { status: i16 },
        /// The current floor and block are those of event point `num` (the first with that number).
        InPoint = "in_point" { num: i16 },
        /// All six scene fields equal.
        Scene = "scene" { area: i16, town: i16, field: i16, dungeon: i16, floor: i16, block: i16 },
        /// Area 0 and this town.
        InTown = "in_town" { town: i16 },
        /// Area 1, this town and field.
        InField = "in_field" { town: i16, field: i16 },
        /// Area 2, this town, field and dungeon.
        InDungeon = "in_dungeon" { town: i16, field: i16, dungeon: i16 },
        /// `saveData.eventStatus[index]` against `num`.
        Status = "status" { index: i16, num: i16, comp: Cmp },
        /// `lo <= saveData.eventStatus[index] <= hi`.
        StatusRange = "status_range" { index: i16, lo: i16, hi: i16 },
        /// The words entered at the Chaos Gate are story area `area`'s, on its
        /// server; `except` 1 negates, anything but 0 or 1 never passes. Fails
        /// whatever `except` is while any word is unset.
        GateWords = "gate_words" { area: i16, except: i16 },
        /// The player tried to talk (operation 9) to this character.
        TalkedTo = "talked_to" { ty = "type": i16, code: i16 },
        /// The operation the player tried this frame is `num` (-1: one that is
        /// not intercepted); `except` 1: it is not `num` (-1: it is intercepted).
        Operate = "operate" { num: i16, except: i16 },
        /// Player-to-marker distance against `bounds` * 10.
        NearMarker = "near_marker" { marker: i16, bounds: i16, comp: Cmp },
        /// The last question was message `msg` and the answer was `select`.
        Answer = "answer" { msg: i16, select: i16 },
        /// `bbsList[thread][post] == 3`.
        BbsRead = "bbs_read" { thread: i16, post: i16 },
        /// `mailList[mail]` is 4, 5 or 6.
        MailGot = "mail_got" { mail: i16 },
        /// `mailList[mail] == 4`.
        Mail4 = "mail_4" { mail: i16 },
        /// `mailList[mail] == 5`.
        Mail5 = "mail_5" { mail: i16 },
        /// `mailList[mail] == 6`.
        Mail6 = "mail_6" { mail: i16 },
        /// `webnewsList[news] == 3`.
        NewsRead = "news_read" { news: i16 },
        /// `pc` is in the party (-1: the party has two or more).
        InParty = "in_party" { pc: i16 },
        /// `pc` is not in the party (-1: the party has fewer than two).
        NotInParty = "not_in_party" { pc: i16 },
        /// The party has exactly two and `pc` is one of them.
        InPartyOf2 = "in_party_of_2" { pc: i16 },
        /// Party slot 1 or 2 holds someone other than `pc`.
        PartyOther = "party_other" { pc: i16 },
        /// `partyMemberCall` bit `pc`.
        Callable = "callable" { pc: i16 },
        /// The character is present (type 2 party, 5/6 entries, 20 gimmicks, 7 the boss still fighting).
        Present = "present" { ty = "type": i16, code: i16 },
        /// Not present (type 2 always passes; type 7: the boss has finished).
        Absent = "absent" { ty = "type": i16, code: i16 },
        /// No active object and no active gimmick.
        NoActive = "no_active" {},
        /// No registered entries.
        NoEntries = "no_entries" {},
        /// No field menu open.
        NoMenu = "no_menu" {},
        /// Item (`category`, `id`) count in `pc`'s list against `num`; category 15 is `impItemList`.
        HasItem = "has_item" { pc: i16, category: i16, id: i16, num: i16, comp: Cmp },
        /// `spcParam[pc].friendship` against `num`.
        Friendship = "friendship" { pc: i16, num: i16, comp: Cmp },
        /// A pad button in `mask` was pushed this frame.
        Pad = "pad" { mask: i16 },
        /// Enemy `enemy`'s PP count is not zero (`type` is not used).
        EnemyPp = "enemy_pp" { ty = "type": i16, enemy: i16 },
        /// `partyMemberFlag` bit `pc`.
        Member = "member" { pc: i16 },
        /// `partyMemberSave` bit `pc`.
        MemberSaved = "member_saved" { pc: i16 },
        /// `volumeNum` against `num`.
        Volume = "volume" { num: i16, comp: Cmp },
        /// The current floor and block.
        InRoom = "in_room" { floor: i16, block: i16 },
    }
}

instruction_set! {
    /// An instruction. The effects are described on each variant; see the
    /// interpreter ([`crate::vm`]) for which run when replaying (lv 1) and
    /// which only when playing (lv 2).
    Op, OpKind {
        /// Close event `grp` (flag bit 63); `grp` < 0 is this event, and -2 also marks it done (bit 62).
        EndEvent = "end_event" { grp: i16 },
        /// Set this event's block bit `num`; `num` < 0 sets bits 0 up to this block; `num` >= 30 halts the game.
        SetBlock = "set_block" { num: i16 },
        /// Clear this event's block bit `num`.
        ClearBlock = "clear_block" { num: i16 },
        /// This block does not mark itself as run, so it can run again.
        Repeatable = "repeatable" {},
        /// Wait `count` + 1 frames.
        Wait = "wait" { count: i16 },
        /// Speech window with message `msg`; a question stores its answer.
        Message = "message" { msg: i16 },
        /// Forget the last answer.
        ClearAnswer = "clear_answer" {},
        /// Information window with message `msg`.
        Info = "info" { msg: i16 },
        /// Information window, shown without its opening fade.
        InfoNow = "info_now" { msg: i16 },
        /// Play cutscene stream `num`.
        Stream = "stream" { num: i16 },
        /// Register a character to appear.
        Entry = "entry" { ty = "type": i16, code: i16, marker: i16, param: i16 },
        /// Register a character to appear (the `Mc` list).
        EntryMc = "entry_mc" { ty = "type": i16, code: i16, marker: i16, param: i16 },
        /// Remove a character.
        Remove = "remove" { ty = "type": i16, code: i16 },
        /// Intercept player operation `num` (-1: all of 0-17; `except` 1: all but `num`).
        AddOperate = "add_operate" { num: i16, except: i16 },
        /// Stop intercepting operation `num` (same encoding).
        DelOperate = "del_operate" { num: i16, except: i16 },
        /// Talking to this character goes to the event.
        AddTarget = "add_target" { ty = "type": i16, code: i16 },
        /// Undo `add_target`.
        DelTarget = "del_target" { ty = "type": i16, code: i16 },
        /// Cutscene on: menus forbidden and hidden, party under manual control.
        MenuBan = "menu_ban" {},
        /// Cutscene off.
        MenuClear = "menu_clear" {},
        AddAreaCode = "add_area_code" { code: i16, except: i16 },
        DelAreaCode = "del_area_code" { code: i16, except: i16 },
        CamLookPos = "cam_look_pos" { x: i16, y: i16, z: i16 },
        CamLookChar = "cam_look_char" { ty = "type": i16, code: i16, height: i16 },
        CamFollowChar = "cam_follow_char" { ty = "type": i16, code: i16, height: i16 },
        CamLookMarker = "cam_look_marker" { marker: i16 },
        CamLookPosHalf = "cam_look_pos_half" { x: i16, y: i16, z: i16 },
        CamLookCharHalf = "cam_look_char_half" { ty = "type": i16, code: i16, height: i16 },
        CamLookMarkerHalf = "cam_look_marker_half" { marker: i16 },
        CamPanPos = "cam_pan_pos" { x: i16, y: i16, z: i16, rate: i16 },
        CamPanChar = "cam_pan_char" { ty = "type": i16, code: i16, height: i16, rate: i16 },
        CamPanFollow = "cam_pan_follow" { ty = "type": i16, code: i16, height: i16, rate: i16 },
        CamPanMarker = "cam_pan_marker" { marker: i16, rate: i16 },
        CamOrbit = "cam_orbit" { rotx: i16, roty: i16, dist: i16 },
        CamOrbitMove = "cam_orbit_move" { rotx: i16, roty: i16, dist: i16, rate: i16 },
        CamOrbitTurn = "cam_orbit_turn" { rotx: i16, roty: i16, dist: i16, rate: i16 },
        CamMode4 = "cam_mode4" {},
        /// Camera tutorial, part 1 (this event's message 7 or 55).
        TeachCamera1 = "teach_camera1" {},
        /// Camera tutorial, part 2 (messages 9 or 56).
        TeachCamera2 = "teach_camera2" {},
        /// Camera tutorial, part 3 (messages 11 or 57).
        TeachCamera3 = "teach_camera3" {},
        CamzSet = "camz_set" { vx: i16, vy: i16, vz: i16, cx: i16, cy: i16, cz: i16 },
        CamzMove = "camz_move" { vx: i16, vy: i16, vz: i16, cx: i16, cy: i16, cz: i16, vprate: i16, cprate: i16 },
        CamzSpeed = "camz_speed" { vstype: i16, cstype: i16 },
        CamzPoint = "camz_point" { num: i16, vx: i16, vy: i16, vz: i16, cx: i16, cy: i16, cz: i16 },
        CamzPath = "camz_path" { num: i16, vprate: i16, cprate: i16, alpha: i16 },
        Radiator = "radiator" { rtype: i16, ty = "type": i16, code: i16, x: i16, y: i16, z: i16, roty: i16, rotz: i16 },
        BossSmoke = "boss_smoke" { floor: i16, block: i16, x: i16, y: i16, z: i16 },
        DeleteGimmick19 = "delete_gimmick19" {},
        Camera = "camera" { ty = "type": i16, code: i16, height: i16, rotx: i16, roty: i16, dist: i16 },
        CameraEnd = "camera_end" {},
        CameraEndReset = "camera_end_reset" {},
        NpcAct = "npc_act" { npc: i16, param: i16 },
        NpcWalkPos = "npc_walk_pos" { npc: i16, x: i16, y: i16, z: i16 },
        NpcWalkDir = "npc_walk_dir" { npc: i16, rot: i16, dist: i16 },
        NpcWalkMarker = "npc_walk_marker" { npc: i16, marker: i16 },
        NpcWalkChar = "npc_walk_char" { npc: i16, ty = "type": i16, code: i16, rot: i16, dist: i16 },
        NpcPutMarker = "npc_put_marker" { npc: i16, marker: i16 },
        NpcPut = "npc_put" { npc: i16, x: i16, y: i16, z: i16 },
        NpcTurn = "npc_turn" { npc: i16, dirc: i16, chg: i16 },
        NpcFace = "npc_face" { npc: i16, ty = "type": i16, code: i16, chg: i16 },
        PcAct = "pc_act" { pc: i16, act: i16 },
        PcWalkPos = "pc_walk_pos" { pc: i16, x: i16, y: i16, z: i16 },
        PcWalkDir = "pc_walk_dir" { pc: i16, rot: i16, dist: i16 },
        PcWalkMarker = "pc_walk_marker" { pc: i16, marker: i16 },
        PcWalkChar = "pc_walk_char" { pc: i16, ty = "type": i16, code: i16, rot: i16, dist: i16 },
        PcMode = "pc_mode" { pc: i16, param: i16 },
        PcCommand = "pc_command" { pc: i16, on: i16 },
        PcPutMarker = "pc_put_marker" { pc: i16, marker: i16 },
        PartyPutMarker = "party_put_marker" { pc: i16, marker: i16 },
        PartyPut = "party_put" { pc: i16, x: i16, y: i16, z: i16 },
        PcPut = "pc_put" { pc: i16, x: i16, y: i16, z: i16 },
        PcTurn = "pc_turn" { pc: i16, dirc: i16, chg: i16 },
        PcFace = "pc_face" { pc: i16, ty = "type": i16, code: i16, chg: i16 },
        PcUseSkill = "pc_use_skill" { pc: i16, skill: i16 },
        EnemyPut = "enemy_put" { enemy: i16, posnum: i16 },
        AffectOn = "affect_on" { ty = "type": i16, code: i16, bit: i16 },
        AffectOff = "affect_off" { ty = "type": i16, code: i16, bit: i16 },
        PartyAdd = "party_add" { pc: i16 },
        PartyRemove = "party_remove" { pc: i16 },
        /// `partyMemberFlag` and `partyMemberExp` bit `pc`.
        MemberAdd = "member_add" { pc: i16 },
        /// As `member_add`; when playing, a sound and an information box.
        MemberAddMsg = "member_add_msg" { pc: i16 },
        /// `partyMemberCall` bit `pc` (and the stored copy while calls are locked).
        CallOn = "call_on" { pc: i16 },
        /// `partyMemberCall` bit `pc`, or only the stored copy while locked.
        CallOnLater = "call_on_later" { pc: i16 },
        /// Clear `partyMemberCall` bit `pc` (and in the stored copy while locked).
        CallOff = "call_off" { pc: i16 },
        /// Lock calls: store `partyMemberCall`, set bit 31, clear bits 0-17.
        CallLock = "call_lock" {},
        /// Unlock calls: restore the stored copy.
        CallUnlock = "call_unlock" {},
        ExpOn = "exp_on" { pc: i16 },
        ExpOff = "exp_off" { pc: i16 },
        /// `partyMemberSave` = the current party.
        SaveParty = "save_party" {},
        /// Story area into its server's Chaos Gate list, its words into the word list.
        GateAdd = "gate_add" { area: i16 },
        /// As `gate_add`; when playing, an information box with the address.
        GateAddMsg = "gate_add_msg" { area: i16 },
        GateMark = "gate_mark" { area: i16 },
        GateUnmark = "gate_unmark" { area: i16 },
        ItemAdd = "item_add" { pc: i16, category: i16, id: i16, num: i16 },
        ItemAddMenu = "item_add_menu" { pc: i16, category: i16, id: i16, num: i16 },
        ItemDel = "item_del" { pc: i16, category: i16, id: i16, num: i16 },
        GoldAdd = "gold_add" { pc: i16, num: i16 },
        Noise2 = "noise2" {},
        Noise1 = "noise1" {},
        /// Infection's form of 99, without an operand.
        Noise3 = "noise3" {},
        /// Mutation on: interference level (1-3; anything else turns it off).
        Noise = "noise" { level: i16 },
        Scene = "scene" { area: i16, town: i16, field: i16, dungeon: i16, floor: i16, block: i16 },
        /// Change mode (`ccGame::ChangeRequest`).
        Mode = "mode" { num: i16 },
        WallpaperAdd = "wallpaper_add" { num: i16 },
        BgmAdd = "bgm_add" { num: i16 },
        /// Board post appears (state 1) if absent; replaying marks it read (3).
        BbsPost = "bbs_post" { thread: i16, post: i16 },
        BbsRemove = "bbs_remove" { thread: i16, post: i16 },
        /// As `bbs_post` with state 7.
        BbsPost7 = "bbs_post7" { thread: i16, post: i16 },
        /// New mail if never received; replaying marks it read.
        Mail = "mail" { mail: i16 },
        /// As `mail`, only while `clearFlag` is at most this event's volume index.
        MailVol = "mail_vol" { mail: i16 },
        /// As `mail`, only if `pc` is in `partyMemberSave`.
        MailMember = "mail_member" { mail: i16, pc: i16 },
        MailRemove = "mail_remove" { mail: i16 },
        NewsAdd = "news_add" { news: i16 },
        NewsRemove = "news_remove" { news: i16 },
        FrameRate = "frame_rate" { rate: i16 },
        /// Load overlay 0 DEMO, 1 DESKTOP, 2 TOPPAGE, 3 GCMN.
        Overlay = "overlay" { num: i16 },
        Crisis = "crisis" { num: i16 },
        SetPoint = "set_point" { floor: i16, block: i16, evnum: i16 },
        SetPos = "set_pos" { floor: i16, block: i16, posnum: i16, dirc: i16, x: i16, y: i16, z: i16 },
        Area = "area" { area: i16 },
        ClearOperate = "clear_operate" {},
        ClearAreaCode = "clear_area_code" {},
        MapOn = "map_on" {},
        ShowMap = "show_map" {},
        RemoveTrap = "remove_trap" {},
        /// Replaying only: regenerate the area, protect it, take its protect items from Kite.
        VirusCore = "virus_core" { area: i16 },
        TownMove = "town_move" { town: i16 },
        DataDrain = "data_drain" {},
        PrevRoom = "prev_room" {},
        AreaBan = "area_ban" { field: i16, dungeon: i16, floor: i16, block: i16 },
        AreaUnban = "area_unban" { field: i16, dungeon: i16, floor: i16, block: i16 },
        Room = "room" { floor: i16, block: i16 },
        RoomPoint = "room_point" { num: i16 },
        Hold = "hold" { ty = "type": i16, code: i16 },
        HoldEnd = "hold_end" {},
        PcTint = "pc_tint" { pc: i16, r: i16, g: i16, b: i16, rate: i16 },
        PcTintOff = "pc_tint_off" { pc: i16 },
        PirosColour = "piros_colour" { code: i16 },
        StatusSet = "status_set" { index: i16, num: i16 },
        StatusAdd = "status_add" { index: i16, num: i16 },
        StatusSub = "status_sub" { index: i16, num: i16 },
        Protect = "protect" { area: i16 },
        TalkNum = "talk_num" { pc: i16, num: i16 },
        RegistNpc16 = "regist_npc16" {},
        MarkerPos = "marker_pos" { marker: i16, posnum: i16 },
        Fade = "fade" { count: i16, alpha: i16 },
        FadeMore = "fade_more" { count: i16, alpha: i16 },
        StaffRoll = "staff_roll" {},
        ClearCount = "clear_count" {},
        Friendship = "friendship" { pc: i16, num: i16 },
        Menu = "menu" { num: i16 },
        ConditionFxOn = "condition_fx_on" {},
        ConditionFxOff = "condition_fx_off" {},
        PlayerSkill = "player_skill" {},
        TargetForbid = "target_forbid" { num: i16 },
        LastTown = "last_town" { town: i16 },
        NameEntry = "name_entry" {},
        GateHackAnim = "gate_hack_anim" {},
        OpenDoor = "open_door" {},
        CloseDoor = "close_door" {},
        Sound = "sound" { cmd: i16, p0: i16, p1: i16, p2: i16 },
        TransOff = "trans_off" { ty = "type": i16, code: i16 },
        TransOn = "trans_on" { ty = "type": i16, code: i16 },
        BattleReady = "battle_ready" {},
        PcRunPos = "pc_run_pos" { pc: i16, x: i16, y: i16, z: i16 },
        AddOperateSf = "add_operate_sf" { num: i16, except: i16 },
        DelOperateSf = "del_operate_sf" { num: i16, except: i16 },
        /// Wallpaper (0), BGM (1) or movie (2) `id` for the desktop.
        DesktopItem = "desktop_item" { ty = "type": i16, id: i16 },
        /// Mutation on: mail 324 once a town has all three Grunty types grown up.
        GruntyMail = "grunty_mail" {},
        /// Quarantine: the ending's kanji sequence.
        EndingKanji = "ending_kanji" {},
    }
}

/// A block: precondition settings, conditions, body.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Block {
    pub tags: Vec<Tag>,
    pub conds: Vec<Cond>,
    pub ops: Vec<Op>,
}

/// A script: the event is live while every open condition holds.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Script {
    pub open: Vec<Cond>,
    pub blocks: Vec<Block>,
}

/// Text in the game's encoding: ASCII plus Shift-JIS, with the escapes
/// `ccKanji::Disp` understands (`#0` the character's name, `#1` the
/// player's, `#R #G #B #Y #W` colours, `%x` extended font codes). Kept as
/// bytes so it round-trips exactly.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct GameText(pub Vec<u8>);

impl fmt::Debug for GameText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", crate::text::quote(&self.0))
    }
}

impl GameText {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<&str> for GameText {
    /// ASCII only; anything else goes through [`crate::text::unquote`].
    fn from(s: &str) -> Self {
        GameText(s.as_bytes().to_vec())
    }
}

/// One message record (`ccMsgData`: `emode`, speaker, text).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Message {
    /// `emode`. The game tests only the low byte: 0 a plain line, 3 a
    /// question; the rest is not traced.
    pub mode: i32,
    /// The speaker's name, if the record has one.
    pub name: Option<GameText>,
    /// Up to three lines; trailing empty lines are not kept.
    pub lines: Vec<GameText>,
}

impl Message {
    pub fn is_question(&self) -> bool {
        self.mode & 0xff == 3
    }
}

/// One event: its number, label, script and message tables.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Event {
    pub number: u16,
    /// A short label such as a scene code; not shown to the player.
    pub label: GameText,
    pub script: Script,
    /// The normal message table.
    pub messages: Vec<Message>,
    /// Parody Mode's table.
    pub parody: Vec<Message>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_and_args_agree() {
        for &k in OpKind::ALL {
            let args: Vec<i16> = (0..k.arity() as i16).map(|i| i * 3 - 1).collect();
            let op = k.build(&args).unwrap();
            assert_eq!(op.kind(), k);
            assert_eq!(op.args(), args);
            assert!(k.build(&[0; 20]).is_none());
        }
        for &k in CondKind::ALL {
            assert_eq!(CondKind::from_name(k.name()), Some(k));
        }
        for &k in TagKind::ALL {
            assert_eq!(TagKind::from_name(k.name()), Some(k));
        }
    }

    #[test]
    fn names_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for &k in OpKind::ALL {
            assert!(seen.insert(k.name()), "{}", k.name());
        }
        assert_eq!(OpKind::ALL.len(), 169);
        assert_eq!(CondKind::ALL.len(), 40);
        assert_eq!(TagKind::ALL.len(), 10);
    }
}
