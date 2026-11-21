use super::*;

impl View<'_> {
    /// Stored and Core-published facts for one logical key. Stored shift/bend
    /// come from the document (so deleted keys keep their carried values);
    /// cumulative totals come from Core's scene (so only scene-present keys
    /// have them).
    pub(super) fn key_facts(&self, row: u32, column: u32) -> AggregatedKey {
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

    pub(super) fn row_keys(&self, row: u32) -> Vec<AggregatedKey> {
        (0..self.matrix.columns)
            .map(|column| self.key_facts(row, column))
            .collect()
    }

    pub(super) fn column_keys(&self, column: u32) -> Vec<AggregatedKey> {
        (0..self.matrix.rows)
            .map(|row| self.key_facts(row, column))
            .collect()
    }

    pub(super) fn all_keys(&self) -> Vec<AggregatedKey> {
        (0..self.matrix.rows)
            .flat_map(|row| (0..self.matrix.columns).map(move |column| (row, column)))
            .map(|(row, column)| self.key_facts(row, column))
            .collect()
    }

    pub(super) fn row_stored_shift(&self, row: u32) -> Vec2 {
        self.matrix
            .row_layout
            .get(row as usize)
            .map(|record| record.shift)
            .unwrap_or(ZERO)
    }

    pub(super) fn row_stored_bend(&self, row: u32) -> f64 {
        self.matrix
            .row_layout
            .get(row as usize)
            .map(|record| record.bend)
            .unwrap_or(0.0)
    }

    pub(super) fn row(&self, row: u32) -> Option<MatrixRelations> {
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
        // from the anchor key, or one entry per aggregate part.
        for frame in self.anchor_frame_items() {
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

    pub(super) fn column(&self, column: u32) -> Option<MatrixRelations> {
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
        // derived from the anchor key, or one entry per aggregate part.
        for frame in self.anchor_frame_items() {
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

    pub(super) fn contributions(&self, row: u32, column: u32) -> Option<CoreCellFacts> {
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

    pub(super) fn key(&self, row: u32, column: u32) -> Option<MatrixRelations> {
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
        for frame in self.anchor_frame_items() {
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
                        "live logical pose only; inherited parts stay suppressed. Clear the omission in the Key inspector to rejoin.".into(),
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
        // Ticket 13: independent parts anchored to this key move with it too.
        let mut anchored: Vec<&Part> = self
            .document
            .parts
            .iter()
            .filter(|part| {
                part.anchor.as_ref().is_some_and(|anchor| {
                    anchor.matrix_id == self.matrix.id
                        && anchor.row == row
                        && anchor.column == column
                })
            })
            .collect();
        anchored.sort_by(|a, b| a.id.cmp(&b.id));
        for part in anchored {
            structure.insert(
                insert_at,
                item(
                    "Anchors part",
                    self.label(&part.id),
                    Some(TreeContext::Component {
                        part_id: Some(part.id.clone()),
                        matrix_id: None,
                        row: None,
                        column: None,
                        assembly_id: None,
                    }),
                    vec![part.id.clone()],
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

    pub(super) fn component(&self, part_id: &str) -> Option<MatrixRelations> {
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
