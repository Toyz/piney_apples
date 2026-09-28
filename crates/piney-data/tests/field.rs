//! The field generator against numbers `tools/field.py` gives (which
//! `tools/test_field.py` checks against the game): fixed fields, the height
//! map by a hash of its bits. The tables are statics, so these run without
//! the disc; the check that every model name is in its CCS file is skipped
//! when the disc image is not extracted (set PINEY_ISO to point at it
//! elsewhere). `tools/test_field_rs.py` compares many more cases field by
//! field.

use std::path::PathBuf;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::field::{self, FIELD_TYPES, INF, Kind, Light, Params, ee};
use piney_data::iso::Iso;
use piney_data::model;

/// FNV-1a over 32-bit words' little-endian bytes.
fn fnv(map: &[u32]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for v in map {
        for b in v.to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

fn kinds(f: &field::Field) -> [usize; 6] {
    let mut n = [0; 6];
    for o in &f.objects {
        let i = [Kind::Entrance, Kind::Key, Kind::Sub, Kind::Base, Kind::Tree, Kind::Lake]
            .iter()
            .position(|&k| k == o.kind);
        n[i.unwrap()] += 1;
    }
    n
}

const RANDOM: Params =
    Params { seed: 12345, field_type: 7, weather: 0, ground: 2, object: 2, event: 0, protect: false, skip_init: false };

#[test]
fn the_first_story_area() {
    // Bursting Passed Over Aqua Field: event 14, field type 10.
    let p = Params { seed: 1_420_855, field_type: 10, weather: 0, ground: 1, object: 1, event: 14, ..RANDOM };
    let f = field::generate(&INF, &p).unwrap();
    assert_eq!(f.init_draws, 0);
    let hills: Vec<_> = f.hills.iter().map(|h| (h.x, h.y, h.size)).collect();
    assert_eq!(hills, [(22, 11, 24), (48, 45, 16), (48, 1, 12), (40, 65, 12), (60, 9, 16), (26, 59, 14)]);
    assert_eq!(f.map[..4], [0x4220_0000, 0x40a0_0000, 0x41f0_0000, 0x4040_0000]);
    assert_eq!(f.map.iter().max(), Some(&0x443f_8000)); // 766.0
    assert_eq!(fnv(&f.map), 0xb984_752f_20a9_6f55);
    assert_eq!(f.objects.len(), 107);
    assert_eq!(kinds(&f), [1, 17, 26, 31, 31, 1]);
    assert_eq!(f.covers.len(), 1550);
    // The entrance within 8,000 units of the start, as this area wants.
    assert_eq!(f.entrance, Some([13, 19]));
    assert_eq!(f.dungeon_pos, Some([0x4687_f000, 0x46bb_8000, 0]));
    assert_eq!(f.start, [20, 20]);
    assert_eq!(f.start_pos, [0x46c0_3000, 0x46c0_3000, 0]); // 24,600
    assert_eq!((f.rng.seed, f.rng.count), (2_613_795_997, 10186));
    let lake = f.objects.iter().find(|o| o.kind == Kind::Lake).unwrap();
    assert_eq!((lake.index, lake.x, lake.y, lake.info.clump), (0, 25, 19, "CMP_sfp4lak1"));

    // What a viewer draws.
    let placed = f.placements(&INF);
    assert_eq!(placed.len(), 107);
    assert!(placed.iter().all(|p| p.ccs == "field_p"));
    assert_eq!(placed[0].kind, Kind::Entrance);
    assert_eq!(placed[0].pos, [17_400.0, 24_000.0, 0.0]);
    assert_eq!(f.vertex(79, 1), [47_400.0, 600.0, f32::from_bits(f.map[79 * 80 + 1])]);

    // Object heights: WORLD::GetHeight as each was placed; 0 where levelled.
    let z: Vec<u32> = f.objects[..8].iter().map(|o| o.z).collect();
    assert_eq!(z, [0, 0, 0x4140_0300, 0x4160_0300, 0x41a0_0180, 0x41f0_0100, 0x4180_0180, 0x4120_0300]);
    assert_eq!(placed[2].pos[2], f32::from_bits(0x4140_0300));
    let h = |x: f32, y: f32| f.get_height(x.to_bits(), y.to_bits());
    assert_eq!(h(1000.0, 2000.0), 0x4222_ab40);
    assert_eq!(h(30000.5, 12345.25), 0x3d2b_0000);
    assert_eq!(h(24600.0, 24600.0), 0x3980_0000);
    assert_eq!(h(16200.0, 23100.0), (-500f32).to_bits()); // under the entrance, a hidden chip
}

/// bg_p1's light (field type 10, weather 0), as its animation's frame 0 sets it.
const BG_P1: Light =
    Light { ambient: 0x8094_786e, rotation: [0x420d_0ebc, 0x41f0_0000, 0x420d_0ebc], colour: 0x00df_f0f7 };

#[test]
fn the_first_story_area_drawn() {
    let p = Params { seed: 1_420_855, field_type: 10, weather: 0, ground: 1, object: 1, event: 14, ..RANDOM };
    let f = field::generate(&INF, &p).unwrap();
    assert_eq!(fnv(&f.vertex_normals().concat()), 0x9cf0_6dd8_2905_da3d);
    let colours = f.vertex_colours(&BG_P1);
    assert_eq!(fnv(&colours.concat()), 0xc522_cee8_59bf_e024);

    let t = f.tile(&INF, 5, 7, 600f32.to_bits(), Some(&colours));
    assert!(t.visible);
    assert_eq!(t.pos, [0x45ce_4000, 0x460c_a000, 0]); // 6,600 and 9,000
    let z: Vec<i16> = t.z.iter().map(|&(_, z)| z).collect();
    assert_eq!(
        z,
        [
            47, 341, 375, 232, 375, 341, 375, 232, 266, 122, 266, 232, 382, 47, 273, 375, 273, 47, 273, 375, 163, 266,
            163, 375
        ]
    );
    assert_eq!(t.colours[..3], [(0, [151, 153, 160]), (1, [147, 149, 157]), (2, [150, 152, 159])]);
    let visible =
        (0..40).flat_map(|x| (0..40).map(move |y| (x, y))).filter(|&(x, y)| f.tile(&INF, x, y, 0, None).visible);
    assert_eq!(visible.count(), 1550);

    let c = f.cover_mesh(&INF, &f.covers[0], 300f32.to_bits(), Some(&colours));
    assert_eq!((f.covers[0].x, f.covers[0].y, f.covers[0].quadrant, c.mesh), (0, 0, 0, "MDL_sfp5tur3"));
    assert_eq!(c.pos, [300f32.to_bits(), 300f32.to_bits(), 2.5f32.to_bits()]);
    let z: Vec<i16> = c.z.iter().map(|&(_, z)| z).collect();
    assert_eq!(z, [68, 546, 723, 327, 723, 546]);
    assert_eq!(c.colours[0], (0, [153, 155, 163]));

    // CalcObjectVertexColor: a normal facing the light brightens, one away
    // from it keeps only the ambient share.
    let lit = field::object_colours(&[[0, 0, 64, 0], [0, 0, 192, 0]], &[[128, 128, 128, 7]; 2], &BG_P1);
    assert_eq!(lit[1][3], 7);
    assert!(lit[0][0] > lit[1][0]);
}

#[test]
fn random_fields() {
    let f = field::generate(&INF, &RANDOM).unwrap();
    assert_eq!((f.hills.len(), f.objects.len(), f.covers.len()), (9, 69, 1562));
    assert_eq!(kinds(&f), [1, 10, 30, 11, 17, 0]);
    assert_eq!((f.entrance, f.start), (Some([0, 21]), [6, 1]));
    assert_eq!((f.rng.seed, f.rng.count), (1_518_714_998, 10125));
    assert_eq!(fnv(&f.map), 0xc15f_a9ca_25b6_7914);
    let key = f.objects[1];
    assert_eq!((key.kind, key.index, key.x, key.y, key.w, key.h), (Kind::Key, 0, 12, 27, 3, 1));
    assert_eq!(key.info.anm, Some("ANM_sfi2tow1a"));

    // Field type 1: trees in rows.
    let p = Params { seed: 0x00c0_ffee, field_type: 1, weather: 3, ground: 1, object: 0, ..RANDOM };
    let f = field::generate(&INF, &p).unwrap();
    assert_eq!((f.hills.len(), f.objects.len(), f.covers.len()), (6, 88, 1540));
    assert_eq!(kinds(&f), [1, 16, 24, 28, 19, 0]);
    assert_eq!((f.entrance, f.start), (Some([21, 2]), [3, 1]));
    assert_eq!((f.rng.seed, f.rng.count), (4_016_937_597, 10103));
    assert_eq!(fnv(&f.map), 0x76eb_e64c_2d3c_8681);
    assert!(f.objects.iter().filter(|o| o.kind == Kind::Tree).all(|o| o.level));

    // Field type 5 in heavy weather: 1,621 draws before Generate.
    let p = Params { seed: 987_654_321, field_type: 5, weather: 6, ground: 0, object: 1, ..RANDOM };
    let f = field::generate(&INF, &p).unwrap();
    assert_eq!(f.init_draws, 1621);
    assert_eq!((f.hills.len(), f.objects.len(), f.covers.len()), (4, 106, 1462));
    assert_eq!((f.entrance, f.start), (Some([24, 22]), [10, 0]));
    assert_eq!((f.rng.seed, f.rng.count), (486_618_396, 11495));
    assert_eq!(fnv(&f.map), 0x216a_7798_61ec_2dbe);
}

#[test]
fn inputs_out_of_range() {
    let p = Params { field_type: 11, ..RANDOM };
    assert_eq!(field::generate(&INF, &p).unwrap_err(), field::GenerateError::FieldType(11));
    let p = Params { ground: 3, ..RANDOM };
    assert_eq!(field::generate(&INF, &p).unwrap_err(), field::GenerateError::Ground(3));
}

#[test]
fn counts_and_draws() {
    // Type 7 halves key and tree objects and thirds base ones; type 1 has a
    // sixth of the trees; `object` takes 80% or 90%, truncated.
    let c = |ft, kind, obj| INF.count(ft, kind, obj);
    assert_eq!([c(0, Kind::Key, 2), c(0, Kind::Sub, 2), c(0, Kind::Base, 2), c(0, Kind::Tree, 2)], [20, 30, 35, 35]);
    assert_eq!([c(7, Kind::Key, 2), c(7, Kind::Sub, 2), c(7, Kind::Base, 2), c(7, Kind::Tree, 2)], [10, 30, 11, 17]);
    assert_eq!([c(1, Kind::Tree, 0), c(1, Kind::Tree, 1), c(1, Kind::Tree, 2)], [4, 4, 5]);
    assert_eq!([c(0, Kind::Base, 0), c(0, Kind::Base, 1)], [28, 31]);
    assert_eq!(
        [INF.init_draws(0, 0), INF.init_draws(2, 9), INF.init_draws(5, 3), INF.init_draws(6, 4)],
        [112, 21, 721, 1621]
    );
    assert_eq!(INF.init_draws(4, 0) + INF.init_draws(7, 0) + INF.init_draws(10, 9), 0);
    assert_eq!(INF.percent[0], ee::bits(0.8));
    assert_eq!(INF.ccs[4], None);
}

fn data_bin() -> Option<Archive> {
    let path = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    if !path.exists() {
        return None;
    }
    let data = Iso::open(path).unwrap().read_path("DATA/DATA.BIN").unwrap();
    Some(Archive::new(data).unwrap())
}

#[test]
fn every_model_is_in_its_field_file() {
    let Some(arc) = data_bin() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let mut checked = 0;
    for ft in 0..FIELD_TYPES {
        let Some(name) = INF.ccs[ft] else { continue };
        let c = Ccs::parse(arc.inflate_named(name).unwrap()).unwrap();
        let tables = [&INF.enter, &INF.lake, &INF.key, &INF.sub, &INF.base, &INF.tree];
        let rows = tables.iter().flat_map(|t| t.by_type[ft].iter());
        let names = rows.flat_map(|r| [Some(r.clump), r.anm]).flatten();
        for n in names.chain(INF.base_mesh[ft].iter().copied()).chain(INF.small_mesh[ft].iter().copied()) {
            assert!(c.find_object(n).is_some(), "{n} not in {name}");
            checked += 1;
        }
    }
    let eff = Ccs::parse(arc.inflate_named(INF.effect_ccs).unwrap()).unwrap();
    assert!(eff.find_object(INF.water).is_some());
    assert_eq!(checked, 359);
}

#[test]
fn the_templates_match_the_mesh_tables() {
    // SetMESH2 and SetSmallMESH write each vertex from a fixed cell; in every
    // field type's ground and cover models that cell is the one under the
    // vertex (x, y in units of a cell from the centre of the tile).
    let Some(arc) = data_bin() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    // A ground vertex at x (-600..600 about the chip's centre, the corner of
    // cell 2mx + 1) is on cell offset 1 + x / 600; a cover vertex at x
    // (-300..300 about its cell's centre) on (x - 300) / 600 from the cell
    // SetSmallMESH is given.
    let ground: fn(f32) -> i8 = |v| (1.0 + v / 600.0).round() as i8;
    let cover: fn(f32) -> i8 = |v| ((v - 300.0) / 600.0).round() as i8;
    let mut models = 0;
    for ft in 0..FIELD_TYPES {
        let Some(name) = INF.ccs[ft] else { continue };
        let c = Ccs::parse(arc.inflate_named(name).unwrap()).unwrap();
        let all = model::models(&c).unwrap();
        let checks =
            [(INF.base_mesh[ft], INF.mesh.tile, 600.0, ground), (INF.small_mesh[ft], INF.mesh.cover, 300.0, cover)];
        for (names, cells, scale, offset) in checks {
            for mdl in names {
                let obj = c.find_object(mdl).unwrap();
                let m = all.iter().find(|m| m.object == obj).unwrap();
                assert_eq!((m.scale, m.mmats.len()), (scale, 1), "{mdl}");
                let mm = &m.mmats[0];
                assert_eq!(mm.positions.len(), cells.len(), "{mdl}");
                for &(k, dx, dy) in cells {
                    let p = m.position(mm.positions[k as usize]);
                    assert_eq!((offset(p.x), offset(p.y)), (dx, dy), "{mdl} vertex {k}");
                }
                models += 1;
            }
        }
    }
    assert_eq!(models, 80);
}

#[test]
fn a_field_scene_from_the_disc() {
    let Some(arc) = data_bin() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    // Every background an area can reach: its file, clumps and light.
    let mut rows = 0;
    for ft in 0..FIELD_TYPES as u32 {
        let reach: &[u32] = match ft {
            0..=4 => &[0, 1, 2, 3],
            5 | 6 => &[0, 1, 2, 3, 4, 5],
            _ => &[0, 1, 2, 3, 4, 5, 6, 7],
        };
        for &w in reach {
            let row = INF.backgrounds.row(ft, w).unwrap();
            let c = Ccs::parse(arc.inflate_named(&INF.backgrounds.ccs_name(ft, w)).unwrap()).unwrap();
            for clump in row.clumps {
                assert!(c.find_object(clump).is_some(), "{clump}");
            }
            Light::from_anime(&c, row.anime, row.light).unwrap();
            rows += 1;
        }
    }
    assert_eq!(rows, 64);

    let p = Params { seed: 1_420_855, field_type: 10, weather: 0, ground: 1, object: 1, event: 14, ..RANDOM };
    let f = field::generate(&INF, &p).unwrap();
    let field_ccs = Ccs::parse(arc.inflate_named(INF.ccs[10].unwrap()).unwrap()).unwrap();
    let bg = Ccs::parse(arc.inflate_named(&INF.backgrounds.ccs_name(10, 0)).unwrap()).unwrap();
    let s = f.scene(&INF, &field_ccs, &bg).unwrap();
    assert_eq!((s.ccs, s.ground_model, s.ground_scale, s.cover_scale), ("field_p", "MDL_sfp5tur1", 600.0, 300.0));
    assert_eq!(s.light, BG_P1);
    assert_eq!((s.tiles.len(), s.tiles.iter().filter(|t| t.visible).count(), s.covers.len()), (1600, 1550, 1550));
    let water = s.water.unwrap();
    assert_eq!((water.ccs, water.anime, water.pos), ("field_eff", "ANM_sfwat1_1a", [30_600.0, 23_400.0, 0.0]));
    assert_eq!(s.background.ccs, "bg_p1");
    assert_eq!(s.background.clumps, ["CMP_sfp7bac1", "CMP_sfp7clo1_1", "CMP_sfp7clo1_2", "CMP_sfp7mou1"]);
    assert_eq!(s.background.sky_material, Some("MAT_sfp7bac1"));
    assert_eq!((s.background.fog.near, s.background.fog.far, s.background.fog.percent), (1000.0, 8800.0, 90.0));
    assert_eq!(s.objects.len(), 107);
}

#[test]
fn lit_clumps() {
    // Field type 8 names two rock clumps twice in its sub table; lake fields
    // light their lake pieces too.
    let lit = INF.lit_clumps(8);
    assert_eq!(lit.iter().find(|(c, _)| *c == "CMP_sfk3roc1"), Some(&("CMP_sfk3roc1", 2)));
    assert!(lit.iter().any(|(c, _)| *c == "CMP_sfk4lak1"));
    assert!(!INF.lit_clumps(0).iter().any(|&(_, n)| n > 1));
}
