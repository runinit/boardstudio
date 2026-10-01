//! Source provenance and output-specific hardware qualification.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareSource {
    pub repository: String,
    pub revision: String,
    pub path: String,
    pub license: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub upstream_status: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum HardwareOutput {
    Footprint,
    Electrical,
    Mechanical,
    Model,
    Firmware,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareGate {
    pub output: HardwareOutput,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum VikRole {
    Host,
    Module,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareProfile {
    pub source: HardwareSource,
    #[serde(default)]
    pub gates: Vec<HardwareGate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub vik_role: Option<VikRole>,
    #[serde(default)]
    pub footprint_surface_volumes: bool,
}

/// Nominal occupied bounds are useful for early fit review even before hardware
/// qualification. They do not establish swept or service clearances.
pub(crate) fn nominal_fit(
    document: &crate::model::ProjectDoc,
) -> (Vec<crate::model::Finding>, Vec<crate::model::FindingMarker>) {
    use crate::model::{CaseOpening, Contour, Finding, FindingMarker, Scope, Severity, Side, Vec2};
    use i_overlay::{
        core::{fill_rule::FillRule, overlay_rule::OverlayRule},
        float::single::SingleFloatOverlay,
    };
    let mut findings = Vec::new();
    let mut markers = Vec::new();
    for board in &document.boards {
        let mut volumes = Vec::new();
        for part in document
            .parts
            .iter()
            .filter(|part| board.part_ids.contains(&part.id))
        {
            let Some(definition) = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
            else {
                continue;
            };
            let surface = definition
                .hardware_profile
                .as_ref()
                .is_some_and(|profile| profile.footprint_surface_volumes);
            let Some(profile) = definition.mechanical_profile.as_ref() else {
                continue;
            };
            let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
            for volume in profile.clearance_volumes.iter().flatten() {
                if volume.points.len() < 3
                    || !volume.z.is_finite()
                    || !volume.height.is_finite()
                    || volume.height <= 0.0
                    || volume
                        .points
                        .iter()
                        .any(|p| !p.x.is_finite() || !p.y.is_finite())
                {
                    continue;
                }
                let transformed = CaseOpening {
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
                    z: if surface && part.side == Side::Back {
                        -board.thickness - volume.z - volume.height
                    } else {
                        volume.z
                    },
                    height: volume.height,
                };
                volumes.push((part, surface, transformed));
            }
        }
        for (index, (a, a_surface, av)) in volumes.iter().enumerate() {
            for (b, b_surface, bv) in &volumes[index + 1..] {
                if a.id == b.id
                    || !(*a_surface || *b_surface)
                    || av.z >= bv.z + bv.height
                    || bv.z >= av.z + av.height
                {
                    continue;
                }
                let paths = |volume: &CaseOpening| {
                    vec![volume.points.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>()]
                };
                let overlap =
                    paths(av).overlay(&paths(bv), OverlayRule::Intersect, FillRule::EvenOdd);
                if overlap.is_empty() {
                    continue;
                }
                let id = format!("hardware-fit/{}/{}/{}", board.id, a.id, b.id);
                let contours = overlap
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
                    .collect();
                if let Some(marker) = markers
                    .iter_mut()
                    .find(|marker: &&mut FindingMarker| marker.finding_id == id)
                {
                    marker.contours.extend(contours);
                    continue;
                }
                findings.push(Finding {id:id.clone(),scope:Scope::Layout,severity:Severity::Warning,message:format!("{} and {}: nominal occupied model bounds overlap. Review the assembly spacing; wheel travel, tool access and tolerances need separate qualification.",a.reference,b.reference),target_ids:vec![a.id.clone(),b.id.clone()]});
                markers.push(FindingMarker {
                    finding_id: id,
                    board_id: board.id.clone(),
                    contours,
                });
            }
        }
    }
    (findings, markers)
}
