//! Links the SHA-256-verified static OCCT 8.0.1 archive that `cad/scripts/cadrum_build.py`
//! prepares (`OCCT_ROOT`). This crate is never a dependency of production or WASM code.
use std::{env, path::PathBuf};

// Same toolkit order Cadrum links; static archives need it.
const LIBS: &[&str] = &[
    "TKernel",
    "TKMath",
    "TKBRep",
    "TKTopAlgo",
    "TKPrim",
    "TKBO",
    "TKBool",
    "TKShHealing",
    "TKMesh",
    "TKGeomBase",
    "TKGeomAlgo",
    "TKG3d",
    "TKG2d",
    "TKBin",
    "TKXSBase",
    "TKDE",
    "TKDECascade",
    "TKOffset",
    "TKFillet",
    "TKDESTEP",
];

fn main() {
    println!("cargo:rerun-if-env-changed=OCCT_ROOT");
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/oracle.cc");
    let root = PathBuf::from(env::var("OCCT_ROOT").expect(
        "OCCT_ROOT must point at the verified OCCT archive (python3 cad/scripts/prepare-cadrum-occt.py native)",
    ));
    let include = root.join("include/opencascade");
    assert!(
        include.is_dir(),
        "{} is not an OCCT include directory",
        include.display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        root.join("lib").display()
    );
    for lib in LIBS {
        println!("cargo:rustc-link-lib=static={lib}");
    }
    cxx_build::bridge("src/main.rs")
        .file("src/oracle.cc")
        .include(&include)
        .std("c++17")
        .define("_USE_MATH_DEFINES", None)
        .compile("boardstudio_step_oracle");
}
