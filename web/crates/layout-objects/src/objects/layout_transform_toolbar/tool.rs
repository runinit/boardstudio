//! The transform tools and their activation rules. Plain logic: compiled natively for tests.
use super::super::layout_selection_kind::LayoutSelectionKind;
use crate::matrix_transform_operation::MatrixTransformField;
use boardstudio_web_ui_model::tree::TreeContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutTransformTool {
    Stagger,
    Spread,
    Offset,
    /// Carries one row or key sideways through its running frame.
    Shift,
    /// Turns one row's or one key's running frame, in degrees.
    Bend,
    Splay,
    Origin,
    /// Moves the selected matrix through its Origin.
    Move,
    /// Turns the selected matrix through its stored rotation.
    Rotate,
    /// Ticket 08: arms canvas Anchor pick mode on the selected matrix or
    /// part. The token toggles; it never starts a drag.
    Anchor,
}

/// Ticket 07: the native part-rail list maps onto the shared rail tools.
impl From<crate::part_rail::PartRailTool> for LayoutTransformTool {
    fn from(tool: crate::part_rail::PartRailTool) -> Self {
        match tool {
            crate::part_rail::PartRailTool::Move => Self::Move,
            crate::part_rail::PartRailTool::Rotate => Self::Rotate,
            crate::part_rail::PartRailTool::Spread => Self::Spread,
            crate::part_rail::PartRailTool::Stagger => Self::Stagger,
            crate::part_rail::PartRailTool::Anchor => Self::Anchor,
        }
    }
}

impl LayoutTransformTool {
    pub(super) const ALL: [(Self, &'static str); 9] = [
        (Self::Stagger, "Stagger"),
        (Self::Spread, "Spread"),
        (Self::Offset, "Offset"),
        (Self::Shift, "Shift"),
        (Self::Bend, "Bend"),
        (Self::Splay, "Splay"),
        (Self::Origin, "Origin"),
        (Self::Move, "Move"),
        (Self::Rotate, "Rotate"),
    ];
}

/// What choosing a menu tool does: the tool that is active afterwards and the selection
/// kind to switch to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolActivation {
    pub active: Option<LayoutTransformTool>,
    pub selection_kind: Option<LayoutSelectionKind>,
}

/// Ticket 06: the canvas label for one transform tool in popup and toast text.
pub(super) fn feedback_tool_name(tool: LayoutTransformTool) -> &'static str {
    match tool {
        LayoutTransformTool::Stagger => "Stagger",
        LayoutTransformTool::Spread => "Spread",
        LayoutTransformTool::Offset => "Offset",
        LayoutTransformTool::Shift => "Shift",
        LayoutTransformTool::Bend => "Bend",
        LayoutTransformTool::Splay => "Splay",
        LayoutTransformTool::Origin => "Origin",
        LayoutTransformTool::Move => "Move",
        LayoutTransformTool::Rotate => "Rotate",
        LayoutTransformTool::Anchor => "Anchor",
    }
}

/// The Inspector field whose amber preview matches one rail token's movers,
/// or `None` when the tool holds its keys still (Origin). Row Shift and Bend
/// carry downward like their Inspector fields; key Shift and Bend carry
/// upward through their column.
pub(super) fn preview_field_for_tool(
    tool: LayoutTransformTool,
    context: &TreeContext,
) -> Option<MatrixTransformField> {
    match (tool, context) {
        (LayoutTransformTool::Stagger, TreeContext::Column { .. }) => {
            Some(MatrixTransformField::ColumnStagger)
        }
        (LayoutTransformTool::Spread, _) => Some(MatrixTransformField::ColumnSpread),
        (LayoutTransformTool::Splay, _) => Some(MatrixTransformField::ColumnSplay),
        (LayoutTransformTool::Offset, TreeContext::Row { .. }) => {
            Some(MatrixTransformField::RowOffsetX)
        }
        (LayoutTransformTool::Offset, _) => Some(MatrixTransformField::ColumnOffsetX),
        (LayoutTransformTool::Shift, TreeContext::Row { .. }) => {
            Some(MatrixTransformField::RowShiftX)
        }
        (LayoutTransformTool::Shift, TreeContext::Key { .. }) => {
            Some(MatrixTransformField::KeyShiftX)
        }
        (LayoutTransformTool::Bend, TreeContext::Row { .. }) => Some(MatrixTransformField::RowBend),
        (LayoutTransformTool::Bend, TreeContext::Key { .. }) => Some(MatrixTransformField::KeyBend),
        (LayoutTransformTool::Move, _) => Some(MatrixTransformField::OriginX),
        (LayoutTransformTool::Rotate, _) => Some(MatrixTransformField::MatrixRotation),
        (LayoutTransformTool::Origin, _)
        | (LayoutTransformTool::Stagger, _)
        | (LayoutTransformTool::Shift, _)
        | (LayoutTransformTool::Anchor, _)
        | (LayoutTransformTool::Bend, _) => None,
    }
}

/// The activation rules for a menu tool. Choosing the active tool again turns it off and
/// leaves the selection alone. Offset keeps a Row selection and turns every other matrix
/// selection into its Column. Shift and Bend keep a Row or Key selection and turn every
/// other matrix selection into its Column. Stagger is a column parameter: a Row selection becomes the
/// column of the retained key when there is one (the conversion reads the retained cell)
/// and otherwise column 1, and every other selection becomes its Column. Splay and Origin
/// turn the selection into its Column. Move and Rotate keep the Matrix selection: they
/// edit the matrix itself, not one of its parameters.
pub fn activate_transform_tool(
    active: Option<LayoutTransformTool>,
    tool: LayoutTransformTool,
    context: &TreeContext,
) -> ToolActivation {
    if active == Some(tool) {
        return ToolActivation {
            active: None,
            selection_kind: None,
        };
    }
    match (tool, context) {
        (LayoutTransformTool::Offset, TreeContext::Row { .. }) => ToolActivation {
            active: Some(tool),
            selection_kind: Some(LayoutSelectionKind::Row),
        },
        (LayoutTransformTool::Shift | LayoutTransformTool::Bend, TreeContext::Row { .. }) => {
            ToolActivation {
                active: Some(tool),
                selection_kind: Some(LayoutSelectionKind::Row),
            }
        }
        (LayoutTransformTool::Shift | LayoutTransformTool::Bend, TreeContext::Key { .. }) => {
            ToolActivation {
                active: Some(tool),
                selection_kind: Some(LayoutSelectionKind::Key),
            }
        }
        (
            LayoutTransformTool::Move | LayoutTransformTool::Rotate | LayoutTransformTool::Anchor,
            _,
        ) => ToolActivation {
            active: Some(tool),
            selection_kind: None,
        },
        _ => ToolActivation {
            active: Some(tool),
            selection_kind: Some(LayoutSelectionKind::Column),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context_of(kind: &str) -> TreeContext {
        let matrix_id = "matrix".to_owned();
        match kind {
            "row" => TreeContext::Row { matrix_id, row: 1 },
            "column" => TreeContext::Column {
                matrix_id,
                column: 2,
            },
            "key" => TreeContext::Key {
                matrix_id,
                row: 1,
                column: 2,
            },
            _ => TreeContext::Matrix { matrix_id },
        }
    }

    #[test]
    fn activation_changes_the_selection_kind_per_tool() {
        use LayoutSelectionKind::{Column, Key, Row};
        use LayoutTransformTool::{
            Anchor, Bend, Move, Offset, Origin, Rotate, Shift, Splay, Stagger,
        };
        for (tool, selection, expected) in [
            // Offset keeps a Row and turns the rest into their Column.
            (Offset, "row", Row),
            (Offset, "column", Column),
            (Offset, "key", Column),
            (Offset, "matrix", Column),
            // Ticket 05: Shift and Bend keep a Row or a Key and turn the
            // rest into their Column.
            (Shift, "row", Row),
            (Shift, "key", Key),
            (Shift, "column", Column),
            (Shift, "matrix", Column),
            (Bend, "row", Row),
            (Bend, "key", Key),
            (Bend, "column", Column),
            (Bend, "matrix", Column),
            // Stagger is a column parameter, so a Row asks for a Column selection. Which
            // column it resolves to (the retained key's, or column 1) is the selection
            // conversion's rule (`context_for_selection_kind`), not covered here.
            (Stagger, "row", Column),
            (Stagger, "column", Column),
            (Stagger, "key", Column),
            (Splay, "row", Column),
            (Origin, "key", Column),
        ] {
            let activation = activate_transform_tool(None, tool, &context_of(selection));
            assert_eq!(
                activation,
                ToolActivation {
                    active: Some(tool),
                    selection_kind: Some(expected),
                },
                "{tool:?} on a {selection} selection"
            );
        }
        // Move, Rotate and Anchor edit or retarget the selection itself, so
        // every selection stays put: no selection change is requested.
        for tool in [Move, Rotate, Anchor] {
            for selection in ["row", "column", "key", "matrix"] {
                assert_eq!(
                    activate_transform_tool(None, tool, &context_of(selection)),
                    ToolActivation {
                        active: Some(tool),
                        selection_kind: None,
                    },
                    "{tool:?} on a {selection} selection"
                );
            }
        }
        // Choosing the active tool again turns it off and leaves the selection alone.
        assert_eq!(
            activate_transform_tool(
                Some(LayoutTransformTool::Offset),
                LayoutTransformTool::Offset,
                &context_of("row")
            ),
            ToolActivation {
                active: None,
                selection_kind: None
            }
        );
        // Switching tool replaces the active one.
        assert_eq!(
            activate_transform_tool(
                Some(LayoutTransformTool::Stagger),
                LayoutTransformTool::Offset,
                &context_of("row")
            )
            .active,
            Some(LayoutTransformTool::Offset)
        );
    }

    #[test]
    fn rail_tokens_preview_the_inspector_field_with_the_same_movers() {
        use LayoutTransformTool::*;
        use MatrixTransformField as Field;
        for (tool, selection, expected) in [
            (Stagger, "column", Some(Field::ColumnStagger)),
            (Stagger, "row", None),
            (Spread, "row", Some(Field::ColumnSpread)),
            (Splay, "key", Some(Field::ColumnSplay)),
            (Offset, "row", Some(Field::RowOffsetX)),
            (Offset, "column", Some(Field::ColumnOffsetX)),
            (Offset, "key", Some(Field::ColumnOffsetX)),
            (Shift, "row", Some(Field::RowShiftX)),
            (Shift, "key", Some(Field::KeyShiftX)),
            (Shift, "column", None),
            (Bend, "row", Some(Field::RowBend)),
            (Bend, "key", Some(Field::KeyBend)),
            (Bend, "matrix", None),
            (Move, "matrix", Some(Field::OriginX)),
            (Rotate, "matrix", Some(Field::MatrixRotation)),
            (Origin, "column", None),
            (Anchor, "matrix", None),
        ] {
            assert_eq!(
                preview_field_for_tool(tool, &context_of(selection)),
                expected,
                "{tool:?} on a {selection} selection"
            );
        }
    }

    #[test]
    fn feedback_names_every_tool_by_its_menu_label() {
        for (tool, label) in LayoutTransformTool::ALL {
            assert_eq!(feedback_tool_name(tool), label);
        }
        assert_eq!(feedback_tool_name(LayoutTransformTool::Anchor), "Anchor");
    }
}
