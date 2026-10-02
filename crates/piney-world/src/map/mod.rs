//! The minimap at the top right of the field's screen, as `ccThFieldDisp`
//! (main 0x001a4430, priority 96) draws it and `ccThGameCtrl` (gcmn
//! 0x00517800) switches it: a town's `DrawMap` ([`town`]), a field's
//! `WORLD::DrawMiniMap` ([`field`]), a dungeon's `MakeMiniMap` and `DrawMap`
//! ([`dungeon`]), on `mapLayer` faded by `WORLD_MAN::SetMapAlpha`. The map
//! button ([`button`]) steps the area's map mode by `ChangeMapMode`;
//! [`show_map`] is the events' `show_map` (`WORLD_MAN::ShowMap`, main
//! 0x001a3ee0). The rules are in docs/engine/map.md.

pub mod dungeon;
pub mod field;
pub mod sprite;
pub mod town;

use piney_desktop::SaveState;
use piney_desktop::anm::Ctx;
use piney_desktop::kanji::{Fonts, Kanji, Kt, Names};
use piney_desktop::view::LayerView;

use crate::area::{Scene, kind};
use crate::ee::{self, F, V4};
use crate::field_world::Place;
use sprite::Out;

/// `mapLayer`'s priority (`ccLayer::Init(50, 0)`).
pub const MAP_LAYER: i16 = 50;
/// `fontLayer` (`fontInit`: `ccLayer::Init(240, 0)`).
pub const FONT_LAYER: i16 = 240;

/// `saveData.assignPADmap` and `saveData.mapMode[3]`.
pub const ASSIGN_PAD_MAP: usize = 0x840c;
pub const MAP_MODE: usize = 0x842d;
/// `ccSaveData::Init`'s map button: select.
pub const DEFAULT_MAP_BUTTON: u16 = 0x100;
/// The operation `ccThGameCtrl` asks `ccEvent::CheckOperate` about.
pub const OPERATION: i32 = 13;

/// `WORLD_MAN.townMapMode`, `fieldMapMode`, `dungeonMapMode` (+0x128 ..
/// +0x130).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapModes {
    pub town: i32,
    pub field: i32,
    pub dungeon: i32,
}

impl MapModes {
    /// `ccThGameCtrl`'s set-up (gcmn 0x00517854): the three from
    /// `saveData.mapMode[]`, signed bytes.
    pub fn from_save(save: &SaveState) -> MapModes {
        let b = |k: usize| i32::from(save.save.u8(MAP_MODE + k) as i8);
        MapModes { town: b(0), field: b(1), dungeon: b(2) }
    }

    /// `WORLD_MAN::ChangeMapMode` by `WORLD_MAN.flag` (0 a town: two modes;
    /// 1 a field: three; 2 a dungeon: two).
    pub fn change(&mut self, flag: i32) {
        match flag {
            kind::TOWN => self.town = if self.town + 1 == 2 { 0 } else { self.town + 1 },
            kind::FIELD => self.field = if self.field + 1 == 3 { 0 } else { self.field + 1 },
            kind::DUNGEON => self.dungeon = if self.dungeon + 1 == 2 { 0 } else { self.dungeon + 1 },
            _ => {}
        }
    }

    /// The mode of area `area`.
    pub fn of(&self, area: i32) -> i32 {
        match area {
            kind::TOWN => self.town,
            kind::FIELD => self.field,
            kind::DUNGEON => self.dungeon,
            _ => 0,
        }
    }
}

/// `ccThGameCtrl`'s map button (gcmn 0x00517d84): `push` the pad's pushed
/// buttons. With the button pushed and operation 13 allowed
/// (`SaveState::check_operate`, which also records the operation tried),
/// the mode of `WORLD_MAN.flag`'s area steps and is stored at
/// `saveData.mapMode[game.area]`. True when the mode changed.
pub fn button(save: &mut SaveState, modes: &mut MapModes, flag: i32, area: i32, push: u32) -> bool {
    let b = save.save.i16(ASSIGN_PAD_MAP) as u16;
    // A save with no assignment takes Init's (select), as the town's other
    // buttons do.
    let b = if b == 0 { DEFAULT_MAP_BUTTON } else { b };
    if push & u32::from(b) == 0 || !save.check_operate(OPERATION) {
        return false;
    }
    modes.change(flag);
    if (kind::TOWN..=kind::DUNGEON).contains(&area) {
        save.save.set_u8(MAP_MODE + area as usize, modes.of(area) as u8);
    }
    true
}

/// `ccRotate(r, 0.3134)` (main 0x00102190) and the arrow's alpha the three
/// maps pulse it with: `fptosi(64 cosf r) + 80`, at most 128.
pub fn pulse(r: &mut F) -> i32 {
    const STEP: F = 0x3ea0_d97c;
    const PI: F = 0x4049_0fdb;
    const TWO_PI: F = 0x40c9_0fdb;
    *r = ee::add(*r, STEP);
    if !ee::le(*r, PI) {
        *r = ee::sub(*r, TWO_PI);
    }
    (ee::to_int(ee::mul(ee::k(64.0), ee::cosf(*r))) + 80).min(128)
}

/// `ccSpriteColorTable[7]` (main 0x002fb468), the kanji's colour.
pub const LABEL_COLOUR: usize = 7;

/// A frame of a map drawn into the field's layers: each send's packets
/// at the front of its sprite's layer, each text through `fonts` (none:
/// not drawn).
pub fn render(
    outs: &[Out],
    place: &dyn Fn(u8) -> Option<sprite::Place>,
    text_place: &dyn Fn(u8) -> Option<(i16, LayerView)>,
    fonts: Option<&Fonts>,
    ctx: &mut Ctx,
) {
    for o in outs {
        match o {
            Out::Send(s) => {
                let Some(p) = place(s.spr) else { continue };
                ctx.layers.prepend(p.layer, sprite::prims(s, &p));
            }
            Out::Text(t) => {
                let (Some(fonts), Some((layer, view))) = (fonts, text_place(t.spr)) else { continue };
                // ccInitKanji(16, 0): Init(2, 16), kt 0; ctrl 0x10.
                let mut k = Kanji::init(2, 16);
                k.kt = Kt::SmallProportional;
                k.shadow = true;
                k.colour = t.rgba;
                k.transp = ee::f(t.transp);
                k.dx = ee::f(t.dx);
                k.dy = ee::f(t.dy);
                ctx.disp(fonts, &mut k, layer, &view, &t.text, &Names::default());
            }
        }
    }
}

/// `ccSpriteColorTable[i]`'s r, g, b with the alpha word `a`.
pub fn colour(i: usize, a: u8) -> [u8; 4] {
    let c = piney_data::tables::kanji::SPRITE_COLOR_TABLE[i];
    [c[0], c[1], c[2], a]
}

/// What a runtime keeps of the map between frames: `WORLD_MAN`'s modes and
/// `mapAlpha`, and the frame last drawn (while every task sleeps the layers
/// keep it).
#[derive(Clone, Debug, Default)]
pub struct MapState {
    pub modes: MapModes,
    /// `WORLD_MAN.mapAlpha` (+0x148), as the field UI last set it.
    pub alpha: F,
    pub last: Vec<Out>,
}

impl MapState {
    /// `ccThGameCtrl`'s set-up: the modes from the save; the alpha 0 until
    /// the field UI sets it.
    pub fn new(save: &SaveState) -> MapState {
        MapState { modes: MapModes::from_save(save), alpha: 0, last: Vec::new() }
    }

    /// `WORLD_MAN::SetMapAlpha(a)` (main 0x001a3b70), the field UI's
    /// `Request::MapAlpha`.
    pub fn set_alpha(&mut self, a: f32) {
        self.alpha = a.to_bits();
    }

    /// The map button ([`button`]) with `WORLD_MAN.flag` and `game.area`
    /// both `area`.
    pub fn button(&mut self, save: &mut SaveState, area: i32, push: u32) -> bool {
        button(save, &mut self.modes, area, area, push)
    }
}

/// `ROOTTOWN01::DrawMap` for a frame of the town's `ccThFieldDisp`: drawn
/// with the player at `pos` heading `dirc` while `awake`, else the last
/// frame again.
pub fn town_frame(map: &mut town::TownMap, st: &mut MapState, pos: V4, dirc: V4, awake: bool, ctx: &mut Ctx) {
    if awake {
        // The flag race is not played, so no racer shows.
        st.last = map.draw(&town::Input { mode: st.modes.town, alpha: st.alpha, pos, dirc, race: None });
    }
    let place = |spr: u8| map.place(spr).cloned();
    render(&st.last, &place, &|_| None, None, ctx);
}

/// The map of the field or dungeon `place` as its set-up makes it (once:
/// a dungeon keeps its map from room to room): `WORLD::Init`'s sprites and
/// `Generate`'s texture, or the `DUNGEON` constructor's sprites over the
/// floors `Generate` laid out.
pub fn setup_area(
    place: &mut Place,
    archive: &std::sync::Arc<piney_data::archive::Archive>,
    volume: piney_data::volume::Volume,
    scene: &Scene,
) -> piney_data::Result<()> {
    match place {
        Place::Field(f) => {
            if f.map.is_none() {
                f.map = Some(Box::new(field::FieldMap::new(archive, volume, &f.field, &f.file)?));
            }
        }
        Place::Dungeon(d) => {
            if d.map.is_none() {
                let edit = if d.event != 0 { d.tables.edit(d.event, scene.dungeon.clamp(0, 2)) } else { None };
                let floors = dungeon::floors_of(d, edit);
                let file = d.file.clone();
                d.map = Some(Box::new(dungeon::DungeonMap::new(archive, volume, &file, i32::from(d.dtype), floors)?));
            }
        }
        // A story map of its own draws no map (EVENTAREA02::Draw).
        Place::Story(_) => {}
    }
    Ok(())
}

/// What the map reads of the entry control's lists (the battle's): a
/// field's magic portals (their `alpha` is the map's to fade, and goes
/// back into the portals) and `ccCheckFountain()`; a dungeon's portals and
/// gimmicks, and whether an event holds the map (`eventMng` +0x78c).
#[derive(Clone, Debug, Default)]
pub struct Entries {
    pub circles: Vec<field::Circle>,
    pub fountain: bool,
    pub dungeon_circles: Vec<dungeon::Ent>,
    pub gims: Vec<dungeon::Ent>,
    pub event_hold: bool,
}

/// A frame of `ccThFieldDisp`'s map outside the towns, the player at `pos`
/// heading `dirc`: in a dungeon first `DUNGEON::Draw`'s `MakeMiniMap` for
/// the room under him. While `ccGame.inBattle` neither `DrawMiniMap`
/// (gcmn 0x005a98a4) nor `DUNGEON::DrawMap` (0x005cef78) runs: no map.
/// While not `awake` the last frame is drawn again. True when
/// `DUNGEON::DrawMap` set `ccMenu.mapStatus` back to 1.
#[allow(clippy::too_many_arguments)]
pub fn area_frame(
    place: &mut Place,
    st: &mut MapState,
    scene: &Scene,
    pos: V4,
    dirc: V4,
    awake: bool,
    in_battle: bool,
    fonts: Option<&Fonts>,
    ents: &mut Entries,
    ctx: &mut Ctx,
) -> bool {
    let mut status = false;
    match place {
        Place::Field(f) => {
            let Some(map) = f.map.as_mut() else { return false };
            if awake && in_battle {
                st.last.clear();
            } else if awake {
                // The entry control's portals and fountain.
                let mut inp = field::Input {
                    mode: st.modes.field,
                    alpha: st.alpha,
                    pos,
                    dirc,
                    event_area: scene.field.max(0),
                    fountain: ents.fountain,
                    circles: &mut ents.circles,
                };
                st.last = map.draw(&mut inp);
            }
            map.render(&st.last, fonts, ctx);
        }
        Place::Dungeon(d) => {
            let Some(mut map) = d.map.take() else { return false };
            if awake {
                let here = d.here(pos);
                map.enter_room(&mut d.hits, d.level, here);
                if in_battle {
                    st.last.clear();
                    d.map = Some(map);
                    return false;
                }
                let inp = dungeon::Input {
                    mode: st.modes.dungeon,
                    special_room: d.special_room,
                    alpha: st.alpha,
                    pos,
                    dirc,
                    event_hold: ents.event_hold,
                    circles: &ents.dungeon_circles,
                    gims: &ents.gims,
                };
                let drawn = map.draw(d.level, &inp);
                status = drawn.map_status;
                st.last = drawn.outs;
            }
            map.render(&st.last, fonts, ctx);
            d.map = Some(map);
        }
        Place::Story(_) => {}
    }
    status
}

/// The events' `show_map`, one frame of `WORLD_MAN::ShowMap` outside the
/// towns (in a town it does nothing and is done): in a field
/// `WORLD::ShowMap` unless the story area's map is its own
/// (`EVENTAREA_INFO.model` 1, `field_model`); in a dungeon one room of
/// `DUNGEON::ShowMap`, `clear` its rooms' `ccCheckActiveObject(f, i)`.
/// True when done.
pub fn show_map(place: &mut Place, field_model: i32, pos: V4, clear: &dyn Fn(i32, i32) -> bool) -> bool {
    match place {
        Place::Field(f) => match f.map.as_mut() {
            Some(map) => show_field_map(map, field_model),
            None => true,
        },
        Place::Dungeon(d) => dungeon::show_map_step(d, pos, clear),
        // The story area's map is its own: nothing to show.
        Place::Story(_) => true,
    }
}

/// `WORLD_MAN::ShowMap`'s field case: `WORLD::ShowMap` (gcmn 0x005ad990,
/// `mapFlag = 1`) unless the story area's map is its own.
pub fn show_field_map(map: &mut field::FieldMap, field_model: i32) -> bool {
    if field_model != 1 {
        map.map_flag = true;
    }
    true
}
