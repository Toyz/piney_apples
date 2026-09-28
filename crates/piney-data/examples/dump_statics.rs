//! Every volume's statics tables, printed (a check of their source).
fn main() {
    for v in piney_data::volume::Volume::ALL {
        println!("{v}: {:#?}", piney_data::statics::of(v));
    }
}
