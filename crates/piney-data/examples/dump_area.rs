//! Every volume's area tables, printed (a check of their source).
use piney_data::store::Table;

fn main() {
    for v in piney_data::volume::Volume::ALL {
        println!("{v}: {:#?}", piney_data::area::AreaTables::of(v));
    }
}
