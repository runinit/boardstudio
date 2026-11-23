//! Pure projections from the accepted document and resolved assembly into the rows the Case
//! mechanical inspector renders, so the field tables stay declarative and natively testable.
use boardstudio_core_mechanical::mechanical::change::MechanicalDimension;
use boardstudio_model::model::{
    MechanicalAssembly, MechanicalBottomStyle, MechanicalConfiguration, MechanicalPartProcess,
    MechanicalPartProfile, MechanicalStabilizerKind, MechanicalStabilizerOverride,
    MechanicalSwitchFamily, Mount, MountKind, Part, PartKind, PlateMethod, ProjectDoc, Vec2,
};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalProfileChoice {
    pub definition_id: String,
    pub name: String,
    pub source: String,
    pub family: Option<MechanicalSwitchFamily>,
    pub plate_to_pcb: f64,
    pub supported_thickness: Option<Vec2>,
    /// True only for a placed switch definition, whose fit family the user may choose.
    pub switch_family_selectable: bool,
    pub profile: MechanicalPartProfile,
    pub has_kicad_source: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalProfileTargetChoice {
    pub definition_id: String,
    pub name: String,
    pub switch_profile: bool,
    pub imported_geometry: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalProcessTarget {
    pub part_id: String,
    pub name: String,
    pub default_thickness: f64,
    pub process: Option<MechanicalPartProcess>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalStabilizerFit {
    pub part_id: String,
    pub reference: String,
    pub key_units: f64,
    pub default: MechanicalStabilizerOverride,
    pub current: MechanicalStabilizerOverride,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalLayerRow {
    pub id: String,
    pub label: String,
    pub z: f64,
    pub thickness: f64,
    /// Present only when the displayed completed Case assembly contains a body with this id.
    pub resolved_body_thickness: Option<f64>,
    /// Retained from the last completed same-owner assembly while the inputs have changed.
    pub is_previous: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalFitPart {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalHardwareMount {
    pub part_id: String,
    pub part_name: String,
    pub feature_id: String,
    pub kind: MountKind,
    pub x: f64,
    pub y: f64,
}

/// Generated parts and mounts the hardware and critical-fit selectors offer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MechanicalSceneRows {
    pub layers: Vec<MechanicalLayerRow>,
    pub fit_parts: Vec<MechanicalFitPart>,
    pub hardware_mounts: Vec<MechanicalHardwareMount>,
}

pub fn mechanical_layer_label(id: &str, internal_gasket: bool) -> String {
    match id {
        "retainer" if internal_gasket => "Top case".into(),
        "pcb" => "PCB".into(),
        _ => {
            let label = id.replace('-', " ");
            let mut chars = label.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        }
    }
}

/// Rows of the resolved assembly; the current resolution precedes the previous display geometry.
pub fn project_scene_rows(
    display: Option<&MechanicalAssembly>,
    current: Option<&MechanicalAssembly>,
    internal_gasket: bool,
    is_previous: bool,
) -> Option<MechanicalSceneRows> {
    let (layer_assembly, layers_previous) =
        current.map_or((display, is_previous), |a| (Some(a), false));
    let layer_assembly = layer_assembly?;
    let layers = layer_assembly
        .stack
        .iter()
        .map(|layer| MechanicalLayerRow {
            id: layer.id.clone(),
            label: mechanical_layer_label(&layer.id, internal_gasket),
            z: layer.z,
            thickness: layer.thickness,
            resolved_body_thickness: layer_assembly
                .case
                .bodies
                .iter()
                .find(|body| body.body.id == layer.id)
                .map(|body| body.body.thickness),
            is_previous: layers_previous,
        })
        .collect();
    let bodies = || {
        current
            .into_iter()
            .flat_map(|assembly| assembly.case.bodies.iter())
    };
    let part_name = |body: &boardstudio_model::model::CaseBody| {
        if body.name.is_empty() {
            body.id.clone()
        } else {
            body.name.clone()
        }
    };
    Some(MechanicalSceneRows {
        layers,
        fit_parts: bodies()
            .map(|body| MechanicalFitPart {
                id: body.body.id.clone(),
                name: part_name(&body.body),
            })
            .collect(),
        hardware_mounts: bodies()
            .flat_map(|body| {
                body.body
                    .mounts
                    .iter()
                    .flatten()
                    .map(move |mount| MechanicalHardwareMount {
                        part_id: body.body.id.clone(),
                        part_name: part_name(&body.body),
                        feature_id: mount.id.clone(),
                        kind: mount.kind.clone(),
                        x: mount.at.x,
                        y: mount.at.y,
                    })
            })
            .collect(),
    })
}

fn is_switch(document: &ProjectDoc, definition_id: &str) -> bool {
    document
        .definitions
        .iter()
        .find(|definition| definition.id == definition_id)
        .is_some_and(|definition| {
            definition.kind == PartKind::Switch
                || definition
                    .generator
                    .as_ref()
                    .is_some_and(|generator| generator.source.ends_with("/switch_choc_v1_v2"))
        })
}

pub fn profile_choices(
    document: &ProjectDoc,
    configuration: &MechanicalConfiguration,
) -> Vec<MechanicalProfileChoice> {
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == configuration.board_id)
    else {
        return Vec::new();
    };
    configuration
        .profiles
        .iter()
        .map(|profile| {
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == profile.definition_id);
            MechanicalProfileChoice {
                definition_id: profile.definition_id.clone(),
                name: definition.map_or_else(
                    || profile.definition_id.clone(),
                    |definition| definition.name.clone(),
                ),
                source: profile.source.clone(),
                family: profile.switch_family,
                plate_to_pcb: profile.plate_to_pcb,
                supported_thickness: profile.supported_thickness,
                switch_family_selectable: document.parts.iter().any(|part: &Part| {
                    board.part_ids.contains(&part.id)
                        && part.definition_id == profile.definition_id
                        && is_switch(document, &part.definition_id)
                }),
                profile: profile.clone(),
                has_kicad_source: definition
                    .is_some_and(|definition| definition.kicad_source.is_some()),
            }
        })
        .collect()
}

/// Placed definitions on the board that have no profile yet.
pub fn profile_targets(
    document: &ProjectDoc,
    board_id: &str,
    configuration: Option<&MechanicalConfiguration>,
) -> Vec<MechanicalProfileTargetChoice> {
    let (Some(board), Some(configuration)) = (
        document.boards.iter().find(|board| board.id == board_id),
        configuration,
    ) else {
        return Vec::new();
    };
    let assigned: HashSet<&str> = configuration
        .profiles
        .iter()
        .map(|profile| profile.definition_id.as_str())
        .collect();
    let mut seen = HashSet::new();
    document
        .parts
        .iter()
        .filter(|part| {
            board.part_ids.contains(&part.id) && seen.insert(part.definition_id.as_str())
        })
        .filter(|part| !assigned.contains(part.definition_id.as_str()))
        .filter_map(|part| {
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)?;
            Some(MechanicalProfileTargetChoice {
                definition_id: definition.id.clone(),
                name: definition.name.clone(),
                switch_profile: is_switch(document, &definition.id),
                imported_geometry: definition.kicad_source.is_some(),
            })
        })
        .collect()
}

pub fn process_targets(
    document: &ProjectDoc,
    configuration: &MechanicalConfiguration,
) -> Vec<MechanicalProcessTarget> {
    let target = |part_id: &str, name: String| MechanicalProcessTarget {
        part_id: part_id.to_owned(),
        name,
        default_thickness: match part_id {
            "plate-foam" => configuration.plate_foam_thickness,
            "bottom-foam" => configuration.bottom_foam_thickness,
            "bottom" => configuration.bottom_thickness,
            _ => configuration.plate_thickness,
        },
        process: configuration
            .part_processes
            .iter()
            .flatten()
            .find(|process| process.part_id == part_id)
            .cloned(),
    };
    let mut targets: Vec<_> = ["plate", "plate-foam", "bottom-foam", "bottom"]
        .into_iter()
        .map(|part_id| target(part_id, part_id.replace('-', " ")))
        .collect();
    targets.extend(document.parts.iter().map(|part| {
        let name = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
            .map_or_else(
                || part.definition_id.clone(),
                |definition| definition.name.clone(),
            );
        target(&part.id, format!("{} · {name}", part.reference))
    }));
    targets
}

/// Wide MX keys on the board that still need a stabilizer decision.
pub fn stabilizer_fits(
    document: &ProjectDoc,
    configuration: &MechanicalConfiguration,
) -> Vec<MechanicalStabilizerFit> {
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == configuration.board_id)
    else {
        return Vec::new();
    };
    let on_board = || {
        document
            .parts
            .iter()
            .filter(|part| board.part_ids.contains(&part.id))
    };
    let definition_of = |part: &Part| {
        document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
    };
    let imported: Vec<_> = on_board()
        .filter(|part| {
            definition_of(part)
                .and_then(|definition| definition.kicad_source.as_ref())
                .is_some_and(|source| source.source.contains("(footprint \"STAB_MX_"))
        })
        .collect();
    on_board()
        .filter_map(|part| {
            let definition = definition_of(part)?;
            let size = part.keycap.or(definition.keycap)?;
            let is_mx = definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source.to_lowercase() == "ceoloide/switch_mx");
            let covered = imported.iter().any(|stabilizer| {
                (stabilizer.pose.at.x - part.pose.at.x).hypot(stabilizer.pose.at.y - part.pose.at.y)
                    < 0.01
            });
            if !is_mx || size.x.max(size.y) < 37.0 || covered {
                return None;
            }
            let key_units = (((size.x.max(size.y) + 1.0) / 19.05 * 4.0).round()) / 4.0;
            let default = MechanicalStabilizerOverride {
                profile: None,
                part_id: part.id.clone(),
                kind: MechanicalStabilizerKind::PcbMount,
                units: if key_units <= 2.75 { 2.0 } else { key_units },
                rotation: Some(if size.y > size.x { 90.0 } else { 0.0 }),
            };
            let mut current = configuration
                .stabilizers
                .iter()
                .flatten()
                .find(|stabilizer| stabilizer.part_id == part.id)
                .cloned()
                .unwrap_or_else(|| default.clone());
            current.units = default.units;
            current.rotation = default.rotation;
            Some(MechanicalStabilizerFit {
                part_id: part.id.clone(),
                reference: part.reference.clone(),
                key_units,
                default,
                current,
            })
        })
        .collect()
}

/// Parts a critical fit can name; the resolved assembly's bodies win once available.
pub fn critical_fit_parts(
    configuration: &MechanicalConfiguration,
    resolved: &[MechanicalFitPart],
    is_resolved: bool,
) -> Vec<MechanicalFitPart> {
    if is_resolved {
        return resolved.to_vec();
    }
    [
        Some(("plate", "Plate")),
        (configuration.plate_foam_thickness > 0.0).then_some(("plate-foam", "Plate foam")),
        (configuration.bottom_foam_thickness > 0.0).then_some(("bottom-foam", "Bottom foam")),
        Some(("bottom", "Bottom")),
        (configuration.bottom_style == Some(MechanicalBottomStyle::Sheet)
            && configuration.middle_frame == Some(true))
        .then_some(("middle-frame", "Middle frame")),
    ]
    .into_iter()
    .flatten()
    .map(|(id, name)| MechanicalFitPart {
        id: id.to_owned(),
        name: name.to_owned(),
    })
    .collect()
}

/// Mounts hardware can link to; the resolved bodies win, otherwise the configured mounts.
pub fn hardware_mount_choices(
    configuration: &MechanicalConfiguration,
    resolved: &[MechanicalHardwareMount],
) -> Vec<MechanicalHardwareMount> {
    if !resolved.is_empty() {
        return resolved.to_vec();
    }
    let choice = |part_id: &str, part_name: &str, mount: &Mount| MechanicalHardwareMount {
        part_id: part_id.to_owned(),
        part_name: part_name.to_owned(),
        feature_id: mount.id.clone(),
        kind: mount.kind.clone(),
        x: mount.at.x,
        y: mount.at.y,
    };
    let (part_id, part_name) =
        if configuration.mount == boardstudio_model::model::MechanicalMount::Rigid {
            ("plate", "Plate")
        } else {
            ("bottom", "Bottom")
        };
    let suspension = (configuration.mount != boardstudio_model::model::MechanicalMount::Gasket)
        .then_some(&configuration.mounts)
        .into_iter()
        .flatten()
        .map(|mount| choice(part_id, part_name, mount));
    let closure = configuration
        .closure_mounts
        .iter()
        .flatten()
        .map(|mount| choice("bottom", "Bottom", mount));
    suspension.chain(closure).collect()
}

pub fn process_material_options(part_id: &str, method: &PlateMethod) -> &'static [&'static str] {
    match (part_id.ends_with("foam"), method) {
        (true, _) => &["EVA"],
        (_, PlateMethod::Printed) => &["PLA", "ABS"],
        (_, PlateMethod::Cnc) => &["Aluminium"],
        (_, PlateMethod::CutSheet) => &["Acrylic"],
        (_, PlateMethod::PcbFr4) => &["FR-4"],
    }
}

/// The thickness the dimensions behind one layer click edit, as `(field, label, value)`.
pub fn contextual_layer_fields(
    id: &str,
    c: &MechanicalConfiguration,
) -> Option<Vec<(MechanicalDimension, &'static str, f64)>> {
    use MechanicalDimension as D;
    Some(match id {
        "plate" => vec![(D::PlateThickness, "Plate thickness", c.plate_thickness)],
        "pcb" => vec![(D::PcbThickness, "PCB thickness", c.pcb_thickness)],
        "plate-foam" => vec![(D::PlateFoamThickness, "Plate foam", c.plate_foam_thickness)],
        "bottom-foam" => vec![(
            D::BottomFoamThickness,
            "Bottom foam",
            c.bottom_foam_thickness,
        )],
        "bottom" => vec![
            (D::BottomThickness, "Bottom thickness", c.bottom_thickness),
            (D::WallThickness, "Wall thickness", c.wall_thickness),
            (D::Clearance, "Clearance", c.clearance),
        ],
        "retainer" => vec![
            (D::WallThickness, "Wall thickness", c.wall_thickness),
            (D::Clearance, "Clearance", c.clearance),
        ],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn configuration() -> MechanicalConfiguration {
        serde_json::from_value(json!({
            "boardId": "board", "method": "printed", "mount": "rigid", "integratedPlateFrame": false,
            "bottomStyle": "sheet", "middleFrame": true, "plateThickness": 1.5, "plateFoamThickness": 3.0,
            "pcbThickness": 1.6, "bottomFoamThickness": 0.0, "batteryHeight": 0.0, "bottomThickness": 3.0,
            "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
            "mounts": [{"id": "m1", "at": {"x": 1.0, "y": 2.0}, "kind": "hole", "holeDiameter": 2.2}],
            "closureMounts": [], "profiles": []
        }))
        .unwrap()
    }

    fn assembly(thickness: f64) -> MechanicalAssembly {
        serde_json::from_value(json!({
            "revision": 1, "case": { "revision": 1, "bodies": [] },
            "stack": [{ "id": "plate", "z": 3.5, "thickness": thickness }],
            "suggestedMounts": [], "nominalPlateContours": [], "plateContours": [], "diagnostics": []
        }))
        .unwrap()
    }

    #[test]
    fn current_resolution_layers_precede_previous_display_layers() {
        let (previous, current) = (assembly(1.5), assembly(2.0));
        let rows = project_scene_rows(Some(&previous), Some(&current), false, true).unwrap();
        assert_eq!(
            (rows.layers[0].thickness, rows.layers[0].is_previous),
            (2.0, false)
        );
        let fallback = project_scene_rows(Some(&previous), None, false, true).unwrap();
        assert_eq!(
            (fallback.layers[0].thickness, fallback.layers[0].is_previous),
            (1.5, true)
        );
        assert!(project_scene_rows(None, None, false, false).is_none());
    }

    #[test]
    fn labels_and_contextual_fields_follow_the_layer_id() {
        assert_eq!(mechanical_layer_label("retainer", true), "Top case");
        assert_eq!(mechanical_layer_label("plate-foam", false), "Plate foam");
        let c = configuration();
        assert_eq!(contextual_layer_fields("bottom", &c).unwrap().len(), 3);
        assert_eq!(contextual_layer_fields("plate", &c).unwrap()[0].2, 1.5);
        assert!(contextual_layer_fields("gaskets", &c).is_none());
    }

    #[test]
    fn selectors_fall_back_to_the_configured_geometry_until_resolved() {
        let c = configuration();
        let ids: Vec<_> = critical_fit_parts(&c, &[], false)
            .into_iter()
            .map(|part| part.id)
            .collect();
        assert_eq!(ids, ["plate", "plate-foam", "bottom", "middle-frame"]);
        let mounts = hardware_mount_choices(&c, &[]);
        assert_eq!(
            (mounts[0].part_id.as_str(), mounts[0].feature_id.as_str()),
            ("plate", "m1")
        );
        let resolved = [MechanicalFitPart {
            id: "x".into(),
            name: "X".into(),
        }];
        assert_eq!(critical_fit_parts(&c, &resolved, true), resolved);
    }

    #[test]
    fn process_targets_default_to_the_layer_thickness_and_foam_is_eva() {
        let c = configuration();
        let doc = ProjectDoc::empty("d", "d");
        let targets = process_targets(&doc, &c);
        assert_eq!(
            targets
                .iter()
                .find(|t| t.part_id == "bottom")
                .unwrap()
                .default_thickness,
            3.0
        );
        assert!(targets.iter().all(|t| t.process.is_none()));
        assert_eq!(
            process_material_options("plate-foam", &PlateMethod::Cnc),
            ["EVA"]
        );
        assert_eq!(
            process_material_options("plate", &PlateMethod::Printed),
            ["PLA", "ABS"]
        );
        assert!(profile_targets(&doc, "board", Some(&c)).is_empty());
        assert!(stabilizer_fits(&doc, &c).is_empty());
    }
}
