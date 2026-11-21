//! Replays one STEP file through `import_step` and prints one JSON line.
use std::time::Instant;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: step_replay <file.step>");
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            println!(
                "{}",
                serde_json::json!({"ok": false, "error": format!("read: {e}"), "ms": 0.0})
            );
            return;
        }
    };
    std::panic::set_hook(Box::new(|_| {}));
    let start = Instant::now();
    let result = std::panic::catch_unwind(|| boardstudio_cad::import_step(bytes));
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    let value = match result {
        Ok(Ok(m)) => {
            let acc = m.accuracy();
            let maxrep = m
                .face_occurrences()
                .iter()
                .map(|f| f.repair_displacement_mm)
                .fold(0.0_f64, f64::max);
            serde_json::json!({
                "ok": true,
                "solids": m.solid_count(),
                "occurrences": m.occurrence_count(),
                "source_faces": m.source_faces().len(),
                "face_occurrences": m.face_occurrences().len(),
                "triangles": m.mesh().positions.len() / 9,
                "bounds": m.bounds(),
                "mesh_closed_oriented": acc.mesh_closed_oriented,
                "bounds_from_source_geometry": acc.bounds_from_source_geometry,
                "max_repair_displacement_mm": maxrep,
                "relaxed_beyond_declared_precision": acc.relaxed_beyond_declared_precision,
                "sampled_support_checks": acc.sampled_support_checks,
                "max_sampled_support_residual_mm": acc.max_sampled_support_residual_mm,
                "fallback_meshed_faces": acc.fallback_meshed_faces,
                "ms": ms,
            })
        }
        Ok(Err(e)) => serde_json::json!({"ok": false, "error": e, "ms": ms}),
        Err(p) => {
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".into());
            serde_json::json!({"ok": false, "error": format!("panic: {msg}"), "ms": ms})
        }
    };
    println!("{value}");
}
