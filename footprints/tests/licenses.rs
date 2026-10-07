//! Every ported generator keeps the licence and attribution of its source.
use std::fs;
use std::path::Path;

use boardstudio_footprints::bundled;

const ALLOWED: [&str; 2] = ["MIT", "CC-BY-NC-SA-4.0"];

#[test]
fn declared_licences_match_the_file_headers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generators");
    let mut sources = Vec::new();
    for directory in ["ceoloide", "infused_kim"] {
        for entry in fs::read_dir(root.join(directory)).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|name| name != "mod.rs") {
                sources.push(fs::read_to_string(path).unwrap());
            }
        }
    }
    let notice =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("NOTICE.md")).unwrap();
    for spec in bundled().specs() {
        let text = sources
            .iter()
            .find(|text| text.contains(&format!("source: \"{}\"", spec.source)))
            .unwrap_or_else(|| panic!("no module for {}", spec.source));
        assert!(
            ALLOWED.contains(&spec.license.spdx),
            "{}: {}",
            spec.source,
            spec.license.spdx
        );
        assert!(
            !spec.license.author.is_empty(),
            "{} has no author",
            spec.source
        );
        assert!(
            text.contains(&format!("SPDX-License-Identifier: {}", spec.license.spdx)),
            "{}: header does not state {}",
            spec.source,
            spec.license.spdx
        );
        assert!(
            notice.contains(&format!("| `{}` | {} |", spec.source, spec.license.spdx)),
            "{} missing from NOTICE.md",
            spec.source
        );
        // Everything under the infused-kim library is NonCommercial.
        if spec.source.starts_with("infused-kim/") {
            assert_eq!(spec.license.spdx, "CC-BY-NC-SA-4.0", "{}", spec.source);
        }
    }
    assert_eq!(bundled().specs().len(), 36);
}
