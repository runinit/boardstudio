//! Pure placement planning: part construction, snap policy and atomic document
//! proposals. Compiles natively so its unit tests run under `cargo test`; the browser
//! hook in `super::hook` is the only part that touches the canvas DOM.
use boardstudio_model::model::{
    EditOperation, OutlineFeature, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Side, Vec2,
};
use boardstudio_web_ui_model::state::LayoutSnapSettings;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PlacementIntent {
    #[default]
    Existing,
    LinkedCatalogueDefault,
    MirrorOnly,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PendingPart {
    pub definition: PartDefinition,
    pub part: Part,
    pub at: Vec2,
}

pub fn controller_part(
    definition: PartDefinition,
    part_id: String,
    reference: String,
    at: Vec2,
) -> Option<PendingPart> {
    matches!(definition.kind, PartKind::Controller)
        .then(|| component_part(definition, part_id, reference, at))
}

pub fn component_part(
    definition: PartDefinition,
    part_id: String,
    reference: String,
    at: Vec2,
) -> PendingPart {
    PendingPart {
        part: Part {
            keycap: None,
            outline: None,
            id: part_id,
            definition_id: definition.id.clone(),
            reference,
            pose: Pose2 { at, rotation: 0.0 },
            side: Side::Front,
            // Ticket 13: drops start detached; Core anchors them to the
            // nearest key of the drop half on AddPart.
            anchor: None,
            locked: None,
            properties: None,
            generator_parameters: definition
                .generator
                .as_ref()
                .map(|generator| generator.parameters.clone()),
        },
        definition,
        at,
    }
}

pub(crate) fn placement_source_definition(
    accepted: &ProjectDoc,
    module_definition_id: &str,
    source: PartDefinition,
) -> PartDefinition {
    if !accepted
        .definitions
        .iter()
        .any(|definition| definition.id == source.id && definition != &source)
    {
        return source;
    }

    let base_id = format!(
        "module-source/{}/{}:{}/{}",
        module_definition_id.len(),
        module_definition_id,
        source.id.len(),
        source.id
    );
    for suffix in 0u32.. {
        let id = if suffix == 0 {
            base_id.clone()
        } else {
            format!("{base_id}/{suffix}")
        };
        let mut candidate = source.clone();
        candidate.id = id;
        if let Some(profile) = &mut candidate.mechanical_profile {
            profile.definition_id = candidate.id.clone();
        }
        match accepted
            .definitions
            .iter()
            .find(|definition| definition.id == candidate.id)
        {
            Some(existing) if existing == &candidate => return candidate,
            Some(_) => continue,
            None => return candidate,
        }
    }
    unreachable!("A finite project cannot exhaust component definition identities")
}

pub fn update_pending_part(pending: &mut PendingPart, at: Vec2) {
    pending.at = at;
    pending.part.pose.at = at;
}

pub fn canvas_world_center(
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    camera_pan: Vec2,
) -> Vec2 {
    Vec2 {
        x: (min_x + max_x) * 0.5 + camera_pan.x,
        y: (min_y + max_y) * 0.5 + camera_pan.y,
    }
}

pub fn pointer_release_commits(button: i16) -> bool {
    button == 0
}

pub(crate) fn document_with_pending_definition(
    document: &ProjectDoc,
    definition: &PartDefinition,
) -> ProjectDoc {
    let mut snap_document = document.clone();
    if !snap_document
        .definitions
        .iter()
        .any(|existing| existing.id == definition.id)
    {
        snap_document.definitions.push(definition.clone());
    }
    snap_document
}

/// Snap-unit pitch: the selected matrix pitch, or 19.05 mm when none is selected
/// (ticket 08 decision 1).
pub(crate) fn snap_pitch(pitch: Option<Vec2>) -> Vec2 {
    pitch.unwrap_or(Vec2 { x: 19.05, y: 19.05 })
}

pub(crate) fn grid_snap_point(at: Vec2, snap_fraction: f64, pitch: Option<Vec2>) -> Vec2 {
    if snap_fraction == 0.0 {
        return at;
    }
    let pitch = snap_pitch(pitch);
    let step = if snap_fraction.is_finite() {
        if snap_fraction < 0.0 {
            -snap_fraction
        } else {
            pitch.x.max(0.001) * snap_fraction
        }
    } else {
        pitch.x.max(0.001) * 0.25
    }
    .max(0.001);
    let step_y = if snap_fraction.is_finite() {
        if snap_fraction < 0.0 {
            -snap_fraction
        } else {
            pitch.y.max(0.001) * snap_fraction
        }
    } else {
        pitch.y.max(0.001) * 0.25
    }
    .max(0.001);
    Vec2 {
        x: (at.x / step).round() * step,
        y: (at.y / step_y).round() * step_y,
    }
}

pub struct PlacementSnapOptions {
    pub snap_fraction: f64,
    pub geometry_snap: bool,
    pub gap: Option<f64>,
    pub free: bool,
    /// Selected matrix pitch; `None` keeps the 19.05 mm default when nothing is
    /// selected (ticket 08 decision 1: snap units are pitch x snap fraction).
    pub pitch: Option<Vec2>,
}

#[cfg(target_arch = "wasm32")]
pub fn placement_gap(settings: &LayoutSnapSettings) -> Option<f64> {
    crate::objects::gesture_snap_inputs(settings, None, None).gap
}

/// Native equivalent of `gesture_snap_inputs(settings, None, None).gap`: with no matrix
/// fallback the gap is the parsed override (or the 1 mm default) when both snap toggles are on.
#[cfg(not(target_arch = "wasm32"))]
pub fn placement_gap(settings: &LayoutSnapSettings) -> Option<f64> {
    if settings.geometry_snap && settings.gap_snap {
        settings
            .gap_override
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .or(Some(1.0))
    } else {
        None
    }
}

pub fn snap_placement_at(
    document: &ProjectDoc,
    board_id: &str,
    pending: &PendingPart,
    at: Vec2,
    options: PlacementSnapOptions,
) -> Vec2 {
    if options.free {
        return at;
    }
    // Ticket 08 decision 1: snap units are the selected pitch x the snap fraction;
    // 19.05 mm applies only when no matrix is selected.
    let grid = grid_snap_point(at, options.snap_fraction, options.pitch);
    if !options.geometry_snap {
        return grid;
    }
    let mut moving = pending.part.clone();
    moving.pose.at = at;
    boardstudio_application::interactions::snap_part_in_board(
        document,
        board_id,
        &moving,
        2.0,
        options.gap,
    )
    .map_or(grid, |guide| guide.at)
}

pub fn placement_operation(
    accepted: &ProjectDoc,
    board_id: &str,
    definition: &PartDefinition,
    part: &Part,
    layout_id: Option<&str>,
) -> Result<EditOperation, String> {
    if part.definition_id != definition.id || part.id.is_empty() || part.reference.is_empty() {
        return Err("The prepared component placement is incomplete.".into());
    }
    if accepted.parts.iter().any(|existing| existing.id == part.id) {
        return Err("The component placement identity already exists.".into());
    }
    let board_index = accepted
        .boards
        .iter()
        .position(|board| board.id == board_id)
        .ok_or_else(|| "The selected board is no longer available.".to_string())?;
    if accepted
        .parts
        .iter()
        .any(|existing| existing.reference == part.reference)
    {
        return Err("The component reference is already in use.".into());
    }

    let mut proposed = accepted.clone();
    match proposed
        .definitions
        .iter()
        .find(|existing| existing.id == definition.id)
    {
        Some(existing) if existing != definition => {
            return Err("The selected component definition changed before placement.".into());
        }
        Some(_) => {}
        None => proposed.definitions.push(definition.clone()),
    }
    proposed.parts.push(part.clone());
    let board = &mut proposed.boards[board_index];
    if !board.part_ids.contains(&part.id) {
        board.part_ids.push(part.id.clone());
    }
    let board_outlines = board
        .outline_ids
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    for outline in &mut proposed.outline {
        if let OutlineFeature::PartEnvelope { id, part_ids, .. } = outline
            && board_outlines.contains(id)
            && !part_ids.contains(&part.id)
        {
            part_ids.push(part.id.clone());
        }
    }
    if let Some(layout_id) = layout_id {
        let layout = proposed
            .layouts
            .iter_mut()
            .find(|layout| layout.id == layout_id && layout.board_id == board_id)
            .ok_or_else(|| {
                "The selected layout is no longer available on this board.".to_string()
            })?;
        if !layout.part_ids.contains(&part.id) {
            layout.part_ids.push(part.id.clone());
        }
    }
    Ok(EditOperation::ReplaceDocument {
        document: Box::new(proposed),
    })
}

fn unique_native_pair_for_board<'a>(
    document: &'a ProjectDoc,
    board_id: &str,
    mirror_only: bool,
) -> Result<Option<&'a boardstudio_model::model::MechanicalMirrorPair>, String> {
    let Some(hardware) = document.hardware.as_ref() else {
        return Ok(None);
    };
    let mut matching_pairs = hardware.pairs.iter().filter(|pair| {
        pair.halves.as_ref().is_some_and(|halves| {
            if mirror_only {
                halves.mirror_board_id == board_id
            } else {
                halves.source_board_id == board_id || halves.mirror_board_id == board_id
            }
        })
    });
    let Some(pair) = matching_pairs.next() else {
        return Ok(None);
    };
    if matching_pairs.next().is_some() {
        return Err("The selected board has ambiguous mirrored-half ownership.".into());
    }
    let Some(halves) = pair.halves.as_ref() else {
        return Ok(None);
    };
    if halves.source_board_id == halves.mirror_board_id {
        return Err("The mirrored halves do not identify distinct boards.".into());
    }
    if [
        halves.source_board_id.as_str(),
        halves.mirror_board_id.as_str(),
    ]
    .into_iter()
    .any(|id| {
        document
            .boards
            .iter()
            .filter(|board| board.id == id)
            .count()
            != 1
    }) {
        return Err("The selected pair no longer owns unique board records.".into());
    }
    Ok(Some(pair))
}

pub(crate) fn placement_operation_for_intent(
    accepted: &ProjectDoc,
    board_id: &str,
    definition: &PartDefinition,
    part: &Part,
    layout_id: Option<&str>,
    intent: PlacementIntent,
) -> Result<EditOperation, String> {
    match intent {
        PlacementIntent::Existing => {
            placement_operation(accepted, board_id, definition, part, layout_id)
        }
        PlacementIntent::LinkedCatalogueDefault => {
            match unique_native_pair_for_board(accepted, board_id, false)? {
                Some(_) => {}
                None => {
                    return placement_operation(accepted, board_id, definition, part, layout_id);
                }
            }
            Ok(EditOperation::AddPart {
                part: part.clone(),
                board_id: Some(board_id.to_owned()),
                definition: Some(definition.clone()),
                layout_id: layout_id.map(str::to_owned),
            })
        }
        PlacementIntent::MirrorOnly => {
            if unique_native_pair_for_board(accepted, board_id, true)?.is_none() {
                return Err("The selected board is no longer the unique mirror half.".into());
            }
            Ok(EditOperation::AddMirrorOnlyPart {
                part: part.clone(),
                board_id: board_id.to_owned(),
                definition: Some(definition.clone()),
                layout_id: layout_id.map(str::to_owned),
            })
        }
    }
}

pub fn next_component_reference(document: &ProjectDoc, kind: &PartKind) -> String {
    let prefix = match kind {
        PartKind::Switch => 'S',
        PartKind::Controller => 'U',
        PartKind::Encoder => 'E',
        PartKind::Connector | PartKind::Passive | PartKind::Custom | PartKind::Utility => 'J',
    };
    let occupied = document
        .parts
        .iter()
        .filter_map(|part| part.reference.strip_prefix(prefix))
        .filter_map(|number| number.parse::<u32>().ok())
        .collect::<std::collections::BTreeSet<_>>();
    let next = (1..=u32::MAX)
        .find(|candidate| !occupied.contains(candidate))
        .unwrap_or(1);
    format!("{prefix}{next}")
}
