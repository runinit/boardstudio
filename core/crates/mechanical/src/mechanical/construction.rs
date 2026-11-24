//! Body construction and final assembly of the resolved layer stack.

use super::{gasket, transform_part_points};
use crate::model::*;

pub(super) struct ConstructionGeometry {
    pub(super) plate_contours: Vec<Contour>,
    pub(super) frame_contours: Vec<Contour>,
    pub(super) plate_foam_contours: Vec<Contour>,
    pub(super) bottom_foam_contours: Vec<Contour>,
    pub(super) profile_openings: Vec<CaseOpening>,
    pub(super) suspension_bosses: Vec<Mount>,
    pub(super) bottom_z: f64,
    pub(super) bottom_foam_z: f64,
    pub(super) battery_z: f64,
    pub(super) support_height: f64,
}

pub(super) struct ConstructedBodies {
    pub(super) bodies: Vec<CaseIR>,
    pub(super) stack: Vec<MechanicalStackLayer>,
}

pub(super) fn construct_bodies(
    revision: u64,
    config: &MechanicalConfiguration,
    geometry: ConstructionGeometry,
    issue: &mut impl FnMut(&str, Severity, &str, Vec<String>),
) -> ConstructedBodies {
    let ConstructionGeometry {
        plate_contours,
        frame_contours,
        plate_foam_contours,
        bottom_foam_contours,
        profile_openings,
        suspension_bosses,
        bottom_z,
        bottom_foam_z,
        battery_z,
        support_height,
    } = geometry;
    let mut bodies: Vec<CaseIR> = Vec::new();
    let mut stack = Vec::new();
    let board_id = config.board_id.clone();
    let sheet_bottom = config.bottom_style == Some(MechanicalBottomStyle::Sheet)
        || config.part_processes.iter().flatten().any(|process| {
            process.part_id == "bottom"
                && matches!(process.method, PlateMethod::PcbFr4 | PlateMethod::CutSheet)
        });

    if sheet_bottom && !config.middle_frame.unwrap_or(false) && !config.integrated_plate_frame {
        issue(
            "sheet-enclosure",
            Severity::Warning,
            "A sheet bottom without a middle frame leaves the assembly sides open.",
            vec!["bottom".into()],
        );
    }
    if config.middle_frame.unwrap_or(false) && (!sheet_bottom || config.integrated_plate_frame) {
        issue(
            "middle-frame-configuration",
            Severity::Error,
            "A separate middle frame requires a sheet bottom and a separate plate.",
            vec!["middle-frame".into()],
        );
    }
    let frame_outer = frame_contours.clone();
    for (id, kind, z, thickness, body_contours) in [
        (
            "plate",
            if config.integrated_plate_frame {
                CaseKind::Lid
            } else {
                CaseKind::Plate
            },
            if config.integrated_plate_frame {
                bottom_z + config.bottom_thickness
            } else {
                config.plate_to_pcb
            },
            config.plate_thickness,
            plate_contours,
        ),
        (
            "bottom",
            if config.integrated_plate_frame || sheet_bottom {
                CaseKind::Plate
            } else {
                CaseKind::Tray
            },
            bottom_z,
            config.bottom_thickness,
            frame_contours,
        ),
        (
            "plate-foam",
            CaseKind::Plate,
            0.0,
            config.plate_foam_thickness,
            plate_foam_contours,
        ),
        (
            "bottom-foam",
            CaseKind::Plate,
            bottom_foam_z,
            config.bottom_foam_thickness,
            bottom_foam_contours,
        ),
    ] {
        if thickness <= 0.0 {
            continue;
        }
        bodies.push(CaseIR {
            revision,
            contours: body_contours,
            body: CaseBody {
                features: None,
                openings: if (id == "bottom" && !config.integrated_plate_frame)
                    || (id == "plate" && config.integrated_plate_frame)
                {
                    let mut openings: Vec<CaseOpening> = config
                        .openings
                        .iter()
                        .flatten()
                        .map(MechanicalOpening::as_case_opening)
                        .collect();
                    openings.extend(profile_openings.clone());
                    Some(openings)
                } else {
                    None
                },
                id: id.into(),
                name: id.into(),
                board_id: board_id.clone(),
                kind,
                thickness,
                clearance: 0.0,
                material_id: None,
                z: Some(z),
                wall_height: if (id == "bottom" && !config.integrated_plate_frame)
                    || (id == "plate" && config.integrated_plate_frame)
                {
                    Some(
                        config.plate_to_pcb
                            - bottom_z
                            - config.bottom_thickness
                            - if config.mount == MechanicalMount::Gasket {
                                config.gasket_travel.unwrap_or(0.0)
                            } else {
                                0.0
                            },
                    )
                } else {
                    None
                },
                wall_thickness: Some(config.wall_thickness),
                mounts: Some(if id == "bottom" {
                    let mut mounts = config.closure_mounts.clone().unwrap_or_default();
                    mounts.extend(suspension_bosses.clone());
                    mounts
                } else if id == "plate" {
                    let mut mounts: Vec<Mount> = config
                        .mounts
                        .iter()
                        .filter(|_| config.mount == MechanicalMount::Rigid)
                        .map(|mount| {
                            let mut hole = mount.clone();
                            hole.kind = MountKind::Hole;
                            hole.height = None;
                            hole.boss_diameter = None;
                            hole
                        })
                        .collect();
                    mounts.extend(config.closure_mounts.iter().flatten().map(|mount| {
                        let mut mating = mount.clone();
                        if !config.integrated_plate_frame {
                            mating.kind = MountKind::Hole;
                            mating.height = None;
                            mating.boss_diameter = None;
                        }
                        mating
                    }));
                    mounts
                } else {
                    vec![]
                }),
                pcb_source: None,
                gasket: None,
            },
        });
    }
    if sheet_bottom
        && config.middle_frame.unwrap_or(false)
        && !config.integrated_plate_frame
        && let Some(bottom) = bodies.iter().find(|body| body.body.id == "bottom")
    {
        let mut frame = bottom.clone();
        frame.body.id = "middle-frame".into();
        frame.body.name = "Middle frame".into();
        frame.body.kind = CaseKind::Tray;
        frame.body.wall_height = Some(config.plate_to_pcb - bottom_z - config.bottom_thickness);
        frame.contours = frame_outer;
        if let Ok(prepared) = crate::case::prepare(&CaseAssemblyIR {
            revision,
            bodies: vec![frame.clone()],
        }) {
            let mut rings = Vec::new();
            for region in &prepared.bodies[0].regions {
                rings.push(Contour {
                    hole: false,
                    points: region.outer.clone(),
                });
                rings.extend(region.cavities.iter().map(|points| Contour {
                    hole: true,
                    points: points.clone(),
                }));
            }
            let frame_z = bottom_z + config.bottom_thickness;
            frame.contours = rings;
            frame.body.kind = CaseKind::Plate;
            frame.body.z = Some(frame_z);
            frame.body.thickness = config.plate_to_pcb - bottom_z - config.bottom_thickness;
            frame.body.wall_height = None;
            frame.body.gasket = None;
            stack.push(MechanicalStackLayer {
                id: "middle-frame".into(),
                z: frame_z,
                thickness: frame.body.thickness,
            });
            bodies.push(frame);
        }
    }
    if sheet_bottom {
        if let Some(bottom) = bodies.iter_mut().find(|body| body.body.id == "bottom") {
            for mount in bottom.body.mounts.iter_mut().flatten() {
                mount.kind = MountKind::Hole;
                mount.boss_diameter = None;
                mount.height = None;
            }
        }
        for support in &suspension_bosses {
            let Some(diameter) = support.boss_diameter else {
                continue;
            };
            let ring = |diameter: f64, hole: bool| Contour {
                hole,
                points: (0..64)
                    .map(|index| {
                        let angle = index as f64 * std::f64::consts::TAU / 64.0;
                        Vec2 {
                            x: support.at.x + diameter / 2.0 * angle.cos(),
                            y: support.at.y + diameter / 2.0 * angle.sin(),
                        }
                    })
                    .collect(),
            };
            let id = format!("spacer:{}", support.id);
            bodies.push(CaseIR {
                revision,
                contours: vec![ring(diameter, false), ring(support.hole_diameter, true)],
                body: CaseBody {
                    features: None,
                    id: id.clone(),
                    name: id.clone(),
                    board_id: config.board_id.clone(),
                    kind: CaseKind::Plate,
                    thickness: support_height,
                    clearance: 0.0,
                    material_id: None,
                    z: Some(battery_z),
                    wall_height: None,
                    wall_thickness: None,
                    mounts: None,
                    pcb_source: None,
                    gasket: None,
                    openings: None,
                },
            });
            stack.push(MechanicalStackLayer {
                id,
                z: battery_z,
                thickness: support_height,
            });
        }
    }
    ConstructedBodies { bodies, stack }
}

pub(super) fn finalize_assembly(
    document: &ProjectDoc,
    config: &MechanicalConfiguration,
    result: &mut MechanicalAssembly,
    component_volumes: &[(String, CaseOpening)],
    accepted_contours: &[Contour],
) {
    crate::mechanical_checks::apply_allowance(config, result);
    if config.mount == MechanicalMount::Gasket
        && !result.generation_blocked
        && let Err(message) = gasket::generate(
            document,
            config,
            result,
            component_volumes,
            accepted_contours,
        )
    {
        result.generation_blocked = true;
        result.diagnostics.push(Finding {
            id: "mechanical:gasket-layout".into(),
            severity: Severity::Error,
            scope: Scope::Case,
            message,
            target_ids: vec!["plate".into()],
        });
    }
    match crate::case::prepare(&result.case) {
        Ok(prepared) => {
            let (findings, markers) = crate::modules::case_findings(
                document,
                &config.board_id,
                &result.modules,
                &prepared,
            );
            result.generation_blocked |= !findings.is_empty();
            result.diagnostics.extend(findings);
            result.finding_markers.extend(markers);
        }
        Err(message) => {
            result.generation_blocked = true;
            result.diagnostics.push(Finding {
                id: "mechanical:case-preparation".into(),
                severity: Severity::Error,
                scope: Scope::Case,
                message,
                target_ids: vec![],
            });
        }
    }
    if config.internal_gasket.is_none() || config.mount != MechanicalMount::Gasket {
        let mut mounting_contours = result.plate_contours.clone();
        let board = document
            .boards
            .iter()
            .find(|board| board.id == config.board_id);
        for part in document
            .parts
            .iter()
            .filter(|part| board.is_some_and(|board| board.part_ids.contains(&part.id)))
        {
            // Ticket 12: mounting holes follow the evaluated board pose of
            // half-frame parts. The borrow extends the owned shadow only
            // while a half-frame offset applies.
            let part: &Part = &crate::matrix::layout::evaluated_part(document, part);
            let Some(definition) = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
            else {
                continue;
            };
            if definition.kind != PartKind::Switch && definition.courtyard.len() >= 3 {
                mounting_contours.push(Contour {
                    hole: true,
                    points: transform_part_points(part, &definition.courtyard),
                });
            }
            for pad in &definition.pads {
                let (sin, cos) = pad.rotation.unwrap_or(0.0).to_radians().sin_cos();
                let points =
                    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| Vec2 {
                        x: pad.at.x + x * pad.size.x / 2.0 * cos - y * pad.size.y / 2.0 * sin,
                        y: pad.at.y + x * pad.size.x / 2.0 * sin + y * pad.size.y / 2.0 * cos,
                    });
                mounting_contours.push(Contour {
                    hole: true,
                    points: transform_part_points(part, &points),
                });
            }
        }
        for opening in config.openings.iter().flatten() {
            mounting_contours.push(Contour {
                hole: true,
                points: opening.points(),
            });
        }
        result.suggested_mounts =
            crate::mechanical_checks::propose_mounts(config, &mounting_contours);
    }
    result.diagnostics.extend(crate::mechanical_checks::check(
        config,
        &result.plate_contours,
    ));
    result
        .diagnostics
        .extend(crate::mechanical_checks::check_specifications(
            config,
            &result.case,
        ));
}
