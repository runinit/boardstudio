//! Contextual Properties and Relationships for one accepted standalone Layout part.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{
    Constraint, ConstraintRelation, HalfExceptionState, MirrorAxis, OffsetAxes, PartOutline, Vec2,
};
use boardstudio_web_ui_shared::controls::BoardButton;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::JsCast;

type InspectorOwnerIdentity = (
    boardstudio_application::Scope,
    u64,
    u64,
    String,
    Vec<String>,
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutComponentInspectorOwnerKey {
    pub scope: Option<Scope>,
    pub workspace: &'static str,
    pub part_id: Option<String>,
    pub selected_part_ids: Vec<String>,
}

#[derive(Default)]
pub struct LayoutComponentInspectorLifetime {
    current: Cell<u64>,
    key: RefCell<Option<LayoutComponentInspectorOwnerKey>>,
}

impl LayoutComponentInspectorLifetime {
    pub fn update(&self, key: Option<LayoutComponentInspectorOwnerKey>) -> u64 {
        let mut current_key = self.key.borrow_mut();
        if *current_key != key {
            self.current.set(self.current.get().wrapping_add(1).max(1));
            *current_key = key;
        }
        self.current.get()
    }

    pub fn current_generation(&self) -> u64 {
        self.current.get()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutComponentInspectorOwner {
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub context_generation: u64,
    pub scope_generation: u64,
    pub part_id: String,
    pub selected_part_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutComponentInspectorProjection {
    pub owner: LayoutComponentInspectorOwner,
    pub reference: String,
    pub definition_name: String,
    pub definition_kind: String,
    pub envelope_notice: Option<String>,
    pub locked: bool,
    pub position: Vec2,
    pub selection_count: usize,
    pub layout_id: Option<String>,
    pub layouts: Vec<LayoutChoice>,
    pub outline: PartOutline,
    pub board_parts: Vec<BoardPartChoice>,
    pub active_constraint: Option<Constraint>,
    pub relationship_summary: String,
    pub mirrored_half: Option<MirroredHalfProjection>,
    /// Ticket 12: an independent part on a stored-gap half shows a
    /// "Mirror on both halves" toggle instead of the member exception
    /// buttons. `Some(true)` mirrors it today; `Some(false)` keeps it local.
    pub part_mirror: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredHalfProjection {
    pub pair_id: String,
    pub mirror_id: String,
    pub state: HalfExceptionState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutChoice {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardPartChoice {
    pub id: String,
    pub reference: String,
    /// A matrix key: a new offset from it defaults to its own axes.
    pub matrix_key: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutConstraintValues {
    Offset { offset: Vec2, rotation: f64 },
    PartOffset { offset: Vec2, rotation: f64 },
    Mirror { axis: MirrorAxis, coordinate: f64 },
    Framed { relation: ConstraintRelation },
}

/// A new offset from a matrix key follows the key's own axes; any other source uses the board's.
pub(crate) fn default_offset_axes(source_is_matrix_key: bool) -> OffsetAxes {
    if source_is_matrix_key {
        OffsetAxes::Part
    } else {
        OffsetAxes::Board
    }
}

/// The default axes for an offset from the board part `source_id`.
fn default_axes_for_source(board_parts: &[BoardPartChoice], source_id: &str) -> OffsetAxes {
    default_offset_axes(
        board_parts
            .iter()
            .any(|part| part.id == source_id && part.matrix_key),
    )
}

/// The value the Offset axes `<select>` uses for `axes`, and the inverse.
fn offset_axes_value(axes: OffsetAxes) -> &'static str {
    match axes {
        OffsetAxes::Board => "board",
        OffsetAxes::Part => "part",
    }
}

fn offset_axes_from_value(value: &str) -> OffsetAxes {
    if value == offset_axes_value(OffsetAxes::Part) {
        OffsetAxes::Part
    } else {
        OffsetAxes::Board
    }
}

/// What the Properties form shows for one part's layout constraint: the source part, the
/// offset axes and the Offset X / Y and rotation drafts. The form's first render, every
/// accepted-constraint resync and the reset all take their values from here.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OffsetDrafts {
    pub source_part_id: String,
    pub axes: OffsetAxes,
    pub offset_x: String,
    pub offset_y: String,
    pub rotation: String,
}

/// The drafts for `part_id` given the constraint that drives it, if any. A saved Framed
/// constraint keeps its endpoint source, even when that part is on the paired board. A legacy
/// constraint with an off-board source falls back to the first other part. Saved offset axes
/// and numbers are preserved; otherwise axes follow the source and numbers are zero.
pub(crate) fn offset_drafts(
    constraint: Option<&Constraint>,
    board_parts: &[BoardPartChoice],
    part_id: &str,
) -> OffsetDrafts {
    let source_part_id = constraint
        .filter(|constraint| {
            matches!(constraint, Constraint::Framed { .. })
                || board_parts
                    .iter()
                    .any(|part| part.id == constraint.source())
        })
        .map(Constraint::source)
        .map(str::to_owned)
        .or_else(|| {
            board_parts
                .iter()
                .find(|part| part.id != part_id)
                .map(|part| part.id.clone())
        })
        .unwrap_or_default();
    let axes = constraint
        .and_then(Constraint::offset_axes)
        .unwrap_or_else(|| default_axes_for_source(board_parts, &source_part_id));
    let (offset, rotation) = constraint
        .and_then(Constraint::offset_numbers)
        .unwrap_or((Vec2::default(), 0.0));
    OffsetDrafts {
        source_part_id,
        axes,
        offset_x: offset.x.to_string(),
        offset_y: offset.y.to_string(),
        rotation: rotation.to_string(),
    }
}

/// The constraint values the Save button submits for the chosen axes.
pub(crate) fn offset_values(
    axes: OffsetAxes,
    offset: Vec2,
    rotation: f64,
) -> LayoutConstraintValues {
    match axes {
        OffsetAxes::Board => LayoutConstraintValues::Offset { offset, rotation },
        OffsetAxes::Part => LayoutConstraintValues::PartOffset { offset, rotation },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentPositionAxis {
    X,
    Y,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PositionCommitTrigger {
    Enter,
    Blur,
}

#[derive(Clone)]
struct PositionEnterCommit {
    owner: LayoutComponentInspectorOwner,
    axis: ComponentPositionAxis,
    value: f64,
}

fn same_position_commit_owner(
    left: &LayoutComponentInspectorOwner,
    right: &LayoutComponentInspectorOwner,
) -> bool {
    left.scope == right.scope
        && left.context_generation == right.context_generation
        && left.scope_generation == right.scope_generation
        && left.part_id == right.part_id
        && left.selected_part_ids == right.selected_part_ids
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutComponentInspectorAction {
    SetPosition {
        owner: LayoutComponentInspectorOwner,
        axis: ComponentPositionAxis,
        value: f64,
        submitted_draft: String,
    },
    AssignLayout {
        owner: LayoutComponentInspectorOwner,
        layout_id: Option<String>,
    },
    SetOutline {
        owner: LayoutComponentInspectorOwner,
        outline: PartOutline,
        submitted_draft: String,
    },
    SetConstraint {
        owner: LayoutComponentInspectorOwner,
        source_part_id: String,
        values: LayoutConstraintValues,
    },
    RemoveConstraint {
        owner: LayoutComponentInspectorOwner,
        constraint_id: String,
    },
    /// Re-expresses the saved offset constraint on the other axes without moving a part.
    ConvertOffsetAxes {
        owner: LayoutComponentInspectorOwner,
        constraint_id: String,
        to: OffsetAxes,
    },
    NavigateElectrical {
        owner: LayoutComponentInspectorOwner,
    },
    SetMirroredHalfState {
        owner: LayoutComponentInspectorOwner,
        pair_id: String,
        mirror_id: String,
        state: HalfExceptionState,
    },
    SetPartMirrored {
        owner: LayoutComponentInspectorOwner,
        part_id: String,
        mirrored: bool,
    },
}

#[derive(Props, Clone, PartialEq)]
pub struct LayoutComponentInspectorProps {
    pub projection: LayoutComponentInspectorProjection,
    pub(in crate::presentation) pending_edits:
        Signal<super::super::layout_component_edits::LayoutComponentInspectorEdits>,
    pub(in crate::presentation) x: Signal<String>,
    pub(in crate::presentation) y: Signal<String>,
    pub(in crate::presentation) margin: Signal<String>,
    pub(in crate::presentation) x_failure: Signal<Option<String>>,
    pub(in crate::presentation) y_failure: Signal<Option<String>>,
    pub(in crate::presentation) margin_failure: Signal<Option<String>>,
    pub(in crate::presentation) error: Signal<Option<String>>,
    pub(in crate::presentation) layout_pending: Signal<bool>,
    pub(in crate::presentation) constraint_pending: Signal<bool>,
    pub(in crate::presentation) remove_pending: Signal<bool>,
    pub(in crate::presentation) mirrored_half_pending: Signal<bool>,
    pub on_action: EventHandler<LayoutComponentInspectorAction>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConstraintKind {
    Offset,
    Mirror,
    MirrorLine,
}

/// X/Y are unavailable when the part is locked or when a saved relationship on
/// this board drives it (the relationship would overwrite any typed value).
fn position_unavailable(projection: &LayoutComponentInspectorProjection) -> bool {
    projection.locked || projection.active_constraint.is_some()
}

fn position_input_disabled(locked: bool) -> bool {
    locked
}

fn position_commit_value(draft: &str, locked: bool) -> Option<f64> {
    if locked {
        return None;
    }
    draft.parse::<f64>().ok().filter(|value| value.is_finite())
}

#[cfg(test)]
mod offset_axes_tests {
    use super::{
        BoardPartChoice, OffsetDrafts, default_offset_axes, offset_axes_from_value,
        offset_axes_value, offset_drafts, offset_values,
    };
    use crate::presentation::inspector::LayoutConstraintValues;
    use boardstudio_core::model::{
        Constraint, ConstraintEndpoint, ConstraintRelation, MirrorAxis, OffsetAxes, Vec2,
    };

    fn offsets() -> (Constraint, Constraint, Constraint) {
        let board = Constraint::Offset {
            id: "c".into(),
            source_part_id: "a".into(),
            target_part_id: "b".into(),
            offset: Vec2 { x: 1.0, y: -4.0 },
            rotation: 5.0,
        };
        let part = Constraint::PartOffset {
            id: "c".into(),
            source_part_id: "a".into(),
            target_part_id: "b".into(),
            offset: Vec2 { x: 1.0, y: -4.0 },
            rotation: 5.0,
        };
        let mirror = Constraint::Mirror {
            id: "m".into(),
            source_part_id: "a".into(),
            target_part_id: "b".into(),
            axis: MirrorAxis::Vertical,
            coordinate: 0.0,
        };
        (board, part, mirror)
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn a_matrix_key_source_defaults_to_part_axes_and_any_other_source_to_board_axes() {
        assert_eq!(default_offset_axes(true), OffsetAxes::Part);
        assert_eq!(default_offset_axes(false), OffsetAxes::Board);
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn the_axes_select_value_round_trips_for_both_axes() {
        for axes in [OffsetAxes::Board, OffsetAxes::Part] {
            assert_eq!(offset_axes_from_value(offset_axes_value(axes)), axes);
        }
        assert_eq!(offset_axes_value(OffsetAxes::Board), "board");
        assert_eq!(offset_axes_value(OffsetAxes::Part), "part");
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn the_axes_choice_and_the_numbers_read_both_offset_kinds() {
        let (board, part, mirror) = offsets();
        assert_eq!(board.offset_axes(), Some(OffsetAxes::Board));
        assert_eq!(part.offset_axes(), Some(OffsetAxes::Part));
        assert_eq!(mirror.offset_axes(), None);
        let numbers = (Vec2 { x: 1.0, y: -4.0 }, 5.0);
        assert_eq!(board.offset_numbers(), Some(numbers));
        assert_eq!(part.offset_numbers(), Some(numbers));
        assert_eq!(mirror.offset_numbers(), None);
    }

    /// A key, a diode and a plain part, in board order.
    fn board() -> Vec<BoardPartChoice> {
        let part = |id: &str, matrix_key| BoardPartChoice {
            id: id.into(),
            reference: id.to_uppercase(),
            matrix_key,
        };
        vec![
            part("key", true),
            part("diode", false),
            part("plain", false),
        ]
    }

    fn zero_drafts(source: &str, axes: OffsetAxes) -> OffsetDrafts {
        OffsetDrafts {
            source_part_id: source.into(),
            axes,
            offset_x: "0".into(),
            offset_y: "0".into(),
            rotation: "0".into(),
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn a_saved_offset_shows_its_own_source_axes_and_numbers_for_either_kind() {
        let (board_axes, part_axes, _) = offsets();
        let saved = |axes| OffsetDrafts {
            source_part_id: "a".into(),
            axes,
            offset_x: "1".into(),
            offset_y: "-4".into(),
            rotation: "5".into(),
        };
        let parts = [
            BoardPartChoice {
                id: "a".into(),
                reference: "A".into(),
                matrix_key: true,
            },
            BoardPartChoice {
                id: "b".into(),
                reference: "B".into(),
                matrix_key: false,
            },
        ];
        // The source is a matrix key, whose default is part axes: the saved kind still wins.
        assert_eq!(
            offset_drafts(Some(&board_axes), &parts, "b"),
            saved(OffsetAxes::Board)
        );
        assert_eq!(
            offset_drafts(Some(&part_axes), &parts, "b"),
            saved(OffsetAxes::Part)
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn an_unconstrained_part_defaults_the_axes_by_its_first_other_part() {
        // The diode's first other part is the key; the key's is the diode. The default is
        // taken for the part now selected, not for the one the form first opened on.
        assert_eq!(
            offset_drafts(None, &board(), "diode"),
            zero_drafts("key", OffsetAxes::Part)
        );
        assert_eq!(
            offset_drafts(None, &board(), "key"),
            zero_drafts("diode", OffsetAxes::Board)
        );
        assert_eq!(
            offset_drafts(None, &board(), "plain"),
            zero_drafts("key", OffsetAxes::Part)
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn a_mirror_constraint_or_an_off_board_source_shows_the_default_and_zeros() {
        let (_, _, mirror) = offsets();
        assert_eq!(
            offset_drafts(Some(&mirror), &board(), "diode"),
            zero_drafts("key", OffsetAxes::Part)
        );
        // `a` is not on this board, so the form falls back to the first other part.
        let (off_board, _, _) = offsets();
        let drafts = offset_drafts(Some(&off_board), &board(), "key");
        assert_eq!(drafts.source_part_id, "diode");
        assert_eq!(
            drafts,
            OffsetDrafts {
                source_part_id: "diode".into(),
                axes: OffsetAxes::Board,
                offset_x: "1".into(),
                offset_y: "-4".into(),
                rotation: "5".into(),
            }
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn an_off_board_part_offset_keeps_saved_axes_instead_of_the_fallback_default() {
        let (_, off_board, _) = offsets();
        assert_eq!(
            offset_drafts(Some(&off_board), &board(), "key"),
            OffsetDrafts {
                source_part_id: "diode".into(),
                axes: OffsetAxes::Part,
                offset_x: "1".into(),
                offset_y: "-4".into(),
                rotation: "5".into(),
            }
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn an_off_board_framed_source_keeps_its_endpoint_axes_and_numbers() {
        let framed = Constraint::Framed {
            id: "framed-offset".into(),
            source: ConstraintEndpoint::Part {
                part_id: "paired-board-source".into(),
            },
            target: ConstraintEndpoint::Part {
                part_id: "key".into(),
            },
            relation: ConstraintRelation::PartOffset {
                offset: Vec2 { x: 1.0, y: -4.0 },
                rotation: 5.0,
            },
        };
        assert_eq!(
            offset_drafts(Some(&framed), &board(), "key"),
            OffsetDrafts {
                source_part_id: "paired-board-source".into(),
                axes: OffsetAxes::Part,
                offset_x: "1".into(),
                offset_y: "-4".into(),
                rotation: "5".into(),
            }
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn saving_builds_the_constraint_kind_the_axes_choice_names() {
        let offset = Vec2 { x: 2.0, y: 3.0 };
        assert_eq!(
            offset_values(OffsetAxes::Board, offset, 7.0),
            LayoutConstraintValues::Offset {
                offset,
                rotation: 7.0
            }
        );
        assert_eq!(
            offset_values(OffsetAxes::Part, offset, 7.0),
            LayoutConstraintValues::PartOffset {
                offset,
                rotation: 7.0
            }
        );
        assert!(super::position_input_disabled(true));
        assert!(!super::position_input_disabled(false));
        assert_eq!(super::position_commit_value("12.5", true), None);
        assert_eq!(super::position_commit_value("12.5", false), Some(12.5));
    }
}

#[component]
pub fn LayoutComponentInspector(props: LayoutComponentInspectorProps) -> Element {
    let projection = props.projection.clone();
    let owner = projection.owner.clone();
    // Selection lifetime is independent from the accepted snapshot. A newer
    // accepted revision must refresh action admission without discarding
    // unrelated dirty fields or the user's selected Inspector tab.
    let identity = (
        owner.scope.clone(),
        owner.context_generation,
        owner.scope_generation,
        owner.part_id.clone(),
        owner.selected_part_ids.clone(),
    );
    let capture_identity = (identity.clone(), owner.snapshot_token, owner.revision);
    let mut latest_capture = use_signal(|| owner.clone());
    use_effect(use_reactive((&capture_identity,), {
        let owner = owner.clone();
        move |_| latest_capture.set(owner.clone())
    }));
    let mut constraint_open = use_signal(|| projection.active_constraint.is_some());
    let mut x = props.x;
    let mut y = props.y;
    let mut margin = props.margin;
    let x_failure = props.x_failure;
    let y_failure = props.y_failure;
    let margin_failure = props.margin_failure;
    let mut constraint_kind = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Mirror { .. }) => ConstraintKind::Mirror,
        Some(Constraint::Framed {
            relation: ConstraintRelation::MirrorLine { .. },
            ..
        }) => ConstraintKind::MirrorLine,
        _ => ConstraintKind::Offset,
    });
    let drafts = offset_drafts(
        projection.active_constraint.as_ref(),
        &projection.board_parts,
        &owner.part_id,
    );
    let mut source_part_id = use_signal(|| drafts.source_part_id.clone());
    let mut offset_axes = use_signal(|| drafts.axes);
    let mut offset_x = use_signal(|| drafts.offset_x.clone());
    let mut offset_y = use_signal(|| drafts.offset_y.clone());
    let mut rotation = use_signal(|| drafts.rotation.clone());
    let mut mirror_axis = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Mirror { axis, .. }) => axis.clone(),
        _ => MirrorAxis::Vertical,
    });
    let mut mirror_coordinate = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Mirror { coordinate, .. }) => coordinate.to_string(),
        _ => "0".to_owned(),
    });
    let mut mirror_line_x = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Framed {
            relation: ConstraintRelation::MirrorLine { at, .. },
            ..
        }) => at.x.to_string(),
        _ => "0".to_owned(),
    });
    let mut mirror_line_y = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Framed {
            relation: ConstraintRelation::MirrorLine { at, .. },
            ..
        }) => at.y.to_string(),
        _ => "0".to_owned(),
    });
    let mut mirror_line_angle = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Framed {
            relation: ConstraintRelation::MirrorLine { angle, .. },
            ..
        }) => angle.to_string(),
        _ => "90".to_owned(),
    });
    let mut error = props.error;
    let pending = props.pending_edits.peek().clone();
    let last_owner = use_hook(|| Rc::new(RefCell::new(None::<LayoutComponentInspectorOwnerKey>)));
    let owner_is_live = {
        let mut previous = last_owner.borrow_mut();
        let changed = previous.as_ref().is_some_and(|old| {
            old != &LayoutComponentInspectorOwnerKey {
                scope: Some(owner.scope.clone()),
                workspace: "Layout",
                part_id: Some(owner.part_id.clone()),
                selected_part_ids: owner.selected_part_ids.clone(),
            }
        });
        *previous = Some(LayoutComponentInspectorOwnerKey {
            scope: Some(owner.scope.clone()),
            workspace: "Layout",
            part_id: Some(owner.part_id.clone()),
            selected_part_ids: owner.selected_part_ids.clone(),
        });
        !changed
    };
    pending.bind_one_shot(
        super::super::layout_component_edits::InspectorField::Layout,
        props.layout_pending,
    );
    pending.bind_one_shot(
        super::super::layout_component_edits::InspectorField::Constraint,
        props.constraint_pending,
    );
    pending.bind_one_shot(
        super::super::layout_component_edits::InspectorField::RemoveConstraint,
        props.remove_pending,
    );
    pending.bind_one_shot(
        super::super::layout_component_edits::InspectorField::MirroredHalfState,
        props.mirrored_half_pending,
    );
    // The member exception buttons and the independent-part toggle never show
    // together, so they share one pending slot.
    pending.bind_one_shot(
        super::super::layout_component_edits::InspectorField::PartMirror,
        props.mirrored_half_pending,
    );
    let mirrored_half_action = |state| {
        let mirrored_half = projection.mirrored_half.clone();
        let owner = latest_capture;
        let action = props.on_action;
        move |_| {
            if let Some(mirrored_half) = mirrored_half.as_ref() {
                action.call(LayoutComponentInspectorAction::SetMirroredHalfState {
                    owner: owner(),
                    pair_id: mirrored_half.pair_id.clone(),
                    mirror_id: mirrored_half.mirror_id.clone(),
                    state,
                });
            }
        }
    };

    let initial_position = projection.position;
    let initial_outline = projection.outline.clone();
    let initial_constraint = projection.active_constraint.clone();
    let has_initial_constraint = initial_constraint.is_some();
    let board_parts = projection.board_parts.clone();
    let part_id = owner.part_id.clone();
    use_effect(use_reactive((&identity,), {
        move |_| {
            constraint_open.set(has_initial_constraint);
            error.set(None);
        }
    }));
    let mut previous_x = use_signal(|| None::<(InspectorOwnerIdentity, f64)>);
    let accepted_x = (identity.clone(), initial_position.x);
    let identity_x = identity.clone();
    let pending_x = pending.clone();
    use_effect(use_reactive((&accepted_x,), move |((_, value),)| {
        let previous = previous_x.peek().clone();
        let clean = previous.as_ref().is_none_or(|(old_identity, old_value)| {
            old_identity != &identity_x || x.peek().as_str() == format!("{old_value:.2}")
        });
        if clean && !pending_x.is_pending(&super::super::layout_component_edits::InspectorField::X)
        {
            x.set(format!("{value:.2}"));
        }
        previous_x.set(Some((identity_x.clone(), value)));
    }));
    let mut previous_y = use_signal(|| None::<(InspectorOwnerIdentity, f64)>);
    let accepted_y = (identity.clone(), initial_position.y);
    let identity_y = identity.clone();
    let pending_y = pending.clone();
    use_effect(use_reactive((&accepted_y,), move |((_, value),)| {
        let previous = previous_y.peek().clone();
        let clean = previous.as_ref().is_none_or(|(old_identity, old_value)| {
            old_identity != &identity_y || y.peek().as_str() == format!("{old_value:.2}")
        });
        if clean && !pending_y.is_pending(&super::super::layout_component_edits::InspectorField::Y)
        {
            y.set(format!("{value:.2}"));
        }
        previous_y.set(Some((identity_y.clone(), value)));
    }));
    let mut previous_margin = use_signal(|| None::<(InspectorOwnerIdentity, Option<f64>)>);
    let accepted_margin = (identity.clone(), initial_outline.margin);
    let identity_margin = identity.clone();
    let pending_margin = pending.clone();
    use_effect(use_reactive((&accepted_margin,), move |((_, value),)| {
        let previous = previous_margin.peek().clone();
        let old_text = previous
            .as_ref()
            .map(|(_, value)| value.map(|value| value.to_string()).unwrap_or_default());
        let clean = previous
            .as_ref()
            .is_none_or(|(old_identity, _)| old_identity != &identity_margin)
            || old_text.as_deref() == Some(margin.peek().as_str());
        if clean
            && !pending_margin
                .is_pending(&super::super::layout_component_edits::InspectorField::Margin)
        {
            margin.set(value.map(|value| value.to_string()).unwrap_or_default());
        }
        previous_margin.set(Some((identity_margin.clone(), value)));
    }));
    let accepted_constraint = (identity.clone(), initial_constraint.clone());
    use_effect(use_reactive((&accepted_constraint,), {
        move |((_, constraint),)| {
            constraint_open.set(constraint.is_some());
            constraint_kind.set(match constraint.as_ref() {
                Some(Constraint::Mirror { .. }) => ConstraintKind::Mirror,
                Some(Constraint::Framed {
                    relation: ConstraintRelation::MirrorLine { .. },
                    ..
                }) => ConstraintKind::MirrorLine,
                _ => ConstraintKind::Offset,
            });
            let drafts = offset_drafts(constraint.as_ref(), &board_parts, &part_id);
            source_part_id.set(drafts.source_part_id);
            offset_axes.set(drafts.axes);
            offset_x.set(drafts.offset_x);
            offset_y.set(drafts.offset_y);
            rotation.set(drafts.rotation);
            match constraint {
                Some(Constraint::Mirror {
                    axis, coordinate, ..
                }) => {
                    mirror_axis.set(axis.clone());
                    mirror_coordinate.set(coordinate.to_string());
                }
                Some(Constraint::Framed {
                    relation: ConstraintRelation::MirrorLine { at, angle },
                    ..
                }) => {
                    mirror_line_x.set(at.x.to_string());
                    mirror_line_y.set(at.y.to_string());
                    mirror_line_angle.set(angle.to_string());
                }
                _ => {
                    mirror_axis.set(MirrorAxis::Vertical);
                    mirror_coordinate.set("0".to_owned());
                    mirror_line_x.set("0".to_owned());
                    mirror_line_y.set("0".to_owned());
                    mirror_line_angle.set("90".to_owned());
                }
            }
            error.set(None);
        }
    }));
    pending.bind_field(
        super::super::layout_component_edits::InspectorField::X,
        x,
        x_failure,
    );
    pending.bind_field(
        super::super::layout_component_edits::InspectorField::Y,
        y,
        y_failure,
    );
    pending.bind_field(
        super::super::layout_component_edits::InspectorField::Margin,
        margin,
        margin_failure,
    );
    let accepted_projection = projection.clone();
    pending.settle(owner_is_live, move |key| match key {
        super::super::layout_component_edits::InspectorField::X => {
            format!("{:.2}", accepted_projection.position.x)
        }
        super::super::layout_component_edits::InspectorField::Y => {
            format!("{:.2}", accepted_projection.position.y)
        }
        super::super::layout_component_edits::InspectorField::Margin => accepted_projection
            .outline
            .margin
            .map(|value| value.to_string())
            .unwrap_or_default(),
        _ => String::new(),
    });
    let retired_pending = pending.clone();
    use_drop(move || {
        retired_pending.settle(false, |_| String::new());
    });

    let last_enter_commit = use_hook(|| Rc::new(RefCell::new(None::<PositionEnterCommit>)));
    let commit_position: Rc<dyn Fn(ComponentPositionAxis, PositionCommitTrigger)> = {
        let latest_capture = latest_capture;
        let position = projection.position;
        let locked = position_unavailable(&projection);
        let action = props.on_action;
        let error = error;
        let last_enter_commit = last_enter_commit.clone();
        Rc::new(move |axis, trigger| {
            if position_input_disabled(locked) {
                last_enter_commit.borrow_mut().take();
                return;
            }
            let mut error = error;
            let (draft, current) = match axis {
                ComponentPositionAxis::X => (x(), position.x),
                ComponentPositionAxis::Y => (y(), position.y),
            };
            let Some(value) = position_commit_value(&draft, locked) else {
                last_enter_commit.borrow_mut().take();
                error.set(Some("Enter a finite position coordinate.".to_owned()));
                return;
            };
            error.set(None);
            let owner = latest_capture();
            if trigger == PositionCommitTrigger::Blur {
                let duplicate_enter = last_enter_commit.borrow_mut().take().is_some_and(|stamp| {
                    stamp.axis == axis
                        && stamp.value == value
                        && same_position_commit_owner(&stamp.owner, &owner)
                });
                if duplicate_enter {
                    return;
                }
            } else {
                // Enter remains an explicit retry after a rejected action. The
                // following blur alone is suppressed when it carries this same
                // axis/value for the same selected Inspector owner.
                *last_enter_commit.borrow_mut() = Some(PositionEnterCommit {
                    owner: owner.clone(),
                    axis,
                    value,
                });
            }
            // Position drafts are displayed to two decimal places. Blurring an
            // untouched field (or one restored by Escape) must not submit that
            // rounded display as a replacement for the accepted raw geometry.
            let untouched_blur =
                trigger == PositionCommitTrigger::Blur && draft == format!("{current:.2}");
            if value != current && !untouched_blur {
                action.call(LayoutComponentInspectorAction::SetPosition {
                    owner,
                    axis,
                    value,
                    submitted_draft: draft,
                });
            }
        })
    };
    let set_outline: Rc<dyn Fn(PartOutline)> = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        Rc::new(move |outline| {
            action.call(LayoutComponentInspectorAction::SetOutline {
                owner: latest_capture(),
                outline,
                submitted_draft: margin(),
            })
        })
    };
    let has_active_constraint = projection.active_constraint.is_some();
    // A choice on a saved offset converts it at once; a new one only sets the default.
    let mut choose_offset_axes = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        let saved_offset_id = projection
            .active_constraint
            .as_ref()
            .filter(|constraint| constraint.offset_axes().is_some())
            .map(|constraint| constraint.id().to_owned());
        let mut offset_axes = offset_axes;
        move |to: OffsetAxes| {
            offset_axes.set(to);
            if let Some(constraint_id) = saved_offset_id.as_ref() {
                action.call(LayoutComponentInspectorAction::ConvertOffsetAxes {
                    owner: latest_capture(),
                    constraint_id: constraint_id.clone(),
                    to,
                });
            }
        }
    };
    let save_constraint = {
        let latest_capture = latest_capture;
        let offset_axes = offset_axes;
        let mirror_line_x = mirror_line_x;
        let mirror_line_y = mirror_line_y;
        let mirror_line_angle = mirror_line_angle;
        let retain_legacy_form = projection
            .active_constraint
            .as_ref()
            .is_some_and(|constraint| !matches!(constraint, Constraint::Framed { .. }));
        let action = props.on_action;
        let mut error = error;
        move |_| {
            let source = source_part_id();
            let owner = latest_capture();
            if source.is_empty() || source == owner.part_id {
                error.set(Some("Choose another part on this board.".to_owned()));
                return;
            }
            let values = match constraint_kind() {
                ConstraintKind::Offset => {
                    let (Ok(x), Ok(y), Ok(rotation)) = (
                        offset_x().parse::<f64>(),
                        offset_y().parse::<f64>(),
                        rotation().parse::<f64>(),
                    ) else {
                        error.set(Some("Enter finite constraint values.".to_owned()));
                        return;
                    };
                    if !x.is_finite() || !y.is_finite() || !rotation.is_finite() {
                        error.set(Some("Enter finite constraint values.".to_owned()));
                        return;
                    }
                    let offset = Vec2 { x, y };
                    if retain_legacy_form {
                        offset_values(offset_axes(), offset, rotation)
                    } else {
                        LayoutConstraintValues::Framed {
                            relation: match offset_axes() {
                                OffsetAxes::Board => {
                                    ConstraintRelation::Offset { offset, rotation }
                                }
                                OffsetAxes::Part => {
                                    ConstraintRelation::PartOffset { offset, rotation }
                                }
                            },
                        }
                    }
                }
                ConstraintKind::Mirror => {
                    let Ok(coordinate) = mirror_coordinate().parse::<f64>() else {
                        error.set(Some("Enter a finite axis coordinate.".to_owned()));
                        return;
                    };
                    if !coordinate.is_finite() {
                        error.set(Some("Enter a finite axis coordinate.".to_owned()));
                        return;
                    }
                    if retain_legacy_form {
                        LayoutConstraintValues::Mirror {
                            axis: mirror_axis(),
                            coordinate,
                        }
                    } else {
                        let axis = mirror_axis();
                        let at = match axis {
                            MirrorAxis::Vertical => Vec2 {
                                x: coordinate,
                                y: 0.0,
                            },
                            MirrorAxis::Horizontal => Vec2 {
                                x: 0.0,
                                y: coordinate,
                            },
                        };
                        LayoutConstraintValues::Framed {
                            relation: ConstraintRelation::MirrorLine {
                                at,
                                angle: if axis == MirrorAxis::Vertical {
                                    90.0
                                } else {
                                    0.0
                                },
                            },
                        }
                    }
                }
                ConstraintKind::MirrorLine => {
                    let (Ok(x), Ok(y), Ok(angle)) = (
                        mirror_line_x().parse::<f64>(),
                        mirror_line_y().parse::<f64>(),
                        mirror_line_angle().parse::<f64>(),
                    ) else {
                        error.set(Some("Enter finite mirror line values.".to_owned()));
                        return;
                    };
                    if !x.is_finite() || !y.is_finite() || !angle.is_finite() {
                        error.set(Some("Enter finite mirror line values.".to_owned()));
                        return;
                    }
                    LayoutConstraintValues::Framed {
                        relation: ConstraintRelation::MirrorLine {
                            at: Vec2 { x, y },
                            angle,
                        },
                    }
                }
            };
            error.set(None);
            action.call(LayoutComponentInspectorAction::SetConstraint {
                owner: owner.clone(),
                source_part_id: source,
                values,
            });
        }
    };
    let active_constraint_id = projection
        .active_constraint
        .as_ref()
        .map(|constraint| constraint.id().to_owned());
    let remove_constraint = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        move |_| {
            if let Some(constraint_id) = active_constraint_id.as_ref() {
                action.call(LayoutComponentInspectorAction::RemoveConstraint {
                    owner: latest_capture(),
                    constraint_id: constraint_id.clone(),
                });
            }
        }
    };
    let position = projection.position;
    let group_selection = projection.selection_count > 1;
    let outline = projection.outline.clone();
    let active_constraint = projection.active_constraint.clone();
    let board_parts = projection.board_parts.clone();
    let constraint_source_reference = active_constraint
        .as_ref()
        .and_then(|constraint| {
            board_parts
                .iter()
                .find(|part| part.id == constraint.source())
                .map(|part| part.reference.clone())
                .or_else(|| Some(constraint.source().to_owned()))
        })
        .unwrap_or_else(|| "A part".to_owned());
    let constraint_target_reference = active_constraint
        .as_ref()
        .and_then(|constraint| {
            board_parts
                .iter()
                .find(|part| part.id == constraint.target())
                .map(|part| part.reference.clone())
                .or_else(|| Some(constraint.target().to_owned()))
        })
        .unwrap_or_else(|| projection.reference.clone());

    rsx! {
        section { class: "m1-layout-component-inspector", aria_label: "Component inspector",
            header { class: "m1-layout-component-inspector-heading",
                h2 { "{projection.reference}" }
                if projection.locked { span { class: "m1-inspector-lock", "Locked" } }
            }
                if group_selection {
                    p { class: "m1-layout-component-selection-note", "{projection.selection_count} parts selected. Position edits apply to the selection." }
                } else {
                    p { class: "m1-layout-component-definition", "{projection.definition_name}", span { "{projection.definition_kind}" } }
                    if let Some(notice) = projection.envelope_notice.as_ref() { p { class: "m1-layout-component-notice", "{notice}" } }
                }
                if let Some(mirrored) = projection.part_mirror {
                    details { class: "m1-layout-component-mirror-state",
                        summary { span { "Mirrored half" } small { if mirrored { "Both halves" } else { "Local" } } }
                        div { class: "m1-layout-component-fields",
                            label { input {
                                r#type: "checkbox", id: "component-inspector-part-mirror",
                                aria_label: "Mirror on both halves", checked: mirrored,
                                disabled: (props.mirrored_half_pending)(),
                                onchange: {
                                    let latest_capture = latest_capture;
                                    let action = props.on_action;
                                    let part_id = owner.part_id.clone();
                                    move |event: FormEvent| action.call(LayoutComponentInspectorAction::SetPartMirrored {
                                        owner: latest_capture(),
                                        part_id: part_id.clone(),
                                        mirrored: event.checked(),
                                    })
                                },
                            } "Mirror on both halves" }
                            p { "A mirrored part edits both halves together and stops at the split axis." }
                        }
                    }
                } else if let Some(mirrored_half) = projection.mirrored_half.as_ref() {
                    details { class: "m1-layout-component-mirror-state",
                        summary { span { "Mirrored half" } small {
                            match mirrored_half.state {
                                HalfExceptionState::Linked => "Linked",
                                HalfExceptionState::Local => "Local",
                                HalfExceptionState::Omitted => "Omitted",
                            }
                        } }
                        div { class: "m1-layout-component-fields",
                            p { "This part's state on the mirrored half." }
                            button {
                                id: "component-inspector-mirrored-half-local",
                                r#type: "button",
                                disabled: (props.mirrored_half_pending)() || mirrored_half.state == HalfExceptionState::Local,
                                onclick: mirrored_half_action(HalfExceptionState::Local),
                                "Make local"
                            }
                            button {
                                id: "component-inspector-mirrored-half-omit",
                                r#type: "button",
                                disabled: (props.mirrored_half_pending)() || mirrored_half.state == HalfExceptionState::Omitted,
                                onclick: mirrored_half_action(HalfExceptionState::Omitted),
                                "Omit"
                            }
                            button {
                                id: "component-inspector-mirrored-half-rejoin",
                                r#type: "button",
                                disabled: (props.mirrored_half_pending)() || mirrored_half.state == HalfExceptionState::Linked,
                                onclick: mirrored_half_action(HalfExceptionState::Linked),
                                "Rejoin"
                            }
                        }
                    }
                }
                if !group_selection && !projection.layouts.is_empty() {
                    label { class: "m1-layout-component-layout", "Layout"
                        select {
                            aria_label: "Component layout",
                            value: projection.layout_id.as_deref().unwrap_or(""),
                            disabled: (props.layout_pending)(),
                            onchange: {
                                let latest_capture = latest_capture;
                                let action = props.on_action;
                                move |event: FormEvent| action.call(LayoutComponentInspectorAction::AssignLayout {
                                    owner: latest_capture(),
                                    layout_id: (!event.value().is_empty()).then_some(event.value()),
                                })
                            },
                            option { value: "", selected: projection.layout_id.is_none(), "Board / ungrouped" }
                            for layout in &projection.layouts {
                                option {
                                    value: "{layout.id}",
                                    selected: projection.layout_id.as_deref() == Some(layout.id.as_str()),
                                    "{layout.name}"
                                }
                            }
                        }
                    }
                }
                h3 { class: "m1-layout-component-subtitle", "Position", small { "millimetres" } }
                div { class: "m1-layout-component-position",
                    label { "X (mm)" input {
                        r#type: "number", step: "0.1", value: "{x}", aria_label: "X mm",
                        disabled: position_input_disabled(position_unavailable(&projection)),
                        oninput: { let last_enter_commit = last_enter_commit.clone(); move |event| { last_enter_commit.borrow_mut().take(); x.set(event.value()); } },
                        onblur: { let commit = commit_position.clone(); move |_| commit(ComponentPositionAxis::X, PositionCommitTrigger::Blur) },
                        onkeydown: { let commit = commit_position.clone(); move |event: KeyboardEvent| {
                            match event.data().key().to_string().as_str() {
                                "Enter" => { event.prevent_default(); commit(ComponentPositionAxis::X, PositionCommitTrigger::Enter); }
                                "Escape" => { x.set(format!("{:.2}", position.x)); error.set(None); }
                                _ => {}
                            }
                        } },
                    } }
                    label { "Y (mm)" input {
                        r#type: "number", step: "0.1", value: "{y}", aria_label: "Y mm",
                        disabled: position_input_disabled(position_unavailable(&projection)),
                        oninput: { let last_enter_commit = last_enter_commit.clone(); move |event| { last_enter_commit.borrow_mut().take(); y.set(event.value()); } },
                        onblur: { let commit = commit_position.clone(); move |_| commit(ComponentPositionAxis::Y, PositionCommitTrigger::Blur) },
                        onkeydown: { let commit = commit_position.clone(); move |event: KeyboardEvent| {
                            match event.data().key().to_string().as_str() {
                                "Enter" => { event.prevent_default(); commit(ComponentPositionAxis::Y, PositionCommitTrigger::Enter); }
                                "Escape" => { y.set(format!("{:.2}", position.y)); error.set(None); }
                                _ => {}
                            }
                        } },
                    } }
                }
                if !group_selection {
                details { class: "m1-layout-component-outline",
                    summary { span { "Board outline" } small { if projection.outline.excluded { "Excluded" } else { "Included" } } }
                    fieldset { disabled: projection.locked,
                    label { input {
                        r#type: "checkbox", aria_label: "Include in outline", checked: !projection.outline.excluded,
                        onchange: { let baseline = outline.clone(); let submit = set_outline.clone(); move |event| { let mut outline = baseline.clone(); outline.excluded = !event.checked(); submit(outline); } },
                    } "Include in outline" }
                    if !projection.outline.excluded {
                        label { input {
                            r#type: "checkbox", aria_label: "Use board margin", checked: projection.outline.margin.is_none(),
                            onchange: { let baseline = outline.clone(); let submit = set_outline.clone(); move |event| { let mut outline = baseline.clone(); outline.margin = if event.checked() { None } else { Some(0.0) }; margin.set(outline.margin.map(|value| value.to_string()).unwrap_or_default()); submit(outline); } },
                        } "Use board margin" }
                        if outline.margin.is_some() {
                            label { "Part edge margin" input {
                                r#type: "number", step: "any", min: "0", value: "{margin}", aria_label: "Part edge margin",
                                oninput: move |event| margin.set(event.value()),
                                onblur: {
                                    let margin_outline = outline.clone();
                                    let save_outline = set_outline.clone();
                                    move |_| {
                                        match margin().parse::<f64>() {
                                            Ok(value) if value.is_finite() && value >= 0.0 => {
                                                if value != margin_outline.margin.unwrap_or(value) {
                                                    let mut next = margin_outline.clone(); next.margin = Some(value); save_outline(next);
                                                }
                                                error.set(None);
                                            }
                                            _ => error.set(Some("Enter 0 or greater for the part edge margin.".to_owned())),
                                        }
                                    }
                                },
                                onkeydown: {
                                    let accepted_margin = outline.margin.unwrap_or_default().to_string();
                                    move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                                        "Enter" => {
                                            event.prevent_default();
                                            if let Some(input) = event.data().try_as_web_event()
                                                .and_then(|event| event.target())
                                                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                                            {
                                                let _ = input.blur();
                                            }
                                        }
                                        "Escape" => {
                                            event.prevent_default();
                                            margin.set(accepted_margin.clone());
                                            error.set(None);
                                        }
                                        _ => {}
                                    }
                                },
                            } }
                        }
                    }
                    label { input {
                        r#type: "checkbox", aria_label: "Allow component body overhang", checked: projection.outline.allow_body_overhang,
                        onchange: { let baseline = outline.clone(); let submit = set_outline.clone(); move |event| { let mut outline = baseline.clone(); outline.allow_body_overhang = event.checked(); submit(outline); } },
                    } "Allow component body overhang" }
                    small { "Pad and drill support remains required when a part is excluded or body overhang is allowed." }
                    }
                }
                details { class: "m1-layout-component-constraint", open: constraint_open(),
                    summary { onclick: move |_| constraint_open.set(!constraint_open()), span { "Layout constraint" } small { if active_constraint.is_some() { "Active" } else { "Optional" } } }
                    if board_parts.iter().filter(|part| part.id != owner.part_id).count() > 0 {
                        label { "Constraint type"
                            select { aria_label: "Constraint type", value: match constraint_kind() { ConstraintKind::Offset => "offset", ConstraintKind::Mirror => "mirror", ConstraintKind::MirrorLine => "mirror-line" }, onchange: move |event| constraint_kind.set(match event.value().as_str() { "mirror" => ConstraintKind::Mirror, "mirror-line" => ConstraintKind::MirrorLine, _ => ConstraintKind::Offset }),
                                option { value: "offset", selected: constraint_kind() == ConstraintKind::Offset, "Offset from part" }
                                option { value: "mirror", selected: constraint_kind() == ConstraintKind::Mirror, "Mirror placement across axis" }
                                option { value: "mirror-line", selected: constraint_kind() == ConstraintKind::MirrorLine, "Mirror line" }
                            }
                        }
                        label { "Source part"
                            select { aria_label: "Constraint source part", value: "{source_part_id}", onchange: {
                                    let board_parts = board_parts.clone();
                                    move |event: FormEvent| {
                                        let value = event.value();
                                        if !has_active_constraint {
                                            offset_axes.set(default_axes_for_source(&board_parts, &value));
                                        }
                                        source_part_id.set(value);
                                    }
                                },
                                if !board_parts.iter().any(|part| part.id == source_part_id()) {
                                    option { value: "{source_part_id}", selected: true, "{constraint_source_reference}" }
                                }
                                for part in board_parts.iter().filter(|part| part.id != owner.part_id) {
                                    option {
                                        value: "{part.id}",
                                        selected: source_part_id().as_str() == part.id.as_str(),
                                        "{part.reference}"
                                    }
                                }
                            }
                        }
                        if constraint_kind() == ConstraintKind::Offset {
                            label { "Offset axes"
                                select { aria_label: "Offset axes", value: offset_axes_value(offset_axes()), onchange: move |event| choose_offset_axes(offset_axes_from_value(&event.value())),
                                    option { value: offset_axes_value(OffsetAxes::Board), selected: offset_axes() == OffsetAxes::Board, "Offset on board axes" }
                                    option { value: offset_axes_value(OffsetAxes::Part), selected: offset_axes() == OffsetAxes::Part, "Offset along part axes" }
                                }
                            }
                            div { class: "m1-layout-component-constraint-grid",
                                label { "Offset X (mm)" input { r#type: "number", step: "any", aria_label: "Offset X (mm)", value: "{offset_x}", oninput: move |event| offset_x.set(event.value()) } }
                                label { "Offset Y (mm)" input { r#type: "number", step: "any", aria_label: "Offset Y (mm)", value: "{offset_y}", oninput: move |event| offset_y.set(event.value()) } }
                            }
                            label { "Rotation (degrees)" input { r#type: "number", step: "any", aria_label: "Rotation (degrees)", value: "{rotation}", oninput: move |event| rotation.set(event.value()) } }
                        } else if constraint_kind() == ConstraintKind::Mirror {
                            label { "Axis"
                                select { aria_label: "Mirror axis", value: if mirror_axis() == MirrorAxis::Vertical { "vertical" } else { "horizontal" }, onchange: move |event| mirror_axis.set(if event.value() == "horizontal" { MirrorAxis::Horizontal } else { MirrorAxis::Vertical }),
                                    option { value: "vertical", selected: mirror_axis() == MirrorAxis::Vertical, "Vertical" }
                                    option { value: "horizontal", selected: mirror_axis() == MirrorAxis::Horizontal, "Horizontal" }
                                }
                            }
                            label { "Axis coordinate (mm)" input { r#type: "number", step: "any", aria_label: "Axis coordinate (mm)", value: "{mirror_coordinate}", oninput: move |event| mirror_coordinate.set(event.value()) } }
                            p { "Footprint geometry stays unchanged. Use a handed definition where needed." }
                        } else {
                            div { class: "m1-layout-component-constraint-grid",
                                label { "Line anchor X (mm)" input { r#type: "number", step: "any", aria_label: "Mirror line anchor X (mm)", value: "{mirror_line_x}", oninput: move |event| mirror_line_x.set(event.value()) } }
                                label { "Line anchor Y (mm)" input { r#type: "number", step: "any", aria_label: "Mirror line anchor Y (mm)", value: "{mirror_line_y}", oninput: move |event| mirror_line_y.set(event.value()) } }
                            }
                            label { "Line angle (degrees)" input { r#type: "number", step: "any", aria_label: "Mirror line angle (degrees)", value: "{mirror_line_angle}", oninput: move |event| mirror_line_angle.set(event.value()) } }
                            p { "Footprint geometry stays unchanged. Use a handed definition where needed." }
                        }
                        if active_constraint.is_some() {
                            p { class: "m1-layout-component-constraint-note", "{constraint_source_reference} drives {constraint_target_reference}." }
                        }
                        div { class: "m1-layout-component-constraint-actions",
                            button { r#type: "button", disabled: (props.constraint_pending)(), onclick: save_constraint, if active_constraint.is_some() { "Save constraint" } else { "Add constraint" } }
                            if active_constraint.is_some() { BoardButton { r#type: "button", disabled: (props.remove_pending)(), onclick: remove_constraint, "Remove" } }
                        }
                    } else { p { "Add another part on this board to create a layout constraint." } }
                }
                }
                if let Some(message) = x_failure()
                    .or_else(&*y_failure)
                    .or_else(&*margin_failure)
                    .or_else(&*error)
                { p { role: "alert", "{message}" } }
                button { class: "m1-layout-component-electrical", r#type: "button", onclick: {
                    let latest_capture = latest_capture; let action = props.on_action;
                    move |_| action.call(LayoutComponentInspectorAction::NavigateElectrical { owner: latest_capture() })
                }, "Edit electrical connections" }
                section { class: "m1-layout-component-relationships", aria_labelledby: "m1-component-relationships-heading",
                    h3 { id: "m1-component-relationships-heading", "Relationships" }
                    p { class: "m1-layout-component-relation-summary", "{projection.relationship_summary}" }
                    p { class: "m1-layout-component-matrix-note", "Matrix rows and columns share pitch, stagger and splay. Edit those in Properties." }
                }
        }
    }
}
