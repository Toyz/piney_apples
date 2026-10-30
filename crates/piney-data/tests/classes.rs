//! The game's classes (`classes::GameClass`, generated) against the code
//! that names them.

use std::path::{Path, PathBuf};

use piney_data::classes::{BossGameClass, CLASSES, GameClass};

/// Every `.rs` file under `dir`.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// A `#[doc(alias)]` that names a class (its name in any case) spells it
/// as `GameClass::name` does.
#[test]
fn doc_aliases_spell_their_class() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    sources(&crates, &mut files);
    let mut bad = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        for part in text.split("doc(alias = \"").skip(1) {
            let Some(alias) = part.split('"').next() else { continue };
            let class = GameClass::all().find(|c| c.name().eq_ignore_ascii_case(alias));
            if let Some(c) = class.filter(|c| c.name() != alias) {
                bad.push(format!("{}: {alias} is {}", f.display(), c.name()));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// Names, bases and addresses read as the symbols have them.
#[test]
fn bosses_read_as_the_symbols() {
    use piney_data::volume::Volume;
    assert_eq!(GameClass::CcBoss02.name(), "ccBoss02");
    assert_eq!(GameClass::CcBoss08C.name(), "ccBoss08_c");
    assert_eq!(GameClass::CcBoss02.base(), Some(GameClass::CcBoss));
    assert_eq!(GameClass::CcBoss.constructor(Volume::Inf), Some(0x0045_bcf0));
    assert_eq!(GameClass::CcBoss01.constructor(Volume::Inf), Some(0x0047_b410));
    assert_eq!(GameClass::CcBossKyvia01.vtable(Volume::Mut), Some(0x0038_a180));
}

/// Each row of the table is its class's: row `k` is the class whose
/// discriminant is `k`, so `info` reads its own row.
#[test]
fn the_table_is_in_discriminant_order() {
    for (k, row) in CLASSES.iter().enumerate() {
        assert_eq!(row.class as usize, k, "{}", row.name);
        assert_eq!(row.class.info(), row);
    }
}

/// `c`'s ancestry reaches `root`.
fn descends(c: GameClass, root: GameClass) -> bool {
    std::iter::successors(Some(c), |k| k.base()).any(|k| k == root)
}

/// The boss family is exactly the classes whose ancestry in the game's tree
/// reaches `ccBoss`, each with its class's discriminant. Its members are read
/// from the generated enum's text by their `Debug` names, the family having no
/// list of its own.
#[test]
fn boss_family_is_ccbosss_subtree() {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/classes.rs")).unwrap();
    let body = text.split("pub enum BossGameClass {").nth(1).unwrap().split('}').next().unwrap();
    let members: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.split(' ').next().unwrap())
        .collect();
    let named: Vec<GameClass> =
        members.iter().map(|m| GameClass::all().find(|c| format!("{c:?}") == *m).unwrap()).collect();
    for c in &named {
        assert!(descends(*c, GameClass::CcBoss), "{} is not a ccBoss", c.name());
    }
    let subtree: Vec<GameClass> = GameClass::all().filter(|c| descends(*c, GameClass::CcBoss)).collect();
    assert_eq!(named, subtree);
    assert_eq!(BossGameClass::CcBoss08C.class(), GameClass::CcBoss08C);
    assert_eq!(GameClass::from(BossGameClass::KyviaGomora).name(), "kyviaGomora");
}
