//! Every volume's area tables, printed (a check of their source).
fn main() {
    for v in piney_data::volume::Volume::ALL {
        println!("{v}: {:#?}", piney_data::area::AreaTables::of(v));
    }
}
