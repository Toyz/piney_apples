//! The battle's characters as the field draws them: each scene character's
//! body and `ccAnm` ([`Actor`]), made when the character first plays a clip
//! and taken away when it goes. Kite and the party are made with their own
//! files when the party is built; an enemy or a magic portal the first time
//! the entry control sets its clip ([`piney_battle::world::World::anim_set`]),
//! from the area's [`Looks`]: the one whose file has that clip.

use std::collections::BTreeMap;
use std::rc::Rc;

use piney_battle::geom::M4;
use piney_battle::world::{AnmSlot, Note};

use crate::body::Body;
use crate::char::Char;
use crate::ee::{F, ONE, V4};

/// Who an actor is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// Kite (`ctu1body`, his blades hung on his hands).
    Kite,
    /// A party member, its `charTbl` row.
    Member(i32),
    /// An enemy of `enemyTbl` row.
    Enemy(i32),
    /// A magic portal (`xmagcir`).
    Circle,
    /// A box, breakable or idol of that gimmick row.
    Gimmick(i32),
    /// Anything else the entry control makes.
    Other,
    /// A boss (`ccBoss`: Skeith's `x11`).
    Boss,
}

/// One character's drawable state and its animation player.
#[derive(Clone)]
pub struct Actor {
    pub ch: Char,
    pub look: Look,
    /// A middle boss's second model's player (`ccEnemy` +0x1bc).
    pub second: Option<crate::pose::Play>,
    /// The notes the last `_AnimateForward` passed (`NoteProcess` hands
    /// them on), by slot.
    pub notes: Vec<Note>,
    pub notes2: Vec<Note>,
    /// This frame's draw: the matrix `dispEnemy` set (an enemy), or the
    /// place and heading (`SetMatrix_PosRotZYX`) and the transparency the
    /// character's code left.
    pub matrix: Option<M4>,
    /// A middle boss's second model's draw this frame (`Call::DrawSecond`:
    /// the matrix and alpha `dispEnemy` gave it).
    pub second_draw: Option<(M4, F)>,
    pub drawn: bool,
    pub alpha: F,
    /// `ccChar` +0x90 `transDist`: the near fade on.
    pub trans_dist: bool,
    /// The clut swaps to draw with (Kite before the bracelet).
    pub swaps: Vec<(u32, u32)>,
    /// Kite's blades at their own transparency this frame
    /// (`GateHackingOut`'s acts 23 and 24), None as his body.
    pub arms: Option<f32>,
    /// An enemy's `ccChar::Draw` as it decided this frame (its fog blend,
    /// transparency, whether drawn and shaded).
    pub how: Option<crate::foe::CharDraw>,
    /// A party character's weapon trails and points
    /// (`ccSpcChar::ArmsEffect`), from its first `ArmsEffect`.
    pub weapon: Option<Box<crate::arms::Arms>>,
}

impl Actor {
    pub fn new(body: Rc<Body>, clip: &str, look: Look, pos: V4, dirc: V4, height: F, width: F) -> Option<Actor> {
        let ch = match Char::new(body.clone(), clip, pos, dirc, height, width) {
            Some(c) => c,
            // A clip not in the file: the body stands in its first.
            None => {
                let first = body.file.anims.first()?;
                let name = body.file.ccs.object_name(first.object)?.to_string();
                Char::new(body, &name, pos, dirc, height, width)?
            }
        };
        Some(Actor {
            ch,
            look,
            second: None,
            second_draw: None,
            notes: Vec::new(),
            notes2: Vec::new(),
            matrix: None,
            drawn: false,
            alpha: ONE,
            trans_dist: true,
            swaps: Vec::new(),
            arms: None,
            how: None,
            weapon: None,
        })
    }

    /// `ccAnm::SetAnm` of the named clip on `slot`: frame 0, the same
    /// frame speed. The notes the last step collected stay (`SetAnm` does
    /// not touch `noteRoot`, +0xa0).
    pub fn set(&mut self, slot: AnmSlot, name: &str) {
        match slot {
            AnmSlot::Main => {
                self.ch.set_anim(name);
            }
            AnmSlot::Second => {
                let spd = self.second.as_ref().map_or(256, |p| p.frame_spd);
                self.second = self.ch.body.play(name).map(|mut p| {
                    p.frame_spd = spd;
                    p
                });
            }
        }
    }

    /// `ccAnm.frameNow`.
    pub fn frame(&self, slot: AnmSlot) -> u16 {
        match slot {
            AnmSlot::Main => (self.ch.play.time >> 8) as u16,
            AnmSlot::Second => self.second.as_ref().map_or(0, |p| (p.time >> 8) as u16),
        }
    }

    /// `ccAnm::_AnimateForward(step)`: whether a play-once clip ended; the
    /// note list freed and filled with the notes of the frames the step
    /// entered (`DecodeFrameChunk`), newest first, for [`Actor::notes`].
    pub fn forward(&mut self, slot: AnmSlot, step: u16) -> i16 {
        let file = self.ch.body.file.clone();
        let (play, notes) = match slot {
            AnmSlot::Main => (&mut self.ch.play, &mut self.notes),
            AnmSlot::Second => match self.second.as_mut() {
                Some(p) => (p, &mut self.notes2),
                None => return 0,
            },
        };
        play.frame_spd = u32::from(step);
        let (ended, passed) = forward_notes(play, &file);
        notes.clear();
        notes.extend(passed.into_iter().map(|(event, param)| Note { event, param }));
        i16::from(ended)
    }

    /// `ccAnm::NoteProcess`: the notes the last step collected, in the
    /// order it hands them on; the list is kept until the next step.
    pub fn notes(&self, slot: AnmSlot) -> Vec<Note> {
        match slot {
            AnmSlot::Main => self.notes.clone(),
            AnmSlot::Second => self.notes2.clone(),
        }
    }
}

/// One `_AnimateForward` of `play` and the notes it collected, in the
/// order `NoteProcess` hands them on.
fn forward_notes(play: &mut crate::pose::Play, file: &piney_desktop::assets::SceneFile) -> (bool, Vec<(u32, u32)>) {
    play.forward_notes(file)
}

/// The bodies the area's enemies and portals may be drawn with, found by
/// the clip their constructors set first; each enemy row's look
/// ([`crate::foe::EnemyLook`]) and the portal's ([`crate::foe::Circle`]).
#[derive(Clone, Default)]
pub struct Looks {
    pub bodies: Vec<(Rc<Body>, Look, F, F)>,
    pub enemies: Vec<Rc<crate::foe::EnemyLook>>,
    pub circle: Option<Rc<crate::foe::Circle>>,
    /// The boxes', breakables' and idols' looks by gimmick row.
    pub gimmicks: Vec<Rc<crate::foe::GimLook>>,
}

impl Looks {
    /// The body whose file has `clip`: (body, look, height, width). Of
    /// several (the spring's two clumps share their file), the one whose
    /// nodes the clip drives.
    pub fn find(&self, clip: &str) -> Option<(Rc<Body>, Look, F, F)> {
        let mut first = None;
        for e in &self.bodies {
            let Some(a) = e.0.anim(clip) else { continue };
            if first.is_none() {
                first = Some(e.clone());
            }
            if e.0.file.anims[a].tracks.iter().any(|t| e.0.nodes.contains(&t.target)) {
                return Some(e.clone());
            }
        }
        first
    }

    /// An enemy row's look.
    pub fn enemy(&self, row: i32) -> Option<&crate::foe::EnemyLook> {
        self.enemies.iter().find(|l| l.ene_id == row).map(|l| &**l)
    }

    /// A gimmick row's look.
    pub fn gimmick(&self, row: i32) -> Option<&crate::foe::GimLook> {
        self.gimmicks.iter().find(|l| l.row == row).map(|l| &**l)
    }
}

/// Every actor by scene index.
#[derive(Clone, Default)]
pub struct Cast {
    pub actors: BTreeMap<usize, Actor>,
    pub looks: Looks,
    /// A gate-hacked arrival's camera animation and flags
    /// (`ccPlayer::GateHackingOut`).
    pub gate: super::gate_out::GateOut,
}

impl Cast {
    pub fn get(&self, who: usize) -> Option<&Actor> {
        self.actors.get(&who)
    }

    pub fn get_mut(&mut self, who: usize) -> Option<&mut Actor> {
        self.actors.get_mut(&who)
    }

    /// `anim_set` on `who`, making its actor from the looks when it has
    /// none.
    pub fn set(&mut self, who: usize, slot: AnmSlot, name: &str) {
        if !self.actors.contains_key(&who)
            && let Some((body, look, h, w)) = self.looks.find(name)
            && let Some(a) = Actor::new(body, name, look, crate::ee::VF0, [0; 4], h, w)
        {
            self.actors.insert(who, a);
        }
        if let Some(a) = self.actors.get_mut(&who) {
            a.set(slot, name);
        }
    }

    /// The frame's draws forgotten before the tasks run.
    pub fn new_frame(&mut self) {
        for a in self.actors.values_mut() {
            a.matrix = None;
            a.second_draw = None;
            a.drawn = false;
            a.arms = None;
        }
    }
}

/// The looks of an area: the rows `ccRegisterEnemyList(server, ty, rank)`
/// registers (three from `rank`, the last kept past the end), their
/// drained forms, the `extra` rows (an event's), each as
/// `ccEntryCtrl::initEntryCCS` and the race's constructor make it
/// ([`crate::foe::EnemyLook`]); and the magic portal
/// ([`crate::foe::Circle`], `xmagcir`'s `CMP_xmagcir0`).
pub fn load_looks(
    archive: &std::sync::Arc<piney_data::archive::Archive>,
    data: &super::BattleData,
    server: i32,
    ty: i32,
    rank: i32,
    extra: &[i32],
) -> Looks {
    let mut rows: Vec<i32> = Vec::new();
    if let Some(list) =
        data.st.enemy_lists.get(server.clamp(0, 4) as usize).and_then(|l| l.get(ty.clamp(0, 6) as usize))
        && !list.is_empty()
    {
        for k in 0..3 {
            let i = (rank.max(0) as usize + k).min(list.len() - 1);
            rows.push(list[i]);
        }
    }
    rows.extend_from_slice(extra);
    let drains: Vec<i32> = rows.iter().map(|&r| piney_battle::enemy_ai::drain_id(&data.t, r)).collect();
    rows.extend(drains);
    rows.retain(|&r| r >= 0);
    rows.sort_unstable();
    rows.dedup();
    let mut looks = Looks::default();
    let mut files = crate::foe::Files::new(archive.clone());
    for r in rows {
        let Ok(look) = crate::foe::EnemyLook::load(&mut files, &data.t, r) else { continue };
        let body = Rc::new(look.model.body.clone());
        looks.bodies.push((body, Look::Enemy(r), look.height, look.width));
        looks.enemies.push(Rc::new(look));
    }
    if let Ok(c) = crate::foe::Circle::load(&mut files, data.volume) {
        let body = Rc::new(c.model.body.clone());
        looks.bodies.push((body, Look::Circle, 0x4348_0000, 0x4348_0000));
        looks.circle = Some(Rc::new(c));
    }
    // The boxes and breakables (rows 0-14), the story dungeons' symbol
    // (17), the Grunty foods (22-37) and the idols (38-44): a row sharing
    // its file and clips with an earlier one draws with that body, its own
    // palette put on when it is drawn.
    for row in (0..15).chain(17..18).chain(piney_battle::gimmick::FOOD_ROWS).chain(38..45) {
        let Ok(g) = crate::foe::GimLook::load(&mut files, data.volume, row) else { continue };
        let body = Rc::new(g.model.body.clone());
        looks.bodies.push((body, Look::Gimmick(row), g.height, g.width));
        looks.gimmicks.push(Rc::new(g));
    }
    // The Spring of Myst (row 20, XGWATER0.CCS): both clumps, the clip
    // picking one ([`Looks::find`]); `SetBlendType(4)`.
    for (clump, _) in piney_battle::gimetc::SPRING_MODEL {
        let Ok(g) = crate::foe::GimLook::load_clump(&mut files, data.volume, 20, clump) else { continue };
        let mut body = g.model.body.clone();
        body.blend = Some(4);
        looks.bodies.push((Rc::new(body), Look::Gimmick(20), g.height, g.width));
        looks.gimmicks.push(Rc::new(g));
    }
    looks
}
