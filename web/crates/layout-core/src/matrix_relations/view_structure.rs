use super::*;
use boardstudio_model::model::{ColumnLayout, RowLayout};

impl View<'_> {
    pub(super) fn key_state(&self, row: u32, column: u32) -> KeyState {
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

    pub(super) fn member(&self, row: u32, column: u32) -> Option<&str> {
        self.scene
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .and_then(|cell| cell.member_id.as_deref())
    }

    pub(super) fn scene_of(&self, matrix_id: &str) -> Option<&MatrixScene> {
        self.scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix_id)
    }

    /// The live member of another matrix's key, resolved against the current
    /// accepted scenes. `None` when the key has no live member, so references
    /// never highlight or navigate a stale identity.
    pub(super) fn member_of(&self, matrix_id: &str, row: u32, column: u32) -> Option<&str> {
        self.scene_of(matrix_id)?
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .and_then(|cell| cell.member_id.as_deref())
    }

    pub(super) fn scene_members(&self, matrix_id: &str) -> Vec<String> {
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

    /// Mirrors Core's exception lookup over same-board links and derived
    /// pairs, read-only: what Local/Omitted overlay covers this linked item,
    /// if any.
    pub(super) fn exception(
        &self,
        kind: MirroredItemKind,
        id: &str,
    ) -> Option<MirroredExceptionState> {
        self.document
            .layouts
            .iter()
            .filter_map(|layout| layout.mirror_link.as_ref())
            .flat_map(|link| &link.exceptions)
            .find(|exception| exception.kind == kind && exception.mirror_id == id)
            .map(|exception| exception.state)
            .or_else(|| {
                boardstudio_model::mirrored_half::derived_override_state(self.document, kind, id)
            })
    }

    /// The canonical member Core assigns a matrix slot. Only a fallback for
    /// naming a Local/Omitted overlay whose part is suppressed from the scene;
    /// never a navigation or highlight target.
    pub(super) fn canonical_member(matrix_id: &str, row: u32, column: u32) -> String {
        format!("matrix/{matrix_id}/r{row}c{column}")
    }

    /// Whether Core's `sync` would copy this matrix's edits to its partner:
    /// a Local/Omitted matrix never authors its partner, and a Local
    /// partner never takes the copy. Mirrors the sync gate, read-only.
    pub(super) fn matrix_linked(&self, matrix_id: &str) -> bool {
        !matches!(
            self.exception(MirroredItemKind::Matrix, matrix_id),
            Some(MirroredExceptionState::Local | MirroredExceptionState::Omitted)
        )
    }

    pub(super) fn key_overlay(&self, row: u32, column: u32) -> Option<MirroredExceptionState> {
        let member = self
            .member(row, column)
            .map(str::to_owned)
            .unwrap_or_else(|| Self::canonical_member(&self.matrix.id, row, column));
        self.exception(MirroredItemKind::Part, &member)
    }

    /// Matrices directly anchored to this matrix in document order: through the
    /// single key, or through any aggregate part.
    pub(super) fn dependants(&self) -> Vec<&Matrix> {
        self.document
            .matrices
            .iter()
            .filter(|matrix| {
                matrix.anchor.as_ref().is_some_and(|anchor| {
                    anchor
                        .targets()
                        .iter()
                        .any(|part| part.matrix_id == self.matrix.id)
                })
            })
            .collect()
    }

    /// Matrices directly anchored to this key; their poses follow its result.
    /// Aggregate parts count: a part anchored to this key moves with it.
    pub(super) fn key_dependants(&self, row: u32, column: u32) -> Vec<&Matrix> {
        self.document
            .matrices
            .iter()
            .filter(|matrix| {
                matrix.anchor.as_ref().is_some_and(|anchor| {
                    anchor.targets().iter().any(|part| {
                        part.matrix_id == self.matrix.id && part.row == row && part.column == column
                    })
                })
            })
            .collect()
    }

    /// "X, Y, rotation" for the Follow subset, or "none" when nothing follows.
    pub(super) fn follow_text(follow: &Follow) -> String {
        follow_text(follow)
    }

    /// "row 2, column 2 of matrix main".
    pub(super) fn anchor_part_text(matrix_id: &str, row: u32, column: u32) -> String {
        format!(
            "row {}, column {} of matrix {}",
            row + 1,
            column + 1,
            matrix_id,
        )
    }

    /// "row 2, column 2 of matrix main (follows X, Y)".
    pub(super) fn anchor_key_text(anchor: &MatrixAnchor) -> String {
        format!(
            "{} (follows {})",
            Self::anchor_part_text(&anchor.matrix_id, anchor.row, anchor.column),
            Self::follow_text(&anchor.follow),
        )
    }

    /// "Average of 2 keys" or "Intersection of 2 keys", from the Core-admitted
    /// aggregate method. `None` for single-key anchors.
    pub(super) fn aggregate_kind_text(anchor: &MatrixAnchor) -> Option<String> {
        aggregate_kind_text(anchor)
    }

    pub(super) fn anchor_target(&self, anchor: &MatrixAnchor) -> TreeContext {
        TreeContext::Key {
            matrix_id: anchor.matrix_id.clone(),
            row: anchor.row,
            column: anchor.column,
        }
    }

    /// The anchor frame item for row/column/key chains: the anchor key this
    /// matrix follows, with its Follow subset. `None` when unanchored.
    pub(super) fn anchor_frame_item(&self) -> Option<RelationItem> {
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

    /// The anchor frame items for row/column/key chains: the single anchor key
    /// this matrix follows, or one entry per aggregate part. Every target and
    /// highlight resolves against the live accepted scenes, never fabricated
    /// coordinates.
    pub(super) fn anchor_frame_items(&self) -> Vec<RelationItem> {
        let Some(anchor) = self.matrix.anchor.as_ref() else {
            return Vec::new();
        };
        let Some(aggregate) = anchor.aggregate.as_ref() else {
            return self.anchor_frame_item().into_iter().collect();
        };
        aggregate
            .parts
            .iter()
            .map(|part| {
                item(
                    "Anchor frame",
                    format!(
                        "{} (follows {})",
                        Self::anchor_part_text(&part.matrix_id, part.row, part.column),
                        Self::follow_text(&anchor.follow),
                    ),
                    Some(TreeContext::Key {
                        matrix_id: part.matrix_id.clone(),
                        row: part.row,
                        column: part.column,
                    }),
                    self.member_of(&part.matrix_id, part.row, part.column)
                        .map(str::to_owned)
                        .into_iter()
                        .collect(),
                )
            })
            .collect()
    }

    pub(super) fn members(&self, keep: impl Fn(u32, u32) -> bool) -> Vec<String> {
        self.scene
            .cells
            .iter()
            .filter(|cell| keep(cell.row, cell.column))
            .filter_map(|cell| cell.member_id.clone())
            .collect()
    }

    pub(super) fn companions_of(&self, key_id: &str) -> Vec<&MatrixCompanionPlacement> {
        self.scene
            .companions
            .iter()
            .filter(|companion| companion.key_id == key_id)
            .collect()
    }

    pub(super) fn label(&self, part_id: &str) -> String {
        self.document
            .parts
            .iter()
            .find(|part| part.id == part_id)
            .map_or_else(|| part_id.to_owned(), |part| part.reference.clone())
    }

    /// Where a part lives in the tree: a key, a matrix companion, or a plain component.
    pub(super) fn context_of(&self, part_id: &str) -> TreeContext {
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

    pub(super) fn reference(&self, part_id: &str) -> PartReference {
        PartReference {
            part_id: part_id.to_owned(),
            label: self.label(part_id),
            context: self.context_of(part_id),
        }
    }

    pub(super) fn partner(&self) -> Option<(&boardstudio_model::model::Layout, &Matrix)> {
        let owner = self
            .document
            .layouts
            .iter()
            .find(|layout| layout.matrix_ids.contains(&self.matrix.id))?;
        for pair in &self.document.mirrored_pairs {
            let (
                boardstudio_model::mirrored_half::MirrorDestination::SameBoardLayout(source_id),
                boardstudio_model::mirrored_half::MirrorDestination::SameBoardLayout(dest_id),
            ) = (&pair.source, &pair.destination)
            else {
                continue;
            };
            let prefix = boardstudio_model::mirrored_half::mirrored_matrix_id_prefix(&pair.id);
            let (partner_layout_id, partner_matrix_id) = if owner.id == *source_id {
                (dest_id.as_str(), format!("{prefix}{}", self.matrix.id))
            } else if owner.id == *dest_id {
                let Some(source_matrix_id) = self.matrix.id.strip_prefix(&prefix) else {
                    continue;
                };
                (source_id.as_str(), source_matrix_id.to_owned())
            } else {
                continue;
            };
            if let Some(partner_layout) = self
                .document
                .layouts
                .iter()
                .find(|layout| layout.id == partner_layout_id)
                && let Some(partner_matrix) = self
                    .document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == partner_matrix_id)
            {
                return Some((partner_layout, partner_matrix));
            }
        }
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

    pub(super) fn column_context(&self, column: u32) -> TreeContext {
        TreeContext::Column {
            matrix_id: self.matrix.id.clone(),
            column,
        }
    }

    fn column_record(&self, column: u32) -> Option<&ColumnLayout> {
        self.matrix.column_layout.get(column as usize)
    }

    fn row_record(&self, row: u32) -> Option<&RowLayout> {
        self.matrix.row_layout.get(row as usize)
    }

    /// This column's Spread, or the pitch when absent.
    fn spread_text(&self, column: u32) -> String {
        self.column_record(column)
            .and_then(|record| record.spread)
            .map(|spread| format!("{} mm", mm(spread)))
            .unwrap_or_else(|| format!("{} mm (pitch)", mm(self.matrix.pitch.x)))
    }

    /// This row's padding, or the pitch when absent.
    fn padding_text(&self, row: u32) -> String {
        self.row_record(row)
            .and_then(|record| record.padding)
            .map(|padding| format!("{} mm", mm(padding)))
            .unwrap_or_else(|| format!("{} mm (pitch)", mm(self.matrix.pitch.y)))
    }

    pub(super) fn row_context(&self, row: u32) -> TreeContext {
        TreeContext::Row {
            matrix_id: self.matrix.id.clone(),
            row,
        }
    }

    pub(super) fn stagger(&self, column: u32) -> f64 {
        self.column_record(column)
            .map(|record| record.stagger)
            .unwrap_or(0.0)
    }

    pub(super) fn cumulative(&self, column: u32) -> f64 {
        (0..=column).map(|index| self.stagger(index)).sum()
    }

    pub(super) fn splay(&self, column: u32) -> f64 {
        self.column_record(column)
            .map(|record| record.splay)
            .unwrap_or(0.0)
    }

    pub(super) fn column_turn(&self, column: u32) -> (f64, f64) {
        let record = self.column_record(column);
        (
            record.map(|record| record.splay).unwrap_or(0.0),
            record.map(|record| record.rotation).unwrap_or(0.0),
        )
    }

    /// The column's own step: Spread (absent on column 0, otherwise the pitch
    /// unless explicitly stored) and stagger, before any turns.
    pub(super) fn column_step_text(&self, column: u32) -> String {
        let stagger = format!("{} mm", mm(self.stagger(column)));
        if column == 0 {
            return format!("no spread, stagger {stagger}");
        }
        let spread = self.spread_text(column);
        format!("spread {spread}, stagger {stagger}")
    }

    /// This column's own turn: Splay plus Column rotation, which never carry to
    /// later columns.
    pub(super) fn column_turn_text(&self, column: u32) -> String {
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
    pub(super) fn row_padding_text(&self, row: u32) -> String {
        if row == 0 {
            return "no padding on row 1".to_owned();
        }
        self.padding_text(row)
    }

    /// This column's Spread: absent on column 0, the pitch unless explicitly
    /// stored.
    pub(super) fn column_spread_text(&self, column: u32) -> String {
        if column == 0 {
            return "no spread on column 1".to_owned();
        }
        self.spread_text(column)
    }

    pub(super) fn custom_origin(&self, column: u32) -> bool {
        self.column_record(column)
            .is_some_and(|record| record.splay_origin != Vec2::default())
    }

    pub(super) fn column_offset(&self, column: u32) -> Vec2 {
        self.column_record(column)
            .map(|record| record.offset)
            .unwrap_or(ZERO)
    }

    pub(super) fn row_offset(&self, row: u32) -> Vec2 {
        self.row_record(row)
            .map(|record| record.offset)
            .unwrap_or(ZERO)
    }

    /// Constraints whose source or target is one of `parts` or an attached companion of one.
    pub(super) fn constraints_touching(&self, parts: &[String]) -> Vec<ConstraintItem> {
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

    pub(super) fn constraint_item(&self, constraint: &Constraint) -> ConstraintItem {
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

    pub(super) fn mode_text(&self, mode: &MatrixCompanionMode) -> String {
        match mode {
            MatrixCompanionMode::Follows => "follows key".to_owned(),
            MatrixCompanionMode::Authored => "authored position relative to key".to_owned(),
            MatrixCompanionMode::Constrained { source_part_id, .. } => format!(
                "explicit constraint takes priority (source {})",
                self.label(source_part_id)
            ),
        }
    }

    pub(super) fn matrix(&self) -> MatrixRelations {
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
        // derives its world frame from its anchor key, then follows it. An
        // aggregate anchor derives one combined frame from N part poses, so
        // the chain names one entry per part from Core-published values only.
        if let Some(anchor) = &matrix.anchor {
            let matrix_context = Some(TreeContext::Matrix {
                matrix_id: matrix.id.clone(),
            });
            match anchor.aggregate.as_ref() {
                None => {
                    structure.push(parameter(
                        "Anchor",
                        Self::anchor_key_text(anchor),
                        matrix_context,
                        all.clone(),
                    ));
                    let mut follows = Self::anchor_key_text(anchor);
                    if let Some(member) =
                        self.member_of(&anchor.matrix_id, anchor.row, anchor.column)
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
                Some(aggregate) => {
                    let members: Vec<String> = aggregate
                        .parts
                        .iter()
                        .filter_map(|part| self.member_of(&part.matrix_id, part.row, part.column))
                        .map(str::to_owned)
                        .collect();
                    structure.push(parameter(
                        "Anchor",
                        Self::aggregate_kind_text(anchor)
                            .expect("the aggregate arm names its method"),
                        matrix_context,
                        all.clone(),
                    ));
                    for part in &aggregate.parts {
                        structure.push(item(
                            "Anchor part",
                            Self::anchor_part_text(&part.matrix_id, part.row, part.column),
                            Some(TreeContext::Key {
                                matrix_id: part.matrix_id.clone(),
                                row: part.row,
                                column: part.column,
                            }),
                            self.member_of(&part.matrix_id, part.row, part.column)
                                .map(str::to_owned)
                                .into_iter()
                                .collect(),
                        ));
                    }
                    let mut follows = Self::follow_text(&anchor.follow);
                    let mut local: Vec<String> = aggregate
                        .parts
                        .iter()
                        .filter(|part| {
                            self.member_of(&part.matrix_id, part.row, part.column)
                                .is_some_and(|member| {
                                    self.exception(MirroredItemKind::Part, member)
                                        == Some(MirroredExceptionState::Local)
                                })
                        })
                        .map(|part| Self::anchor_part_text(&part.matrix_id, part.row, part.column))
                        .collect();
                    local.sort();
                    local.dedup();
                    for text in local {
                        follows.push_str(&format!("; {text} is Local and holds its final pose"));
                    }
                    structure.push(item("Follows", follows, None, members));
                }
            }
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
            .map(|column| self.spread_text(column))
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
        let paddings: Vec<String> = (1..matrix.rows).map(|row| self.padding_text(row)).collect();
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
}
