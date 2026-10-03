use super::catalogue::CatalogEntry;
use boardstudio_core::model::{
    AssemblyDefinition, Asset, Matrix, MatrixAssembly, MatrixCell, Part, PartDefinition, PartKind,
    PartOutline, ProjectDoc, Side, Vec2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum MatrixPresetId {
    MxSolder,
    MxHotswap,
    ChocSolder,
    ChocHotswap,
    MxRgb,
    ChocRgb,
    MxHotswapRgb,
    ChocHotswapRgb,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::presentation) enum SwitchOrientation {
    #[default]
    South,
    North,
}

#[derive(Clone, Copy)]
pub(super) struct Preset {
    pub(super) id: MatrixPresetId,
    pub(super) name: &'static str,
    pub(super) definition_id: &'static str,
    family: &'static str,
    hotswap: bool,
    led: bool,
}

pub(super) const PRESETS: [Preset; 8] = [
    Preset {
        id: MatrixPresetId::MxSolder,
        name: "MX Solder",
        definition_id: "ergogen:ceoloide/switch_mx",
        family: "mx",
        hotswap: false,
        led: false,
    },
    Preset {
        id: MatrixPresetId::MxHotswap,
        name: "MX Hotswap",
        definition_id: "ergogen:ceoloide/switch_mx",
        family: "mx",
        hotswap: true,
        led: false,
    },
    Preset {
        id: MatrixPresetId::ChocSolder,
        name: "Choc V1 Solder",
        definition_id: "ergogen:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: false,
        led: false,
    },
    Preset {
        id: MatrixPresetId::ChocHotswap,
        name: "Choc V1 Hotswap",
        definition_id: "ergogen:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: true,
        led: false,
    },
    Preset {
        id: MatrixPresetId::MxRgb,
        name: "MX RGB",
        definition_id: "ergogen:ceoloide/switch_mx",
        family: "mx",
        hotswap: false,
        led: true,
    },
    Preset {
        id: MatrixPresetId::ChocRgb,
        name: "Choc V1 RGB",
        definition_id: "ergogen:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: false,
        led: true,
    },
    Preset {
        id: MatrixPresetId::MxHotswapRgb,
        name: "MX Hotswap RGB",
        definition_id: "ergogen:ceoloide/switch_mx",
        family: "mx",
        hotswap: true,
        led: true,
    },
    Preset {
        id: MatrixPresetId::ChocHotswapRgb,
        name: "Choc V1 Hotswap RGB",
        definition_id: "ergogen:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: true,
        led: true,
    },
];

pub(super) fn name(id: MatrixPresetId) -> &'static str {
    preset(id).name
}

fn preset(id: MatrixPresetId) -> &'static Preset {
    PRESETS
        .iter()
        .find(|preset| preset.id == id)
        .expect("all preset ids are listed")
}

pub(super) async fn resolve(
    id: MatrixPresetId,
    entries: &[CatalogEntry],
    reversible: bool,
    orientation: SwitchOrientation,
    preview_definition: Option<PartDefinition>,
) -> Result<Vec<crate::parts_preview::PartsPreviewRecipeMember>, String> {
    let preset = preset(id);
    let find_source = |source: &str| {
        entries.iter().find(|entry| {
            entry
                .definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == source)
        })
    };
    let main_source = preset
        .definition_id
        .strip_prefix("ergogen:")
        .unwrap_or(preset.definition_id);
    let main = find_source(main_source)
        .ok_or_else(|| format!("Missing switch footprint: {main_source}"))?;
    let mut members = vec![member(
        "switch",
        preview_definition
            .as_ref()
            .filter(|definition| definition.id == main.definition.id)
            .unwrap_or(&main.definition),
        Vec2 { x: 0.0, y: 0.0 },
        0.0,
        Side::Front,
        switch_parameters(preset, reversible),
    )];
    if let Some(diode) = find_source("ceoloide/diode_tht_sod123") {
        members.push(member(
            "diode",
            &diode.definition,
            Vec2 { x: 7.4, y: -1.5 },
            90.0,
            Side::Back,
            serde_json::from_value(serde_json::json!({
                "side": "B", "reversible": reversible, "include_tht": false
            }))
            .expect("fixed diode parameters are valid"),
        ));
    }
    if preset.led {
        let led = find_source("ceoloide/led_sk6812mini-e")
            .ok_or_else(|| "Missing SK6812 MINI-E footprint".to_owned())?;
        members.push(member(
            "led",
            &led.definition,
            Vec2 {
                x: 0.0,
                y: if preset.family == "choc" { -4.7 } else { -4.75 },
            },
            180.0,
            Side::Back,
            serde_json::from_value(serde_json::json!({
                "side": "B", "reverse_mount": true, "reversible": reversible
            }))
            .expect("fixed LED parameters are valid"),
        ));
    }
    for member in &mut members {
        if let Some(generator) = member.definition.generator.as_mut() {
            generator
                .parameters
                .extend(member.generator_parameters.clone());
        }
        if orientation == SwitchOrientation::North {
            member.at.x = -member.at.x;
            member.at.y = -member.at.y;
            member.rotation = (member.rotation + 180.0) % 360.0;
        }
        member.definition =
            super::catalogue::normalize_generator_definition(member.definition.clone()).await?;
    }
    Ok(members)
}

fn switch_parameters(
    preset: &Preset,
    reversible: bool,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    let mut parameters: std::collections::BTreeMap<String, serde_json::Value> =
        serde_json::from_value(serde_json::json!({
            "hotswap": preset.hotswap,
            "solder": !preset.hotswap,
            "reversible": reversible,
            "side": "B",
            "include_keycap": true
        }))
        .expect("fixed switch parameters are valid");
    if preset.family == "choc" {
        parameters.extend(
            serde_json::from_value::<std::collections::BTreeMap<String, serde_json::Value>>(
                serde_json::json!({
                    "choc_v1_support": true,
                    "choc_v2_support": false,
                    "include_choc_v1_led_cutout_marks": true
                }),
            )
            .expect("fixed Choc parameters are valid"),
        );
    }
    parameters
}

fn member(
    id: &str,
    definition: &PartDefinition,
    at: Vec2,
    rotation: f64,
    side: Side,
    generator_parameters: std::collections::BTreeMap<String, serde_json::Value>,
) -> crate::parts_preview::PartsPreviewRecipeMember {
    crate::parts_preview::PartsPreviewRecipeMember {
        id: id.to_owned(),
        definition: definition.clone(),
        assets: Vec::new(),
        at,
        rotation,
        side,
        generator_parameters,
    }
}

/// Applies the saved-assembly recipe to every cell while retaining matrix
/// geometry and enabled state, matching the pinned `matrixWithAssembly` helper.
/// Definitions receive operation-specific snapshot identities so Core can
/// commit recipe edits without changing already-placed part snapshots.
pub(in crate::presentation) fn matrix_with_assembly(
    matrix: &Matrix,
    assembly: &AssemblyDefinition,
    catalogue_definitions: &[PartDefinition],
    document: &ProjectDoc,
    snapshot_nonce: &str,
) -> Result<(Matrix, Vec<PartDefinition>), String> {
    let Some(primary) = assembly.members.first() else {
        return Err("Name the assembly and add at least one member".into());
    };
    if primary.definition_id.is_none() {
        return Err(
            "The first assembly member must have a component definition to apply it to a matrix."
                .into(),
        );
    }
    if primary.pose.at.x != 0.0 || primary.pose.at.y != 0.0 || primary.side != Side::Front {
        return Err(
            "For matrix placement, keep the first member at the origin on the front".into(),
        );
    }
    if assembly.members.iter().any(|member| {
        !member.pose.at.x.is_finite()
            || !member.pose.at.y.is_finite()
            || !member.pose.rotation.is_finite()
            || member.models.iter().any(|model| {
                [
                    model.offset.x,
                    model.offset.y,
                    model.offset.z,
                    model.rotation.x,
                    model.rotation.y,
                    model.rotation.z,
                    model.scale.x,
                    model.scale.y,
                    model.scale.z,
                ]
                .into_iter()
                .any(|value| !value.is_finite())
                    || [model.scale.x, model.scale.y, model.scale.z]
                        .into_iter()
                        .any(|value| value <= 0.0)
            })
    }) {
        return Err(
            "Assembly positions and angles must be finite, and model scales must be positive."
                .into(),
        );
    }
    let member_ids = assembly
        .members
        .iter()
        .map(|member| member.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if member_ids.len() != assembly.members.len()
        || member_ids.iter().any(|id| id.trim().is_empty())
    {
        return Err("Assembly members must have unique, non-empty identities.".into());
    }
    let asset_exists = |id: &str| {
        document.assets.iter().any(|asset| asset.id == id)
            || crate::bundled_models::bundled_model(id).is_some()
            || id.starts_with("unresolved-model:")
    };
    let mut definitions = Vec::with_capacity(assembly.members.len());
    for member in &assembly.members {
        let source = member.definition_id.as_deref().and_then(|id| {
            document
                .definitions
                .iter()
                .chain(catalogue_definitions)
                .find(|definition| definition.id == id)
        });
        let mut definition = if let Some(source) = source {
            source.clone()
        } else if member.definition_id.is_none() {
            PartDefinition {
                hardware_profile: None,
                input_profile: None,
                id: format!("assembly-model-only:{}", member.id),
                name: format!("{} model", member.id),
                kind: PartKind::Custom,
                keycap: None,
                envelope_source: None,
                kicad_source: None,
                terminals: Default::default(),
                matrix_terminals: None,
                envelope_notice: None,
                courtyard: Vec::new(),
                pads: Vec::new(),
                models: Some(member.models.clone()),
                generator: None,
                mechanical_profile: None,
            }
        } else {
            return Err(format!(
                "Missing component definition: {}",
                member.definition_id.as_deref().unwrap_or_default()
            ));
        };
        if let Some(generator) = definition.generator.as_mut()
            && let Some(parameters) = member.parameters.as_ref()
        {
            generator.parameters.extend(parameters.clone());
        }
        if matches!(
            &member.model_mode,
            Some(boardstudio_core::model::AssemblyModelMode::Custom)
        ) || (member.model_mode.is_none() && !member.models.is_empty())
        {
            definition.models = Some(member.models.clone());
        }
        for model in definition.models.as_deref().unwrap_or_default() {
            if !asset_exists(&model.asset_id) {
                return Err(format!(
                    "The model asset '{}' is not available in this project.",
                    model.asset_id
                ));
            }
        }
        definition.id = format!(
            "{}/assembly-{}/definition/{}",
            matrix.id,
            safe_identity(snapshot_nonce),
            member.id
        );
        definitions.push(definition);
    }

    let primary_rotation = primary.pose.rotation;
    let angle = -primary_rotation.to_radians();
    let (sin, cos) = angle.sin_cos();
    let assemblies = assembly
        .members
        .iter()
        .skip(1)
        .enumerate()
        .map(|(index, member)| MatrixAssembly {
            id: member.id.clone(),
            definition_id: definitions[index + 1].id.clone(),
            offset: Vec2 {
                x: member.pose.at.x * cos - member.pose.at.y * sin,
                y: member.pose.at.x * sin + member.pose.at.y * cos,
            },
            rotation: Some(member.pose.rotation - primary_rotation),
            side: Some(member.side.clone()),
        })
        .collect::<Vec<_>>();
    let mut cells = Vec::with_capacity((matrix.rows * matrix.columns) as usize);
    for row in 0..matrix.rows {
        for column in 0..matrix.columns {
            let previous = matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column);
            cells.push(MatrixCell {
                row,
                column,
                enabled: previous.is_none_or(|cell| cell.enabled),
                definition_id: Some(definitions[0].id.clone()),
                variant: previous.and_then(|cell| cell.variant.clone()),
                offset: previous.and_then(|cell| cell.offset),
                rotation: Some(
                    previous.and_then(|cell| cell.rotation).unwrap_or(0.0) + primary_rotation,
                ),
                assemblies: assemblies.clone(),
                assemblies_local: Some(true),
            });
        }
    }
    let mut matrix = matrix.clone();
    matrix.definition_id = definitions[0].id.clone();
    matrix.cells = cells;
    Ok((matrix, definitions))
}

/// Reproduces the React editor's selected-board placement as one accepted
/// project proposal. Each assembly member gets an immutable definition/part
/// snapshot; later recipe edits therefore cannot move existing placements.
pub(super) fn document_with_assembly(
    document: &ProjectDoc,
    assembly: &AssemblyDefinition,
    source_definitions: &[PartDefinition],
    draft_assets: &[Asset],
    board_id: &str,
    origin: Vec2,
    placement_id: &str,
) -> Result<(ProjectDoc, Vec<String>), String> {
    if assembly.members.is_empty() {
        return Err("Add at least one component before placing this assembly.".into());
    }
    if !origin.x.is_finite() || !origin.y.is_finite() {
        return Err("Assembly placement coordinates must be finite numbers.".into());
    }
    let Some(board_index) = document
        .boards
        .iter()
        .position(|board| board.id == board_id)
    else {
        return Err("The selected board is no longer available.".into());
    };
    let member_ids = assembly
        .members
        .iter()
        .map(|member| member.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if member_ids.len() != assembly.members.len()
        || member_ids.iter().any(|id| id.trim().is_empty())
    {
        return Err("Assembly members must have unique, non-empty identities.".into());
    }
    if assembly.members.iter().any(|member| {
        !member.pose.at.x.is_finite()
            || !member.pose.at.y.is_finite()
            || !member.pose.rotation.is_finite()
            || member.models.iter().any(|model| {
                [
                    model.offset.x,
                    model.offset.y,
                    model.offset.z,
                    model.rotation.x,
                    model.rotation.y,
                    model.rotation.z,
                    model.scale.x,
                    model.scale.y,
                    model.scale.z,
                ]
                .into_iter()
                .any(|value| !value.is_finite())
                    || [model.scale.x, model.scale.y, model.scale.z]
                        .into_iter()
                        .any(|value| value <= 0.0)
            })
    }) {
        return Err(
            "Assembly positions and angles must be finite, and model scales must be positive."
                .into(),
        );
    }

    let asset_exists = |id: &str| {
        document.assets.iter().any(|asset| asset.id == id)
            || draft_assets.iter().any(|asset| asset.id == id)
            || crate::bundled_models::bundled_model(id).is_some()
            || id.starts_with("unresolved-model:")
    };
    let mut next = document.clone();
    for asset in draft_assets {
        if let Some(existing) = next.assets.iter().find(|existing| existing.id == asset.id) {
            if existing != asset {
                return Err(format!(
                    "The model asset identity '{}' conflicts with the accepted project.",
                    asset.id
                ));
            }
        } else {
            next.assets.push(asset.clone());
        }
    }
    let mut part_ids = Vec::with_capacity(assembly.members.len());
    for member in &assembly.members {
        let source = member.definition_id.as_deref().and_then(|id| {
            document
                .definitions
                .iter()
                .chain(source_definitions)
                .find(|definition| definition.id == id)
        });
        let mut definition = if let Some(source) = source {
            source.clone()
        } else if member.definition_id.is_none() {
            PartDefinition {
                hardware_profile: None,
                input_profile: None,
                id: format!("assembly-model-only:{}", member.id),
                name: "Visual model".into(),
                kind: PartKind::Custom,
                keycap: None,
                envelope_source: None,
                kicad_source: None,
                terminals: Default::default(),
                matrix_terminals: None,
                envelope_notice: None,
                courtyard: Vec::new(),
                pads: Vec::new(),
                models: None,
                generator: None,
                mechanical_profile: None,
            }
        } else {
            return Err(format!(
                "Missing component definition: {}",
                member.definition_id.as_deref().unwrap_or_default()
            ));
        };
        if let Some(generator) = definition.generator.as_mut()
            && let Some(parameters) = member.parameters.as_ref()
        {
            generator.parameters.extend(parameters.clone());
        }
        if !member.models.is_empty()
            || matches!(
                &member.model_mode,
                Some(boardstudio_core::model::AssemblyModelMode::Custom)
            )
        {
            definition.models = Some(member.models.clone());
        }
        for model in definition.models.as_deref().unwrap_or_default() {
            if !asset_exists(&model.asset_id) {
                return Err(format!(
                    "The model asset '{}' is not available in this project.",
                    model.asset_id
                ));
            }
        }
        definition.id = format!("{placement_id}/definition/{}", member.id);
        let part_id = format!("{placement_id}/{}", member.id);
        if next
            .definitions
            .iter()
            .any(|existing| existing.id == definition.id)
            || next.parts.iter().any(|existing| existing.id == part_id)
        {
            return Err("The assembly placement identity is already in use.".into());
        }
        let reference = format!("{} {}", assembly.name, member.id);
        if next
            .parts
            .iter()
            .any(|existing| existing.reference == reference)
        {
            return Err(format!(
                "The component reference '{reference}' is already in use."
            ));
        }
        let mut properties = std::collections::BTreeMap::new();
        properties.insert(
            "assemblyId".into(),
            serde_json::Value::String(placement_id.to_owned()),
        );
        properties.insert(
            "visualOnly".into(),
            serde_json::Value::Bool(member.definition_id.is_none()),
        );
        next.definitions.push(definition.clone());
        next.parts.push(Part {
            keycap: None,
            outline: member.definition_id.is_none().then_some(PartOutline {
                excluded: true,
                ..Default::default()
            }),
            id: part_id.clone(),
            definition_id: definition.id,
            reference,
            pose: boardstudio_core::model::Pose2 {
                at: Vec2 {
                    x: origin.x + member.pose.at.x,
                    y: origin.y + member.pose.at.y,
                },
                rotation: member.pose.rotation,
            },
            side: member.side.clone(),
            locked: None,
            properties: Some(properties),
            generator_parameters: None,
        });
        part_ids.push(part_id);
    }
    let board = &mut next.boards[board_index];
    board.part_ids.extend(part_ids.iter().cloned());
    Ok((next, part_ids))
}

/// Retain unrelated accepted changes made while definition normalization was
/// in flight, and attach only this operation's assembly snapshots and assets.
pub(super) fn rebase_assembly_placement(
    latest: &ProjectDoc,
    proposal: &ProjectDoc,
    draft_assets: &[Asset],
    board_id: &str,
    placement_id: &str,
    part_ids: &[String],
) -> Result<ProjectDoc, String> {
    let mut next = latest.clone();
    for asset in draft_assets {
        if let Some(existing) = next.assets.iter().find(|existing| existing.id == asset.id) {
            if existing != asset {
                return Err(format!(
                    "The model asset identity '{}' conflicts with the accepted project.",
                    asset.id
                ));
            }
        } else {
            next.assets.push(asset.clone());
        }
    }

    let definition_prefix = format!("{placement_id}/definition/");
    for definition in proposal
        .definitions
        .iter()
        .filter(|definition| definition.id.starts_with(&definition_prefix))
    {
        if next
            .definitions
            .iter()
            .any(|existing| existing.id == definition.id)
        {
            return Err("The assembly placement definition identity is already in use.".into());
        }
        next.definitions.push(definition.clone());
    }
    for part_id in part_ids {
        let Some(part) = proposal.parts.iter().find(|part| part.id == *part_id) else {
            return Err("The assembly placement proposal is incomplete.".into());
        };
        if next.parts.iter().any(|existing| existing.id == *part_id)
            || next
                .parts
                .iter()
                .any(|existing| existing.reference == part.reference)
        {
            return Err("A placed assembly component identity is already in use.".into());
        }
        next.parts.push(part.clone());
    }
    let Some(board) = next.boards.iter_mut().find(|board| board.id == board_id) else {
        return Err("The selected board is no longer available.".into());
    };
    for part_id in part_ids {
        if !board.part_ids.contains(part_id) {
            board.part_ids.push(part_id.clone());
        }
    }
    Ok(next)
}

fn safe_identity(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}
