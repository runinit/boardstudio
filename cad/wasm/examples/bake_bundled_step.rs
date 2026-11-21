use sha2::{Digest, Sha256};
use std::{error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() || !args.len().is_multiple_of(3) {
        return Err("expected one or more SOURCE SHA DESTINATION triples".into());
    }
    for [source, expected_sha, destination] in args.as_chunks::<3>().0 {
        let source = Path::new(source);
        let bytes = fs::read(source)?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if expected_sha != digest.as_str() {
            return Err(format!("{}: source changed while baking", source.display()).into());
        }
        match boardstudio_cad_wasm::bake_bundled_step(bytes, digest) {
            Ok(payload) => fs::write(destination, payload)?,
            Err(error) => eprintln!("cad: keeping live STEP for {}: {error}", source.display()),
        }
    }
    Ok(())
}
