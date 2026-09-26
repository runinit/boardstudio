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
pub use math::{component_model_transform, pcb_model_transform, transform_point};
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
