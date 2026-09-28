//! setbl.cpp's tables, printed (a check of their source): `dump_setbl
//! [inf|mut|out|qua]`, Infection's by default.
fn main() {
    use piney_audio::setbl;
    use piney_data::volume::Volume;
    let v = match std::env::args().nth(1).as_deref() {
        Some("mut") => Volume::Mut,
        Some("out") => Volume::Out,
        Some("qua") => Volume::Qua,
        _ => Volume::Inf,
    };
    println!("base 0x{:08x} inu {}", setbl::base(v), setbl::inu(v));
    println!("spc {:?}", setbl::spc(v));
    println!("enemy {:?}", setbl::enemy(v));
    for r in setbl::rows(v) {
        println!("{} {} {}", r.code, r.note, r.velocity);
    }
}
