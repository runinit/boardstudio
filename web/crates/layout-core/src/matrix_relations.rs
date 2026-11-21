//! Matrix relationships for the Relations panel: a pure projection of the accepted
//! document and Core's matrix scene into a view model, with no state and no Core calls.
//!
//! Structure is what the matrix and its assemblies place by inheritance. Departures
//! are the values that override it, each with the actions that undo or convert them.
//! Constraints are separately authored. Companion placement modes come from the scene
//! (Core classifies them once); this module never re-reads constraints or overrides to
//! decide precedence.

use crate::matrix_transform_operation::MatrixTransformField;
use boardstudio_model::model::{
    AggregateMethod, Constraint, ConstraintEndpoint, ConstraintRelation, Follow, Matrix,
    MatrixAnchor, MatrixCompanionMode, MatrixCompanionPlacement, MatrixScene, MatrixSplayAffect,
    Mirror, MirroredExceptionState, MirroredItemKind, Part, ProjectDoc, Vec2,
};
use boardstudio_web_ui_model::tree::TreeContext;

/// "X, Y, rotation" for the Follow subset, or "none" when nothing follows.
/// Free so the canvas anchor badge (ticket 17) reads the same words as the chain.
pub fn follow_text(follow: &Follow) -> String {
    let mut parts = Vec::new();
    if follow.x {
        parts.push("X");
    }
    if follow.y {
        parts.push("Y");
    }
    if follow.rotation {
        parts.push("rotation");
    }
    if parts.is_empty() {
        "none".to_owned()
    } else {
        parts.join(", ")
    }
}

/// "Average of 2 keys" or "Intersection of 2 keys", from the Core-admitted
/// aggregate method. `None` for single-key anchors. Free so the canvas anchor
/// badge (ticket 17) reads the same words as the chain.
pub fn aggregate_kind_text(anchor: &MatrixAnchor) -> Option<String> {
    let aggregate = anchor.aggregate.as_ref()?;
    let method = match aggregate.method {
        AggregateMethod::Average => "Average",
        AggregateMethod::Intersect => "Intersection",
    };
    let count = aggregate.parts.len();
    Some(format!(
        "{method} of {count} {}",
        if count == 1 { "key" } else { "keys" }
    ))
}

/// One inherited fact, with an optional object to navigate to and the parts to highlight.
#[derive(Clone, Debug, PartialEq)]
pub struct RelationItem {
    pub label: String,
    pub value: String,
    pub target: Option<TreeContext>,
    pub highlight: Vec<String>,
    /// A parameter entry: activating it selects its owner and opens the matching
    /// tool/editor. Receiver references (followers, anchor dependants, linked
    /// partners) navigate without leaving the Relations tab instead.
    pub opens_editor: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepartureAction {
    Reset,
    /// Column offsets only: move the Y offset into the column's stagger.
    Convert,
}

/// A value that overrides what the matrix would inherit.
#[derive(Clone, Debug, PartialEq)]
pub struct Departure {
    /// The parameter it overrides, for example "Column offset".
    pub parameter: String,
    pub value: String,
    pub target: Option<TreeContext>,
    pub highlight: Vec<String>,
    pub actions: Vec<DepartureAction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PartReference {
    pub part_id: String,
    pub label: String,
    pub context: TreeContext,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintKind {
    Offset,
    PartOffset,
    Mirror,
    MirrorLine,
}

/// A separately authored constraint touching the selection.
#[derive(Clone, Debug, PartialEq)]
pub struct ConstraintItem {
    pub id: String,
    pub kind: ConstraintKind,
    /// The authored endpoint views. These retain LinkedMirror binding data as well as the
    /// visible IDs used by the reference buttons below.
    pub source_endpoint: ConstraintEndpoint,
    pub target_endpoint: ConstraintEndpoint,
    /// Present for Framed records. Legacy records retain their existing kind and are not
    /// rewritten into a guessed framed relation during projection.
    pub relation: Option<ConstraintRelation>,
    pub source: PartReference,
    pub target: PartReference,
    /// The constraint wins over the target companion's attachment to its key.
    pub takes_precedence: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixRelations {
    pub structure: Vec<RelationItem>,
    pub departures: Vec<Departure>,
    pub constraints: Vec<ConstraintItem>,
    /// Set when the selected parts span matrices: the chain covers one context
    /// only, never a merged cross-matrix frame.
    pub selection_note: Option<String>,
}

/// Explain a cross-matrix selection instead of merging it into one chain.
/// Returns a note when the selected parts name live members of more than one
/// matrix scene; the chain still covers only `context`.
pub fn cross_matrix_note(
    scenes: &[MatrixScene],
    context: &TreeContext,
    selected_part_ids: &[String],
) -> Option<String> {
    let mut owners = std::collections::BTreeSet::new();
    for id in selected_part_ids {
        for scene in scenes {
            let member = scene
                .cells
                .iter()
                .any(|cell| cell.member_id.as_deref() == Some(id.as_str()))
                || scene
                    .companions
                    .iter()
                    .any(|companion| companion.part_id == *id);
            if member {
                owners.insert(scene.matrix_id.as_str());
            }
        }
    }
    if owners.len() < 2 {
        return None;
    }
    Some(format!(
        "Parts in {} matrices are selected. This chain covers {}; select one matrix context for detail.",
        owners.len(),
        describe_context(context)
    ))
}

fn describe_context(context: &TreeContext) -> String {
    match context {
        TreeContext::Matrix { matrix_id } => format!("matrix {matrix_id}"),
        TreeContext::Row { matrix_id, row } => {
            format!("row {} of matrix {matrix_id}", row + 1)
        }
        TreeContext::Column { matrix_id, column } => {
            format!("column {} of matrix {matrix_id}", column + 1)
        }
        TreeContext::Key {
            matrix_id,
            row,
            column,
        } => format!(
            "row {}, column {} of matrix {matrix_id}",
            row + 1,
            column + 1
        ),
        TreeContext::Component { part_id, .. } => {
            format!("component {}", part_id.as_deref().unwrap_or("unknown"))
        }
        _ => "this selection".to_owned(),
    }
}

/// The relationships of one matrix selection, or `None` when the context does not name a
/// live object of this matrix (another matrix, or a removed row, column, key or component).
/// `scenes` carries every matrix scene so cross-matrix receiver references resolve
/// against current accepted identities instead of fabricated coordinates.
pub fn matrix_relations(
    document: &ProjectDoc,
    scene: &MatrixScene,
    context: &TreeContext,
    scenes: &[MatrixScene],
) -> Option<MatrixRelations> {
    let matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == scene.matrix_id)?;
    let view = View {
        document,
        scene,
        scenes,
        matrix,
    };
    match context {
        TreeContext::Matrix { matrix_id } if *matrix_id == matrix.id => Some(view.matrix()),
        TreeContext::Row { matrix_id, row } if *matrix_id == matrix.id => view.row(*row),
        TreeContext::Column { matrix_id, column } if *matrix_id == matrix.id => {
            view.column(*column)
        }
        TreeContext::Key {
            matrix_id,
            row,
            column,
        } if *matrix_id == matrix.id => view.key(*row, *column),
        TreeContext::Component {
            part_id: Some(part_id),
            ..
        } => view.component(part_id),
        _ => None,
    }
}

/// Ticket 16: the amber at-rest preview for one Inspector number field. The
/// affected members a change to `field` would move: the field's structural
/// carry set on its matrix plus the mirror-partner matrix's matching slots,
/// resolved against the live accepted scenes. `None` for fields with no
/// geometric carry (toggles, picks, resets, splay-origin moves, foreign
/// contexts). Never writes; the caller shows `parts` on the canvas and
/// `hint` beside it, then releases both on leave, blur or selection change.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldPreview {
    pub parts: Vec<String>,
    pub hint: String,
}

/// The members a change to one Inspector number field would move, or `None`
/// when the field carries nothing. `splay_affect` is the live Splay toggle:
/// a Following splay turns later columns, a Column one stays local.
/// Mirroring follows Core's sync: layout records (matrix frame, column and
/// row layout) copy to the partner, while a key's own offset/rotation stays
/// put once either side's part is Local. A Local/Omitted matrix never
/// authors its partner, so the partner stays out as well.
pub fn field_preview(
    document: &ProjectDoc,
    scenes: &[MatrixScene],
    context: &TreeContext,
    field: MatrixTransformField,
    splay_affect: MatrixSplayAffect,
) -> Option<FieldPreview> {
    let matrix_id = match context {
        TreeContext::Matrix { matrix_id }
        | TreeContext::Row { matrix_id, .. }
        | TreeContext::Column { matrix_id, .. }
        | TreeContext::Key { matrix_id, .. } => matrix_id,
        _ => return None,
    };
    let matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == *matrix_id)?;
    let scene = scenes.iter().find(|scene| scene.matrix_id == *matrix_id)?;
    let (slots, key_local) = preview_slots(matrix, context, field, splay_affect)?;
    let view = View {
        document,
        scene,
        scenes,
        matrix,
    };
    if matches!(context, TreeContext::Key { .. })
        && view.key_state(slots[0].0, slots[0].1) == KeyState::Removed
    {
        return None;
    }
    let kept = |row: u32, column: u32| slots.contains(&(row, column));
    let mut parts = view.members(kept);
    let own = parts.len();
    if let Some((_, partner)) = view.partner()
        && view.matrix_linked(&view.matrix.id)
        && view.matrix_linked(&partner.id)
        && let Some(partner_scene) = view.scene_of(&partner.id)
    {
        let partner_view = View {
            document,
            scene: partner_scene,
            scenes,
            matrix: partner,
        };
        for (row, column) in &slots {
            if key_local
                && (view.exception(
                    MirroredItemKind::Part,
                    &View::canonical_member(&view.matrix.id, *row, *column),
                ) == Some(MirroredExceptionState::Local)
                    || view.exception(
                        MirroredItemKind::Part,
                        &View::canonical_member(&partner.id, *row, *column),
                    ) == Some(MirroredExceptionState::Local))
            {
                continue;
            }
            if let Some(member) = partner_view.member(*row, *column) {
                parts.push(member.to_owned());
            }
        }
    }
    if parts.is_empty() {
        return None;
    }
    let keys = if parts.len() == 1 { "key" } else { "keys" };
    let hint = if parts.len() > own {
        format!("Moves {} {keys} ({own} per half)", parts.len())
    } else {
        format!("Moves {} {keys}", parts.len())
    };
    Some(FieldPreview { parts, hint })
}

/// The (row, column) slots a field carries, with whether the field is a
/// key-local final pose (offset/rotation) rather than a synced record.
/// Column Splay follows the Splay toggle; splay-origin moves hold keys
/// still ("Move the origin without moving the keys"), so they preview
/// nothing. Anchored dependants stay out: their Relations "Followed by"
/// rows already name them.
fn preview_slots(
    matrix: &Matrix,
    context: &TreeContext,
    field: MatrixTransformField,
    splay_affect: MatrixSplayAffect,
) -> Option<(Vec<(u32, u32)>, bool)> {
    use MatrixTransformField as Field;
    let all: Vec<(u32, u32)> = (0..matrix.rows)
        .flat_map(|row| (0..matrix.columns).map(move |column| (row, column)))
        .collect();
    let at = |row: u32, column: u32| {
        (row < matrix.rows && column < matrix.columns).then_some(vec![(row, column)])
    };
    match context {
        TreeContext::Matrix { matrix_id } if *matrix_id == matrix.id => match field {
            Field::OriginX | Field::OriginY | Field::MatrixRotation => Some((all, false)),
            _ => None,
        },
        TreeContext::Column { matrix_id, column } if *matrix_id == matrix.id => {
            if *column >= matrix.columns {
                return None;
            }
            match field {
                Field::ColumnSpread | Field::ColumnStagger => Some((
                    all.into_iter().filter(|&(_, c)| c >= *column).collect(),
                    false,
                )),
                Field::ColumnSplay if splay_affect == MatrixSplayAffect::Following => Some((
                    all.into_iter().filter(|&(_, c)| c >= *column).collect(),
                    false,
                )),
                Field::ColumnSplay | Field::ColumnOffsetX | Field::ColumnOffsetY => Some((
                    all.into_iter().filter(|&(_, c)| c == *column).collect(),
                    false,
                )),
                _ => None,
            }
        }
        TreeContext::Row { matrix_id, row } if *matrix_id == matrix.id => {
            if *row >= matrix.rows {
                return None;
            }
            match field {
                Field::RowShiftX | Field::RowShiftY | Field::RowBend | Field::RowPadding => {
                    Some((all.into_iter().filter(|&(r, _)| r >= *row).collect(), false))
                }
                Field::RowOffsetX | Field::RowOffsetY => {
                    Some((all.into_iter().filter(|&(r, _)| r == *row).collect(), false))
                }
                _ => None,
            }
        }
        TreeContext::Key {
            matrix_id,
            row,
            column,
        } if *matrix_id == matrix.id => {
            let slots = at(*row, *column)?;
            match field {
                Field::KeyShiftX | Field::KeyShiftY | Field::KeyBend => Some((
                    (0..matrix.rows)
                        .filter(|r| *r >= *row)
                        .map(|r| (r, *column))
                        .collect(),
                    false,
                )),
                Field::KeyOffsetX | Field::KeyOffsetY | Field::KeyRotation => Some((slots, true)),
                _ => None,
            }
        }
        _ => None,
    }
}

struct View<'a> {
    document: &'a ProjectDoc,
    scene: &'a MatrixScene,
    scenes: &'a [MatrixScene],
    matrix: &'a Matrix,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum KeyState {
    Enabled,
    Disabled,
    Removed,
}

mod view_details;
mod view_structure;

const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

/// Carried per-key facts published by Core's kernel through the scene. Relations
/// reads them; it never recomputes layout maths.
#[derive(Clone, Copy, Debug, Default)]
struct CoreCellFacts {
    row_shift: Vec2,
    key_shift: Vec2,
    row_bend: f64,
    key_bend: f64,
    cumulative_shift: Vec2,
    cumulative_bend: f64,
}

fn item(
    label: &str,
    value: String,
    target: Option<TreeContext>,
    highlight: Vec<String>,
) -> RelationItem {
    RelationItem {
        label: label.to_owned(),
        value,
        target,
        highlight,
        opens_editor: false,
    }
}

/// A parameter entry: activating it selects its owner and opens the matching
/// tool/editor instead of staying on the Relations tab.
fn parameter(
    label: &str,
    value: String,
    target: Option<TreeContext>,
    highlight: Vec<String>,
) -> RelationItem {
    RelationItem {
        label: label.to_owned(),
        value,
        target,
        highlight,
        opens_editor: true,
    }
}

fn offset_text(offset: Vec2) -> String {
    if offset == ZERO {
        "none".to_owned()
    } else {
        format!("{}, {} mm", mm(offset.x), mm(offset.y))
    }
}

fn bend_text(bend: f64) -> String {
    if bend == 0.0 {
        "none".to_owned()
    } else {
        format!("{}°", mm(bend))
    }
}

/// One aggregated logical key: stored shift/bend from the document (deleted
/// keys keep theirs) and Core's cumulative totals when the scene carries them.
struct AggregatedKey {
    shift: Vec2,
    bend: f64,
    cumulative_shift: Option<Vec2>,
    cumulative_bend: Option<f64>,
    state: KeyState,
}

/// One shared text when every value agrees, otherwise "Varies".
fn uniform_or_varies(mut values: Vec<String>) -> String {
    values.sort();
    values.dedup();
    if values.len() == 1 {
        values
            .into_iter()
            .next()
            .unwrap_or_else(|| "Varies".to_owned())
    } else {
        "Varies".to_owned()
    }
}

/// Names the non-enabled contributors behind an aggregate, so disabled and
/// deleted keys are not silently dropped from the chain.
fn state_suffix(disabled: bool, deleted: bool) -> &'static str {
    match (disabled, deleted) {
        (true, true) => " (includes disabled and deleted keys)",
        (true, false) => " (includes disabled keys)",
        (false, true) => " (includes deleted keys)",
        (false, false) => "",
    }
}

/// Stored key shifts across an aggregate, labelled with contributor state.
fn aggregate_shift(keys: &[AggregatedKey]) -> String {
    let texts: Vec<String> = keys.iter().map(|key| offset_text(key.shift)).collect();
    let mut text = uniform_or_varies(texts);
    let (mut disabled, mut deleted) = (false, false);
    for key in keys {
        if key.shift != ZERO {
            match key.state {
                KeyState::Disabled => disabled = true,
                KeyState::Removed => deleted = true,
                KeyState::Enabled => {}
            }
        }
    }
    text.push_str(state_suffix(disabled, deleted));
    text
}

/// Stored key bends across an aggregate, labelled with contributor state.
fn aggregate_bend(keys: &[AggregatedKey]) -> String {
    let texts: Vec<String> = keys.iter().map(|key| bend_text(key.bend)).collect();
    let mut text = uniform_or_varies(texts);
    let (mut disabled, mut deleted) = (false, false);
    for key in keys {
        if key.bend != 0.0 {
            match key.state {
                KeyState::Disabled => disabled = true,
                KeyState::Removed => deleted = true,
                KeyState::Enabled => {}
            }
        }
    }
    text.push_str(state_suffix(disabled, deleted));
    text
}

/// Core's cumulative shifts across scene-present keys; keys removed from the
/// scene carry no published total.
fn aggregate_cumulative_shift(keys: &[AggregatedKey]) -> String {
    let texts: Vec<String> = keys
        .iter()
        .filter_map(|key| key.cumulative_shift)
        .map(|shift| format!("{}, {} mm in matrix axes", mm(shift.x), mm(shift.y)))
        .collect();
    if texts.is_empty() {
        return "not shown (keys removed)".to_owned();
    }
    let mut text = uniform_or_varies(texts);
    let disabled = keys.iter().any(|key| {
        key.cumulative_shift.is_some_and(|shift| shift != ZERO) && key.state == KeyState::Disabled
    });
    text.push_str(state_suffix(disabled, false));
    text
}

fn aggregate_cumulative_bend(keys: &[AggregatedKey]) -> String {
    let texts: Vec<String> = keys
        .iter()
        .filter_map(|key| key.cumulative_bend)
        .map(|bend| format!("{}° (row and key only)", mm(bend)))
        .collect();
    if texts.is_empty() {
        return "not shown (keys removed)".to_owned();
    }
    let mut text = uniform_or_varies(texts);
    let disabled = keys.iter().any(|key| {
        key.cumulative_bend.is_some_and(|bend| bend != 0.0) && key.state == KeyState::Disabled
    });
    text.push_str(state_suffix(disabled, false));
    text
}

/// A millimetre or degree value without trailing zeros; a minus sign is U+2212.
fn mm(value: f64) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    let text = if text.is_empty() || text == "-0" {
        "0"
    } else {
        text
    };
    text.replacen('-', "\u{2212}", 1)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
