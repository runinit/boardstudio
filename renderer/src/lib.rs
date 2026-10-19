use serde::{Deserialize, Serialize};

mod assets;
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod geometry;
mod math;
mod wrl;

#[cfg(test)]
#[path = "mechanical.rs"]
mod mechanical;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub use assets::decode_stl;
pub use math::{
    component_model_transform, mat4_to_f64_cols, pcb_model_transform, physical_pcb_transform,
    transform_point,
};
pub use wrl::decode_wrl;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelMesh {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<f32>>,
}

const MAX_MODEL_BYTES: usize = 32 * 1024 * 1024;
const MAX_MODEL_POSITION_FLOATS: usize = 18_000_000;

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) fn is_stale_scene_revision(current: u64, incoming: u64) -> bool {
    incoming < current
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(source: &str) -> Vec<u8> {
        source.as_bytes().to_vec()
    }

    #[test]
    fn reads_ascii_stl_into_flat_millimeter_triangles() {
        let source = b"solid part\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 2 0 0\nvertex 0 3 0\nendloop\nendfacet\nendsolid part";
        let mesh = decode_stl(source).unwrap();
        assert_eq!(
            mesh.positions,
            vec![0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0, 0.0]
        );
        assert_eq!(mesh.normals.len(), 9);
    }

    #[test]
    fn reads_binary_stl_without_treating_it_as_text() {
        let mut source = vec![0; 134];
        source[80..84].copy_from_slice(&1_u32.to_le_bytes());
        for (index, value) in [
            0.0_f32, 0.0, 1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0, 0.0,
        ]
        .into_iter()
        .enumerate()
        {
            let start = 84 + index * 4;
            source[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }

        let mesh = decode_stl(&source).unwrap();
        assert_eq!(mesh.positions.len(), 9);
        assert_eq!(mesh.positions[7], 3.0);
    }

    #[test]
    fn reads_static_wrl_material_colors_and_kicad_units() {
        let mesh = decode_wrl(&bytes(
            "#VRML V2.0 utf8\nShape { appearance Appearance { material Material { diffuseColor 1 0 0 } } geometry IndexedFaceSet { coord Coordinate { point [ 0 0 0, 1 0 0, 0 1 0 ] } coordIndex [ 0, 1, 2, -1 ] } }",
        ))
        .unwrap();
        assert!((mesh.positions.iter().copied().fold(0.0_f32, f32::max) - 2.54).abs() < 0.0001);
        assert_eq!(mesh.colors.as_ref().unwrap()[0..3], [1.0, 0.0, 0.0]);
    }

    #[test]
    fn supports_wrl_box_primitives_and_vertex_colors() {
        let box_mesh = decode_wrl(&bytes(
            "#VRML V2.0 utf8\nShape { geometry Box { size 10 8 4 } }",
        ))
        .unwrap();
        assert!(!box_mesh.positions.is_empty());

        let colored = decode_wrl(&bytes(
            "#VRML V2.0 utf8\nShape { geometry IndexedFaceSet { coord Coordinate { point [ 0 0 0, 1 0 0, 0 1 0 ] } coordIndex [ 0 1 2 -1 ] color Color { color [ 1 0 0, 0 1 0, 0 0 1 ] } colorPerVertex TRUE } }",
        ))
        .unwrap();
        assert_eq!(
            colored.colors.as_ref().unwrap(),
            &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
        );
    }

    #[test]
    fn applies_static_wrl_translation_and_axis_rotation() {
        let mesh = decode_wrl(&bytes(
            "#VRML V2.0 utf8\nTransform { translation 1 2 3 rotation 0 0 1 1.57079632679 children [ Shape { geometry IndexedFaceSet { coord Coordinate { point [ 0 0 0, 1 0 0, 0 1 0 ] } coordIndex [ 0 1 2 -1 ] } } ] }",
        )).unwrap();
        for (actual, expected) in mesh.positions[..3].iter().zip([2.54, 5.08, 7.62]) {
            assert!((actual - expected).abs() < 0.0001);
        }
        for (actual, expected) in mesh.positions[3..6].iter().zip([2.54, 7.62, 7.62]) {
            assert!((actual - expected).abs() < 0.0001);
        }
    }

    #[test]
    fn resolves_static_wrl_def_use_nodes() {
        let mesh = decode_wrl(&bytes(
            "#VRML V2.0 utf8\nDEF triangle Shape { geometry IndexedFaceSet { coord DEF points Coordinate { point [ 0 0 0, 1 0 0, 0 1 0 ] } coordIndex [ 0 1 2 -1 ] } } USE triangle",
        ))
        .unwrap();
        assert_eq!(mesh.positions.len(), 18);
        assert_eq!(mesh.positions[0..9], mesh.positions[9..18]);
    }

    #[test]
    fn rejects_cyclic_wrl_use_nodes() {
        let source = bytes("#VRML V2.0 utf8\nDEF loop Group { children [ USE loop ] }");
        assert!(decode_wrl(&source).unwrap_err().contains("cyclic USE"));
    }

    /// Nested groups around one shape, counting `depth` traversed node frames.
    fn nested_wrl(depth: usize) -> String {
        let mut source = String::from("#VRML V2.0 utf8\n");
        for _ in 1..depth {
            source.push_str("Group { children [ ");
        }
        source.push_str("Shape { geometry Box {} }");
        for _ in 1..depth {
            source.push_str(" ] }");
        }
        source
    }

    /// A doubling chain of shared groups over one triangle. Level N visits 2^(N+1)-1
    /// frames; the file's top-level scan also walks each `DEF`, and the last level's
    /// `USE` is the root. Every level stays inside the depth limit.
    fn doubling_chain(prefix: &str, levels: usize) -> Vec<String> {
        let mut definitions = vec![format!(
            "DEF {prefix}0 Shape {{ geometry IndexedFaceSet {{ coord Coordinate {{ point [ 0 0 0, 1 0 0, 0 1 0 ] }} coordIndex [ 0 1 2 -1 ] }} }}"
        )];
        for level in 1..=levels {
            definitions.push(format!(
                "DEF {prefix}{level} Group {{ children [ USE {prefix}{} USE {prefix}{} ] }}",
                level - 1,
                level - 1
            ));
        }
        definitions
    }

    #[test]
    fn admits_wrl_nesting_to_the_supported_depth_and_refuses_deeper_input() {
        let admitted = nested_wrl(crate::wrl::MAX_WRL_TRAVERSAL_DEPTH);
        assert!(decode_wrl(admitted.as_bytes()).is_ok());

        let refused = nested_wrl(crate::wrl::MAX_WRL_TRAVERSAL_DEPTH + 1);
        let error = decode_wrl(refused.as_bytes()).unwrap_err();
        assert!(error.contains("depth"), "unexpected error: {error}");
    }

    #[test]
    fn admits_wrl_shared_node_work_within_the_budget_and_refuses_more() {
        // 14 doubling levels visit 98286 frames; 18 visit 1572842.
        let mut admitted = String::from("#VRML V2.0 utf8\n");
        for definition in doubling_chain("a", 14) {
            admitted.push_str(&definition);
            admitted.push('\n');
        }
        admitted.push_str("USE a14");
        assert!(decode_wrl(admitted.as_bytes()).is_ok());

        let mut refused = String::from("#VRML V2.0 utf8\n");
        for definition in doubling_chain("b", 18) {
            refused.push_str(&definition);
            refused.push('\n');
        }
        refused.push_str("USE b18");
        let error = decode_wrl(refused.as_bytes()).unwrap_err();
        assert!(error.contains("work"), "unexpected error: {error}");
    }

    #[test]
    fn charges_separate_admitted_wrl_roots_against_one_shared_budget() {
        // Two 15-level chains visit 196589 frames each: each fits the budget alone, together not.
        let mut source = String::from("#VRML V2.0 utf8\n");
        for definition in doubling_chain("a", 15)
            .into_iter()
            .chain(doubling_chain("b", 15))
        {
            source.push_str(&definition);
            source.push('\n');
        }
        source.push_str("USE a15\nUSE b15");
        let error = decode_wrl(source.as_bytes()).unwrap_err();
        assert!(error.contains("work"), "unexpected error: {error}");
    }

    #[test]
    fn admits_bounded_noncyclic_wrl_reuse() {
        let mut source = String::from(
            "#VRML V2.0 utf8\nDEF tri Shape { geometry IndexedFaceSet { coord Coordinate { point [ 0 0 0, 1 0 0, 0 1 0 ] } coordIndex [ 0 1 2 -1 ] } }\n",
        );
        for _ in 0..1000 {
            source.push_str("USE tri\n");
        }
        // The DEF root is walked once before its 1000 references.
        let mesh = decode_wrl(source.as_bytes()).unwrap();
        assert_eq!(mesh.positions.len(), 1001 * 9);
    }

    #[test]
    fn keeps_the_wrl_output_size_refusal_within_the_traversal_budget() {
        // 166_667 box references stay well inside the traversal budget but exceed the
        // 18M position-float limit, so the output-size refusal must still fire.
        let mut source = String::from("#VRML V2.0 utf8\nDEF b Shape { geometry Box {} }\n");
        for _ in 0..166_667 {
            source.push_str("USE b\n");
        }
        let error = decode_wrl(source.as_bytes()).unwrap_err();
        assert!(
            error.contains("triangle limits"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn refuses_nonconvex_declared_nontriangular_wrl_faces() {
        let u_shape = "#VRML V2.0 utf8\nShape { geometry IndexedFaceSet { convex FALSE coord Coordinate { point [ 0 0 0, 3 0 0, 3 3 0, 2 3 0, 2 1 0, 1 1 0, 1 3 0, 0 3 0 ] } coordIndex [ 0 1 2 3 4 5 6 7 -1 ] } }";
        let error = decode_wrl(u_shape.as_bytes()).unwrap_err();
        assert!(error.contains("convex"), "unexpected error: {error}");

        for control in [
            // `convex` omitted: VRML default TRUE, still fan-triangulated.
            "#VRML V2.0 utf8\nShape { geometry IndexedFaceSet { coord Coordinate { point [ 0 0 0, 2 0 0, 2 2 0, 0 2 0 ] } coordIndex [ 0 1 2 3 -1 ] } }",
            // Explicitly convex polygon.
            "#VRML V2.0 utf8\nShape { geometry IndexedFaceSet { convex TRUE coord Coordinate { point [ 0 0 0, 2 0 0, 2 2 0, 0 2 0 ] } coordIndex [ 0 1 2 3 -1 ] } }",
            // A nonconvex declaration does not refuse a triangle.
            "#VRML V2.0 utf8\nShape { geometry IndexedFaceSet { convex FALSE coord Coordinate { point [ 0 0 0, 2 0 0, 0 2 0 ] } coordIndex [ 0 1 2 -1 ] } }",
        ] {
            assert!(
                decode_wrl(control.as_bytes()).is_ok(),
                "supported control was refused: {control}"
            );
        }
    }

    #[test]
    fn rejects_malformed_wrl_triangle_indexes() {
        let source = bytes(
            "#VRML V2.0 utf8\nShape { geometry IndexedFaceSet { coord Coordinate { point [ 0 0 0, 1 0 0, 0 1 0 ] } coordIndex [ 0 1 9 -1 ] } }",
        );
        assert!(
            decode_wrl(&source)
                .unwrap_err()
                .contains("invalid coordinate index")
        );
    }

    #[test]
    fn loads_bundled_kicad_static_wrl_preview() {
        let source =
            include_bytes!("../../ergogen/library/vendor/koktoh/3d_models/Choc_V2_Red.wrl");
        let mesh = decode_wrl(source).unwrap();
        assert!(mesh.positions.len() > 100_000);
        assert_eq!(mesh.colors.as_ref().unwrap().len(), mesh.positions.len());
    }

    #[test]
    fn rejects_external_content_invalid_and_oversized_assets() {
        for node in [
            "Inline",
            "ImageTexture",
            "MovieTexture",
            "Script",
            "EXTERNPROTO",
        ] {
            let source =
                format!("#VRML V2.0 utf8\n{node} {{ url \"https://example.com/model.wrl\" }}");
            assert!(
                decode_wrl(source.as_bytes())
                    .unwrap_err()
                    .contains("external resources")
            );
        }
        assert!(decode_stl(&[]).is_err());
        assert!(decode_wrl(&[]).is_err());
        let oversized = vec![0; MAX_MODEL_BYTES + 1];
        assert!(decode_stl(&oversized).is_err());
    }

    #[test]
    fn composes_kicad_component_and_pcb_model_transforms() {
        let component =
            component_model_transform([2.0, 3.0, 4.0], [20.0, 40.0, 70.0], [1.0, 2.0, 3.0], false);
        let transformed = transform_point(component, [0.0, 0.0, 0.0]);
        assert!((transformed[0] - 2.0).abs() < 0.0001);
        assert!((transformed[1] + 3.0).abs() < 0.0001);
        assert!((transformed[2] - 4.0).abs() < 0.0001);

        let pcb = pcb_model_transform([10.0, 20.0, 1.6], 0.0, false, [0.0; 3], [0.0; 3], [1.0; 3]);
        assert_eq!(transform_point(pcb, [2.0, 3.0, 4.0]), [12.0, 23.0, 5.6]);

        let bottom =
            pcb_model_transform([10.0, 20.0, 0.0], 0.0, true, [0.0; 3], [0.0; 3], [1.0; 3]);
        let point = transform_point(bottom, [2.0, 3.0, 4.0]);
        assert!((point[0] - 12.0).abs() < 0.0001);
        assert!((point[1] - 17.0).abs() < 0.0001);
        assert!((point[2] + 4.0).abs() < 0.0001);
    }

    #[test]
    fn tessellates_board_surface_and_silkscreen_strokes() {
        let outline = [[0.0, 0.0], [10.0, 0.0], [10.0, 8.0], [0.0, 8.0]];
        let board = geometry::board_mesh(
            &[geometry::BoardContour {
                points: &outline,
                hole: false,
            }],
            1.6,
        )
        .unwrap();
        assert!(!board.indices.is_empty());
        assert!(board.colors.is_none());
        assert!(board.positions.iter().any(|point| point[2] == 1.6));
        assert!(
            geometry::surface_mesh(&outline, 1.602, [0.0, 0.0, 1.0])
                .unwrap()
                .indices
                .len()
                >= 6
        );
        assert!(
            geometry::stroke_mesh(&outline, 0.2, 1.602, [0.0, 0.0, 1.0])
                .unwrap()
                .indices
                .len()
                >= 6
        );
    }

    #[test]
    fn discards_only_scene_packets_older_than_the_current_revision() {
        assert!(is_stale_scene_revision(8, 7));
        assert!(!is_stale_scene_revision(8, 8));
        assert!(!is_stale_scene_revision(8, 9));
    }
}
