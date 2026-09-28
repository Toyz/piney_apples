//! A field's weather and ambient pictures as the effects draw them
//! (`piney_world::field_ambient`): each `ccEff` of `WORLD::Draw` and
//! `DrawEffect` (the rain and its splashes, the thunder, the snow and the
//! embers, the fireflies, the lens flare) and of a dungeon's `DrawEff` (its
//! rooms' sparks and glows) made once per name as `ccEff::Init` left it and
//! drawn where the frame put it, and the steam's and the fires' `effSmoke`
//! puffs.
//!
//! The game calls `effSmoke` from `WORLD::Draw` (ccThFieldDisp, 96), before
//! `ccThParticle` (98) steps the frame's particles. Here the drawing runs
//! after the tasks, so a puff starts at the next frame's first effect task
//! and steps a frame later than the game's; its `rand()` draws come after
//! that task's shows' own.

use piney_effect::Effects;
use piney_effect::draw::Camera as FxCamera;
use piney_effect::eff::Eff;
use piney_effect::files::ObjRef;
use piney_world::field_ambient::{Op, Sprite};

use super::{AreaFx, BattleHost};

impl AreaFx {
    /// The sprite's `ccEff`: `Init(chunk, 1)`, the fog bit cleared when
    /// the drawing cleared it (`+0x62 &= ~0x20`), the depth test off for
    /// the lens flare's (`SetRenderState(CCRS_ZENABLE, 0)`), the palette
    /// changed (`ccEff::ChangeClut(new, old)`: only while it is `old`).
    fn ambient_eff(&mut self, s: &Sprite) -> Option<Eff> {
        let assets = &mut self.fx.assets;
        let archive = &self.archive;
        self.ambient
            .entry((s.file, s.name, s.fog, s.ztest, s.clut))
            .or_insert_with(|| {
                assets.add_file(archive, s.file).ok()?;
                let o = assets.find(s.file, s.name)?;
                let (j, chunk) = assets.eff_chunk(o)?;
                let mut e = Eff::init(o.file, j, chunk, true, &assets.alpha_blend);
                if !s.fog {
                    e.prim &= !0x20;
                }
                if !s.ztest {
                    e.test &= !0x1_0000;
                }
                if let Some((new, old)) = s.clut {
                    let own = assets.eff_tex.get(o.file).and_then(|v| v.get(j)).copied().flatten();
                    let current = e.clut.or(own.map(|t| ObjRef { file: o.file, object: t.clut }));
                    if old.is_empty() {
                        // set outright (`eff->tex.clutChunk = chunk`: an
                        // enemy's breath)
                        e.clut = assets.find(s.file, new).or(e.clut);
                    } else if let (Some(n), Some(o)) = (assets.find(s.file, new), assets.find(s.file, old))
                        && current == Some(o)
                    {
                        e.clut = Some(n);
                    }
                }
                Some(e)
            })
            .clone()
    }
}

/// The frame's sprites on their layers, and with `fresh` its smoke puffs
/// queued for the next effect task.
pub(super) fn draw(
    fx: &mut AreaFx,
    ops: &[Op],
    fresh: bool,
    camera: &piney_world::camera::Camera,
    ctx: &mut piney_desktop::anm::Ctx,
) {
    let cam = FxCamera::from_field(camera);
    for op in ops {
        match op {
            Op::Sprite(s) => {
                let Some(mut e) = fx.ambient_eff(s) else { continue };
                e.pos = s.pos;
                e.scale_x = s.scale[0];
                e.scale_y = s.scale[1];
                e.rotate = s.rotate;
                e.transparency = s.transparency;
                if let Some(c) = s.colour {
                    e.color = c;
                }
                let layer = piney_world::field_area::ambient_layer(s.layer);
                e.render(&fx.fx.assets, &mut ctx.layers, layer, s.pattern, &cam);
            }
            Op::Smoke { .. } if fresh => fx.ambient_smoke.push(op.clone()),
            _ => {}
        }
    }
}

/// `effSmoke(pos, v, s, life, t, in, out)` for each queued puff, in order.
pub(super) fn start_smoke(fx: &mut Effects, host: &mut BattleHost, queued: &mut Vec<Op>) {
    for op in std::mem::take(queued) {
        if let Op::Smoke { pos, v, s, life, t, fade_in, fade_out } = op {
            fx.smoke(host, pos, v, s, life, t, fade_in as i16, fade_out as i16);
        }
    }
}
