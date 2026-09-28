//! setbl.cpp's tables, printed (a check of their source).
fn main() {
    use piney_audio::setbl;
    println!("base 0x{:08x} inu {}", *setbl::BASE, *setbl::INU);
    println!("spc {:?}", *setbl::SPC);
    println!("enemy {:?}", *setbl::ENEMY);
    for r in setbl::ROWS.iter() {
        println!("{} {} {}", r.code, r.note, r.velocity);
    }
}
