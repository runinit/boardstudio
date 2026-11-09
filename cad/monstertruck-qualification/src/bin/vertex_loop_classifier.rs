use monstertruck_io::step::load::{convert::*, *};

fn main() {
    let input = std::env::args().nth(1).expect("fixture path");
    let bytes = std::fs::read(&input).expect("read STEP fixture");
    let table = Table::from_step_bytes(&bytes).expect("parse STEP fixture");
    let id = *table.manifold_solid_brep.keys().next().expect("solid");
    let (solid, reports) = table
        .to_compressed_solid_reported(&table.manifold_solid_brep[&id])
        .expect("convert STEP solid");
    let lossless = reports.iter().all(|report| report.require_lossless().is_ok());
    let face_count: usize = solid.boundaries.iter().map(|shell| shell.faces.len()).sum();
    let empty_bounds: usize = solid
        .boundaries
        .iter()
        .flat_map(|shell| shell.faces.iter())
        .flat_map(|face| face.boundaries.iter())
        .filter(|boundary| boundary.is_empty())
        .count();
    println!(
        "conversion={}; solids=1; faces={face_count}; empty_bounds={empty_bounds}; reports={reports:?}",
        if lossless { "LOSSLESS" } else { "LOSS" }
    );
    if !lossless {
        std::process::exit(1);
    }
}
