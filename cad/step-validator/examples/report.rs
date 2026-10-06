//! `cargo run --release --example report -- <file.step>...` prints one JSON object per file.
use std::time::Instant;

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("readable file");
        let started = Instant::now();
        let outcome = std::panic::catch_unwind(|| {
            boardstudio_step_validator::inspect_with(
                &bytes,
                std::env::var("STEP_CHORD")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(boardstudio_step_validator::DEFAULT_CHORD_TOLERANCE),
            )
        });
        let ms = started.elapsed().as_millis();
        let json_string = |s: &str| format!("{:?}", s);
        let line = match outcome {
            Ok(Ok(r)) => format!(
                "{{\"file\":{},\"load\":true,\"valid\":{},\"solids\":{},\"shells\":{},\"volume\":{:.4},\"min\":[{:.5},{:.5},{:.5}],\"max\":[{:.5},{:.5},{:.5}],\"unitMm\":{},\"chordMm\":{},\"nonManifoldEdges\":{},\"openMeshEdges\":{},\"ms\":{},\"lost\":[{}],\"problems\":[{}]}}",
                json_string(&path), r.valid, r.solid_count, r.shell_count, r.volume,
                r.min[0], r.min[1], r.min[2], r.max[0], r.max[1], r.max[2], r.length_unit_mm, r.chord_tolerance_mm, r.non_manifold_edges, r.open_mesh_edges, ms,
                r.lost.iter().map(|s| json_string(s)).collect::<Vec<_>>().join(","),
                r.problems.iter().map(|s| json_string(s)).collect::<Vec<_>>().join(",")),
            Ok(Err(e)) => format!("{{\"file\":{},\"load\":false,\"error\":{},\"ms\":{}}}", json_string(&path), json_string(&e.to_string()), ms),
            Err(_) => format!("{{\"file\":{},\"load\":false,\"error\":\"panic\",\"ms\":{}}}", json_string(&path), ms),
        };
        println!("{line}");
    }
}
