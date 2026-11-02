//! The `ccEff` sprites the world asks for ([`EffSprite`]: the towns' and
//! story maps' flares, clouds and glows, the fields' weather, the dungeons'
//! sparks, the enemies' breaths), each `ccEff` made once per file, name and
//! state as its owner's constructor left it, then drawn with
//! `ccEff::Draw(pos, pattern)`. The town's and the area's effect hosts each
//! keep one.

use std::collections::HashMap;

use piney_data::archive::Archive;
use piney_desktop::layers::Layers;
use piney_effect::draw::Camera as FxCamera;
use piney_effect::eff::Eff;
use piney_effect::files::{Assets, ObjRef};
use piney_world::sprite::{Clut, EffSprite, EffState};

/// The sprites' `ccEff`s by file, name and state.
#[derive(Default)]
pub struct EffSprites {
    effs: HashMap<(&'static str, &'static str, EffState), Option<Eff>>,
}

impl EffSprites {
    /// The sprite's `ccEff`: `Init(chunk, 1)`, then what its owner did to
    /// it: PRIM's fog bit cleared (`+0x62 &= ~0x20`), the depth test off
    /// (`SetRenderState(CCRS_ZENABLE, 0)`), the palette set or changed.
    fn eff(&mut self, assets: &mut Assets, archive: &Archive, s: &EffSprite) -> Option<Eff> {
        self.effs
            .entry((s.file, s.eff, s.state))
            .or_insert_with(|| {
                assets.add_file(archive, s.file).ok()?;
                let o = assets.find(s.file, s.eff)?;
                let (j, chunk) = assets.eff_chunk(o)?;
                let mut e = Eff::init(o.file, j, chunk, true, &assets.alpha_blend);
                if !s.state.fog {
                    e.prim &= !0x20;
                }
                if !s.state.ztest {
                    e.test &= !0x1_0000;
                }
                match s.state.clut {
                    // `eff->tex.clutChunk = chunk`: an enemy's breath.
                    Some(Clut::Set(new)) => e.clut = assets.find(s.file, new).or(e.clut),
                    // `ccEff::ChangeClut(new, old)`: only while it is `old`.
                    Some(Clut::Change { new, old }) => {
                        let own = assets.eff_tex.get(o.file).and_then(|v| v.get(j)).copied().flatten();
                        let current = e.clut.or(own.map(|t| ObjRef { file: o.file, object: t.clut }));
                        if let (Some(n), Some(o)) = (assets.find(s.file, new), assets.find(s.file, old))
                            && current == Some(o)
                        {
                            e.clut = Some(n);
                        }
                    }
                    None => {}
                }
                Some(e)
            })
            .clone()
    }

    /// Each sprite's `ccEff::Draw(pos, pattern)` on its layer.
    pub fn draw(
        &mut self,
        assets: &mut Assets,
        archive: &Archive,
        sprites: &[EffSprite],
        cam: &FxCamera,
        layers: &mut Layers,
    ) {
        for s in sprites {
            self.draw_one(assets, archive, s, cam, layers);
        }
    }

    /// One sprite's `ccEff::Draw(pos, pattern)`.
    pub fn draw_one(
        &mut self,
        assets: &mut Assets,
        archive: &Archive,
        s: &EffSprite,
        cam: &FxCamera,
        layers: &mut Layers,
    ) {
        let Some(mut e) = self.eff(assets, archive, s) else { return };
        e.pos = s.pos;
        e.scale_x = s.scale[0];
        e.scale_y = s.scale[1];
        e.rotate = s.rotate;
        e.transparency = s.transparency;
        if let Some(c) = s.colour {
            e.color = c;
        }
        e.render(assets, layers, s.layer, s.pattern, cam);
    }
}
