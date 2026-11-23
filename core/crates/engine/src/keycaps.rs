//! Independently designed keycap presets and conservative swept-envelope checks.
//! No KeyV2 source, dimensions, or generated geometry is used here.
use crate::model::*;
use boardstudio_contracts::polygon_contains;
mod edits;
pub(crate) use edits::apply_edit;

fn mount_for_family(family: MechanicalSwitchFamily) -> KeycapMount {
    match family {
        MechanicalSwitchFamily::ChocV1 => KeycapMount::ChocV1,
        MechanicalSwitchFamily::ChocV2 => KeycapMount::ChocV2,
        MechanicalSwitchFamily::Mx => KeycapMount::Mx,
    }
}

fn catalog_mount(part: &Part, definition: &PartDefinition) -> Option<KeycapMount> {
    crate::mechanical::generator_switch_family(definition, part.generator_parameters.as_ref())
        .map(mount_for_family)
}

fn finding(id: String, message: String, targets: Vec<String>, severity: Severity) -> Finding {
    Finding {
        id,
        message,
        target_ids: targets,
        severity,
        scope: Scope::Case,
    }
}
fn color_valid(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].bytes().all(|c| c.is_ascii_hexdigit())
}
pub(crate) fn binding_legend(binding: &str) -> String {
    let Some(code) = binding.strip_prefix("&kp ") else {
        return String::new();
    };
    match code {
        "SPACE" => "Space".into(),
        "ENTER" => "Enter".into(),
        "ESC" => "Esc".into(),
        "BSPC" => "Backspace".into(),
        "TAB" => "Tab".into(),
        "LSHFT" => "Shift".into(),
        "LCTRL" => "Ctrl".into(),
        "LALT" => "Alt".into(),
        "LGUI" => "Super".into(),
        "MINUS" => "-".into(),
        "EQUAL" => "=".into(),
        "LBKT" => "[".into(),
        "RBKT" => "]".into(),
        "BSLH" => "\\".into(),
        "SEMI" => ";".into(),
        "SQT" => "'".into(),
        "COMMA" => ",".into(),
        "DOT" => ".".into(),
        "FSLH" => "/".into(),
        "GRAVE" => "`".into(),
        "UP" => "↑".into(),
        "DOWN" => "↓".into(),
        "LEFT" => "←".into(),
        "RIGHT" => "→".into(),
        value
            if value.len() == 2
                && value.starts_with('N')
                && value.as_bytes()[1].is_ascii_digit() =>
        {
            value[1..].into()
        }
        value => value.into(),
    }
}

pub(crate) struct KeycapProjection {
    pub resolution: KeycapResolution,
    footprints: Vec<[Vec2; 4]>,
}

const CLEARANCE_TOLERANCE: f64 = 1e-7;

struct ClearanceBounds {
    min: Vec2,
    max: Vec2,
}

impl ClearanceBounds {
    fn from_footprint([min, right, max, left]: &[Vec2; 4]) -> Option<Self> {
        // Axis-aligned edges avoid cancellation in the legacy crossing test.
        // Within this coordinate range, roundoff in the axis separation and
        // point-to-segment projection stays well below CLEARANCE_TOLERANCE.
        const LIMIT: f64 = 1_048_576.0;
        (min.x >= -LIMIT
            && min.y >= -LIMIT
            && max.x <= LIMIT
            && max.y <= LIMIT
            && min.x < max.x
            && min.y < max.y
            && right.x == max.x
            && right.y == min.y
            && left.x == min.x
            && left.y == max.y)
            .then_some(Self {
                min: *min,
                max: *max,
            })
    }

    fn separated_from(&self, other: &Self, distance: f64) -> bool {
        other.min.x - self.max.x > distance
            || self.min.x - other.max.x > distance
            || other.min.y - self.max.y > distance
            || self.min.y - other.max.y > distance
    }
}

pub(crate) fn resolve(
    doc: &ProjectDoc,
    board_id: &str,
    cases: Option<&PreparedCaseAssemblyIR>,
) -> KeycapResolution {
    project(doc, board_id, cases).resolution
}

pub(crate) fn project(
    doc: &ProjectDoc,
    board_id: &str,
    cases: Option<&PreparedCaseAssemblyIR>,
) -> KeycapProjection {
    let mut projection = KeycapProjection {
        resolution: KeycapResolution {
            revision: doc.revision,
            specs: vec![],
            findings: vec![],
        },
        footprints: vec![],
    };
    let result = &mut projection.resolution;
    let Some(config) = &doc.keycaps else {
        return projection;
    };
    let Some(board) = doc.boards.iter().find(|b| b.id == board_id) else {
        return projection;
    };
    let defaults = config.boards.get(board_id).cloned().unwrap_or_default();
    if !color_valid(&defaults.color)
        || !color_valid(&defaults.legend_color)
        || !defaults.clearance.is_finite()
        || !(0.0..=5.0).contains(&defaults.clearance)
    {
        result.findings.push(finding(
            format!("keycaps/{board_id}/settings"),
            "Keycap colors must be #RRGGBB and clearance must be 0–5 mm".into(),
            vec![board_id.into()],
            Severity::Error,
        ));
        return projection;
    }
    let bindings = doc
        .hardware
        .as_ref()
        .and_then(|h| h.board(board_id))
        .map(|b| &b.key_bindings);
    for part in doc.parts.iter().filter(|p| board.part_ids.contains(&p.id)) {
        let Some(def) = doc.definitions.iter().find(|d| d.id == part.definition_id) else {
            continue;
        };
        let matrix = doc.matrices.iter().find(|m| m.part_ids.contains(&part.id));
        let settings = matrix
            .and_then(|m| config.matrices.get(&m.id))
            .cloned()
            .unwrap_or_default();
        let key = config.keys.get(&part.id).cloned().unwrap_or_default();
        let Some(profile) = key.profile.or(settings.profile) else {
            continue;
        };
        if def.kind != PartKind::Switch && matrix.is_none() && !config.keys.contains_key(&part.id) {
            continue;
        }
        let family = def
            .mechanical_profile
            .as_ref()
            .and_then(|p| p.switch_family)
            .or_else(|| {
                doc.mechanical
                    .as_ref()
                    .and_then(|config| config.profiles.iter().find(|p| p.definition_id == def.id))
                    .and_then(|p| p.switch_family)
            });
        let inferred = family
            .map(mount_for_family)
            .or_else(|| catalog_mount(part, def));
        if def.kind != PartKind::Switch && inferred.is_none() {
            result.findings.push(finding(
                format!("keycaps/{}/unsupported-input", part.id),
                format!("{}: saved keycap settings are retained, but this input has no verified keycap mount. Choose a suitable knob or qualify its fit before generating a keycap.", part.reference),
                vec![part.id.clone()],
                Severity::Warning,
            ));
            continue;
        }
        let Some(mount) = key.mount.or(settings.mount).or(inferred) else {
            result.findings.push(finding(
                format!("keycaps/{}/socket", part.id),
                "Choose a keycap socket or assign the switch a fit family".into(),
                vec![part.id.clone()],
                Severity::Error,
            ));
            continue;
        };
        let row_index = part
            .id
            .split('/')
            .find_map(|s| {
                s.strip_prefix('r')
                    .and_then(|s| s.split('c').next())
                    .and_then(|s| s.parse::<u32>().ok())
            })
            .unwrap_or(0)
            .min(4) as u8;
        let row = key
            .row
            .unwrap_or(settings.first_row.saturating_add(row_index).min(5));
        let size = if let Some(units) = key.units {
            Vec2 {
                x: units.x * 19.05 - 0.85,
                y: units.y * 19.05 - 0.85,
            }
        } else {
            part.keycap
                .or(def.keycap)
                .unwrap_or(Vec2 { x: 18.2, y: 18.2 })
        };
        let color = key.color.unwrap_or_else(|| defaults.color.clone());
        let legend = key.legend.unwrap_or_else(|| {
            let typed = doc
                .keymap
                .as_ref()
                .and_then(|map| map.layers.first())
                .and_then(|layer| layer.bindings.get(&part.id));
            if let Some(binding) = typed {
                return match binding {
                    KeyBinding::KeyPress { keycode } | KeyBinding::StickyKey { keycode } => {
                        binding_legend(&format!("&kp {keycode}"))
                    }
                    KeyBinding::ModTap { tap, .. } | KeyBinding::LayerTap { tap, .. } => {
                        binding_legend(&format!("&kp {tap}"))
                    }
                    _ => String::new(),
                };
            }
            bindings
                .and_then(|b| b.get(&part.id))
                .map(|b| binding_legend(b))
                .unwrap_or_default()
        });
        let error = if inferred.is_some_and(|inferred| mount != inferred) {
            Some("Keycap socket does not match the switch family")
        } else if !(1..=5).contains(&row) || !(1..=5).contains(&settings.first_row) {
            Some("Keycap row must be 1–5")
        } else if !settings.wall_thickness.is_finite()
            || !(0.8..=2.0).contains(&settings.wall_thickness)
        {
            Some("Keycap wall thickness must be 0.8–2 mm")
        } else if !size.x.is_finite()
            || !size.y.is_finite()
            || !(12.0..=150.0).contains(&size.x)
            || !(12.0..=150.0).contains(&size.y)
        {
            Some("Keycap dimensions must be 12–150 mm")
        } else if !color_valid(&color) {
            Some("Keycap color must be #RRGGBB")
        } else if legend.chars().count() > 12 || legend.chars().any(|c| c.is_control()) {
            Some("Keycap legends must contain at most 12 printable characters")
        } else if profile == KeycapProfile::Choc
            && !matches!(mount, KeycapMount::ChocV1 | KeycapMount::ChocV2)
        {
            Some("The low Choc profile requires a Choc switch")
        } else {
            None
        };
        if let Some(message) = error {
            result.findings.push(finding(
                format!("keycaps/{}/settings", part.id),
                message.into(),
                vec![part.id.clone()],
                Severity::Error,
            ));
            continue;
        }
        let (height, tilt, dish_depth, spherical) = match profile {
            KeycapProfile::Cherry => (
                8.0 + f64::from(row.abs_diff(3)) * 0.7,
                (f64::from(row) - 3.0) * 3.0,
                0.55,
                false,
            ),
            KeycapProfile::Oem => (
                10.0 + f64::from(row.abs_diff(3)) * 0.8,
                (f64::from(row) - 3.0) * 4.0,
                0.65,
                false,
            ),
            KeycapProfile::Dcs => (
                8.5 + f64::from(row.abs_diff(3)) * 0.6,
                (f64::from(row) - 3.0) * 3.5,
                0.6,
                false,
            ),
            KeycapProfile::Dsa => (7.5, 0.0, 0.65, true),
            KeycapProfile::Sa => (
                12.0 + f64::from(row.abs_diff(3)),
                (f64::from(row) - 3.0) * 4.0,
                0.7,
                true,
            ),
            KeycapProfile::HiPro => (
                13.0 + f64::from(row.abs_diff(3)),
                (f64::from(row) - 3.0) * 4.0,
                0.7,
                true,
            ),
            KeycapProfile::G20 => (6.5, 0.0, 0.0, false),
            KeycapProfile::Choc => (4.2, 0.0, 0.4, true),
        };
        let socket_depth = if matches!(mount, KeycapMount::ChocV1 | KeycapMount::ChocV2) {
            2.3
        } else {
            3.6
        };
        if height - dish_depth - settings.wall_thickness < socket_depth + 0.2 {
            result.findings.push(finding(
                format!("keycaps/{}/roof", part.id),
                "Keycap roof is too low for the socket; reduce wall thickness".into(),
                vec![part.id.clone()],
                Severity::Error,
            ));
            continue;
        }
        let (base, travel) = match mount {
            KeycapMount::Mx => (8.4, 4.0),
            KeycapMount::ChocV1 => (4.0, 3.0),
            KeycapMount::ChocV2 => (5.0, 3.0),
            KeycapMount::Alps => (8.0, 3.5),
        };
        result.specs.push(KeycapSpec {
            id: part.id.clone(),
            reference: part.reference.clone(),
            profile,
            mount,
            row,
            size,
            top_size: Vec2 {
                x: size.x - 5.0,
                y: size.y - 5.0,
            },
            height,
            tilt,
            dish_depth,
            spherical,
            wall_thickness: settings.wall_thickness,
            // Ticket 12: keycaps render half-frame parts at their evaluated
            // board pose.
            pose: crate::matrix::layout::evaluated_part_pose(doc, part),
            side: part.side.clone(),
            z: if part.side == Side::Front {
                board.thickness + base
            } else {
                -base
            },
            travel,
            legend,
            color,
            legend_color: defaults.legend_color.clone(),
        });
    }
    if cases.is_some_and(|cases| cases.revision != doc.revision) {
        result.findings.push(finding(
            format!("keycaps/{board_id}/case-pending"),
            "Update the Case preview to check keycaps against the current case revision".into(),
            vec![board_id.into()],
            Severity::Warning,
        ));
    }
    projection.footprints = result.specs.iter().map(envelope).collect();
    let bounds: Vec<_> = projection
        .footprints
        .iter()
        .map(ClearanceBounds::from_footprint)
        .collect();
    let broad_phase_distance = defaults.clearance + CLEARANCE_TOLERANCE;
    for (i, (a, footprint)) in result.specs.iter().zip(&projection.footprints).enumerate() {
        let footprint_bounds = bounds[i].as_ref();
        for ((b, other), other_bounds) in result.specs[i + 1..]
            .iter()
            .zip(&projection.footprints[i + 1..])
            .zip(&bounds[i + 1..])
        {
            if a.side != b.side
                || footprint_bounds
                    .zip(other_bounds.as_ref())
                    .is_some_and(|(a, b)| a.separated_from(b, broad_phase_distance))
            {
                continue;
            }
            let gap = rectangle_gap(footprint, other);
            if gap + CLEARANCE_TOLERANCE < defaults.clearance {
                result.findings.push(finding(
                    format!("keycaps/{}/{}", a.id, b.id),
                    format!(
                        "{} and {}: keycap clearance {:.2} mm (required {:.2} mm)",
                        a.reference, b.reference, gap, defaults.clearance
                    ),
                    vec![a.id.clone(), b.id.clone()],
                    Severity::Warning,
                ));
            }
        }
        if let Some(cases) = cases.filter(|c| c.revision == doc.revision) {
            let top = a.height + a.top_size.y * 0.5 * a.tilt.to_radians().tan().abs();
            let (min_z, max_z) = if a.side == Side::Front {
                (a.z - a.travel, a.z + top)
            } else {
                (a.z - top, a.z + a.travel)
            };
            for case in &cases.bodies {
                let body = &case.body;
                let low = body.z.unwrap_or(0.0);
                let high = low + body.thickness + body.wall_height.unwrap_or(0.0);
                for feature in body.features.iter().flatten() {
                    let (id, points, z, height) = match feature {
                        CaseFeature::SupportPrism {
                            id,
                            points,
                            z,
                            height,
                        } => (id, points.clone(), *z, *height),
                        CaseFeature::RoundSeat {
                            id,
                            at,
                            z,
                            height,
                            diameter,
                        } => (id, round_boundary(*at, *diameter), *z, *height),
                        CaseFeature::ConicalSeat {
                            id,
                            at,
                            z,
                            height,
                            diameter,
                            end_diameter,
                        } => (
                            id,
                            round_boundary(*at, diameter.max(*end_diameter)),
                            *z,
                            *height,
                        ),
                    };
                    if z + height >= min_z
                        && z <= max_z
                        && polygon_distance(&points, footprint) < defaults.clearance.max(1e-6)
                    {
                        result.findings.push(finding(
                            format!("keycaps/{}/feature/{id}", a.id),
                            format!(
                                "{}: swept keycap envelope may contact {} / {}",
                                a.reference, body.name, id
                            ),
                            vec![a.id.clone(), body.id.clone()],
                            Severity::Warning,
                        ));
                    }
                }
                if high < min_z || low > max_z {
                    continue;
                }
                let collision = case.regions.iter().any(|region| {
                    let in_cavity = body.kind == CaseKind::Tray
                        && low + body.thickness < min_z
                        && region.cavities.iter().any(|cavity| {
                            polygon_contains_envelope(cavity, footprint, defaults.clearance)
                        });
                    let in_hole = region
                        .holes
                        .iter()
                        .any(|hole| polygon_contains_envelope(hole, footprint, defaults.clearance));
                    !in_cavity
                        && !in_hole
                        && polygon_distance(&region.outer, footprint) < defaults.clearance.max(1e-6)
                });
                if collision {
                    result.findings.push(finding(
                        format!("keycaps/{}/case/{}", a.id, body.id),
                        format!(
                            "{}: swept keycap envelope may contact {} during {:.1} mm travel",
                            a.reference, body.name, a.travel
                        ),
                        vec![a.id.clone(), body.id.clone()],
                        Severity::Warning,
                    ));
                }
            }
        }
    }
    projection
}
fn envelope(s: &KeycapSpec) -> [Vec2; 4] {
    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| {
        crate::geometry::part_to_world(
            Vec2 {
                x: x * s.size.x / 2.0,
                y: y * s.size.y / 2.0,
            },
            &s.pose,
            Side::Front,
        )
    })
}
fn point_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let den = dx * dx + dy * dy;
    let t = if den > 0.0 {
        ((p.x - a.x) * dx + (p.y - a.y) * dy) / den
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
}
fn segments_cross(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let cross = |a: Vec2, b: Vec2, c: Vec2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    cross(a, b, c) * cross(a, b, d) < 0.0 && cross(c, d, a) * cross(c, d, b) < 0.0
}
fn edge_distance(a: &[Vec2], b: &[Vec2]) -> f64 {
    let mut gap = f64::INFINITY;
    for (&a0, &a1) in a.iter().zip(a.iter().cycle().skip(1)).take(a.len()) {
        for (&b0, &b1) in b.iter().zip(b.iter().cycle().skip(1)).take(b.len()) {
            if segments_cross(a0, a1, b0, b1) {
                return 0.0;
            }
            // Each endpoint is the start of the next edge of its closed ring.
            gap = gap
                .min(point_segment(a0, b0, b1))
                .min(point_segment(b0, a0, a1));
        }
    }
    gap
}
fn polygon_distance(a: &[Vec2], b: &[Vec2]) -> f64 {
    if a.iter().any(|p| polygon_contains(b, *p)) || b.iter().any(|p| polygon_contains(a, *p)) {
        0.0
    } else {
        edge_distance(a, b)
    }
}
fn rectangle_gap(a: &[Vec2], b: &[Vec2]) -> f64 {
    // A negative overlap is useful even when the requested clearance is zero.
    let gap = polygon_distance(a, b);
    if gap <= 1e-8 {
        let mut overlap = f64::INFINITY;
        for poly in [a, b] {
            for (p, q) in poly
                .iter()
                .zip(poly.iter().cycle().skip(1))
                .take(poly.len())
            {
                let dx = q.y - p.y;
                let dy = p.x - q.x;
                let length = dx.hypot(dy);
                let project = |r: &[Vec2]| {
                    r.iter()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                            let v = (p.x * dx + p.y * dy) / length;
                            (lo.min(v), hi.max(v))
                        })
                };
                let (a0, a1) = project(a);
                let (b0, b1) = project(b);
                overlap = overlap.min(a1.min(b1) - a0.max(b0));
            }
        }
        -overlap.max(0.0)
    } else {
        gap
    }
}
fn polygon_contains_envelope(poly: &[Vec2], shape: &[Vec2], clearance: f64) -> bool {
    shape.iter().all(|p| polygon_contains(poly, *p))
        && !shape
            .iter()
            .zip(shape.iter().cycle().skip(1))
            .take(shape.len())
            .any(|(&a, &b)| {
                poly.iter()
                    .zip(poly.iter().cycle().skip(1))
                    .take(poly.len())
                    .any(|(&c, &d)| segments_cross(a, b, c, d))
            })
        && edge_distance(poly, shape) + 1e-7 >= clearance
}

fn round_boundary(at: Vec2, diameter: f64) -> Vec<Vec2> {
    (0..32)
        .map(|index| {
            let angle = f64::from(index) * std::f64::consts::TAU / 32.0;
            Vec2 {
                x: at.x + diameter / 2.0 / (std::f64::consts::PI / 32.0).cos() * angle.cos(),
                y: at.y + diameter / 2.0 / (std::f64::consts::PI / 32.0).cos() * angle.sin(),
            }
        })
        .collect()
}

/// Shared scene markers use the same resolved envelopes as clearance validation.
impl KeycapProjection {
    pub fn finding_markers(&self, board_id: &str, findings: &[Finding]) -> Vec<FindingMarker> {
        if self.footprints.is_empty() {
            return vec![];
        }
        findings
            .iter()
            .filter(|finding| finding.id.starts_with("keycaps/"))
            .filter_map(|finding| {
                let contours: Vec<_> = self
                    .resolution
                    .specs
                    .iter()
                    .zip(&self.footprints)
                    .filter(|(spec, _)| finding.target_ids.contains(&spec.id))
                    .map(|(_, footprint)| Contour {
                        points: footprint.to_vec(),
                        hole: false,
                    })
                    .collect();
                (!contours.is_empty()).then(|| FindingMarker {
                    finding_id: finding.id.clone(),
                    board_id: board_id.into(),
                    contours,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clearance_bounds_reject_only_safely_separated_rectangles() {
        let rectangle = |x: f64, y: f64| {
            [(x, y), (x + 18.0, y), (x + 18.0, y + 18.0), (x, y + 18.0)].map(|(x, y)| Vec2 { x, y })
        };
        let origin = rectangle(0.0, 0.0);
        let bounds = ClearanceBounds::from_footprint(&origin).unwrap();
        for clearance in [0.0, 1.5, 5.0] {
            let distance = clearance + CLEARANCE_TOLERANCE;
            let near = rectangle(18.0 + clearance, 0.0);
            let overlap = rectangle(17.0, 0.0);
            for footprint in [near, overlap] {
                let other = ClearanceBounds::from_footprint(&footprint).unwrap();
                assert!(!bounds.separated_from(&other, distance));
            }
            for (x, y) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                let offset = 18.0 + clearance + 2.0 * CLEARANCE_TOLERANCE;
                let far = rectangle(x * offset, y * offset);
                let other = ClearanceBounds::from_footprint(&far).unwrap();
                assert!(bounds.separated_from(&other, distance));
                assert!(rectangle_gap(&origin, &far) + CLEARANCE_TOLERANCE >= clearance);
            }
        }
        let rotated =
            [(0.0, 0.0), (12.0, 12.0), (0.0, 24.0), (-12.0, 12.0)].map(|(x, y)| Vec2 { x, y });
        let mut malformed = origin;
        malformed[1].x = f64::NAN;
        for footprint in [rotated, [origin[0]; 4], rectangle(1.0e15, 0.0), malformed] {
            assert!(ClearanceBounds::from_footprint(&footprint).is_none());
        }
    }

    #[test]
    fn rotated_gap_and_overlap() {
        let a = vec![
            Vec2 { x: -1.0, y: -1.0 },
            Vec2 { x: 1.0, y: -1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: -1.0, y: 1.0 },
        ];
        let b = a
            .iter()
            .map(|p| Vec2 {
                x: p.x + 3.0,
                y: p.y,
            })
            .collect::<Vec<_>>();
        assert!((rectangle_gap(&a, &b) - 1.0).abs() < 1e-9);
        assert!(rectangle_gap(&a, &a) < -1.9);
        let b = vec![
            Vec2 { x: 0.0, y: -2.0 },
            Vec2 { x: 2.0, y: 0.0 },
            Vec2 { x: 0.0, y: 2.0 },
            Vec2 { x: -2.0, y: 0.0 },
        ];
        assert!(rectangle_gap(&a, &b) < 0.0);
    }
    #[test]
    fn concave_cavity_rejects_crossing_edges_even_at_zero_clearance() {
        let cavity = [
            (-2.0, -2.0),
            (2.0, -2.0),
            (2.0, 2.0),
            (0.4, 2.0),
            (0.4, 0.0),
            (-0.4, 0.0),
            (-0.4, 2.0),
            (-2.0, 2.0),
        ]
        .map(|(x, y)| Vec2 { x, y });
        let cap = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| Vec2 { x, y });
        assert!(cap.iter().all(|point| polygon_contains(&cavity, *point)));
        assert!(!polygon_contains_envelope(&cavity, &cap, 0.0));
    }
    #[test]
    fn legends_follow_binding_with_blank_supported() {
        assert_eq!(binding_legend("&kp N5"), "5");
        assert_eq!(binding_legend("&kp A"), "A");
        assert_eq!(binding_legend("&none"), "");
    }
}
