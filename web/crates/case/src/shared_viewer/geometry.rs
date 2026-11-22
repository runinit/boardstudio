/// Apply Core's column-major affine transform exactly once before generic mesh
/// bodies enter the renderer. Core owns module pose and model offsets.
pub(super) fn transformed_module_points(source_positions: &[f32], matrix: &[f64; 16]) -> Vec<f32> {
    let mut positions = vec![0.0; source_positions.len()];
    for index in (0..source_positions.len()).step_by(3) {
        let x = source_positions[index];
        let y = source_positions[index + 1];
        let z = source_positions[index + 2];
        positions[index] = (matrix[0] as f32) * x
            + (matrix[4] as f32) * y
            + (matrix[8] as f32) * z
            + matrix[12] as f32;
        positions[index + 1] = (matrix[1] as f32) * x
            + (matrix[5] as f32) * y
            + (matrix[9] as f32) * z
            + matrix[13] as f32;
        positions[index + 2] = (matrix[2] as f32) * x
            + (matrix[6] as f32) * y
            + (matrix[10] as f32) * z
            + matrix[14] as f32;
    }
    positions
}

pub(super) fn transformed_module_mesh(
    source_positions: &[f32],
    source_normals: &[f32],
    matrix: &[f64; 16],
) -> (Vec<f32>, Vec<f32>) {
    let squared_lengths = [0, 4, 8].map(|index| {
        (matrix[index].powi(2) + matrix[index + 1].powi(2) + matrix[index + 2].powi(2)) as f32
    });
    let positions = transformed_module_points(source_positions, matrix);
    let mut normals = vec![0.0; source_normals.len()];
    for index in (0..source_normals.len()).step_by(3) {
        let nx = source_normals[index] / squared_lengths[0].max(f32::MIN_POSITIVE);
        let ny = source_normals[index + 1] / squared_lengths[1].max(f32::MIN_POSITIVE);
        let nz = source_normals[index + 2] / squared_lengths[2].max(f32::MIN_POSITIVE);
        let a = matrix[0] as f32 * nx + matrix[4] as f32 * ny + matrix[8] as f32 * nz;
        let b = matrix[1] as f32 * nx + matrix[5] as f32 * ny + matrix[9] as f32 * nz;
        let c = matrix[2] as f32 * nx + matrix[6] as f32 * ny + matrix[10] as f32 * nz;
        let length = (a * a + b * b + c * c).sqrt();
        let length = if length > 0.0 { length } else { 1.0 };
        normals[index] = a / length;
        normals[index + 1] = b / length;
        normals[index + 2] = c / length;
    }
    (positions, normals)
}

pub(super) fn layout_module_bodies(
    modules: &[boardstudio_model::model::ResolvedModule],
    document: &boardstudio_model::model::ProjectDoc,
    board_id: &str,
    pcb_top_z: f64,
) -> Vec<serde_json::Value> {
    modules
        .iter()
        .flat_map(|module| {
            let Some(instance) = document
                .modules
                .iter()
                .find(|instance| instance.id == module.id && instance.host_board_id == board_id)
            else {
                return Vec::new();
            };
            let name = document
                .module_definitions
                .iter()
                .find(|definition| definition.id == instance.definition_id)
                .map(|definition| definition.name.as_str())
                .unwrap_or("Mounted module");
            let contour =
                |points: &[_], hole: bool| serde_json::json!({ "points": points, "hole": hole });
            let mut bodies = module
                .board
                .iter()
                .enumerate()
                .map(|(index, solid)| {
                    let mut contours = vec![contour(&solid.points, false)];
                    contours.extend(
                        module
                            .board_holes
                            .iter()
                            .map(|hole| contour(&hole.points, true)),
                    );
                    serde_json::json!({
                        "id": format!("module:{}:pcb:{index}", module.id),
                        "name": format!("{name} · PCB"),
                        "z": solid.z + pcb_top_z,
                        "thickness": solid.height,
                        "contours": contours,
                    })
                })
                .collect::<Vec<_>>();
            bodies.extend(module.volumes.iter().enumerate().map(|(index, volume)| {
                serde_json::json!({
                    "id": format!("module:{}:volume:{index}", module.id),
                    "name": format!("{name} · Assembly volume"),
                    "z": volume.geometry.z + pcb_top_z,
                    "thickness": volume.geometry.height,
                    "contours": [contour(&volume.geometry.points, false)],
                })
            }));
            if instance.attachment == boardstudio_model::model::ModuleAttachment::Board {
                for support in &module.mount_supports {
                    let ring = |diameter: f64, clockwise: bool| {
                        (0..48)
                            .map(|index| {
                                let direction = if clockwise { -1.0 } else { 1.0 };
                                let angle = std::f64::consts::TAU * index as f64 / 48.0;
                                boardstudio_model::model::Vec2 {
                                    x: support.at.x + diameter / 2.0 * (direction * angle).cos(),
                                    y: support.at.y + diameter / 2.0 * (direction * angle).sin(),
                                }
                            })
                            .collect::<Vec<_>>()
                    };
                    bodies.push(serde_json::json!({
                        "id": format!("module:{}:standoff:{}", module.id, support.mount_id),
                        "name": format!("{} · PCB standoff", module.id),
                        "z": support.z + pcb_top_z,
                        "thickness": support.height,
                        "contours": [
                            contour(&ring(support.outer_diameter, false), false),
                            contour(&ring(support.hole_diameter, true), true),
                        ],
                    }));
                }
            }
            bodies
        })
        .collect()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn generic_layout_module_projection_preserves_board_holes_and_omits_opening_voids() {
        use boardstudio_model::model::{
            CaseOpening, ModuleAttachment, ModuleSupportGeometry, MountedModule, ProjectDoc,
            ResolvedModule, Side, Vec2,
        };

        let mut document = ProjectDoc::empty("project-1", "Generic module");
        document.modules.push(MountedModule {
            id: "placement-generic".into(),
            definition_id: "definition-generic".into(),
            host_board_id: "board-1".into(),
            host_instance_id: None,
            host_face: Side::Front,
            facing_face: Side::Front,
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
            gap: 0.0,
            attachment: ModuleAttachment::Board,
            detached: false,
            connection: None,
            service_clearance: 0.0,
            mount_supports: vec![],
        });
        let polygon = vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 6.0, y: 0.0 },
            Vec2 { x: 6.0, y: 2.0 },
            Vec2 { x: 2.0, y: 2.0 },
            Vec2 { x: 2.0, y: 6.0 },
            Vec2 { x: 0.0, y: 6.0 },
        ];
        let module = ResolvedModule {
            id: "placement-generic".into(),
            definition_id: "definition-generic".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
            midplane_z: 1.6,
            flipped: false,
            board: vec![CaseOpening {
                points: polygon.clone(),
                z: 0.0,
                height: 1.2,
            }],
            pcb_source: None,
            round_drills: vec![],
            board_holes: vec![boardstudio_model::model::Contour {
                points: vec![
                    Vec2 { x: 0.5, y: 0.5 },
                    Vec2 { x: 0.5, y: 1.5 },
                    Vec2 { x: 1.5, y: 1.5 },
                    Vec2 { x: 1.5, y: 0.5 },
                ],
                hole: true,
            }],
            volumes: vec![boardstudio_model::model::ModuleVolume {
                id: "volume-1".into(),
                geometry: CaseOpening {
                    points: polygon,
                    z: 1.2,
                    height: 0.4,
                },
                purpose: "assembly".into(),
                source: "fixture".into(),
                qualified: true,
            }],
            openings: vec![boardstudio_model::model::ModuleVolume {
                id: "clearance-void".into(),
                geometry: CaseOpening {
                    points: vec![
                        Vec2 { x: 2.5, y: 2.5 },
                        Vec2 { x: 3.5, y: 2.5 },
                        Vec2 { x: 3.5, y: 3.5 },
                    ],
                    z: 1.0,
                    height: 3.0,
                },
                purpose: "connector-clearance".into(),
                source: "fixture".into(),
                qualified: true,
            }],
            mounts: vec![],
            mount_supports: vec![ModuleSupportGeometry {
                mount_id: "support-1".into(),
                at: Vec2 { x: 4.0, y: 1.0 },
                outer_diameter: 2.0,
                hole_diameter: 0.8,
                z: 0.0,
                height: 1.0,
            }],
            footprints: vec![],
            models: vec![],
            gates: vec![],
        };

        let bodies = layout_module_bodies(&[module], &document, "board-1", 1.6);
        assert_eq!(bodies.len(), 3, "PCB, assembly volume and standoff only");
        assert_eq!(bodies[0]["id"], "module:placement-generic:pcb:0");
        assert_eq!(bodies[0]["contours"].as_array().unwrap().len(), 2);
        assert_eq!(bodies[0]["contours"][1]["hole"], true);
        assert_eq!(
            bodies[0]["contours"][0]["points"].as_array().unwrap().len(),
            6
        );
        assert_eq!(bodies[1]["id"], "module:placement-generic:volume:0");
        assert_eq!(
            bodies[2]["id"],
            "module:placement-generic:standoff:support-1"
        );
        assert_eq!(bodies[2]["contours"][1]["hole"], true);
        assert!(
            bodies
                .iter()
                .all(|body| !body["id"].as_str().unwrap().contains("clearance-void"))
        );
    }
    #[test]
    fn layout_module_model_normals_follow_nonuniform_core_scale() {
        let diagonal = std::f32::consts::FRAC_1_SQRT_2;
        let matrix = [
            2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let (_, normals) =
            transformed_module_mesh(&[0.0, 0.0, 0.0], &[diagonal, diagonal, 0.0], &matrix);
        assert!((normals[0] - 1.0 / 5.0_f32.sqrt()).abs() < 1e-5);
        assert!((normals[1] - 2.0 / 5.0_f32.sqrt()).abs() < 1e-5);
    }

    #[test]
    fn module_edges_use_the_same_transform_as_the_mesh() {
        let matrix = [
            2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 4.0, 0.0, 10.0, 20.0, 30.0, 1.0,
        ];
        assert_eq!(
            transformed_module_points(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &matrix),
            [12.0, 26.0, 42.0, 18.0, 35.0, 54.0],
        );
    }
}
