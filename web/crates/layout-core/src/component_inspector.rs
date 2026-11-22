use super::companion_placement::{layout_relationship_summary, part_anchor_summary};
use super::inspector_types as inspector;
use boardstudio_application::{AcceptedSnapshot, EditResolver, ReadModel, Resolution};
use boardstudio_core::model::{
    EditOperation, HalfExceptionState, MirroredExceptionState, MirroredItemKind, PartKind,
    PartOutline,
};
use boardstudio_web_ui_model::tree as objects;
use std::collections::{BTreeMap, BTreeSet};

pub fn layout_component_inspector_projection(
    model: &ReadModel,
    selected: Option<&objects::ScopedTreeContext>,
    context_generation: u64,
    scope_generation: u64,
) -> Option<inspector::LayoutComponentInspectorProjection> {
    let selected = selected?;
    let objects::TreeContext::Component {
        part_id: Some(part_id),
        matrix_id,
        assembly_id,
        ..
    } = &selected.context
    else {
        return None;
    };
    if model.selected_part_ids.is_empty()
        || model.selected_part_ids.first() != Some(part_id)
        || !objects::selection_context_is_current(model, &selected.scope, &selected.context)
    {
        return None;
    }
    let snapshot = model.accepted.as_ref()?;
    let document = boardstudio_application::resolved_document(&snapshot.document);
    let document = document.as_ref();
    let part = document.parts.iter().find(|part| part.id == *part_id)?;
    let mirrored_half = mirrored_half_projection(document, part_id);
    if matrix_id.is_some() && assembly_id.is_some() && mirrored_half.is_none() {
        return None;
    }
    let definition = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)?;
    let board = document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)?;
    let board_part_ids: BTreeSet<_> = board.part_ids.iter().map(String::as_str).collect();
    let parts_by_id: BTreeMap<_, _> = document
        .parts
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();
    if model.selected_part_ids.iter().any(|selected_id| {
        !board_part_ids.contains(selected_id.as_str())
            || !parts_by_id.contains_key(selected_id.as_str())
    }) {
        return None;
    }
    let mut matrix_key_ids = BTreeSet::new();
    for matrix in &document.matrices {
        let Some(matrix_scene) = snapshot
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix.id)
        else {
            continue;
        };
        let matrix_part_ids: BTreeSet<_> = matrix.part_ids.iter().map(String::as_str).collect();
        matrix_key_ids.extend(
            matrix_scene
                .cells
                .iter()
                .filter(|cell| cell.enabled)
                .filter_map(|cell| cell.member_id.as_deref())
                .filter(|id| matrix_part_ids.contains(id)),
        );
    }
    let board_parts: Vec<_> = board
        .part_ids
        .iter()
        .filter_map(|id| {
            parts_by_id
                .get(id.as_str())
                .map(|part| inspector::BoardPartChoice {
                    id: part.id.clone(),
                    reference: part.reference.clone(),
                    matrix_key: matrix_key_ids.contains(part.id.as_str()),
                })
        })
        .collect();
    let is_matrix_member = document
        .matrices
        .iter()
        .any(|matrix| matrix.part_ids.contains(part_id));
    let layouts: Vec<_> = if is_matrix_member {
        Vec::new()
    } else {
        document
            .layouts
            .iter()
            .filter(|layout| layout.board_id == board.id)
            .map(|layout| inspector::LayoutChoice {
                id: layout.id.clone(),
                name: layout.name.clone(),
            })
            .collect()
    };
    let active_layout = document.layouts.iter().find(|layout| {
        layout.board_id == board.id
            && (layout.part_ids.contains(part_id)
                || document.matrices.iter().any(|matrix| {
                    layout.matrix_ids.contains(&matrix.id) && matrix.part_ids.contains(part_id)
                }))
    });
    let mut active_constraint = None;
    for selected_id in &model.selected_part_ids {
        match document.constraint_driving_part(selected_id) {
            Ok(Some(constraint)) => {
                active_constraint = Some(constraint.clone());
                break;
            }
            Ok(None) => {}
            Err(_) => return None,
        }
    }
    let mut relationship_summary =
        layout_relationship_summary(document, board, active_layout, Some(part));
    // Ticket 13: the anchor fact leads for anchored parts.
    if let Some(lead) = part_anchor_summary(document, part) {
        relationship_summary = format!("{lead} {relationship_summary}");
    }
    let definition_kind = match &definition.kind {
        PartKind::Switch => "switch",
        PartKind::Controller => "controller",
        PartKind::Connector => "connector",
        PartKind::Encoder => "encoder",
        PartKind::Passive => "passive",
        PartKind::Custom => "custom",
        PartKind::Utility => "utility",
    };
    Some(inspector::LayoutComponentInspectorProjection {
        owner: inspector::LayoutComponentInspectorOwner {
            scope: selected.scope.clone(),
            snapshot_token: snapshot.token,
            revision: document.revision,
            context_generation,
            scope_generation,
            part_id: part.id.clone(),
            selected_part_ids: model.selected_part_ids.clone(),
        },
        reference: part.reference.clone(),
        definition_name: definition.name.clone(),
        definition_kind: definition_kind.to_owned(),
        envelope_notice: definition.envelope_notice.clone(),
        locked: model.selected_part_ids.iter().any(|selected_id| {
            document
                .parts
                .iter()
                .find(|part| part.id == *selected_id)
                .is_none_or(|part| part.locked.unwrap_or(false))
        }),
        // Ticket 13: the Inspector edits board-space positions, so an anchored
        // part shows its evaluated pose (what the canvas shows); a detached
        // part keeps its stored half-frame pose.
        position: if part.anchor.is_some() {
            boardstudio_core::evaluated_part_pose(document, part).at
        } else {
            part.pose.at
        },
        selection_count: model.selected_part_ids.len(),
        layout_id: active_layout.map(|layout| layout.id.clone()),
        layouts,
        outline: part.outline.clone().unwrap_or_else(PartOutline::default),
        board_parts,
        active_constraint,
        relationship_summary,
        mirrored_half,
        part_mirror: part_mirror_projection(document, part_id, active_layout, is_matrix_member),
        anchor: part_anchor_projection(document, part),
        // Ticket 13: matrix members and companions follow the matrix refresh,
        // so they get no anchor vocabulary and no Pick form.
        anchor_choices: if is_matrix_member {
            Vec::new()
        } else {
            part_anchor_choices(document, active_layout)
        },
    })
}

/// Ticket 13: the part's column-vocabulary anchor for the Inspector: the
/// anchor key and Follow subset, the carried Spread/Stagger/Splay and the
/// local Offset/Rotation. `None` is a detached part; parts homed on a
/// layout without an anchor stay detached.
pub fn part_anchor_projection(
    document: &boardstudio_core::model::ProjectDoc,
    part: &boardstudio_core::model::Part,
) -> Option<inspector::PartAnchorProjection> {
    let anchor = part.anchor.as_ref()?;
    let matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == anchor.matrix_id)?;
    Some(inspector::PartAnchorProjection {
        matrix_id: anchor.matrix_id.clone(),
        matrix_name: matrix.name.clone().unwrap_or(matrix.id.clone()),
        row: anchor.row,
        column: anchor.column,
        follow: anchor.follow,
        spread: anchor.spread,
        stagger: anchor.stagger,
        splay: anchor.splay,
        offset: anchor.offset,
        rotation: anchor.rotation,
    })
}

/// Ticket 13: the anchor-key choices for Pick: every matrix on the part's
/// layout with its live dimensions.
pub fn part_anchor_choices(
    document: &boardstudio_core::model::ProjectDoc,
    active_layout: Option<&boardstudio_core::model::Layout>,
) -> Vec<inspector::AnchorKeyChoice> {
    let Some(layout) = active_layout else {
        return Vec::new();
    };
    layout
        .matrix_ids
        .iter()
        .filter_map(|matrix_id| {
            document
                .matrices
                .iter()
                .find(|matrix| matrix.id == *matrix_id)
                .map(|matrix| inspector::AnchorKeyChoice {
                    matrix_id: matrix.id.clone(),
                    matrix_name: matrix.name.clone().unwrap_or(matrix.id.clone()),
                    rows: matrix.rows,
                    columns: matrix.columns,
                })
        })
        .collect()
}

/// Ticket 12: the "Mirror on both halves" toggle for an independent part on a
/// stored-gap half. `Some(true)` while a live Part correspondence mirrors it;
/// `Some(false)` while it stays local. Members keep the exception buttons.
pub fn part_mirror_projection(
    document: &boardstudio_core::model::ProjectDoc,
    part_id: &str,
    active_layout: Option<&boardstudio_core::model::Layout>,
    is_matrix_member: bool,
) -> Option<bool> {
    if is_matrix_member {
        return None;
    }
    let layout = active_layout?;
    boardstudio_core::layout_half_offset(document, &layout.id)?;
    // Ticket 12: Part correspondences live on the layout mirror links, next
    // to the matrices they mirror — not on the hardware halves. The toggle
    // reads mirrored only while both ends of the record still exist.
    let mirrored =
        boardstudio_core::part_correspondence(document, part_id).is_some_and(|(_, record)| {
            document
                .parts
                .iter()
                .any(|part| part.id == record.source_id)
                && document
                    .parts
                    .iter()
                    .any(|part| part.id == record.mirror_id)
        });
    Some(mirrored)
}

pub fn mirrored_half_projection(
    document: &boardstudio_core::model::ProjectDoc,
    part_id: &str,
) -> Option<inspector::MirroredHalfProjection> {
    let hardware = document.hardware.as_ref()?;
    for pair in &hardware.pairs {
        let Some(halves) = pair.halves.as_ref() else {
            continue;
        };
        let Some(link) = halves.correspondences.iter().find(|link| {
            link.kind == MirroredItemKind::Part
                && (link.source_id == part_id || link.mirror_id == part_id)
        }) else {
            continue;
        };
        let state = halves
            .exceptions
            .iter()
            .find(|exception| {
                exception.kind == MirroredItemKind::Part && exception.mirror_id == link.mirror_id
            })
            .map(|exception| match exception.state {
                MirroredExceptionState::Local => HalfExceptionState::Local,
                MirroredExceptionState::Omitted => HalfExceptionState::Omitted,
            })
            .unwrap_or(HalfExceptionState::Linked);
        return Some(inspector::MirroredHalfProjection {
            pair_id: pair.id.clone(),
            mirror_id: link.mirror_id.clone(),
            state,
        });
    }
    None
}

/// Resolve a linked part's exception against the accepted mirrored-half record.
pub fn mirrored_half_state_resolver(
    pair_id: String,
    mirror_id: String,
    state: HalfExceptionState,
) -> EditResolver {
    EditResolver::new(
        "layout-mirrored-half-state",
        move |accepted: &AcceptedSnapshot| {
            let Some(pair) = accepted
                .document
                .hardware
                .as_ref()
                .and_then(|hardware| hardware.pairs.iter().find(|pair| pair.id == pair_id))
            else {
                return Resolution::Retire("The mirrored pair no longer exists.".into());
            };
            let Some(halves) = pair.halves.as_ref() else {
                return Resolution::Retire("The mirrored pair no longer links items.".into());
            };
            if !halves
                .correspondences
                .iter()
                .any(|link| link.kind == MirroredItemKind::Part && link.mirror_id == mirror_id)
            {
                return Resolution::Retire("The linked part no longer exists.".into());
            }
            let current = halves
                .exceptions
                .iter()
                .find(|exception| {
                    exception.kind == MirroredItemKind::Part && exception.mirror_id == mirror_id
                })
                .map(|exception| match exception.state {
                    boardstudio_core::model::MirroredExceptionState::Local => {
                        HalfExceptionState::Local
                    }
                    boardstudio_core::model::MirroredExceptionState::Omitted => {
                        HalfExceptionState::Omitted
                    }
                })
                .unwrap_or(HalfExceptionState::Linked);
            if current == state {
                return Resolution::Unchanged;
            }
            Resolution::submit(
                vec![pair_id.clone(), mirror_id.clone()],
                EditOperation::SetHalfException {
                    pair_id: pair_id.clone(),
                    kind: MirroredItemKind::Part,
                    mirror_id: mirror_id.clone(),
                    state,
                },
            )
        },
    )
}
