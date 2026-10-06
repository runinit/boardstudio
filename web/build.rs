use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
    generate_bundled_ergogen_models(&output);
}

fn generate_bundled_ergogen_models(output: &Path) {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR"));
    let vendor_root = manifest_dir.join("../ergogen/library/vendor");
    println!("cargo:rerun-if-changed={}", vendor_root.display());
    let mut files = Vec::new();
    for vendor in fs::read_dir(&vendor_root).unwrap_or_else(|error| {
        panic!(
            "could not read Ergogen model vendor root {}: {error}",
            vendor_root.display()
        )
    }) {
        let vendor = vendor.expect("read vendor entry").path();
        if !vendor.is_dir() {
            continue;
        }
        let model_root = vendor.join("3d_models");
        if model_root.is_dir() {
            collect_model_files(&model_root, &mut files);
        }
    }
    files.sort_by_key(|path| {
        path.strip_prefix(&vendor_root)
            .expect("model under vendor root")
            .to_string_lossy()
            .into_owned()
    });

    let mut rows = String::new();
    let mut emitted_paths = BTreeSet::new();
    for source_path in &files {
        println!("cargo:rerun-if-changed={}", source_path.display());
        let relative = source_path
            .strip_prefix(&vendor_root)
            .expect("model under vendor root");
        let relative = relative
            .to_str()
            .expect("vendor paths must be UTF-8")
            .replace('\\', "/");
        let (vendor, rest) = relative
            .split_once("/3d_models/")
            .expect("model path under vendor 3d_models");
        let filename = react_saved_filename(vendor, rest);
        let id = format!("bundled-model:{vendor}/{filename}");
        let extension = source_path
            .extension()
            .and_then(|value| value.to_str())
            .expect("model extension")
            .to_ascii_lowercase();
        let media_type = match extension.as_str() {
            "wrl" => "model/vrml",
            "stl" => "model/stl",
            _ => "model/step",
        };
        let token = stable_model_token(&relative);
        let url_path = format!("assets/ergogen-models/model-{token:016x}.{extension}");
        let source_bytes = fs::read(source_path).unwrap_or_else(|error| {
            panic!(
                "could not read packaged Ergogen model {}: {error}",
                source_path.display()
            )
        });
        let sha256 = Sha256::digest(source_bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert!(
            emitted_paths.insert(url_path.clone()),
            "bundled model staging path collision: {url_path}"
        );
        rows.push_str(&format!(
            "    BundledModel {{ id: {:?}, filename: {:?}, media_type: {:?}, source_relative_path: {:?}, url_path: {:?}, sha256: {:?} }},\n",
            id, filename, media_type, relative, url_path, sha256
        ));
    }
    let source = format!("pub(super) static BUNDLED_MODELS: &[BundledModel] = &[\n{rows}];\n");
    fs::write(output.join("bundled_ergogen_models.rs"), source)
        .expect("write generated Ergogen model metadata");
}

fn stable_model_token(path: &str) -> u64 {
    path.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

fn collect_model_files(directory: &Path, files: &mut Vec<PathBuf>) {
    println!("cargo:rerun-if-changed={}", directory.display());
    for entry in fs::read_dir(directory).unwrap_or_else(|error| {
        panic!(
            "could not read model directory {}: {error}",
            directory.display()
        )
    }) {
        let path = entry.expect("read model directory entry").path();
        if path.is_dir() {
            collect_model_files(&path, files);
            continue;
        }
        let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if ["step", "stp", "wrl", "stl"].contains(&extension.to_ascii_lowercase().as_str()) {
            files.push(path);
        }
    }
}

fn react_saved_filename<'a>(vendor: &str, source_path: &'a str) -> &'a str {
    if vendor == "thqwgd001" {
        match source_path {
            "THQWGD001-rotation.stp" => "THQWGD001 #1.stp",
            "THQWGD001C-2pin.stp" => "THQWGD001C [2pin] #1.stp",
            "THQWGD001C-4pin.stp" => "THQWGD001C [4pin] #1.stp",
            _ => source_path,
        }
    } else {
        source_path
    }
}
