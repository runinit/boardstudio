//! `step-oracle <file.step>...` prints one JSON report per file on stdout (last line of output).
//!
//! Test-only and native-only. Reads STEP with OCCT 8.0.1's own `STEPControl_Reader` in this
//! separate process, so it shares no state with the production Cadrum build. Adapter-independent,
//! not kernel-independent: it uses the same OCCT release as production.

#[cxx::bridge(namespace = "bs_oracle")]
mod ffi {
    struct Facts {
        /// `IFSelect_ReturnStatus` as an integer; 1 is `IFSelect_RetDone`, 3 is `IFSelect_RetFail`, -1 means never read.
        read_status: i32,
        error: String,
        solid_count: u32,
        shell_count: u32,
        face_count: u32,
        /// Faces present in the shape that belong to no solid.
        orphan_faces: u32,
        /// `BRepCheck_Analyzer::IsValid`.
        valid: bool,
        volume: f64,
        min_x: f64,
        min_y: f64,
        min_z: f64,
        max_x: f64,
        max_y: f64,
        max_z: f64,
    }

    unsafe extern "C++" {
        include!("boardstudio-step-oracle/src/oracle.h");
        fn inspect_file(path: &str) -> Facts;
    }
}

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: step-oracle <file.step>...");
        std::process::exit(2);
    }
    for path in paths {
        let f = ffi::inspect_file(&path);
        println!(
            "{{\"file\":{},\"readStatus\":{},\"error\":{},\"solids\":{},\"shells\":{},\"faces\":{},\"orphanFaces\":{},\"valid\":{},\"volume\":{:?},\"min\":[{:?},{:?},{:?}],\"max\":[{:?},{:?},{:?}]}}",
            json_string(&path), f.read_status, json_string(&f.error), f.solid_count, f.shell_count,
            f.face_count, f.orphan_faces, f.valid, f.volume, f.min_x, f.min_y, f.min_z, f.max_x, f.max_y, f.max_z
        );
    }
}
