//! Matrix relationships for the Relations panel: a pure projection of the accepted
//! document and Core's matrix scene into a view model, with no state and no Core calls.
//!
//! Structure is what the matrix and its assemblies place by inheritance. Departures
//! are the values that override it, each with the actions that undo or convert them.
//! Constraints are separately authored. Companion placement modes come from the scene
//! (Core classifies them once); this module never re-reads constraints or overrides to
//! decide precedence.

use boardstudio_core::model::{
    Constraint, ConstraintEndpoint, ConstraintRelation, Follow, Matrix, MatrixAnchor,
    MatrixCompanionMode, MatrixCompanionPlacement, MatrixScene, Mirror, MirroredExceptionState,
    MirroredItemKind, ProjectDoc, Vec2,
};
use boardstudio_web_ui_model::tree::TreeContext;

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

impl View<'_> {
    fn key_state(&self, row: u32, column: u32) -> KeyState {
        match self
            .matrix
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
        {
            Some(cell) if cell.deleted => KeyState::Removed,
            Some(cell) if !cell.enabled => KeyState::Disabled,
            _ => KeyState::Enabled,
        }
    }

    fn member(&self, row: u32, column: u32) -> Option<&str> {
        self.scene
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .and_then(|cell| cell.member_id.as_deref())
    }

    fn scene_of(&self, matrix_id: &str) -> Option<&MatrixScene> {
        self.scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix_id)
    }

    /// The live member of another matrix's key, resolved against the current
    /// accepted scenes. `None` when the key has no live member, so references
    /// never highlight or navigate a stale identity.
    fn member_of(&self, matrix_id: &str, row: u32, column: u32) -> Option<&str> {
        self.scene_of(matrix_id)?
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .and_then(|cell| cell.member_id.as_deref())
    }

    fn scene_members(&self, matrix_id: &str) -> Vec<String> {
        self.scene_of(matrix_id)
            .map(|scene| {
                scene
                    .cells
                    .iter()
                    .filter_map(|cell| cell.member_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Mirrors Core's exception lookup over same-board links and native pairs,
    /// read-only: what Local/Omitted overlay covers this linked item, if any.
    fn exception(&self, kind: MirroredItemKind, id: &str) -> Option<MirroredExceptionState> {
        self.document
            .layouts
            .iter()
            .filter_map(|layout| layout.mirror_link.as_ref())
            .flat_map(|link| &link.exceptions)
            .chain(
                self.document
                    .hardware
                    .iter()
                    .flat_map(|hardware| &hardware.pairs)
                    .filter_map(|pair| pair.halves.as_ref())
                    .flat_map(|halves| &halves.exceptions),
            )
            .find(|exception| exception.kind == kind && exception.mirror_id == id)
            .map(|exception| exception.state)
    }

    /// The canonical member Core assigns a matrix slot. Only a fallback for
    /// naming a Local/Omitted overlay whose part is suppressed from the scene;
    /// never a navigation or highlight target.
    fn canonical_member(matrix_id: &str, row: u32, column: u32) -> String {
        format!("matrix/{matrix_id}/r{row}c{column}")
    }

    fn key_overlay(&self, row: u32, column: u32) -> Option<MirroredExceptionState> {
        let member = self
            .member(row, column)
            .map(str::to_owned)
            .unwrap_or_else(|| Self::canonical_member(&self.matrix.id, row, column));
        self.exception(MirroredItemKind::Part, &member)
    }

    /// Matrices directly anchored to this matrix, in document order.
    fn dependants(&self) -> Vec<&Matrix> {
        self.document
            .matrices
            .iter()
            .filter(|matrix| {
                matrix
                    .anchor
                    .as_ref()
                    .is_some_and(|anchor| anchor.matrix_id == self.matrix.id)
            })
            .collect()
    }

    /// Matrices directly anchored to this key; their poses follow its result.
    fn key_dependants(&self, row: u32, column: u32) -> Vec<&Matrix> {
        self.document
            .matrices
            .iter()
            .filter(|matrix| {
                matrix.anchor.as_ref().is_some_and(|anchor| {
                    anchor.matrix_id == self.matrix.id
                        && anchor.row == row
                        && anchor.column == column
                })
            })
            .collect()
    }

    /// "X, Y, rotation" for the Follow subset, or "none" when nothing follows.
    fn follow_text(follow: &Follow) -> String {
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

    /// "row 2, column 2 of matrix main (follows X, Y)".
    fn anchor_key_text(anchor: &MatrixAnchor) -> String {
        format!(
            "row {}, column {} of matrix {} (follows {})",
            anchor.row + 1,
            anchor.column + 1,
            anchor.matrix_id,
            Self::follow_text(&anchor.follow),
        )
    }

    fn anchor_target(&self, anchor: &MatrixAnchor) -> TreeContext {
        TreeContext::Key {
            matrix_id: anchor.matrix_id.clone(),
            row: anchor.row,
            column: anchor.column,
        }
    }

    /// The anchor frame item for row/column/key chains: the anchor key this
    /// matrix follows, with its Follow subset. `None` when unanchored.
    fn anchor_frame_item(&self) -> Option<RelationItem> {
        let anchor = self.matrix.anchor.as_ref()?;
        let highlight: Vec<String> = self
            .member_of(&anchor.matrix_id, anchor.row, anchor.column)
            .map(str::to_owned)
            .into_iter()
            .collect();
        Some(item(
            "Anchor frame",
            Self::anchor_key_text(anchor),
            Some(self.anchor_target(anchor)),
            highlight,
        ))
    }

    fn members(&self, keep: impl Fn(u32, u32) -> bool) -> Vec<String> {
        self.scene
            .cells
            .iter()
            .filter(|cell| keep(cell.row, cell.column))
            .filter_map(|cell| cell.member_id.clone())
            .collect()
    }

    fn companions_of(&self, key_id: &str) -> Vec<&MatrixCompanionPlacement> {
        self.scene
            .companions
            .iter()
            .filter(|companion| companion.key_id == key_id)
            .collect()
    }

    fn label(&self, part_id: &str) -> String {
        self.document
            .parts
            .iter()
            .find(|part| part.id == part_id)
            .map_or_else(|| part_id.to_owned(), |part| part.reference.clone())
    }

    /// Where a part lives in the tree: a key, a matrix companion, or a plain component.
    fn context_of(&self, part_id: &str) -> TreeContext {
        if let Some(cell) = self
            .scene
            .cells
            .iter()
            .find(|cell| cell.member_id.as_deref() == Some(part_id))
        {
            return TreeContext::Key {
                matrix_id: self.matrix.id.clone(),
                row: cell.row,
                column: cell.column,
            };
        }
        let key = self
            .scene
            .companions
            .iter()
            .find(|companion| companion.part_id == part_id)
            .and_then(|companion| {
                self.scene
                    .cells
                    .iter()
                    .find(|cell| cell.member_id.as_deref() == Some(companion.key_id.as_str()))
            });
        TreeContext::Component {
            part_id: Some(part_id.to_owned()),
            matrix_id: key.map(|_| self.matrix.id.clone()),
            row: key.map(|cell| cell.row),
            column: key.map(|cell| cell.column),
            assembly_id: None,
        }
    }

    fn reference(&self, part_id: &str) -> PartReference {
        PartReference {
            part_id: part_id.to_owned(),
            label: self.label(part_id),
            context: self.context_of(part_id),
        }
    }

    fn partner(&self) -> Option<(&boardstudio_core::model::Layout, &Matrix)> {
        let owner = self
            .document
            .layouts
            .iter()
            .find(|layout| layout.matrix_ids.contains(&self.matrix.id))?;
        let partner = match &owner.mirror_link {
            Some(link) => self
                .document
                .layouts
                .iter()
                .find(|layout| layout.id == link.source_id)?,
            None => self.document.layouts.iter().find(|layout| {
                layout
                    .mirror_link
                    .as_ref()
                    .is_some_and(|link| link.source_id == owner.id)
            })?,
        };
        let matrix = self.document.matrices.iter().find(|matrix| {
            let link = owner.mirror_link.as_ref().or(partner.mirror_link.as_ref());
            link.and_then(|link| link.partner_matrix(&self.matrix.id)) == Some(matrix.id.as_str())
        })?;
        Some((partner, matrix))
    }

    fn column_context(&self, column: u32) -> TreeContext {
        TreeContext::Column {
            matrix_id: self.matrix.id.clone(),
            column,
        }
    }

    fn row_context(&self, row: u32) -> TreeContext {
        TreeContext::Row {
            matrix_id: self.matrix.id.clone(),
            row,
        }
    }

    fn stagger(&self, column: u32) -> f64 {
        self.matrix
            .column_layout
            .get(column as usize)
            .map(|record| &record.stagger)
            .copied()
            .unwrap_or(0.0)
    }

    fn cumulative(&self, column: u32) -> f64 {
        (0..=column).map(|index| self.stagger(index)).sum()
    }

    fn splay(&self, column: u32) -> f64 {
        self.matrix
            .column_layout
            .get(column as usize)
            .map(|record| &record.splay)
            .copied()
            .unwrap_or(0.0)
    }

    fn column_turn(&self, column: u32) -> (f64, f64) {
        let record = self.matrix.column_layout.get(column as usize);
        (
            record.map(|record| record.splay).unwrap_or(0.0),
            record.map(|record| record.rotation).unwrap_or(0.0),
        )
    }

    /// The column's own step: Spread (absent on column 0, otherwise the pitch
    /// unless explicitly stored) and stagger, before any turns.
    fn column_step_text(&self, column: u32) -> String {
        let stagger = format!("{} mm", mm(self.stagger(column)));
        if column == 0 {
            return format!("no spread, stagger {stagger}");
        }
        let spread = self
            .matrix
            .column_layout
            .get(column as usize)
            .and_then(|record| record.spread)
            .map(|spread| format!("{} mm", mm(spread)))
            .unwrap_or_else(|| format!("{} mm (pitch)", mm(self.matrix.pitch.x)));
        format!("spread {spread}, stagger {stagger}")
    }

    /// This column's own turn: Splay plus Column rotation, which never carry to
    /// later columns.
    fn column_turn_text(&self, column: u32) -> String {
        let (splay, rotation) = self.column_turn(column);
        if splay == 0.0 && rotation == 0.0 && !self.custom_origin(column) {
            return "none (this column only)".to_owned();
        }
        let mut text = format!(
            "splay {}°, column rotation {}° (this column only)",
            mm(splay),
            mm(rotation)
        );
        if self.custom_origin(column) {
            text.push_str(", custom origin");
        }
        text
    }

    /// The padding step into this row: stored per row, defaulting to the pitch.
    /// Row 0 has no padding step.
    fn row_padding_text(&self, row: u32) -> String {
        if row == 0 {
            return "no padding on row 1".to_owned();
        }
        self.matrix
            .row_layout
            .get(row as usize)
            .and_then(|record| record.padding)
            .map(|padding| format!("{} mm", mm(padding)))
            .unwrap_or_else(|| format!("{} mm (pitch)", mm(self.matrix.pitch.y)))
    }

    /// This column's Spread: absent on column 0, the pitch unless explicitly
    /// stored.
    fn column_spread_text(&self, column: u32) -> String {
        if column == 0 {
            return "no spread on column 1".to_owned();
        }
        self.matrix
            .column_layout
            .get(column as usize)
            .and_then(|record| record.spread)
            .map(|spread| format!("{} mm", mm(spread)))
            .unwrap_or_else(|| format!("{} mm (pitch)", mm(self.matrix.pitch.x)))
    }

    fn custom_origin(&self, column: u32) -> bool {
        self.matrix
            .column_layout
            .get(column as usize)
            .map(|record| &record.splay_origin)
            .is_some_and(|origin| *origin != Vec2::default())
    }

    fn column_offset(&self, column: u32) -> Vec2 {
        self.matrix
            .column_layout
            .get(column as usize)
            .map(|record| &record.offset)
            .copied()
            .unwrap_or(ZERO)
    }

    fn row_offset(&self, row: u32) -> Vec2 {
        self.matrix
            .row_layout
            .get(row as usize)
            .map(|record| &record.offset)
            .copied()
            .unwrap_or(ZERO)
    }

    /// Constraints whose source or target is one of `parts` or an attached companion of one.
    fn constraints_touching(&self, parts: &[String]) -> Vec<ConstraintItem> {
        let mut touched: Vec<&str> = parts.iter().map(String::as_str).collect();
        for part in parts {
            touched.extend(
                self.companions_of(part)
                    .into_iter()
                    .map(|companion| companion.part_id.as_str()),
            );
        }
        self.document
            .constraints
            .iter()
            .filter(|constraint| {
                touched.contains(&constraint.source()) || touched.contains(&constraint.target())
            })
            .map(|constraint| self.constraint_item(constraint))
            .collect()
    }

    fn constraint_item(&self, constraint: &Constraint) -> ConstraintItem {
        let takes_precedence = self.scene.companions.iter().any(|companion| {
            matches!(&companion.mode,
                MatrixCompanionMode::Constrained { constraint_id, .. }
                    if constraint_id == constraint.id())
        });
        ConstraintItem {
            id: constraint.id().to_owned(),
            kind: match constraint {
                Constraint::Offset { .. } => ConstraintKind::Offset,
                Constraint::PartOffset { .. } => ConstraintKind::PartOffset,
                Constraint::Mirror { .. } => ConstraintKind::Mirror,
                Constraint::Framed {
                    relation: ConstraintRelation::Offset { .. },
                    ..
                } => ConstraintKind::Offset,
                Constraint::Framed {
                    relation: ConstraintRelation::PartOffset { .. },
                    ..
                } => ConstraintKind::PartOffset,
                Constraint::Framed {
                    relation: ConstraintRelation::MirrorLine { .. },
                    ..
                } => ConstraintKind::MirrorLine,
            },
            source_endpoint: match constraint {
                Constraint::Framed { source, .. } => source.clone(),
                _ => ConstraintEndpoint::Part {
                    part_id: constraint.source().to_owned(),
                },
            },
            target_endpoint: match constraint {
                Constraint::Framed { target, .. } => target.clone(),
                _ => ConstraintEndpoint::Part {
                    part_id: constraint.target().to_owned(),
                },
            },
            relation: match constraint {
                Constraint::Framed { relation, .. } => Some(relation.clone()),
                _ => None,
            },
            source: self.reference(constraint.source()),
            target: self.reference(constraint.target()),
            takes_precedence,
        }
    }

    fn mode_text(&self, mode: &MatrixCompanionMode) -> String {
        match mode {
            MatrixCompanionMode::Follows => "follows key".to_owned(),
            MatrixCompanionMode::Authored => "authored position relative to key".to_owned(),
            MatrixCompanionMode::Constrained { source_part_id, .. } => format!(
                "explicit constraint takes priority (source {})",
                self.label(source_part_id)
            ),
        }
    }

    fn matrix(&self) -> MatrixRelations {
        let matrix = self.matrix;
        let (mut enabled, mut disabled, mut removed) = (0u32, 0u32, 0u32);
        for row in 0..matrix.rows {
            for column in 0..matrix.columns {
                match self.key_state(row, column) {
                    KeyState::Enabled => enabled += 1,
                    KeyState::Disabled => disabled += 1,
                    KeyState::Removed => removed += 1,
                }
            }
        }
        let mut size = format!("{} × {}, {} keys", matrix.rows, matrix.columns, enabled);
        if disabled > 0 {
            size.push_str(&format!(", {disabled} disabled"));
        }
        if removed > 0 {
            size.push_str(&format!(", {removed} removed"));
        }
        let all = self.members(|_, _| true);
        let mut structure = vec![
            item("Matrix", size, None, all.clone()),
            item(
                "Pitch",
                format!("{} × {} mm", mm(matrix.pitch.x), mm(matrix.pitch.y)),
                None,
                vec![],
            ),
        ];
        if let Some(gap) = matrix.edge_gap {
            structure.push(item(
                "Edge gap",
                format!("{} × {} mm", mm(gap.x), mm(gap.y)),
                None,
                vec![],
            ));
        }
        structure.push(item(
            "Origin",
            format!("{}, {} mm", mm(matrix.origin.x), mm(matrix.origin.y)),
            None,
            vec![],
        ));
        if let Some(rotation) = matrix.rotation.filter(|angle| *angle != 0.0) {
            structure.push(item("Rotation", format!("{}°", mm(rotation)), None, vec![]));
        }
        match matrix.mirror {
            Some(Mirror::X) => structure.push(item("Mirror", "X".into(), None, vec![])),
            Some(Mirror::Y) => structure.push(item("Mirror", "Y".into(), None, vec![])),
            _ => {}
        }
        // The anchor frame comes first in kernel order: an anchored matrix
        // derives its world frame from its anchor key, then follows it.
        if let Some(anchor) = &matrix.anchor {
            structure.push(parameter(
                "Anchor",
                Self::anchor_key_text(anchor),
                Some(TreeContext::Matrix {
                    matrix_id: matrix.id.clone(),
                }),
                all.clone(),
            ));
            let mut follows = Self::anchor_key_text(anchor);
            if let Some(member) = self.member_of(&anchor.matrix_id, anchor.row, anchor.column)
                && self.exception(MirroredItemKind::Part, member)
                    == Some(MirroredExceptionState::Local)
            {
                follows.push_str("; the anchor key is Local and holds its final pose");
            }
            structure.push(item(
                "Follows",
                follows,
                Some(self.anchor_target(anchor)),
                self.member_of(&anchor.matrix_id, anchor.row, anchor.column)
                    .map(str::to_owned)
                    .into_iter()
                    .collect(),
            ));
        }
        for dependant in self.dependants() {
            structure.push(item(
                "Followed by",
                format!("Matrix {}", dependant.id),
                Some(TreeContext::Matrix {
                    matrix_id: dependant.id.clone(),
                }),
                self.scene_members(&dependant.id),
            ));
        }
        match self.exception(MirroredItemKind::Matrix, &matrix.id) {
            Some(MirroredExceptionState::Local) => structure.push(item(
                "Local matrix",
                "keeps its own dimensions and layout; its anchor stays detached".into(),
                None,
                all.clone(),
            )),
            Some(MirroredExceptionState::Omitted) => structure.push(item(
                "Omitted matrix",
                "live logical poses only; inherited parts stay suppressed".into(),
                None,
                Vec::new(),
            )),
            None => {}
        }
        // Shared values across the matrix, or "Varies" when members disagree.
        // No per-key value is presented as universal.
        let keys = self.all_keys();
        let spreads: Vec<String> = (1..matrix.columns)
            .map(|column| {
                matrix
                    .column_layout
                    .get(column as usize)
                    .and_then(|record| record.spread)
                    .map(|spread| format!("{} mm", mm(spread)))
                    .unwrap_or_else(|| format!("{} mm (pitch)", mm(matrix.pitch.x)))
            })
            .collect();
        structure.push(item(
            "Column spread",
            if matrix.columns <= 1 {
                "no spread (single column)".to_owned()
            } else {
                uniform_or_varies(spreads)
            },
            None,
            all.clone(),
        ));
        structure.push(item(
            "Column stagger",
            uniform_or_varies(
                (0..matrix.columns)
                    .map(|column| format!("{} mm", mm(self.stagger(column))))
                    .collect(),
            ),
            None,
            all.clone(),
        ));
        structure.push(item(
            "Column splay",
            uniform_or_varies(
                (0..matrix.columns)
                    .map(|column| format!("{}°", mm(self.column_turn(column).0)))
                    .collect(),
            ),
            None,
            all.clone(),
        ));
        structure.push(item(
            "Column rotation",
            uniform_or_varies(
                (0..matrix.columns)
                    .map(|column| format!("{}°", mm(self.column_turn(column).1)))
                    .collect(),
            ),
            None,
            all.clone(),
        ));
        let paddings: Vec<String> = (1..matrix.rows)
            .map(|row| {
                matrix
                    .row_layout
                    .get(row as usize)
                    .and_then(|record| record.padding)
                    .map(|padding| format!("{} mm", mm(padding)))
                    .unwrap_or_else(|| format!("{} mm (pitch)", mm(matrix.pitch.y)))
            })
            .collect();
        structure.push(item(
            "Row padding",
            if matrix.rows <= 1 {
                "no padding (single row)".to_owned()
            } else {
                uniform_or_varies(paddings)
            },
            None,
            all.clone(),
        ));
        structure.push(item(
            "Row shift",
            uniform_or_varies(
                (0..matrix.rows)
                    .map(|row| offset_text(self.row_stored_shift(row)))
                    .collect(),
            ),
            None,
            all.clone(),
        ));
        structure.push(item(
            "Row bend",
            uniform_or_varies(
                (0..matrix.rows)
                    .map(|row| bend_text(self.row_stored_bend(row)))
                    .collect(),
            ),
            None,
            all.clone(),
        ));
        structure.push(item("Key shift", aggregate_shift(&keys), None, all.clone()));
        structure.push(item("Key bend", aggregate_bend(&keys), None, all.clone()));
        structure.push(item(
            "Cumulative shift",
            aggregate_cumulative_shift(&keys),
            None,
            all.clone(),
        ));
        structure.push(item(
            "Cumulative bend",
            aggregate_cumulative_bend(&keys),
            None,
            all.clone(),
        ));
        for column in 0..matrix.columns {
            let members = self.members(|_, c| c == column);
            if self.stagger(column) != 0.0 {
                structure.push(item(
                    &format!("Column {} stagger", column + 1),
                    format!("{} mm", mm(self.stagger(column))),
                    Some(self.column_context(column)),
                    members.clone(),
                ));
            }
            if self.splay(column) != 0.0 {
                structure.push(item(
                    &format!("Column {} splay", column + 1),
                    format!("{}°", mm(self.splay(column))),
                    Some(self.column_context(column)),
                    members,
                ));
            }
        }
        structure.push(match self.partner() {
            Some((layout, _)) => item("Linked half", layout.name.clone(), None, vec![]),
            None => item("Linked half", "Independent".into(), None, vec![]),
        });

        let mut departures = Vec::new();
        let rows = (0..matrix.rows)
            .filter(|row| self.row_offset(*row) != ZERO)
            .count();
        let columns = (0..matrix.columns)
            .filter(|column| self.column_offset(*column) != ZERO)
            .count();
        let local = matrix
            .cells
            .iter()
            .filter(|cell| {
                !cell.deleted
                    && (cell.offset.is_some_and(|offset| offset != ZERO)
                        || cell.rotation.is_some_and(|angle| angle != 0.0))
            })
            .count();
        let authored = self
            .scene
            .companions
            .iter()
            .filter(|companion| companion.mode == MatrixCompanionMode::Authored)
            .count();
        for (parameter, count) in [
            ("Rows with an offset", rows),
            ("Columns with an offset", columns),
            ("Keys with a local offset or rotation", local),
            ("Authored companions", authored),
        ] {
            if count > 0 {
                departures.push(Departure {
                    parameter: parameter.to_owned(),
                    value: count.to_string(),
                    target: None,
                    highlight: vec![],
                    actions: vec![],
                });
            }
        }
        MatrixRelations {
            structure,
            departures,
            constraints: self.constraints_touching(&all),
            selection_note: None,
        }
    }

    /// Stored and Core-published facts for one logical key. Stored shift/bend
    /// come from the document (so deleted keys keep their carried values);
    /// cumulative totals come from Core's scene (so only scene-present keys
    /// have them).
    fn key_facts(&self, row: u32, column: u32) -> AggregatedKey {
        let stored = self
            .matrix
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column);
        let published = self.contributions(row, column);
        AggregatedKey {
            shift: stored.and_then(|cell| cell.shift).unwrap_or(ZERO),
            bend: stored.and_then(|cell| cell.bend).unwrap_or(0.0),
            cumulative_shift: published.map(|facts| facts.cumulative_shift),
            cumulative_bend: published.map(|facts| facts.cumulative_bend),
            state: self.key_state(row, column),
        }
    }

    fn row_keys(&self, row: u32) -> Vec<AggregatedKey> {
        (0..self.matrix.columns)
            .map(|column| self.key_facts(row, column))
            .collect()
    }

    fn column_keys(&self, column: u32) -> Vec<AggregatedKey> {
        (0..self.matrix.rows)
            .map(|row| self.key_facts(row, column))
            .collect()
    }

    fn all_keys(&self) -> Vec<AggregatedKey> {
        (0..self.matrix.rows)
            .flat_map(|row| (0..self.matrix.columns).map(move |column| (row, column)))
            .map(|(row, column)| self.key_facts(row, column))
            .collect()
    }

    fn row_stored_shift(&self, row: u32) -> Vec2 {
        self.matrix
            .row_layout
            .get(row as usize)
            .map(|record| record.shift)
            .unwrap_or(ZERO)
    }

    fn row_stored_bend(&self, row: u32) -> f64 {
        self.matrix
            .row_layout
            .get(row as usize)
            .map(|record| record.bend)
            .unwrap_or(0.0)
    }

    fn row(&self, row: u32) -> Option<MatrixRelations> {
        if row >= self.matrix.rows {
            return None;
        }
        let members = self.members(|r, _| r == row);
        let keys = self.row_keys(row);
        let (mut enabled, mut disabled, mut removed) = (0u32, 0u32, 0u32);
        for key in &keys {
            match key.state {
                KeyState::Enabled => enabled += 1,
                KeyState::Disabled => disabled += 1,
                KeyState::Removed => removed += 1,
            }
        }
        let mut keys_text = format!("{enabled} keys in Row {}", row + 1);
        if disabled > 0 {
            keys_text.push_str(&format!(", {disabled} disabled"));
        }
        if removed > 0 {
            keys_text.push_str(&format!(", {removed} removed"));
        }
        let mut structure = vec![item("Keys", keys_text, None, members.clone())];
        // The anchor frame heads the chain: this row inherits a frame derived
        // from the anchor key.
        if let Some(frame) = self.anchor_frame_item() {
            structure.push(frame);
        }
        structure.push(parameter(
            "Padding",
            self.row_padding_text(row),
            (row > 0).then(|| self.row_context(row)),
            members.clone(),
        ));
        structure.push(parameter(
            "Row shift",
            offset_text(self.row_stored_shift(row)),
            Some(self.row_context(row)),
            members.clone(),
        ));
        structure.push(parameter(
            "Row bend",
            bend_text(self.row_stored_bend(row)),
            Some(self.row_context(row)),
            members.clone(),
        ));
        structure.push(item(
            "Key shift",
            aggregate_shift(&keys),
            None,
            members.clone(),
        ));
        structure.push(item(
            "Key bend",
            aggregate_bend(&keys),
            None,
            members.clone(),
        ));
        structure.push(item(
            "Cumulative shift",
            aggregate_cumulative_shift(&keys),
            None,
            members.clone(),
        ));
        structure.push(item(
            "Cumulative bend",
            aggregate_cumulative_bend(&keys),
            None,
            members.clone(),
        ));
        // Padding carries to later rows, so later rows inherit this row's steps.
        for later in row + 1..self.matrix.rows {
            structure.push(item(
                "Inherited by",
                format!("Row {}", later + 1),
                Some(self.row_context(later)),
                self.members(|r, _| r == later),
            ));
        }
        let offset = self.row_offset(row);
        let departures = (offset != ZERO)
            .then(|| Departure {
                parameter: "Row offset".into(),
                value: format!("{}, {} mm", mm(offset.x), mm(offset.y)),
                target: Some(self.row_context(row)),
                highlight: members.clone(),
                actions: vec![DepartureAction::Reset],
            })
            .into_iter()
            .collect();
        Some(MatrixRelations {
            structure,
            departures,
            constraints: self.constraints_touching(&members),
            selection_note: None,
        })
    }

    fn column(&self, column: u32) -> Option<MatrixRelations> {
        if column >= self.matrix.columns {
            return None;
        }
        let members = self.members(|_, c| c == column);
        let stagger = self.stagger(column);
        let cumulative = self.cumulative(column);
        let reference = if column == 0 {
            "the matrix origin".to_owned()
        } else {
            format!("Column {column}")
        };
        let mut value = format!("{} mm from {reference}", mm(stagger));
        if cumulative != 0.0 {
            value.push_str(&format!(", cumulative {} mm", mm(cumulative)));
        }
        // Stagger totals are steps before turns, not final key positions.
        value.push_str(", before turns");
        let keys = self.column_keys(column);
        let mut structure = vec![item("Stagger", value, None, members.clone())];
        // The anchor frame heads the chain: this column inherits a frame
        // derived from the anchor key.
        if let Some(frame) = self.anchor_frame_item() {
            structure.push(frame);
        }
        structure.push(parameter(
            "Column spread",
            self.column_spread_text(column),
            Some(self.column_context(column)),
            members.clone(),
        ));
        structure.push(parameter(
            "Column turn",
            self.column_turn_text(column),
            Some(self.column_context(column)),
            members.clone(),
        ));
        structure.push(item(
            "Key shift",
            aggregate_shift(&keys),
            None,
            members.clone(),
        ));
        structure.push(item(
            "Key bend",
            aggregate_bend(&keys),
            None,
            members.clone(),
        ));
        structure.push(item(
            "Cumulative shift",
            aggregate_cumulative_shift(&keys),
            None,
            members.clone(),
        ));
        structure.push(item(
            "Cumulative bend",
            aggregate_cumulative_bend(&keys),
            None,
            members.clone(),
        ));
        for later in column + 1..self.matrix.columns {
            structure.push(item(
                "Inherited by",
                format!("Column {}", later + 1),
                Some(self.column_context(later)),
                self.members(|_, c| c == later),
            ));
        }
        let offset = self.column_offset(column);
        let departures = (offset != ZERO)
            .then(|| Departure {
                parameter: "Column offset".into(),
                value: format!("{}, {} mm", mm(offset.x), mm(offset.y)),
                target: Some(self.column_context(column)),
                highlight: members.clone(),
                // Convert moves the Y offset into stagger; an x-only offset has none.
                actions: if offset.y != 0.0 {
                    vec![DepartureAction::Reset, DepartureAction::Convert]
                } else {
                    vec![DepartureAction::Reset]
                },
            })
            .into_iter()
            .collect();
        Some(MatrixRelations {
            structure,
            departures,
            constraints: self.constraints_touching(&members),
            selection_note: None,
        })
    }

    fn contributions(&self, row: u32, column: u32) -> Option<CoreCellFacts> {
        self.scene
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .map(|cell| CoreCellFacts {
                row_shift: cell.row_shift,
                key_shift: cell.key_shift,
                row_bend: cell.row_bend,
                key_bend: cell.key_bend,
                cumulative_shift: cell.cumulative_shift,
                cumulative_bend: cell.cumulative_bend,
            })
    }

    fn key(&self, row: u32, column: u32) -> Option<MatrixRelations> {
        if row >= self.matrix.rows
            || column >= self.matrix.columns
            || self.key_state(row, column) == KeyState::Removed
        {
            return None;
        }
        let member = self.member(row, column).map(str::to_owned);
        let highlight: Vec<String> = member.iter().cloned().collect();
        let facts = self.contributions(row, column).unwrap_or_default();
        let row_offset = self.row_offset(row);
        let column_offset = self.column_offset(column);
        let row_members = self.members(|r, _| r == row);
        let column_members = self.members(|_, c| c == column);
        let mirror = match self.matrix.mirror {
            Some(Mirror::X) => "X",
            Some(Mirror::Y) => "Y",
            _ => "none",
        };
        // The chain follows kernel order: matrix frame, column step and turn,
        // row padding, carried row/key shift and bend with their matrix-axes
        // totals, then the independent offsets that never carry.
        let mut structure = vec![
            item(
                "Pitch cell",
                format!("Row {}, Column {}", row + 1, column + 1),
                None,
                highlight.clone(),
            ),
            parameter(
                "Matrix frame",
                format!(
                    "rotation {}°, mirror {mirror}",
                    mm(self.matrix.rotation.unwrap_or(0.0))
                ),
                Some(TreeContext::Matrix {
                    matrix_id: self.matrix.id.clone(),
                }),
                self.members(|_, _| true),
            ),
            parameter(
                "Column step",
                self.column_step_text(column),
                Some(self.column_context(column)),
                column_members.clone(),
            ),
            parameter(
                "Cumulative stagger",
                format!("{} mm", mm(self.cumulative(column))),
                Some(self.column_context(column)),
                column_members.clone(),
            ),
            parameter(
                "Column turn",
                self.column_turn_text(column),
                Some(self.column_context(column)),
                column_members.clone(),
            ),
            parameter(
                "Row padding",
                self.row_padding_text(row),
                (row > 0).then(|| self.row_context(row)),
                row_members.clone(),
            ),
            parameter(
                "Row shift",
                offset_text(facts.row_shift),
                Some(self.row_context(row)),
                row_members.clone(),
            ),
            item(
                "Key shift",
                offset_text(facts.key_shift),
                None,
                highlight.clone(),
            ),
            parameter(
                "Row bend",
                bend_text(facts.row_bend),
                Some(self.row_context(row)),
                row_members.clone(),
            ),
            item(
                "Key bend",
                bend_text(facts.key_bend),
                None,
                highlight.clone(),
            ),
            item(
                "Cumulative shift",
                format!(
                    "{}, {} mm in matrix axes",
                    mm(facts.cumulative_shift.x),
                    mm(facts.cumulative_shift.y)
                ),
                None,
                highlight.clone(),
            ),
            item(
                "Cumulative bend",
                format!("{}° (row and key only)", mm(facts.cumulative_bend)),
                None,
                highlight.clone(),
            ),
            parameter(
                "Row offset",
                offset_text(row_offset),
                Some(self.row_context(row)),
                row_members,
            ),
            parameter(
                "Column offset",
                offset_text(column_offset),
                Some(self.column_context(column)),
                column_members,
            ),
        ];
        if self.key_state(row, column) == KeyState::Disabled {
            structure.insert(
                1,
                item(
                    "Key state",
                    "disabled (stored values still carry)".into(),
                    None,
                    highlight.clone(),
                ),
            );
        }
        // Anchor frame right after the matrix frame: an anchored matrix derives
        // its world frame from its anchor key, then follows it.
        let mut insert_at = structure
            .iter()
            .position(|item| item.label == "Matrix frame")
            .map(|index| index + 1)
            .unwrap_or(1);
        if let Some(frame) = self.anchor_frame_item() {
            structure.insert(insert_at, frame);
            insert_at += 1;
        }
        match self.key_overlay(row, column) {
            Some(MirroredExceptionState::Local) => {
                structure.insert(
                    insert_at,
                    item(
                        "Local key",
                        "holds its final pose; carried fields stay shared".into(),
                        None,
                        highlight.clone(),
                    ),
                );
                insert_at += 1;
            }
            Some(MirroredExceptionState::Omitted) => {
                structure.insert(
                    insert_at,
                    item(
                        "Omitted key",
                        "live logical pose only; inherited parts stay suppressed".into(),
                        None,
                        Vec::new(),
                    ),
                );
                insert_at += 1;
            }
            None => {}
        }
        // An independent offset here can still move another matrix anchored to
        // this key's resulting pose; list that relationship directly.
        for dependant in self.key_dependants(row, column) {
            structure.insert(
                insert_at,
                item(
                    "Followed by",
                    format!("Matrix {}", dependant.id),
                    Some(TreeContext::Matrix {
                        matrix_id: dependant.id.clone(),
                    }),
                    self.scene_members(&dependant.id),
                ),
            );
            insert_at += 1;
        }
        if let Some((layout, partner)) = self.partner() {
            let partner_member = format!("matrix/{}/r{row}c{column}", partner.id);
            structure.push(item(
                "Linked partner",
                format!("{}, Row {}, Column {}", layout.name, row + 1, column + 1),
                Some(TreeContext::Key {
                    matrix_id: partner.id.clone(),
                    row,
                    column,
                }),
                vec![partner_member],
            ));
        }
        let companions = member
            .as_deref()
            .map(|key| self.companions_of(key))
            .unwrap_or_default();
        for companion in &companions {
            structure.push(item(
                &format!("Companion {}", self.label(&companion.part_id)),
                self.mode_text(&companion.mode),
                Some(self.context_of(&companion.part_id)),
                vec![companion.part_id.clone()],
            ));
        }

        let mut departures = Vec::new();
        let cell = self
            .matrix
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column);
        if let Some(offset) = cell.and_then(|cell| cell.offset).filter(|v| *v != ZERO) {
            departures.push(Departure {
                parameter: "Key local offset".into(),
                value: format!("{}, {} mm", mm(offset.x), mm(offset.y)),
                target: None,
                highlight: highlight.clone(),
                actions: vec![DepartureAction::Reset],
            });
        }
        if let Some(rotation) = cell.and_then(|cell| cell.rotation).filter(|v| *v != 0.0) {
            departures.push(Departure {
                parameter: "Key local rotation".into(),
                value: format!("{}°", mm(rotation)),
                target: None,
                highlight: highlight.clone(),
                actions: vec![DepartureAction::Reset],
            });
        }
        for companion in companions
            .iter()
            .filter(|companion| companion.mode == MatrixCompanionMode::Authored)
        {
            departures.push(Departure {
                parameter: "Authored companion".into(),
                value: self.label(&companion.part_id),
                target: Some(self.context_of(&companion.part_id)),
                highlight: vec![companion.part_id.clone()],
                actions: vec![],
            });
        }
        let touched: Vec<String> = member.into_iter().collect();
        Some(MatrixRelations {
            structure,
            departures,
            constraints: self.constraints_touching(&touched),
            selection_note: None,
        })
    }

    fn component(&self, part_id: &str) -> Option<MatrixRelations> {
        let companion = self
            .scene
            .companions
            .iter()
            .find(|companion| companion.part_id == part_id)?;
        let key = self
            .scene
            .cells
            .iter()
            .find(|cell| cell.member_id.as_deref() == Some(companion.key_id.as_str()))?;
        let structure = vec![
            item(
                "Attached to",
                self.label(&companion.key_id),
                Some(TreeContext::Key {
                    matrix_id: self.matrix.id.clone(),
                    row: key.row,
                    column: key.column,
                }),
                vec![companion.key_id.clone()],
            ),
            item(
                "Placement",
                self.mode_text(&companion.mode),
                None,
                vec![part_id.to_owned()],
            ),
        ];
        let departures = (companion.mode == MatrixCompanionMode::Authored)
            .then(|| Departure {
                parameter: "Authored position".into(),
                value: format!("relative to {}", self.label(&companion.key_id)),
                target: None,
                highlight: vec![part_id.to_owned()],
                actions: vec![],
            })
            .into_iter()
            .collect();
        let constraints = self
            .document
            .constraints
            .iter()
            .filter(|constraint| constraint.source() == part_id || constraint.target() == part_id)
            .map(|constraint| self.constraint_item(constraint))
            .collect();
        Some(MatrixRelations {
            structure,
            departures,
            constraints,
            selection_note: None,
        })
    }
}

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
