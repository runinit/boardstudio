use super::*;
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
};

fn intersect_contours(a: &[Contour], b: &[Contour]) -> Vec<Contour> {
    let paths = |contours: &[Contour]| {
        contours
            .iter()
            .map(|contour| {
                contour
                    .points
                    .iter()
                    .map(|point| [point.x, point.y])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    paths(a)
        .overlay(&paths(b), OverlayRule::Intersect, FillRule::EvenOdd)
        .into_iter()
        .flat_map(|shape| {
            shape
                .into_iter()
                .enumerate()
                .map(|(index, points)| Contour {
                    points: points
                        .into_iter()
                        .map(|point| Vec2 {
                            x: point[0],
                            y: point[1],
                        })
                        .collect(),
                    hole: index > 0,
                })
        })
        .collect()
}

struct Solid {
    contours: Vec<Contour>,
    z: f64,
    height: f64,
}

// Inspect the same prepared plate and shell contours consumed by exact CAD.
// Slice at every aperture elevation so a shallow cut never clears deeper material.
pub(crate) fn case_findings(
    doc: &ProjectDoc,
    board_id: &str,
    modules: &[ResolvedModule],
    prepared: &PreparedCaseAssemblyIR,
) -> (Vec<Finding>, Vec<FindingMarker>) {
    let mut findings = Vec::new();
    let mut markers = Vec::new();
    let travel = doc
        .mechanical
        .as_ref()
        .filter(|config| config.board_id == board_id && config.mount == MechanicalMount::Gasket)
        .and_then(|config| config.gasket_travel)
        .unwrap_or(0.0);
    for module in modules {
        let Some(instance) =
            active_instances(doc, board_id).find(|instance| instance.id == module.id)
        else {
            continue;
        };
        let relative_travel = if instance.attachment == ModuleAttachment::Board {
            travel
        } else {
            0.0
        };
        let solids = module
            .board
            .iter()
            .map(|volume| occupied(volume, &module.board_holes, instance.service_clearance))
            .chain(
                module
                    .volumes
                    .iter()
                    .map(|volume| occupied(&volume.geometry, &[], instance.service_clearance)),
            )
            .collect::<Result<Vec<_>, _>>();
        let Ok(solids) = solids else { continue }; // Resolution already reports invalid service envelopes.
        for body in &prepared.bodies {
            let base = body.body.z.unwrap_or(0.0);
            let wall_height = if body.body.kind == CaseKind::Plate {
                0.0
            } else {
                body.body.wall_height.unwrap_or(0.0)
            };
            let top = base + body.body.thickness + wall_height;
            let cavity_base = if body.body.kind == CaseKind::Tray {
                base + body.body.thickness
            } else {
                base
            };
            let cavity_top = cavity_base + wall_height;
            let mut elevations = vec![base, top, cavity_base, cavity_top];
            for opening in body.body.openings.iter().flatten() {
                elevations.extend([
                    opening.z.clamp(base, top),
                    (opening.z + opening.height).clamp(base, top),
                ]);
            }
            elevations.sort_by(f64::total_cmp);
            elevations.dedup();
            let mut collisions = Vec::new();
            for interval in elevations
                .windows(2)
                .filter(|interval| interval[1] - interval[0] > 1e-9)
            {
                let midpoint = (interval[0] + interval[1]) / 2.0;
                for region in &body.regions {
                    let mut material = vec![Contour {
                        points: region.outer.clone(),
                        hole: false,
                    }];
                    material.extend(region.holes.iter().map(|points| Contour {
                        points: points.clone(),
                        hole: true,
                    }));
                    if midpoint > cavity_base && midpoint < cavity_top {
                        for cavity in &region.cavities {
                            material = subtract(&material, cavity);
                        }
                    }
                    for mount in &region.mounts {
                        material =
                            subtract(&material, &circle(mount.at, mount.hole_diameter, false));
                    }
                    for opening in body.body.openings.iter().flatten().filter(|opening| {
                        midpoint > opening.z && midpoint < opening.z + opening.height
                    }) {
                        material = subtract(&material, &opening.points);
                    }
                    let shell = Solid {
                        contours: material,
                        z: interval[0],
                        height: interval[1] - interval[0],
                    };
                    collisions.extend(
                        solids
                            .iter()
                            .flat_map(|solid| overlap(solid, &shell, relative_travel)),
                    );
                }
            }
            for region in &body.regions {
                let mut additions = region
                    .mounts
                    .iter()
                    .filter(|mount| mount.kind == MountKind::Boss)
                    .map(|mount| {
                        let height = mount.height.unwrap_or(0.0);
                        (
                            CaseOpening {
                                points: circle(mount.at, mount.boss_diameter.unwrap_or(0.0), true),
                                z: if body.body.kind == CaseKind::Lid {
                                    base + wall_height - height
                                } else {
                                    base + body.body.thickness
                                },
                                height,
                            },
                            true,
                        )
                    })
                    .collect::<Vec<_>>();
                additions.extend(body.body.features.iter().flatten().filter_map(|feature| {
                    if let CaseFeature::SupportPrism {
                        points, z, height, ..
                    } = feature
                    {
                        let clipped = intersect_contours(
                            &[Contour {
                                points: points.clone(),
                                hole: false,
                            }],
                            &[Contour {
                                points: region.outer.clone(),
                                hole: false,
                            }],
                        );
                        (!clipped.is_empty()).then(|| {
                            (
                                CaseOpening {
                                    points: points.clone(),
                                    z: *z,
                                    height: *height,
                                },
                                false,
                            )
                        })
                    } else {
                        None
                    }
                }));
                for (addition, drilled) in additions {
                    let mut elevations = vec![addition.z, addition.z + addition.height];
                    for opening in body.body.openings.iter().flatten() {
                        elevations.extend([
                            opening.z.clamp(addition.z, addition.z + addition.height),
                            (opening.z + opening.height)
                                .clamp(addition.z, addition.z + addition.height),
                        ]);
                    }
                    elevations.extend([
                        base.clamp(addition.z, addition.z + addition.height),
                        top.clamp(addition.z, addition.z + addition.height),
                    ]);
                    elevations.sort_by(f64::total_cmp);
                    elevations.dedup();
                    for interval in elevations
                        .windows(2)
                        .filter(|interval| interval[1] - interval[0] > 1e-9)
                    {
                        let midpoint = (interval[0] + interval[1]) / 2.0;
                        let mut material = vec![Contour {
                            points: addition.points.clone(),
                            hole: false,
                        }];
                        // Exact CAD drills the shell/bosses before unioning supports.
                        // A SupportPrism may refill the drilled column.
                        if drilled && midpoint > base && midpoint < top {
                            for mount in &region.mounts {
                                material = subtract(
                                    &material,
                                    &circle(mount.at, mount.hole_diameter, false),
                                );
                            }
                        }
                        for opening in body.body.openings.iter().flatten().filter(|opening| {
                            midpoint > opening.z && midpoint < opening.z + opening.height
                        }) {
                            material = subtract(&material, &opening.points);
                        }
                        let solid = Solid {
                            contours: material,
                            z: interval[0],
                            height: interval[1] - interval[0],
                        };
                        collisions.extend(
                            solids
                                .iter()
                                .flat_map(|module| overlap(module, &solid, relative_travel)),
                        );
                    }
                }
            }
            if !collisions.is_empty() {
                let id = format!("module/{}/case-material/{}", module.id, body.body.id);
                findings.push(Finding { id: id.clone(), severity: Severity::Error, scope: Scope::Case,
                    message: format!("Module clearance intersects solid {} material. Move the module or provide a reviewed aperture.", body.body.name),
                    target_ids: vec![module.id.clone(), body.body.id.clone()] });
                markers.push(FindingMarker {
                    finding_id: id,
                    board_id: board_id.into(),
                    contours: collisions,
                });
            }
        }
    }
    (findings, markers)
}

/// Adds designer-selected module supports to the matching generated case body.
/// The configured support must bridge the body mount plane to the resolved module
/// PCB face; no source dimensions are inferred here.
pub(crate) fn attach_case_supports(
    doc: &ProjectDoc,
    board_id: &str,
    modules: &[ResolvedModule],
    bodies: &mut [CaseIR],
) -> (Vec<Finding>, Vec<FindingMarker>) {
    let mut findings = Vec::new();
    let mut markers = Vec::new();
    for module in modules {
        let Some(instance) = active_instances(doc, board_id).find(|m| m.id == module.id) else {
            continue;
        };
        if instance.attachment != ModuleAttachment::Case || module.mounts.is_empty() {
            continue;
        }
        let supports = super::placement::resolved_mount_supports(instance, module);
        let configured = supports
            .iter()
            .map(|support| support.mount_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let missing = module
            .mounts
            .iter()
            .filter(|mount| !configured.contains(mount.source_id.as_str()))
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            for mount in missing {
                let id = format!("module/{}/mount-support/{}", module.id, mount.source_id);
                findings.push(Finding {
                    id: id.clone(), severity: Severity::Error, scope: Scope::Case,
                    message: format!("Module mount {} needs a designer-defined case support before exact case output.", mount.source_id),
                    target_ids: vec![module.id.clone(), board_id.into()],
                });
                markers.push(FindingMarker {
                    finding_id: id,
                    board_id: board_id.into(),
                    contours: vec![Contour {
                        points: circle(mount.at, mount.diameter, false),
                        hole: false,
                    }],
                });
            }
            continue;
        }
        for support in supports {
            let fits = |body: &CaseIR| {
                if body.body.board_id != board_id {
                    return false;
                }
                let base = body.body.z.unwrap_or(0.0);
                let boss_z = if body.body.kind == CaseKind::Lid {
                    base + body.body.wall_height.unwrap_or(0.0) - support.height
                } else {
                    base + body.body.thickness
                };
                (boss_z - support.z).abs() <= 1e-5
                    && body.body.thickness.is_finite()
                    && body.body.thickness > 0.0
                    && support_contacts_body(body, &support)
            };
            let candidates = bodies
                .iter()
                .enumerate()
                .filter(|(_, body)| fits(body))
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            if candidates.len() != 1 {
                let id = format!("module/{}/mount-support/{}", module.id, support.mount_id);
                findings.push(Finding { id: id.clone(), severity: Severity::Error, scope: Scope::Case,
                    message: "The selected module support does not terminate on exactly one generated case-body boss plane with uninterrupted anchor material.".into(), target_ids: vec![module.id.clone(), board_id.into()] });
                if let Some(mount) = module
                    .mounts
                    .iter()
                    .find(|mount| mount.source_id == support.mount_id)
                {
                    markers.push(FindingMarker {
                        finding_id: id,
                        board_id: board_id.into(),
                        contours: vec![Contour {
                            points: circle(mount.at, mount.diameter, false),
                            hole: false,
                        }],
                    });
                }
                continue;
            }
            let body = &mut bodies[candidates[0]].body;
            let mounts = body.mounts.get_or_insert_with(Vec::new);
            mounts.push(Mount {
                id: format!("module:{}:{}", module.id, support.mount_id),
                at: support.at,
                kind: MountKind::Boss,
                hole_diameter: support.hole_diameter,
                boss_diameter: Some(support.outer_diameter),
                height: Some(support.height),
            });
        }
    }
    (findings, markers)
}

/// Adds designer-selected board-to-board standoffs and host PCB drill contours.
/// The host annulus must remain in host material; partial daughter-board support
/// is allowed with an explicit warning because the source mount can sit near an edge.
pub(crate) fn attach_board_supports(
    doc: &ProjectDoc,
    board_id: &str,
    modules: &[ResolvedModule],
    pcb_reference: Option<&mut CaseIR>,
    bodies: &mut Vec<CaseIR>,
) -> (Vec<Finding>, Vec<FindingMarker>) {
    let mut findings = Vec::new();
    let mut markers = Vec::new();
    let Some(pcb_reference) = pcb_reference else {
        return (findings, markers);
    };
    for module in modules {
        let Some(instance) = active_instances(doc, board_id).find(|m| m.id == module.id) else {
            continue;
        };
        if instance.attachment != ModuleAttachment::Board || module.mounts.is_empty() {
            continue;
        }
        let supports = super::placement::resolved_mount_supports(instance, module);
        let configured = supports
            .iter()
            .map(|support| support.mount_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let missing = module
            .mounts
            .iter()
            .filter(|mount| !configured.contains(mount.source_id.as_str()))
            .collect::<Vec<_>>();
        for mount in &missing {
            let id = format!("module/{}/board-support/{}", module.id, mount.source_id);
            findings.push(Finding {
                id: id.clone(),
                severity: Severity::Error,
                scope: Scope::Pcb,
                message: format!(
                    "Module mount {} needs designer-defined PCB standoff and host drill dimensions before fabrication.",
                    mount.source_id
                ),
                target_ids: vec![module.id.clone(), board_id.into()],
            });
            markers.push(FindingMarker {
                finding_id: id,
                board_id: board_id.into(),
                contours: vec![Contour {
                    points: circle(mount.at, mount.diameter, false),
                    hole: false,
                }],
            });
        }
        if !missing.is_empty() {
            continue;
        }
        let Some(host) = doc.boards.iter().find(|board| board.id == board_id) else {
            continue;
        };
        let Some(module_board) = module_board_material(doc.revision, board_id, module) else {
            continue;
        };
        for support in supports {
            let module_contact_z = if instance.host_face == Side::Front {
                module
                    .board
                    .iter()
                    .map(|board| board.z)
                    .fold(f64::INFINITY, f64::min)
            } else {
                module
                    .board
                    .iter()
                    .map(|board| board.z + board.height)
                    .fold(f64::NEG_INFINITY, f64::max)
            };
            let host_contact_z = if instance.host_face == Side::Front {
                0.0
            } else {
                -host.thickness
            };
            let low = module_contact_z.min(host_contact_z);
            let high = module_contact_z.max(host_contact_z);
            let id = format!("module/{}/board-support/{}", module.id, support.mount_id);
            if (support.z - low).abs() > 1e-6
                || (support.z + support.height - high).abs() > 1e-6
                || !support_contacts_body(pcb_reference, &support)
            {
                findings.push(Finding {
                    id: id.clone(),
                    severity: Severity::Error,
                    scope: Scope::Case,
                    message: "The selected PCB standoff must contact both PCB faces, and its full annulus and host drill must fit within host PCB material.".into(),
                    target_ids: vec![module.id.clone(), board_id.into()],
                });
                markers.push(FindingMarker {
                    finding_id: id,
                    board_id: board_id.into(),
                    contours: vec![Contour {
                        points: circle(support.at, support.outer_diameter, false),
                        hole: false,
                    }],
                });
                continue;
            }
            let daughter_contact = support_contact_area(&module_board, &support);
            let annulus_area = std::f64::consts::PI
                * (support.outer_diameter.powi(2) - support.hole_diameter.powi(2))
                / 4.0;
            if daughter_contact <= annulus_area * 0.01 {
                findings.push(Finding {
                    id: id.clone(),
                    severity: Severity::Error,
                    scope: Scope::Case,
                    message: "The selected PCB standoff has no meaningful contact with module PCB material.".into(),
                    target_ids: vec![module.id.clone(), board_id.into()],
                });
                markers.push(FindingMarker {
                    finding_id: id,
                    board_id: board_id.into(),
                    contours: vec![Contour {
                        points: circle(support.at, support.outer_diameter, false),
                        hole: false,
                    }],
                });
                continue;
            }
            if daughter_contact < annulus_area * 0.999 {
                findings.push(Finding {
                    id: id.clone(),
                    severity: Severity::Warning,
                    scope: Scope::Pcb,
                    message: format!(
                        "The selected PCB standoff overhangs the module PCB; {:.0}% of its annulus contacts source-board material. Verify the fastening method and support during human review.",
                        100.0 * daughter_contact / annulus_area
                    ),
                    target_ids: vec![module.id.clone(), board_id.into()],
                });
                markers.push(FindingMarker {
                    finding_id: id,
                    board_id: board_id.into(),
                    contours: vec![Contour {
                        points: circle(support.at, support.outer_diameter, false),
                        hole: false,
                    }],
                });
            }
            pcb_reference.contours.push(Contour {
                points: circle(support.at, support.hole_diameter, false),
                hole: true,
            });
            bodies.push(CaseIR {
                revision: doc.revision,
                contours: vec![
                    Contour {
                        points: circle(support.at, support.outer_diameter, true),
                        hole: false,
                    },
                    Contour {
                        points: circle(support.at, support.hole_diameter, false),
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
                    z: Some(support.z),
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
    (findings, markers)
}

fn module_board_material(revision: u64, board_id: &str, module: &ResolvedModule) -> Option<CaseIR> {
    let first = module.board.first()?;
    let mut contours = module
        .board
        .iter()
        .map(|board| Contour {
            points: board.points.clone(),
            hole: false,
        })
        .collect::<Vec<_>>();
    contours.extend(module.board_holes.clone());
    Some(CaseIR {
        revision,
        contours,
        body: CaseBody {
            id: format!("module-board/{}", module.id),
            name: "Module PCB material".into(),
            board_id: board_id.into(),
            kind: CaseKind::Plate,
            thickness: first.height,
            clearance: 0.0,
            z: Some(first.z),
            features: None,
            openings: None,
            material_id: None,
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            gasket: None,
        },
    })
}

fn support_contacts_body(body: &CaseIR, support: &ResolvedModuleSupport) -> bool {
    let annulus_area = std::f64::consts::PI
        * (support.outer_diameter.powi(2) - support.hole_diameter.powi(2))
        / 4.0;
    support_contact_area(body, support) >= annulus_area * (1.0 - 1e-6)
}

fn support_contact_area(body: &CaseIR, support: &ResolvedModuleSupport) -> f64 {
    let Ok(prepared) = crate::case::prepare(&CaseAssemblyIR {
        revision: body.revision,
        bodies: vec![body.clone()],
    }) else {
        return 0.0;
    };
    let Some(prepared) = prepared.bodies.first() else {
        return 0.0;
    };
    let annulus = vec![
        Contour {
            points: circle(support.at, support.outer_diameter, true),
            hole: false,
        },
        Contour {
            points: circle(support.at, support.hole_diameter, false),
            hole: true,
        },
    ];
    let mut contact_area = 0.0;
    for region in &prepared.regions {
        let mut material = vec![Contour {
            points: region.outer.clone(),
            hole: false,
        }];
        material.extend(region.holes.iter().map(|hole| Contour {
            points: hole.clone(),
            hole: true,
        }));
        let contact = intersect_contours(&annulus, &material);
        contact_area += contact
            .iter()
            .map(|contour| {
                let area = polygon_area(&contour.points);
                if contour.hole { -area } else { area }
            })
            .sum::<f64>();
    }
    contact_area.max(0.0)
}

fn polygon_area(points: &[Vec2]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let sum = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| a.x * b.y - b.x * a.y)
        .sum::<f64>();
    sum.abs() / 2.0
}

// Circumscribe positive round material and inscribe its drill cuts: the polygon
// approximation cannot silently erase a possible collision with exact CAD.
fn circle(at: Vec2, diameter: f64, positive: bool) -> Vec<Vec2> {
    let radius = diameter
        / 2.0
        / if positive {
            (std::f64::consts::PI / 128.0).cos()
        } else {
            1.0
        };
    (0..128)
        .map(|index| {
            let (sin, cos) = (index as f64 * std::f64::consts::TAU / 128.0).sin_cos();
            Vec2 {
                x: at.x + radius * cos,
                y: at.y + radius * sin,
            }
        })
        .collect()
}

fn subtract(contours: &[Contour], cutter: &[Vec2]) -> Vec<Contour> {
    let paths = contours
        .iter()
        .map(|contour| {
            contour
                .points
                .iter()
                .map(|p| [p.x, p.y])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let cutter = cutter.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>();
    paths
        .overlay(&[cutter], OverlayRule::Difference, FillRule::EvenOdd)
        .into_iter()
        .flat_map(|shape| {
            shape
                .into_iter()
                .enumerate()
                .map(|(index, points)| Contour {
                    hole: index > 0,
                    points: points
                        .into_iter()
                        .map(|p| Vec2 { x: p[0], y: p[1] })
                        .collect(),
                })
        })
        .collect()
}

fn occupied(volume: &CaseOpening, holes: &[Contour], clearance: f64) -> Result<Solid, String> {
    let mut contours = vec![Contour {
        points: volume.points.clone(),
        hole: false,
    }];
    if clearance > 0.0 {
        // A positive offset shrinks holes. If a hole's bounding box is no wider
        // than twice the offset on either axis, its inradius is at most the
        // clearance and the prepared contour cannot retain it. Skip those
        // contours before the expensive offset operation (splitter PCBs have
        // dozens of sub-millimetre drills that all collapse at 1 mm clearance).
        contours.extend(
            holes
                .iter()
                .filter(|hole| !hole_collapses_under_clearance(hole, clearance))
                .cloned(),
        );
        let prepared = crate::case::prepare(&CaseAssemblyIR {
            revision: 0,
            bodies: vec![CaseIR {
                revision: 0,
                contours,
                body: CaseBody {
                    id: "module-service-envelope".into(),
                    name: "Module service envelope".into(),
                    board_id: "host".into(),
                    kind: CaseKind::Plate,
                    thickness: 1.0,
                    clearance,
                    z: None,
                    features: None,
                    openings: None,
                    material_id: None,
                    wall_height: None,
                    wall_thickness: None,
                    mounts: None,
                    gasket: None,
                },
            }],
        })?;
        contours = prepared
            .bodies
            .into_iter()
            .flat_map(|body| {
                body.regions.into_iter().flat_map(|region| {
                    std::iter::once(Contour {
                        points: region.outer,
                        hole: false,
                    })
                    .chain(
                        region
                            .holes
                            .into_iter()
                            .map(|points| Contour { points, hole: true }),
                    )
                })
            })
            .collect();
    } else {
        contours.extend_from_slice(holes);
    }
    Ok(Solid {
        contours,
        z: volume.z - clearance,
        height: volume.height + 2.0 * clearance,
    })
}

fn hole_collapses_under_clearance(hole: &Contour, clearance: f64) -> bool {
    let Some(first) = hole.points.first() else {
        return false;
    };
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (first.x, first.x, first.y, first.y);
    for point in &hole.points[1..] {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }
    (max_x - min_x) / 2.0 <= clearance || (max_y - min_y) / 2.0 <= clearance
}

fn overlap(a: &Solid, b: &Solid, relative_travel: f64) -> Vec<Contour> {
    if a.z - relative_travel < b.z + b.height - 1e-9
        && a.z + a.height + relative_travel > b.z + 1e-9
    {
        intersect_contours(&a.contours, &b.contours)
    } else {
        vec![]
    }
}

fn report(
    result: &mut ModuleResolution,
    board_id: &str,
    id: String,
    targets: Vec<String>,
    message: &str,
    contours: Vec<Contour>,
) {
    if contours.is_empty() {
        return;
    }
    result.findings.push(Finding {
        id: id.clone(),
        scope: Scope::Case,
        severity: Severity::Error,
        message: message.into(),
        target_ids: targets,
    });
    result.markers.push(FindingMarker {
        finding_id: id,
        board_id: board_id.into(),
        contours,
    });
}

pub(super) fn collisions(doc: &ProjectDoc, board_id: &str, result: &mut ModuleResolution) {
    if result.modules.is_empty() {
        return;
    }
    let Some(host) = doc.boards.iter().find(|board| board.id == board_id) else {
        return;
    };
    let mut components = Vec::new();
    for part in doc
        .parts
        .iter()
        .filter(|part| host.part_ids.contains(&part.id))
    {
        let definition = doc
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id);
        let profile = definition
            .and_then(|definition| definition.mechanical_profile.as_ref())
            .or_else(|| {
                doc.mechanical
                    .as_ref()
                    .filter(|config| config.board_id == board_id)
                    .and_then(|config| {
                        config
                            .profiles
                            .iter()
                            .find(|profile| profile.definition_id == part.definition_id)
                    })
            });
        for volume in profile
            .into_iter()
            .flat_map(|profile| profile.clearance_volumes.iter().flatten())
        {
            let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
            let surface_frame = definition.is_some_and(|definition| {
                definition
                    .hardware_profile
                    .as_ref()
                    .is_some_and(|profile| profile.footprint_surface_volumes)
            });
            components.push((
                part.id.clone(),
                Solid {
                    contours: vec![Contour {
                        hole: false,
                        points: volume
                            .points
                            .iter()
                            .map(|point| {
                                let x = if part.side == Side::Back {
                                    -point.x
                                } else {
                                    point.x
                                };
                                Vec2 {
                                    x: part.pose.at.x + x * cos - point.y * sin,
                                    y: part.pose.at.y + x * sin + point.y * cos,
                                }
                            })
                            .collect(),
                    }],
                    z: if surface_frame && part.side == Side::Back {
                        -host.thickness - volume.z - volume.height
                    } else {
                        volume.z
                    },
                    height: volume.height,
                },
            ));
        }
    }
    let (cache, _, _) = crate::geometry::outlines(doc, None, &[]);
    let (boards, _, _) = crate::geometry::board_contours(doc, &cache);
    let pcb = Solid {
        contours: boards
            .into_iter()
            .find(|board| board.board_id == board_id)
            .map_or_else(Vec::new, |board| board.contours),
        z: -host.thickness,
        height: host.thickness,
    };
    let travel = doc
        .mechanical
        .as_ref()
        .filter(|config| config.board_id == board_id && config.mount == MechanicalMount::Gasket)
        .and_then(|config| config.gasket_travel)
        .unwrap_or(0.0);
    let mut modules = Vec::new();
    for module in &result.modules {
        let Some(instance) = doc.modules.iter().find(|instance| instance.id == module.id) else {
            continue;
        };
        let solids: Result<Vec<_>, _> = module
            .board
            .iter()
            .map(|volume| occupied(volume, &module.board_holes, instance.service_clearance))
            .chain(
                module
                    .volumes
                    .iter()
                    .map(|volume| occupied(&volume.geometry, &[], instance.service_clearance)),
            )
            .collect();
        match solids {
            Ok(solids) => modules.push((module.id.clone(), instance.attachment, solids)),
            Err(message) => result.findings.push(Finding {
                id: format!("module/{}/service-envelope", module.id),
                scope: Scope::Case,
                severity: Severity::Error,
                message,
                target_ids: vec![module.id.clone()],
            }),
        }
    }
    for (id, attachment, solids) in &modules {
        let relative_travel = if *attachment == ModuleAttachment::Case {
            travel
        } else {
            0.0
        };
        let contours = solids
            .iter()
            .flat_map(|solid| overlap(solid, &pcb, relative_travel))
            .collect();
        report(
            result,
            board_id,
            format!("module/{id}/host-pcb"),
            vec![id.clone(), board_id.into()],
            "Mounted geometry or required service space intersects the host PCB material. Increase the gap or provide a verified PCB opening.",
            contours,
        );
        for (part_id, body) in &components {
            let contours = solids
                .iter()
                .flat_map(|solid| overlap(solid, body, relative_travel))
                .collect();
            report(
                result,
                board_id,
                format!("module/{id}/host-component/{part_id}"),
                vec![id.clone(), part_id.clone(), board_id.into()],
                "Mounted geometry overlaps a host component, including required service clearance and relative PCB travel. Move the module or increase its surface gap.",
                contours,
            );
        }
    }
    for (index, (a_id, a_attachment, a_solids)) in modules.iter().enumerate() {
        for (b_id, b_attachment, b_solids) in modules.iter().skip(index + 1) {
            let relative_travel = if a_attachment != b_attachment {
                travel
            } else {
                0.0
            };
            let contours = a_solids
                .iter()
                .flat_map(|a| {
                    b_solids
                        .iter()
                        .flat_map(move |b| overlap(a, b, relative_travel))
                })
                .collect();
            report(
                result,
                board_id,
                format!("module/{a_id}/collision/{b_id}"),
                vec![a_id.clone(), b_id.clone(), board_id.into()],
                "Mounted occupied volumes overlap, including required service space and relative board/case travel. Move a module or increase its surface gap.",
                contours,
            );
        }
    }
}
