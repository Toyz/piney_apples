//! The readers against Infection's disc, checked against the counts the
//! Python tools measured (docs/formats/data-bin.md, ccs.md, ccs-model.md).
//! Skipped when the disc image is not extracted; set PINEY_ISO to point at
//! it elsewhere.

use std::collections::BTreeMap;
use std::path::PathBuf;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_data::model::{self, Kind};
use piney_data::texture;

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

fn data_bin() -> Option<Archive> {
    let path = iso_path()?;
    let mut iso = Iso::open(path).unwrap();
    let data = iso.read_path("DATA/DATA.BIN").unwrap();
    assert_eq!(data.len(), 136_665_088);
    Some(Archive::new(data).unwrap())
}

#[test]
fn data_bin_walks_and_decodes() {
    let Some(arc) = data_bin() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    assert_eq!(arc.members().len(), 1023);
    assert_eq!(arc.members()[0].name.to_ascii_lowercase(), arc.members()[0].name);

    let mut inflated = 0usize;
    let mut models = 0usize;
    let mut mmats: BTreeMap<&str, usize> = BTreeMap::new();
    let (mut verts, mut tris) = (0usize, 0usize);
    let mut psm: BTreeMap<u8, usize> = BTreeMap::new();
    let mut mips: BTreeMap<u8, usize> = BTreeMap::new();
    let mut no_clut = 0usize;
    for m in arc.members() {
        let data = arc.inflate(m).unwrap();
        inflated += data.len();
        let c = Ccs::parse(data).unwrap();
        assert_eq!(c.name.to_ascii_lowercase(), m.stem(), "{}", m.name);
        let walk = c.walk();
        assert!(walk.clean(c.data.len()), "{}: walk lost at {:?}", m.name, walk.lost_at);
        for md in model::models(&c).unwrap() {
            models += 1;
            for mm in &md.mmats {
                let k = match mm.kind {
                    Kind::Rigid => "rigid",
                    Kind::Bone => "bone",
                    Kind::Skin => "skin",
                    Kind::Shadow => "shadow",
                };
                *mmats.entry(k).or_default() += 1;
                verts += mm.vertex_count();
                tris += mm.triangles.len();
            }
        }
        let (textures, cluts) = texture::read(&c).unwrap();
        for t in &textures {
            *psm.entry(t.psm).or_default() += 1;
            *mips.entry(t.mipmap).or_default() += 1;
            let bits = if t.psm == texture::PSMT8 { 8 } else { 4 };
            assert_eq!(
                t.levels[0].pixels.len() * 8,
                (t.width(0) * t.height(0)) as usize * bits,
                "{} {:?}",
                m.name,
                c.object_name(t.object)
            );
            match cluts.get(&t.clut) {
                Some(cl) => {
                    let rgba = t.rgba(cl, 0).unwrap();
                    assert_eq!(rgba.len(), (t.width(0) * t.height(0) * 4) as usize);
                }
                None => no_clut += 1,
            }
        }
    }
    assert_eq!(inflated, 338_646_324);
    assert_eq!(models, 17_254);
    assert_eq!(mmats["rigid"], 16_407);
    assert_eq!(mmats["shadow"], 3_129);
    assert_eq!(mmats["bone"], 2_994);
    assert_eq!(mmats["skin"], 185);
    assert_eq!(verts, 5_417_645);
    assert_eq!(tris, 2_819_008);
    assert_eq!(psm[&texture::PSMT8], 4_046);
    assert_eq!(psm[&texture::PSMT4], 260);
    assert_eq!((mips[&0], mips[&2], mips[&3]), (2_146, 353, 1_807));
    assert_eq!(no_clut, 0, "textures whose palette is not in their own file");
}

/// Every generated placement table names objects that exist in each scene
/// file it lists, and every dummy it places at is a DummyPos chunk there.
#[test]
fn statics_tables_match_the_disc() {
    use piney_data::scene::Scene;
    use piney_data::statics;
    let Some(arc) = data_bin() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let mut checked = 0;
    for t in statics::tables() {
        assert!(!t.scenes.is_empty(), "{} places no scene", t.name);
        for scene in t.scenes {
            let c = Ccs::parse(arc.inflate_named(scene).unwrap()).unwrap();
            let sc = Scene::read(&c).unwrap();
            for r in t.rows {
                assert!(c.find_object(r.model).is_some(), "{} {scene}: {}", t.name, r.model);
                if let Some(p) = r.position.dummy() {
                    let d = c.find_object(p).unwrap_or_else(|| panic!("{} {scene}: {p}", t.name));
                    assert!(sc.dummies.contains_key(&d), "{} {scene}: {p} is not a dummy", t.name);
                }
                checked += 1;
            }
        }
    }
    for t in statics::obj_tables() {
        for scene in t.scenes {
            let c = Ccs::parse(arc.inflate_named(scene).unwrap()).unwrap();
            let sc = Scene::read(&c).unwrap();
            for r in t.rows {
                for name in [r.clump, r.anime].into_iter().flatten() {
                    assert!(c.find_object(name).is_some(), "{} {scene}: {name}", t.name);
                }
                if let Some(p) = r.position.dummy() {
                    let d = c.find_object(p).unwrap_or_else(|| panic!("{} {scene}: {p}", t.name));
                    assert!(sc.dummies.contains_key(&d), "{} {scene}: {p} is not a dummy", t.name);
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 200, "{checked} rows checked");
}

/// Mac Anu placed as `ROOTTOWN01` places it: every row of its table found,
/// the districts at their dummies, and the flags stood at theirs.
#[test]
fn mac_anu_is_placed() {
    use glam::Vec3;
    use piney_data::scene::Scene;
    use piney_data::statics;
    let Some(arc) = data_bin() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let c = Ccs::parse(arc.inflate_named("town01").unwrap()).unwrap();
    let sc = Scene::read(&c).unwrap();
    let t = statics::for_scene("town01").unwrap();
    assert_eq!(t.name, "RT_MODELTABLE00");
    let placed = t.place(&c, &sc);
    assert_eq!(placed.len(), t.rows.len());
    let at = |name: &str| placed.iter().find(|p| c.object_name(p.model) == Some(name)).unwrap().pos;
    assert_eq!(at("MDL_floor_01"), Vec3::new(0.0, 9600.0, 0.0));
    assert_eq!(at("MDL_obj_03"), Vec3::new(-6000.0, 0.0, 0.0));
    assert_eq!(at("MDL_obj_02lod"), at("MDL_obj_02"));
    let objs = statics::obj_for_scene("town01").unwrap();
    for r in objs.rows {
        let root = r.root(&c, &sc);
        match r.position.find(&c, &sc) {
            Some(d) => assert!((root.w_axis.truncate() - d.pos).length() < 1e-3, "{:?}", r.position),
            None => assert_eq!(root, glam::Mat4::IDENTITY),
        }
    }
}
