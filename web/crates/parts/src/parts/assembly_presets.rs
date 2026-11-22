use super::catalogue::CatalogEntry;
use boardstudio_model::model::{
    AssemblyDefinition, AssemblyMember, AssemblyModelMode, Asset, Matrix, MatrixAssembly,
    MatrixCell, Part, PartDefinition, PartKind, PartOutline, ProjectDoc, Side, Vec2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixPresetId {
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
pub enum SwitchOrientation {
    #[default]
    South,
    North,
}

#[derive(Clone, Copy)]
pub struct Preset {
    pub id: MatrixPresetId,
    pub name: &'static str,
    pub definition_id: &'static str,
    family: &'static str,
    hotswap: bool,
    led: bool,
}

pub const PRESETS: [Preset; 8] = [
    Preset {
        id: MatrixPresetId::MxSolder,
        name: "MX Solder",
        definition_id: "generator:ceoloide/switch_mx",
        family: "mx",
        hotswap: false,
        led: false,
    },
    Preset {
        id: MatrixPresetId::MxHotswap,
        name: "MX Hotswap",
        definition_id: "generator:ceoloide/switch_mx",
        family: "mx",
        hotswap: true,
        led: false,
    },
    Preset {
        id: MatrixPresetId::ChocSolder,
        name: "Choc V1 Solder",
        definition_id: "generator:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: false,
        led: false,
    },
    Preset {
        id: MatrixPresetId::ChocHotswap,
        name: "Choc V1 Hotswap",
        definition_id: "generator:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: true,
        led: false,
    },
    Preset {
        id: MatrixPresetId::MxRgb,
        name: "MX RGB",
        definition_id: "generator:ceoloide/switch_mx",
        family: "mx",
        hotswap: false,
        led: true,
    },
    Preset {
        id: MatrixPresetId::ChocRgb,
        name: "Choc V1 RGB",
        definition_id: "generator:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: false,
        led: true,
    },
    Preset {
        id: MatrixPresetId::MxHotswapRgb,
        name: "MX Hotswap RGB",
        definition_id: "generator:ceoloide/switch_mx",
        family: "mx",
        hotswap: true,
        led: true,
    },
    Preset {
        id: MatrixPresetId::ChocHotswapRgb,
        name: "Choc V1 Hotswap RGB",
        definition_id: "generator:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: true,
        led: true,
    },
];

pub fn name(id: MatrixPresetId) -> &'static str {
    preset(id).name
}

fn preset(id: MatrixPresetId) -> &'static Preset {
    PRESETS
        .iter()
        .find(|preset| preset.id == id)
        .expect("all preset ids are listed")
}

pub async fn resolve(
    id: MatrixPresetId,
    entries: &[CatalogEntry],
    reversible: bool,
    orientation: SwitchOrientation,
    preview_definition: Option<PartDefinition>,
) -> Result<Vec<crate::parts_preview::PartsPreviewRecipeMember>, String> {
    let preset = preset(id);
    let mut members = recipe_members(
        preset,
        |source| {
            entries
                .iter()
                .find(|entry| {
                    entry
                        .definition
                        .generator
                        .as_ref()
                        .is_some_and(|generator| generator.source == source)
                })
                .map(|entry| (*entry.definition).clone())
        },
        reversible,
        orientation,
        preview_definition,
    )?;
    for member in &mut members {
        if let Some(generator) = member.definition.generator.as_mut() {
            generator
                .parameters
                .extend(member.generator_parameters.clone());
        }
        member.definition =
            super::catalogue::normalize_generator_definition(member.definition.clone()).await?;
    }
    Ok(members)
}

/// Resolve the library recipe used to start a customizable assembly. It uses
/// the same preset member construction as preview, but the reference creates a
/// single-sided assembly from library definitions even in reversible layouts.
pub fn customization_recipe(
    id: MatrixPresetId,
    definitions: &[PartDefinition],
    orientation: SwitchOrientation,
) -> Result<Vec<crate::parts_preview::PartsPreviewRecipeMember>, String> {
    recipe_members(
        preset(id),
        |source| {
            definitions
                .iter()
                .find(|definition| {
                    definition
                        .generator
                        .as_ref()
                        .is_some_and(|generator| generator.source == source)
                })
                .cloned()
        },
        false,
        orientation,
        None,
    )
}

fn recipe_members(
    preset: &Preset,
    mut find_definition: impl FnMut(&str) -> Option<PartDefinition>,
    reversible: bool,
    orientation: SwitchOrientation,
    preview_definition: Option<PartDefinition>,
) -> Result<Vec<crate::parts_preview::PartsPreviewRecipeMember>, String> {
    let main_source = preset
        .definition_id
        .strip_prefix("generator:")
        .unwrap_or(preset.definition_id);
    let main = find_definition(main_source)
        .ok_or_else(|| format!("Missing switch footprint: {main_source}"))?;
    let main = preview_definition
        .filter(|definition| definition.id == main.id)
        .unwrap_or(main);
    let mut members = vec![member(
        "switch",
        &main,
        Vec2 { x: 0.0, y: 0.0 },
        0.0,
        Side::Front,
        switch_parameters(preset, reversible),
    )];
    if let Some(diode) = find_definition("ceoloide/diode_tht_sod123") {
        members.push(member(
            "diode",
            &diode,
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
        let led = find_definition("ceoloide/led_sk6812mini-e")
            .ok_or_else(|| "Missing SK6812 MINI-E footprint".to_owned())?;
        members.push(member(
            "led",
            &led,
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
        if orientation == SwitchOrientation::North {
            member.at.x = -member.at.x;
            member.at.y = -member.at.y;
            member.rotation = (member.rotation + 180.0) % 360.0;
        }
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
pub fn matrix_with_assembly(
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
    validate_members(assembly, document, &[])?;
    let nonce = safe_identity(snapshot_nonce);
    let definitions = assembly
        .members
        .iter()
        .map(|member| {
            let mut definition = member_definition(member, document, catalogue_definitions, &[])?;
            definition.id = format!("{}/assembly-{nonce}/definition/{}", matrix.id, member.id);
            Ok(definition)
        })
        .collect::<Result<Vec<_>, String>>()?;

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
                shift: None,
                bend: None,
                deleted: false,
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
pub fn document_with_assembly(
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
    validate_members(assembly, document, draft_assets)?;
    let mut next = document.clone();
    merge_assets(&mut next, draft_assets)?;
    let mut part_ids = Vec::with_capacity(assembly.members.len());
    for member in &assembly.members {
        let mut definition = member_definition(member, document, source_definitions, draft_assets)?;
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
            pose: boardstudio_model::model::Pose2 {
                at: Vec2 {
                    x: origin.x + member.pose.at.x,
                    y: origin.y + member.pose.at.y,
                },
                rotation: member.pose.rotation,
            },
            side: member.side.clone(),
            anchor: None,
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
pub fn rebase_assembly_placement(
    latest: &ProjectDoc,
    proposal: &ProjectDoc,
    draft_assets: &[Asset],
    board_id: &str,
    placement_id: &str,
    part_ids: &[String],
) -> Result<ProjectDoc, String> {
    let mut next = latest.clone();
    merge_assets(&mut next, draft_assets)?;

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

pub(super) fn merge_assets(next: &mut ProjectDoc, assets: &[Asset]) -> Result<(), String> {
    for asset in assets {
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

    Ok(())
}

fn asset_exists(document: &ProjectDoc, assets: &[Asset], id: &str) -> bool {
    document
        .assets
        .iter()
        .chain(assets)
        .any(|asset| asset.id == id)
        || crate::bundled_models::bundled_model(id).is_some()
        || id.starts_with("unresolved-model:")
}

pub(super) fn validate_members(
    assembly: &AssemblyDefinition,
    document: &ProjectDoc,
    assets: &[Asset],
) -> Result<(), String> {
    let mut ids = std::collections::BTreeSet::new();
    for member in &assembly.members {
        if member.id.trim().is_empty() || !ids.insert(&member.id) {
            return Err("Assembly members must have unique, non-empty identities.".into());
        }
        if ![member.pose.at.x, member.pose.at.y, member.pose.rotation]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err("Assembly member positions and angles must be finite.".into());
        }
        for model in &member.models {
            let scale = model.scale;
            let coordinates = [model.offset, model.rotation, scale];
            if !coordinates
                .into_iter()
                .flat_map(|point| [point.x, point.y, point.z])
                .all(f64::is_finite)
                || [scale.x, scale.y, scale.z]
                    .into_iter()
                    .any(|value| value <= 0.0)
            {
                return Err("Model positions and rotations must be finite, and model scales must be positive.".into());
            }
            if !asset_exists(document, assets, &model.asset_id) {
                return Err(format!(
                    "The model asset '{}' is not available in this project.",
                    model.asset_id
                ));
            }
        }
    }
    Ok(())
}

fn member_definition(
    member: &AssemblyMember,
    document: &ProjectDoc,
    catalogue: &[PartDefinition],
    assets: &[Asset],
) -> Result<PartDefinition, String> {
    let mut definition = match member.definition_id.as_deref() {
        Some(id) => document
            .definitions
            .iter()
            .chain(catalogue)
            .find(|definition| definition.id == id)
            .cloned()
            .ok_or_else(|| format!("Missing component definition: {id}"))?,
        None => model_only_definition(member),
    };
    if let (Some(generator), Some(parameters)) = (&mut definition.generator, &member.parameters) {
        generator.parameters.extend(parameters.clone());
    }
    if member.model_mode == Some(AssemblyModelMode::Custom) || !member.models.is_empty() {
        definition.models = Some(member.models.clone());
    }
    for model in definition.models.as_deref().unwrap_or_default() {
        if !asset_exists(document, assets, &model.asset_id) {
            return Err(format!(
                "The model asset '{}' is not available in this project.",
                model.asset_id
            ));
        }
    }
    Ok(definition)
}

pub(super) fn model_only_definition(member: &AssemblyMember) -> PartDefinition {
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
