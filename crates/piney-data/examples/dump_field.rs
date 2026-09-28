//! Each volume's field tables, as Debug: compared across a move of where
//! they are read from.
fn main() {
    for v in piney_data::volume::Volume::ALL {
        println!("{v}: {:#?}", piney_data::field::tables_of(v));
    }
}
