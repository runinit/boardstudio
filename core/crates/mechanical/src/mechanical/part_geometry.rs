//! Per-part footprint geometry and battery space collection.

use super::{
    add_foam_clearance, builtin_profile, convex_envelope, fallback_foam_geometry,
    inferred_switch_family, inside_outline, is_switch_position, same_ring, transform_part_points,
};
use crate::hash::sha256_hex;
use crate::model::*;

pub(super) struct PartGeometry {
    pub(super) pcb_reference_contours: Vec<Contour>,
    pub(super) component_volumes: Vec<(String, CaseOpening)>,
    pub(super) profile_openings: Vec<CaseOpening>,
    pub(super) plate_foam_clearances: Vec<Contour>,
    pub(super) bottom_foam_clearances: Vec<Contour>,
}

/// A finding reporter for one part under review.
type IssueFn<'a> = dyn FnMut(&str, Severity, &str, Vec<String>) + 'a;

/// Reports findings for the part under review.
///
/// Every per-part finding names the same part, so the call sites name only
/// the code suffix, severity and message. Sites with bespoke codes use
/// [`PartReporter::report_raw`].
struct PartReporter<'a> {
    issue: &'a mut IssueFn<'a>,
    part: &'a Part,
}

impl PartReporter<'_> {
    fn report(&mut self, code: &str, severity: Severity, message: &str) {
        let code = format!("{code}:{}", self.part.id);
        self.report_raw(&code, severity, message);
    }

    fn report_raw(&mut self, code: &str, severity: Severity, message: &str) {
        (self.issue)(code, severity, message, vec![self.part.id.clone()]);
    }
}

pub(super) fn collect_part_geometry(
    document: &ProjectDoc,
    config: &MechanicalConfiguration,
    board: &Board,
    contours: &[Contour],
    plate_contours: &mut Vec<Contour>,
    issue: &mut impl FnMut(&str, Severity, &str, Vec<String>),
) -> PartGeometry {
    let mut pcb_reference_contours = contours.to_vec();
    let mut component_volumes = Vec::new();
    let mut profile_openings = Vec::new();
    let mut plate_foam_clearances = Vec::new();
    let mut bottom_foam_clearances = Vec::new();
    let pcb_bottom = -config.pcb_thickness;
    let bottom_foam_z = pcb_bottom - config.bottom_foam_thickness;
    for part in document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
    {
        let definition = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id);
        let is_switch = definition.is_some_and(|definition| is_switch_position(part, definition));
        let mut reporter = PartReporter { issue, part };
        if !part.pose.at.x.is_finite()
            || !part.pose.at.y.is_finite()
            || !part.pose.rotation.is_finite()
        {
            reporter.report(
                "pose",
                Severity::Error,
                "Mechanical part pose must be finite.",
            );
            continue;
        }
        // Ticket 12: case prep reads half-frame parts at their evaluated
        // board pose; storage stays within-half. The borrow extends the
        // owned shadow only while a half-frame offset applies.
        let part: &Part = &crate::matrix::layout::evaluated_part(document, part);
        let mut reporter = PartReporter { issue, part };
        let profile = config
            .profiles
            .iter()
            .find(|p| p.definition_id == part.definition_id);
        let explicit_clearances = profile.is_some_and(|profile| {
            profile
                .clearances
                .as_ref()
                .is_some_and(|items| !items.is_empty())
                || profile
                    .clearance_volumes
                    .as_ref()
                    .is_some_and(|items| !items.is_empty())
        });
        if !explicit_clearances
            && (config.plate_foam_thickness > 0.0 || config.bottom_foam_thickness > 0.0)
        {
            if let Some(definition) = definition {
                if let Some((polygon, source)) = fallback_foam_geometry(part, definition) {
                    let target = if part.side == Side::Back {
                        &mut bottom_foam_clearances
                    } else {
                        &mut plate_foam_clearances
                    };
                    match add_foam_clearance(
                        target,
                        part,
                        &polygon,
                        config.clearance,
                        document.revision,
                    ) {
                        Ok(()) => {
                            let message = format!(
                                "Foam clearance uses the {source} as a conservative footprint approximation because no explicit mechanical clearance is assigned; a 3D model is not required."
                            );
                            reporter.report("foam-approximation", Severity::Warning, &message);
                        }
                        Err(message) => {
                            reporter.report("foam-approximation", Severity::Error, &message);
                        }
                    }
                } else {
                    reporter.report(
                        "foam-bounds",
                        Severity::Warning,
                        "No courtyard, closed pad bounds, or keycap dimensions are available for a foam clearance; no component model was used to guess one.",
                    );
                }
            } else {
                reporter.report(
                    "foam-bounds",
                    Severity::Warning,
                    "The footprint definition is missing, so no outline-based foam clearance could be generated.",
                );
            }
        }
        let Some(profile) = profile else {
            if is_switch {
                let unresolved = definition
                    .is_some_and(|definition| inferred_switch_family(part, definition).is_none());
                let (code, message) = if unresolved {
                    (
                        "switch-family",
                        "Choose the MX, Choc v1, or Choc v2 fit for this switch before generating its plate cutout.",
                    )
                } else {
                    (
                        "profile",
                        "Switch has no qualified mechanical profile; no cutout was guessed.",
                    )
                };
                reporter.report_raw(code, Severity::Error, message);
            }
            continue;
        };
        if is_switch && profile.switch_family.is_none() {
            reporter.report_raw(
                "switch-family",
                Severity::Error,
                "Choose the switch family before generating a plate for this imported footprint.",
            );
        }
        let has_access = profile.openings.as_ref().is_some_and(|v| !v.is_empty())
            || profile.clearances.as_ref().is_some_and(|v| !v.is_empty())
            || profile
                .clearance_volumes
                .as_ref()
                .is_some_and(|v| !v.is_empty());
        if (profile.cutouts.is_empty() && (is_switch || !has_access))
            || profile.source.trim().is_empty()
        {
            reporter.report(
                "profile-empty",
                Severity::Error,
                "Mechanical profile requires cutout geometry and source provenance.",
            );
        }
        if !profile.plate_to_pcb.is_finite()
            || ((is_switch || !profile.cutouts.is_empty())
                && (profile.plate_to_pcb - config.plate_to_pcb).abs() > 0.001)
        {
            reporter.report(
                "engagement",
                Severity::Error,
                "Selected mechanical profiles have incompatible plate-to-PCB engagement distances.",
            );
        }
        if let Some(range) = profile.supported_thickness {
            if !range.x.is_finite()
                || !range.y.is_finite()
                || range.x <= 0.0
                || range.y < range.x
                || config.plate_thickness < range.x
                || config.plate_thickness > range.y
            {
                reporter.report(
                    "thickness",
                    Severity::Error,
                    "Plate thickness is outside this profile's supported range.",
                );
            }
        } else {
            reporter.report(
                "qualification",
                Severity::Warning,
                "Profile plate thickness compatibility has not been qualified.",
            );
        }
        let mut polygons = profile.cutouts.clone();
        let mut pcb_holes = profile.pcb_holes.clone().unwrap_or_default();
        let stabilizer = config
            .stabilizers
            .iter()
            .flatten()
            .find(|s| s.part_id == part.id);
        if let Some(stabilizer) = stabilizer {
            if stabilizer.kind == MechanicalStabilizerKind::PlateMount {
                if let Some(custom) = &stabilizer.profile {
                    if custom.cutouts.is_empty()
                        || custom.source.trim().is_empty()
                        || !custom.plate_to_pcb.is_finite()
                        || (custom.plate_to_pcb - config.plate_to_pcb).abs() > 0.001
                    {
                        reporter.report(
                            "plate-stabilizer",
                            Severity::Error,
                            "Custom plate-mount profile requires source geometry and matching engagement.",
                        );
                    } else {
                        let (sin, cos) = stabilizer.rotation.unwrap_or(0.0).to_radians().sin_cos();
                        polygons.extend(
                            custom.cutouts.iter().map(|polygon| {
                                crate::geometry2d::rotate_polygon(polygon, sin, cos)
                            }),
                        );
                        reporter.report(
                            "custom-stabilizer",
                            Severity::Warning,
                            "Custom plate-mount geometry requires supplier review; compatibility has not been qualified.",
                        );
                    }
                } else {
                    reporter.report(
                        "plate-stabilizer",
                        Severity::Error,
                        "Plate-mount stabilizer requires an explicit custom profile; PCB-mount geometry cannot substitute.",
                    );
                }
            } else if stabilizer.kind == MechanicalStabilizerKind::PcbMount {
                let source = if (stabilizer.units - 2.0).abs() < 0.001 {
                    Some(MechanicalBuiltinProfile::MxStab2u)
                } else if (stabilizer.units - 6.25).abs() < 0.001 {
                    Some(MechanicalBuiltinProfile::MxStab625u)
                } else {
                    None
                };
                if let Some(source) = source {
                    match builtin_profile(part.definition_id.clone(), source, config.plate_to_pcb) {
                        Ok(stab) => {
                            let (sin, cos) =
                                stabilizer.rotation.unwrap_or(0.0).to_radians().sin_cos();
                            pcb_holes.extend(stab.pcb_holes.unwrap_or_default().into_iter().map(
                                |mut hole| {
                                    hole.at = crate::geometry2d::rotate_point(hole.at, sin, cos);
                                    hole
                                },
                            ));
                            polygons.extend(stab.cutouts.into_iter().map(|polygon| {
                                crate::geometry2d::rotate_polygon(&polygon, sin, cos)
                            }));
                        }
                        Err(message) => {
                            reporter.report("stabilizer-source", Severity::Error, &message)
                        }
                    }
                } else {
                    reporter.report(
                        "stabilizer-size",
                        Severity::Error,
                        "No qualified stabilizer profile exists for this key size.",
                    );
                }
            }
        } else if part.keycap.is_some_and(|size| size.x.max(size.y) >= 37.0) {
            reporter.report(
                "stabilizer-proposal",
                Severity::Warning,
                "Wide keycap needs a stabilizer decision: confirm key units and PCB-mount orientation; no cutout was guessed.",
            );
        }
        if let Some(source) = &profile.source_geometry {
            let hash = sha256_hex(source.text.as_bytes());
            if hash != source.sha256 {
                reporter.report(
                    "source-hash",
                    Severity::Error,
                    "Mechanical source hash does not match retained source text.",
                );
            }
        }
        for hole in &pcb_holes {
            if !hole.diameter.is_finite()
                || hole.diameter <= 0.0
                || !hole.at.x.is_finite()
                || !hole.at.y.is_finite()
            {
                reporter.report(
                    "pcb-hole",
                    Severity::Error,
                    "PCB hole references require finite positions and positive diameters.",
                );
                continue;
            }
            let at = crate::geometry::part_to_world(hole.at, &part.pose, part.side.clone());
            let points = (0..64)
                .map(|index| {
                    let angle = index as f64 * std::f64::consts::TAU / 64.0;
                    Vec2 {
                        x: at.x + hole.diameter / 2.0 * angle.cos(),
                        y: at.y + hole.diameter / 2.0 * angle.sin(),
                    }
                })
                .collect();
            let linked = document
                .definitions
                .iter()
                .find(|d| d.id == part.definition_id)
                .is_some_and(|definition| {
                    definition.pads.iter().any(|pad| {
                        pad.drill
                            .is_some_and(|diameter| (diameter - hole.diameter).abs() < 0.001)
                            && (pad.at.x - hole.at.x).abs() < 0.001
                            && (pad.at.y - hole.at.y).abs() < 0.001
                    })
                });
            if linked {
                pcb_reference_contours.push(Contour { hole: true, points });
            }
            if !linked {
                let code = format!("pcb-hole-link:{}:{}", part.id, hole.source_id);
                let severity = if board.traces.is_empty() {
                    Severity::Warning
                } else {
                    Severity::Error
                };
                reporter.report_raw(
                    &code,
                    severity,
                    "Mechanical mounting hole is not linked to a matching electrical PCB hole; the PCB remains unchanged.",
                );
            }
        }
        for volume in profile.clearance_volumes.iter().flatten() {
            let surface_frame = definition.is_some_and(|d| {
                d.hardware_profile
                    .as_ref()
                    .is_some_and(|p| p.footprint_surface_volumes)
            });
            let transformed = CaseOpening {
                z: if surface_frame && part.side == Side::Back {
                    -config.pcb_thickness - volume.z - volume.height
                } else {
                    volume.z
                },
                height: volume.height,
                points: transform_part_points(part, &volume.points),
            };
            if transformed.points.len() < 3
                || transformed
                    .points
                    .iter()
                    .any(|p| !p.x.is_finite() || !p.y.is_finite())
                || !volume.z.is_finite()
                || !volume.height.is_finite()
                || volume.height <= 0.0
            {
                reporter.report(
                    "clearance-volume",
                    Severity::Error,
                    "Clearance volumes need finite polygons/Z and positive height.",
                );
            } else {
                let volume_bottom = transformed.z;
                let volume_top = transformed.z + transformed.height;
                let plate_foam_top = config.plate_foam_thickness;
                if config.plate_foam_thickness > 0.0
                    && volume_bottom < plate_foam_top
                    && volume_top > 0.0
                {
                    match add_foam_clearance(
                        &mut plate_foam_clearances,
                        part,
                        &volume.points,
                        config.clearance,
                        document.revision,
                    ) {
                        Ok(()) => {}
                        Err(message) => {
                            reporter.report("clearance-volume", Severity::Error, &message)
                        }
                    }
                }
                if config.bottom_foam_thickness > 0.0
                    && volume_bottom < pcb_bottom
                    && volume_top > bottom_foam_z
                {
                    match add_foam_clearance(
                        &mut bottom_foam_clearances,
                        part,
                        &volume.points,
                        config.clearance,
                        document.revision,
                    ) {
                        Ok(()) => {}
                        Err(message) => {
                            reporter.report("clearance-volume", Severity::Error, &message)
                        }
                    }
                }
                component_volumes.push((part.id.clone(), transformed));
            }
        }
        for opening in profile.openings.iter().flatten() {
            profile_openings.push(CaseOpening {
                z: opening.z,
                height: opening.height,
                points: transform_part_points(part, &opening.points),
            });
        }
        for polygon in profile.clearances.iter().flatten() {
            if !crate::geometry2d::is_finite_polygon(polygon) {
                reporter.report(
                    "clearance",
                    Severity::Error,
                    "Profile clearance geometry must be finite polygons.",
                );
            } else {
                let target = if part.side == Side::Back {
                    &mut bottom_foam_clearances
                } else {
                    &mut plate_foam_clearances
                };
                if let Err(message) =
                    add_foam_clearance(target, part, polygon, config.clearance, document.revision)
                {
                    reporter.report("clearance", Severity::Error, &message);
                }
                reporter.report(
                    "access",
                    Severity::Warning,
                    "Explicit component clearance is excluded from foam on the component side; case wall access still requires review.",
                );
            }
        }
        for polygon in &polygons {
            if !crate::geometry2d::is_finite_polygon(polygon) {
                reporter.report(
                    "polygon",
                    Severity::Error,
                    "Mechanical profile has an invalid polygon.",
                );
                continue;
            }
            let cutout = Contour {
                hole: true,
                points: polygon
                    .iter()
                    .map(|p| crate::geometry::part_to_world(*p, &part.pose, part.side.clone()))
                    .collect(),
            };
            if !plate_contours
                .iter()
                .any(|existing| same_ring(existing, &cutout))
            {
                plate_contours.push(cutout);
            }
        }
    }
    PartGeometry {
        pcb_reference_contours,
        component_volumes,
        profile_openings,
        plate_foam_clearances,
        bottom_foam_clearances,
    }
}

pub(super) struct BatterySpace {
    pub(super) height: f64,
    pub(super) bottom_foam_contours: Vec<Contour>,
}

pub(super) fn resolve_battery_space(
    config: &MechanicalConfiguration,
    contours: &[Contour],
    issue: &mut impl FnMut(&str, Severity, &str, Vec<String>),
) -> Option<BatterySpace> {
    let battery_height = config
        .battery
        .as_ref()
        .map_or(config.battery_height, |b| b.size.z);
    let mut bottom_foam_contours = contours.to_vec();
    if let Some(battery) = &config.battery {
        if [battery.size.x, battery.size.y, battery.size.z]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0)
        {
            issue(
                "battery-size",
                Severity::Error,
                "Battery dimensions must be positive finite millimetres.",
                vec!["battery".into()],
            );
            return None;
        }
        let margin = config.clearance;
        let hx = battery.size.x / 2.0 + margin;
        let hy = battery.size.y / 2.0 + margin;
        let envelope = vec![
            Vec2 {
                x: battery.at.x - hx,
                y: battery.at.y - hy,
            },
            Vec2 {
                x: battery.at.x + hx,
                y: battery.at.y - hy,
            },
            Vec2 {
                x: battery.at.x + hx,
                y: battery.at.y + hy,
            },
            Vec2 {
                x: battery.at.x - hx,
                y: battery.at.y + hy,
            },
        ];
        if ![
            battery.at.x,
            battery.at.y,
            battery.cable_exit.x,
            battery.cable_exit.y,
        ]
        .iter()
        .all(|v| v.is_finite())
            || envelope.iter().any(|p| !inside_outline(*p, contours))
            || !inside_outline(battery.cable_exit, contours)
        {
            issue(
                "battery-containment",
                Severity::Error,
                "Battery envelope or cable exit lies outside the assembly outline.",
                vec!["battery".into()],
            );
        }
        for mount in config
            .mounts
            .iter()
            .chain(config.closure_mounts.iter().flatten())
        {
            let radius = mount.boss_diameter.unwrap_or(mount.hole_diameter) / 2.0;
            if (mount.at.x - battery.at.x).abs() < hx + radius
                && (mount.at.y - battery.at.y).abs() < hy + radius
            {
                issue(
                    &format!("battery-mount:{}", mount.id),
                    Severity::Error,
                    "Mount overlaps the battery access envelope.",
                    vec!["battery".into(), mount.id.clone()],
                );
            }
        }
        let cable_width = battery.cable_width.unwrap_or(2.0);
        if !cable_width.is_finite() || cable_width <= 0.0 {
            issue(
                "battery-cable-width",
                Severity::Error,
                "Battery cable clearance width must be positive finite millimetres.",
                vec!["battery".into()],
            );
        } else {
            let clearance = cable_width / 2.0 + config.clearance;
            let mut access_points = envelope;
            for (x, y) in [
                (-clearance, -clearance),
                (clearance, -clearance),
                (clearance, clearance),
                (-clearance, clearance),
            ] {
                access_points.push(Vec2 {
                    x: battery.cable_exit.x + x,
                    y: battery.cable_exit.y + y,
                });
            }
            let access = convex_envelope(access_points);
            if access.iter().any(|point| !inside_outline(*point, contours)) {
                issue(
                    "battery-cable-containment",
                    Severity::Error,
                    "Battery cable clearance envelope crosses the assembly outline.",
                    vec!["battery".into()],
                );
            }
            bottom_foam_contours.push(Contour {
                points: access,
                hole: true,
            });
            issue(
                "battery-cable-envelope",
                Severity::Info,
                if battery.cable_width.is_none() {
                    "Bottom foam excludes a conservative battery-to-cable envelope using the default 2 mm cable width; review cable fit."
                } else {
                    "Bottom foam excludes a conservative battery-to-cable envelope using the configured cable width."
                },
                vec!["battery".into(), "bottom-foam".into()],
            );
        }
        issue(
            "battery-fit",
            Severity::Warning,
            "Battery envelope and cable exit require mechanical fit review; retention features are not generated.",
            vec!["battery".into()],
        );
    }
    Some(BatterySpace {
        height: battery_height,
        bottom_foam_contours,
    })
}
