//! Ticket 170 application-boundary regression.
//!
//! `boardstudio_web_runtime::renderer_host_page::decode_wrl` is the exact function the page
//! Runtime supplies as the model-delivery `decode_wrl` port (see `runtime.rs`); it imports the
//! packaged renderer once and caches that one module instance. These checks drive that real
//! path against the packaged `web/assets/renderer` asset that `scripts/build-web.py` stages, so
//! a hostile WRL must surface as an `Err` through the application decoder and the same shared
//! module instance must still decode an ordinary box afterwards. The pre-fix decoder trapped
//! that instance instead, which this test cannot survive.
use boardstudio_web_runtime::renderer_host_page::decode_wrl;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn wrl(source: &str) -> Vec<u8> {
    source.as_bytes().to_vec()
}

/// `depth` nested groups around one box, mirroring the audit's 4096-level hostile input.
fn nested_groups(depth: usize) -> String {
    format!(
        "#VRML V2.0 utf8\n{}Shape {{ geometry Box {{}} }}{}",
        "Group { children [ ".repeat(depth),
        " ] }".repeat(depth),
    )
}

/// A reference graph that doubles its (empty) traversal work per level and adds no geometry.
fn doubling_chain(levels: usize) -> String {
    let mut source =
        String::from("#VRML V2.0 utf8\nShape { geometry Box {} }\nDEF e0 Group { children [ ] }\n");
    for level in 1..=levels {
        source.push_str(&format!(
            "DEF e{level} Group {{ children [ USE e{} USE e{} ] }}\n",
            level - 1,
            level - 1
        ));
    }
    source.push_str(&format!("USE e{levels}"));
    source
}

const U_SHAPE: &str = "#VRML V2.0 utf8\nShape { geometry IndexedFaceSet { convex FALSE coord Coordinate { point [ 0 0 0, 3 0 0, 3 3 0, 2 3 0, 2 1 0, 1 1 0, 1 3 0, 0 3 0 ] } coordIndex [ 0 1 2 3 4 5 6 7 -1 ] } }";
const BOX: &str = "#VRML V2.0 utf8\nShape { geometry Box { size 10 8 4 } }";
const DEFAULT_CONVEX: &str = "#VRML V2.0 utf8\nShape { geometry IndexedFaceSet { coord Coordinate { point [ 0 0 0, 2 0 0, 2 2 0, 0 2 0 ] } coordIndex [ 0 1 2 3 -1 ] } }";
const EXPLICIT_CONVEX: &str = "#VRML V2.0 utf8\nShape { geometry IndexedFaceSet { convex TRUE coord Coordinate { point [ 0 0 0, 2 0 0, 2 2 0, 0 2 0 ] } coordIndex [ 0 1 2 3 -1 ] } }";
const NONCONVEX_TRIANGLE: &str = "#VRML V2.0 utf8\nShape { geometry IndexedFaceSet { convex FALSE coord Coordinate { point [ 0 0 0, 2 0 0, 0 2 0 ] } coordIndex [ 0 1 2 -1 ] } }";
const COLORED_TRANSFORM: &str = "#VRML V2.0 utf8\nTransform { translation 1 2 3 children [ Shape { geometry IndexedFaceSet { coord Coordinate { point [ 0 0 0, 1 0 0, 0 1 0 ] } coordIndex [ 0 1 2 -1 ] color Color { color [ 1 0 0, 0 1 0, 0 0 1 ] } colorPerVertex TRUE } } ] }";

/// Decode one box through the same shared module instance and require its expected mesh.
async fn assert_box_still_decodes(after: &str) {
    let mesh = decode_wrl(wrl(BOX))
        .await
        .unwrap_or_else(|error| panic!("shared renderer module unusable after {after}: {error}"));
    assert_eq!(mesh.positions.len(), 108, "box positions after {after}");
    assert_eq!(mesh.normals.len(), 108, "box normals after {after}");
}

#[wasm_bindgen_test]
async fn refuses_hostile_wrl_at_the_application_decoder_without_poisoning_the_shared_module() {
    // One module instance for the whole test: `decode_wrl` caches its imported renderer.
    let error = decode_wrl(wrl(&nested_groups(4096))).await.unwrap_err();
    assert!(error.contains("depth"), "unexpected depth refusal: {error}");
    assert_box_still_decodes("a depth refusal").await;

    let error = decode_wrl(wrl(&doubling_chain(24))).await.unwrap_err();
    assert!(error.contains("work"), "unexpected work refusal: {error}");
    assert_box_still_decodes("a work refusal").await;

    let error = decode_wrl(wrl(U_SHAPE)).await.unwrap_err();
    assert!(
        error.contains("convex"),
        "the U-shaped face must be refused, not filled: {error}"
    );
    assert_box_still_decodes("a nonconvex refusal").await;

    for (name, control) in [
        ("default-convex polygon", DEFAULT_CONVEX),
        ("explicitly convex polygon", EXPLICIT_CONVEX),
        ("nonconvex-declared triangle", NONCONVEX_TRIANGLE),
        ("colored transformed triangle", COLORED_TRANSFORM),
    ] {
        assert!(
            decode_wrl(wrl(control)).await.is_ok(),
            "supported {name} was refused",
        );
    }
}
