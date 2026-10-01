//! Presentation geometry uses the same resolved occupied solids as mechanical generation.
use super::*;

pub(crate) fn prepare_preview(
    doc: &ProjectDoc,
    board_id: &str,
    result: &mut ModuleResolution,
    top_z: f64,
) -> Result<(), String> {
    if !top_z.is_finite() {
        return Err("Module preview frame must be finite".into());
    }
    let host = doc
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or("Module preview host PCB is missing")?;
    let mut bodies = Vec::new();
    for module in &result.modules {
        for (index, solid) in module
            .board
            .iter()
            .chain(module.volumes.iter().map(|volume| &volume.geometry))
            .enumerate()
        {
            let is_board = index < module.board.len();
            let mut contours = vec![Contour {
                points: solid.points.clone(),
                hole: false,
            }];
            if is_board {
                contours.extend(module.board_holes.clone())
            }
            bodies.push(CaseIR {
                revision: doc.revision,
                contours,
                body: CaseBody {
                    id: format!("module-body/{}/{index}", module.id),
                    name: format!(
                        "{} · {}",
                        doc.module_definitions
                            .iter()
                            .find(|def| def.id == module.definition_id)
                            .map_or("Module", |def| def.name.as_str()),
                        if is_board { "PCB" } else { "Assembly volume" }
                    ),
                    board_id: board_id.into(),
                    kind: CaseKind::Plate,
                    thickness: solid.height,
                    clearance: 0.0,
                    z: Some(solid.z + top_z),
                    features: None,
                    openings: None,
                    material_id: None,
                    wall_height: None,
                    wall_thickness: None,
                    mounts: None,
                    gasket: None,
                },
            });
        }
        let board_attached = doc
            .modules
            .iter()
            .find(|instance| instance.id == module.id)
            .is_some_and(|instance| instance.attachment == ModuleAttachment::Board);
        if board_attached {
            for support in &module.mount_supports {
                let circle = |diameter: f64, clockwise: bool| {
                    (0..48)
                        .map(|index| {
                            let angle = std::f64::consts::TAU * index as f64 / 48.0;
                            let direction = if clockwise { -1.0 } else { 1.0 };
                            Vec2 {
                                x: support.at.x + diameter / 2.0 * (direction * angle).cos(),
                                y: support.at.y + diameter / 2.0 * (direction * angle).sin(),
                            }
                        })
                        .collect::<Vec<_>>()
                };
                bodies.push(CaseIR {
                    revision: doc.revision,
                    contours: vec![
                        Contour {
                            points: circle(support.outer_diameter, false),
                            hole: false,
                        },
                        Contour {
                            points: circle(support.hole_diameter, true),
                            hole: true,
                        },
                    ],
                    body: CaseBody {
                        id: format!("module-standoff/{}/{}", module.id, support.mount_id),
                        name: format!("{} · PCB standoff", module.id),
                        board_id: board_id.into(),
                        kind: CaseKind::Plate,
                        thickness: support.height,
                        clearance: 0.0,
                        z: Some(support.z + top_z),
                        features: None,
                        openings: None,
                        material_id: None,
                        wall_height: None,
                        wall_thickness: None,
                        mounts: None,
                        gasket: None,
                    },
                });
            }
        }
        for (index, model) in module.models.iter().enumerate() {
            result.model_placements.push(ModuleModelPlacement {
                id: format!("module-model/{}/{index}", module.id),
                asset_id: model.asset_id.clone(),
                matrix: model_matrix(module, model, host.thickness, top_z),
            });
        }
    }
    if !bodies.is_empty() {
        result.preview = Some(crate::case::prepare(&CaseAssemblyIR {
            revision: doc.revision,
            bodies,
        })?)
    }
    Ok(())
}

fn model_matrix(
    module: &ResolvedModule,
    model: &PartModel,
    host_thickness: f64,
    top_z: f64,
) -> [f64; 16] {
    let point = |p: Vec3| {
        let mut p = Vec3 {
            x: p.x * model.scale.x,
            y: p.y * model.scale.y,
            z: p.z * model.scale.z,
        };
        let (s, c) = (-model.rotation.x).to_radians().sin_cos();
        p = Vec3 {
            x: p.x,
            y: p.y * c - p.z * s,
            z: p.y * s + p.z * c,
        };
        let (s, c) = (-model.rotation.y).to_radians().sin_cos();
        p = Vec3 {
            x: p.x * c + p.z * s,
            y: p.y,
            z: -p.x * s + p.z * c,
        };
        let (s, c) = (-model.rotation.z).to_radians().sin_cos();
        p = Vec3 {
            x: p.x * c - p.y * s,
            y: p.x * s + p.y * c,
            z: p.z,
        };
        p = Vec3 {
            x: p.x + model.offset.x,
            y: p.y + model.offset.y,
            z: p.z + model.offset.z,
        };
        if module.flipped {
            p.x = -p.x;
            p.z = -p.z
        }
        let (s, c) = module.rotation.to_radians().sin_cos();
        Vec3 {
            x: module.at.x + p.x * c - p.y * s,
            y: module.at.y + p.x * s + p.y * c,
            z: module.midplane_z - host_thickness / 2.0 + top_z + p.z,
        }
    };
    let origin = point(Vec3::default());
    let mut matrix = [0.0; 16];
    for (column, unit) in [
        Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        Vec3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    ]
    .into_iter()
    .enumerate()
    {
        let p = point(unit);
        matrix[column * 4] = p.x - origin.x;
        matrix[column * 4 + 1] = p.y - origin.y;
        matrix[column * 4 + 2] = p.z - origin.z;
    }
    matrix[12] = origin.x;
    matrix[13] = origin.y;
    matrix[14] = origin.z;
    matrix[15] = 1.0;
    matrix
}
