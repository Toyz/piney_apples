//! What runs: one mode at a time, stepped once per game frame, as
//! `ccThMother` runs the demo, the desktop, the top page or the game.

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_draw::{Blend, Cmd, DrawState, Frame, Prim, PrimKind, Rgba, TexRef, TexState, Vertex};
use piney_gs::Assets;
use piney_input::{Buttons, Pad};

static VOICE_ENGLISH: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// `--voice en|jp`: the voice language for the whole run, over the save's
/// `voice` (what the game's Options screen sets). The first call wins.
pub fn set_voice_override(english: bool) {
    let _ = VOICE_ENGLISH.set(english);
}

/// Whether voices play in English: `--voice` when given, else the save's
/// `voice` (+0x842c, 1 for English, `VOICE_E/`), which the game's sound
/// driver and stream player read.
pub fn voice_english(save: &piney_data::save::SaveData) -> bool {
    VOICE_ENGLISH.get().copied().unwrap_or(save.u8(piney_data::save::offset::VOICE) != 0)
}

/// What the sound driver reads of the save for a voice request: the
/// Voiceover option ([`voice_english`]), Parody Mode, and `talkNum` by
/// `charTbl` row (0-17 in `ccSaveData`, 18-20 in the extension).
pub fn voice_options(save: &piney_data::save::SaveData) -> Event {
    let talk_num = std::array::from_fn(|id| save.u8(piney_data::save::by_id::talk_num(id)) as i8);
    Event::VoiceOptions {
        english: voice_english(save),
        parody: save.u8(piney_data::save::offset::PARODY_FLAG) != 0,
        talk_num,
    }
}

/// `ccSndEvRequest(cmd, p0, p1, p2)` (0x0017de80), the event instruction
/// `sound`, as the sound it asks for. Its switch has eleven cases:
///
/// - 0: `ccSqPlay(p0)`.
/// - 2: `ccSqFade(p0, p1, p2, 3)`: to `p1` 256ths over `p2` frames, then
///   stop.
/// - 4: `ccSeOn(p0)`, or with `p1` not -1 `ccSeOnNote(p0, note)`, the note
///   being `p1`'s low byte, below 0 taken as 0.
/// - 7: `ccEvVoiceStop`.
/// - 8: `ccPortVolSet(p0, p1)`.
/// - 10: the next `ccSndBgmCtrl` held.
/// - 1, 3, 5, 6, 9: nothing (None).
pub fn sound_request(cmd: i16, p0: i16, p1: i16, p2: i16) -> Option<Event> {
    match cmd {
        0 => Some(Event::SqPlay(i32::from(p0))),
        2 => Some(Event::SqFade { seq: i32::from(p0), volume: p1 as u16, time: i32::from(p2), mode: 3 }),
        4 if p1 == -1 => Some(Event::Se(i32::from(p0))),
        4 => usize::try_from(p0).ok().map(|n| Event::SeNote { n, note: (p1 as i8).max(0) }),
        7 => Some(Event::VoiceStop),
        8 => usize::try_from(p0).ok().map(|port| Event::PortVolume { port, volume: p1 as u16 }),
        10 => Some(Event::HoldBgm),
        _ => None,
    }
}

/// What a mode asks of the rest of the game after a frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// `ccSeOn(n)`.
    Se(i32),
    /// Not the game's: the launcher's choice, the disc to start the game on
    /// (a `.disc` of a build).
    Boot(std::path::PathBuf),
    /// `ccSeOnNote(n, note)`.
    SeNote {
        n: usize,
        note: i8,
    },
    /// `ccSeOn3D(n, pos)` / `ccSeOn3DNote(n, pos, note)` heard from the
    /// active camera (`None`: no camera, full and centred).
    Se3d {
        n: usize,
        pos: [u32; 4],
        note: Option<i8>,
        ear: Option<piney_audio::se3d::Listener>,
    },
    /// `ccSndGameOver()`: the game over's fade of the music.
    SoundGameOver,
    /// `tobjSeLoopStart`: a field's TOBJ hum started, silent.
    TobjSeLoopStart,
    /// `tobjSeLoop(pos, rate)`: the hum's pan and volume, heard from the
    /// active camera.
    TobjSeLoop {
        pos: [u32; 4],
        rate: u32,
        ear: piney_audio::se3d::Listener,
    },
    /// The desktop's music at start-up (`ccSndChangeData(&Wave[n], -1)`).
    DesktopBgm(usize),
    /// Change to music `n` of the jukebox (`Wave[n]`), fading the old out.
    Bgm(usize),
    /// `ccSoundFadeOut()`.
    SoundFadeOut,
    /// `ccBgmPlay(n)`: `VOICE/BGM.BIN` track `n` (the staff roll's 1).
    BgmStream(usize),
    /// `ccBgmStop()`.
    BgmStreamStop,
    /// The riding Grunty's music: `ccPgBgmInit`, `ccPgBgmEnd(n)`,
    /// `pgRideFlag` (`piney_audio::Audio::pg_bgm`).
    PgBgm(piney_audio::PgBgm),
    /// `ccWordsPlay(sid, ch)`: a party member names the skill it uses
    /// (`char_type` its base's type, `char_id` its `charTbl` row, `type_bit`
    /// bit 0 of `ccGetSkillParam(sid)+0x2c`, `event_running` eventMng
    /// +0x78c).
    /// Raised by the fights' skill requests (`skill::Request::words`,
    /// `piney_world::combat::Show::Words`).
    SkillWords {
        event_running: bool,
        char_type: u32,
        char_id: i16,
        sid: i32,
        type_bit: bool,
    },
    /// `game.area` for the sound task (0 a town, 1 a field, 2 a dungeon).
    GameArea(i32),
    /// `ccSoundMain`'s scene sounds' inputs this frame
    /// (`piney_audio::scene`): the canals, the church, the breeder.
    SceneSound(piney_audio::scene::SceneInput),
    /// A movie stream to play before the next frame (`SimplePlayStream`).
    Movie(i32),
    /// `ccSaveData::SetSoundEnv`: the volumes, 0-256.
    Volumes {
        main: i32,
        se: i32,
        bgm: i32,
    },
    /// `ccEvVoiceRequest(event, msg)`: a message window's voice line.
    Voice {
        event: i32,
        msg: i32,
    },
    /// `ccEvVoiceStop`: the line cut short.
    VoiceStop,
    /// `ccPad::SetActuater(pad 0, small, power, ms)`: the pad rumbles (a
    /// hit on Kite, the Vibration menus switched on).
    Actuate {
        small: bool,
        power: u8,
        ms: i32,
    },
    /// The Voiceover option (English), Parody Mode and `talkNum`, from the
    /// save ([`voice_options`]).
    VoiceOptions {
        english: bool,
        parody: bool,
        talk_num: [i8; 21],
    },
    /// `ccSndSQLoad(context)`: a mode's sequence bank.
    SqLoad(piney_audio::SqContext),
    /// `ccSndBgmCtrl()`: the loaded bank's music for the area
    /// (`piney_audio::Audio::bgm_ctrl`).
    BgmCtrl(piney_audio::BgmWorld),
    /// `ccSnd.gameStart = 1`: the game's fades and the battle switch run.
    GameStart,
    /// `game.inBattle` not 0, which the sound task's `bgmChange` reads each
    /// frame (`piney_audio::Audio::set_battle`): the battle music in and
    /// out.
    InBattle(bool),
    /// `ccSqPlay`, `ccSqStop`, `ccSqFade`: the loaded bank's sequences.
    SqPlay(i32),
    SqStop(i32),
    SqFade {
        seq: i32,
        volume: u16,
        time: i32,
        mode: u8,
    },
    /// `ccSetMainVol`.
    MainVolume(i32),
    /// `ccPortVolSet(port, volume)` (the event instruction `sound 8`).
    PortVolume {
        port: usize,
        volume: u16,
    },
    /// The event instruction `sound 10`: the next `ccSndBgmCtrl` starts
    /// nothing (`ccSnd +0x138`).
    HoldBgm,
    /// `ccAllSoundOff`, first in each mode's setup.
    AllSoundOff,
    /// `ccSound::gameInterrupt`, at `ccGame::ChangeRequest`.
    GameInterrupt,
    /// `audioDecStart`: a PSS movie's sound, interleaved 48 kHz stereo.
    MovieAudio(Vec<i16>),
    /// `audioDecReset`: the movie's sound stopped (also a stream's).
    MovieAudioStop,
    /// A stream's samples, queued after the last ones (`CcspcmSetData`).
    StreamPcm(Vec<i16>),
    /// `ccSndStreamCtrl(num, sd, after)`: the loaded bank's music before an
    /// event's stream and after it (`piney_audio::stream::stream_ctrl`);
    /// `size` is the stream header's music bits.
    StreamMusic {
        num: usize,
        size: i32,
        after: bool,
        game: piney_audio::stream::StreamGame,
    },
    /// `ccSndStreamBGM`: a record of the stream's `strSndTbl` BGM table,
    /// at one of its notes of event 4.
    StreamBgm(piney_audio::stream::StrBgm),
    /// `ccGame::ChangeArea(area, town)`: the scene the next mode enters
    /// (Log in: area 0, a town, and `saveData.lastTown`).
    ChangeArea {
        area: i32,
        town: i32,
    },
    /// `ccGame::ChangeRequest(num, sf)`: another mode.
    ChangeMode {
        num: i32,
        sf: i32,
    },
    /// `ccGame::ChangeScene` (or `ChangeArea`) in The World, then its
    /// `ChangeRequest(6, 7)`: `ccSetupGameCtrl` for this scene, after the
    /// fade out over the one that asked.
    ChangeScene(piney_world::area::Scene),
    /// The same from a mode that does not hold the scene (the town's
    /// menus, the event instructions): the call, which the session makes on
    /// its scene.
    Go(piney_data::area::Go),
    /// The gate hack's OK (`GtHackMenu`): `ccGame.setupMode = 1` and
    /// `ccSetGtHack()` (`gtHackFlag`), which the next scene's set-up reads;
    /// [`Event::GoToArea`] follows.
    GateHacked,
    /// `WORLD_MAN::SetGenerateCode(a, b, c)` (the Chaos Gate's warp): the
    /// area the three word IDs make, and the change of scene to its field
    /// or dungeon.
    GoToArea([i32; 3]),
    /// `WORLD_MAN::SimGenerateCode`'s area (the event instruction `area`,
    /// `piney_data::area::ev_area`): what the next field and dungeon are
    /// made from. The event host raises it once it runs in The World.
    #[allow(dead_code)]
    WorldMan(piney_world::area::WorldMan),
}

pub trait Mode {
    /// One game frame: read the pad, move on, say what to draw.
    fn step(&mut self, pad: &Pad) -> Frame;

    /// What the frames since the last call asked for, in order.
    fn take_events(&mut self) -> Vec<Event> {
        Vec::new()
    }

    /// Vertical blanks per game frame (`ccSystem::frameRate`): 1 on the
    /// desktop, 2 in the field.
    fn frame_rate(&self) -> u32 {
        1
    }

    /// Files the frames name that are not in `DATA.BIN`: a stream's own
    /// archive while one plays.
    fn archive(&self) -> Option<Arc<Archive>> {
        None
    }

    /// For the window's title.
    fn title(&self) -> String;

    /// A test hook by name (`--press F:NAME` for the ones that are not
    /// buttons: `gateout`, `gofield`): true when the mode took it.
    fn hook(&mut self, _name: &str) -> bool {
        false
    }

    /// A debug console line ([`crate::console`]): the answer to show.
    fn console(&mut self, _line: &str) -> String {
        "no console commands here".into()
    }

    /// `ccPad::actuaterSw`: the save's Vibration option (+0x8428), which
    /// the game sets it from at a load and a new game and with it in the
    /// Vibration menus; None without a save.
    fn vibration(&mut self) -> Option<bool> {
        None
    }

    /// Whether the mode plays something that ends by itself (a loose
    /// stream), so a `--shot` run with no `--frames` goes until [`finished`].
    ///
    /// [`finished`]: Mode::finished
    fn ends(&self) -> bool {
        false
    }

    /// The thing [`ends`] promised has played through once.
    ///
    /// [`ends`]: Mode::ends
    fn finished(&self) -> bool {
        false
    }

    /// A copy of the save the mode holds, for the port's quit prompt's OK
    /// and Cancel buttons; None where there is none.
    fn save_copy(&mut self) -> Option<piney_data::save::SaveData> {
        None
    }
}

/// Every texture of one file laid out on the screen, with a cursor the pad
/// moves: a check of the GS path and the pad before anything real runs.
pub struct TestCard {
    file: String,
    /// (TEX_ object, CLT_ object, width, height).
    textures: Vec<(u32, u32, u32, u32)>,
    cursor: usize,
    frame: u32,
}

/// The cell each texture is shown in.
const CELL: f32 = 64.0;
const COLUMNS: usize = 8;

impl TestCard {
    pub fn new(assets: &mut Assets, file: &str) -> Option<Self> {
        let f = assets.file(file)?;
        let textures = f.textures.iter().map(|t| (t.object, t.clut, t.width(0), t.height(0))).collect();
        Some(TestCard { file: file.to_string(), textures, cursor: 0, frame: 0 })
    }
}

fn vertex(x: f32, y: f32, u: f32, v: f32, rgba: Rgba) -> Vertex {
    Vertex { x, y, z: 0, u, v, rgba }
}

impl Mode for TestCard {
    fn step(&mut self, pad: &Pad) -> Frame {
        self.frame += 1;
        let n = self.textures.len().max(1);
        let r = pad.repeat;
        if r.contains(Buttons::RIGHT) {
            self.cursor = (self.cursor + 1) % n;
        }
        if r.contains(Buttons::LEFT) {
            self.cursor = (self.cursor + n - 1) % n;
        }
        if r.contains(Buttons::DOWN) {
            self.cursor = (self.cursor + COLUMNS) % n;
        }
        if r.contains(Buttons::UP) {
            self.cursor = (self.cursor + n - COLUMNS % n) % n;
        }

        let mut frame = Frame::new();
        frame.clear = Rgba::new(0x10, 0x18, 0x30, 0);
        // A shaded backdrop: a Gouraud strip, untextured.
        let (w, h) = (f32::from(frame.width), f32::from(frame.height));
        frame.cmds.push(Cmd::Prim(Prim {
            kind: PrimKind::Strip,
            gouraud: true,
            state: DrawState::sprite(Blend::MIX, None),
            verts: vec![
                vertex(0.0, 0.0, 0.0, 0.0, Rgba::new(0x20, 0x30, 0x60, 0x80)),
                vertex(w, 0.0, 0.0, 0.0, Rgba::new(0x10, 0x10, 0x30, 0x80)),
                vertex(0.0, h, 0.0, 0.0, Rgba::new(0x00, 0x00, 0x10, 0x80)),
                vertex(w, h, 0.0, 0.0, Rgba::new(0x30, 0x10, 0x30, 0x80)),
            ],
        }));
        for (i, &(tex, clut, tw, th)) in self.textures.iter().enumerate() {
            let (col, row) = (i % COLUMNS, i / COLUMNS);
            let (x, y) = (16.0 + col as f32 * (CELL + 8.0), 16.0 + row as f32 * (CELL + 8.0));
            let scale = CELL / tw.max(th) as f32;
            let (dw, dh) = (tw as f32 * scale, th as f32 * scale);
            let t = TexState::modulate(TexRef::Ccs { file: self.file.clone(), texture: tex, clut });
            if i == self.cursor {
                // The cursor: a pulsing additive box behind the texture.
                let pulse = (0x40 as f32 + 0x3f as f32 * (self.frame as f32 * 0.1).sin()) as u8;
                frame.cmds.push(Cmd::Prim(Prim {
                    kind: PrimKind::Sprite,
                    gouraud: false,
                    state: DrawState::sprite(Blend::ADD, None),
                    verts: vec![
                        vertex(x - 4.0, y - 4.0, 0.0, 0.0, Rgba::NEUTRAL),
                        vertex(x + dw + 4.0, y + dh + 4.0, 0.0, 0.0, Rgba::new(0x80, 0x80, 0x40, pulse)),
                    ],
                }));
            }
            frame.cmds.push(Cmd::Prim(Prim {
                kind: PrimKind::Sprite,
                gouraud: false,
                state: DrawState::sprite(Blend::MIX, Some(t)),
                // Rows are stored bottom-up: the image's top is the last row.
                verts: vec![
                    vertex(x, y, 0.0, th as f32, Rgba::NEUTRAL),
                    vertex(x + dw, y + dh, tw as f32, 0.0, Rgba::NEUTRAL),
                ],
            }));
        }
        frame
    }

    fn title(&self) -> String {
        let t = self.textures.get(self.cursor);
        format!(
            "test card {} - texture {}/{}{}",
            self.file,
            self.cursor + 1,
            self.textures.len(),
            t.map(|t| format!(" ({}x{})", t.2, t.3)).unwrap_or_default()
        )
    }
}

#[cfg(test)]
mod sound_request_tests {
    use super::{Event, sound_request};

    /// The scripts' uses: a sound effect (`sound 4 74`), one at a note
    /// (`sound 4 160 58`), the SE port back to full (`sound 8 0 256`), the
    /// music held and faded, the voice stopped; 9 does nothing.
    #[test]
    fn the_scripts_sound_requests() {
        assert_eq!(sound_request(4, 74, -1, -1), Some(Event::Se(74)));
        assert_eq!(sound_request(4, 160, 58, 64), Some(Event::SeNote { n: 160, note: 58 }));
        assert_eq!(sound_request(4, 3, 200, -1), Some(Event::SeNote { n: 3, note: 0 }));
        assert_eq!(sound_request(8, 0, 256, -1), Some(Event::PortVolume { port: 0, volume: 256 }));
        assert_eq!(sound_request(10, -1, -1, -1), Some(Event::HoldBgm));
        assert_eq!(sound_request(2, 0, 0, 10), Some(Event::SqFade { seq: 0, volume: 0, time: 10, mode: 3 }));
        assert_eq!(sound_request(0, 2, -1, -1), Some(Event::SqPlay(2)));
        assert_eq!(sound_request(7, -1, -1, -1), Some(Event::VoiceStop));
        assert_eq!(sound_request(9, 1, -1, -1), None);
    }
}
