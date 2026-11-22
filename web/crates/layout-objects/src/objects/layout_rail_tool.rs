//! The rail tools and their activation rules. Plain logic: compiled natively for tests.
use super::layout_selection_kind::LayoutSelectionKind;
use crate::matrix_transform_operation::MatrixTransformField;
use boardstudio_web_ui_model::tree::TreeContext;

/// The rail tools. Matrix selections offer the placement tools; an independent part offers Move,
/// Rotate, Spread and Stagger; both end in the Anchor pick token.
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
    /// Moves the selected matrix (or independent part) through its Origin.
    Move,
    /// Turns the selected matrix (or independent part).
    Rotate,
    /// Arms canvas Anchor pick mode; the token toggles and never starts a drag.
    Anchor,
}

impl From<crate::part_rail::PartRailTool> for LayoutTransformTool {
    fn from(tool: crate::part_rail::PartRailTool) -> Self {
        use crate::part_rail::PartRailTool as P;
        match tool {
            P::Move => Self::Move,
            P::Rotate => Self::Rotate,
            P::Spread => Self::Spread,
            P::Stagger => Self::Stagger,
            P::Anchor => Self::Anchor,
        }
    }
}

impl LayoutTransformTool {
    pub(super) const MENU: [(Self, &'static str); 9] = [
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Stagger => "Stagger",
            Self::Spread => "Spread",
            Self::Offset => "Offset",
            Self::Shift => "Shift",
            Self::Bend => "Bend",
            Self::Splay => "Splay",
            Self::Origin => "Origin",
            Self::Move => "Move",
            Self::Rotate => "Rotate",
            Self::Anchor => "Anchor",
        }
    }

    /// The Inspector field whose amber preview shows this token's movers, if the tool moves any.
    pub(super) fn preview_field(self, context: &TreeContext) -> Option<MatrixTransformField> {
        use MatrixTransformField as F;
        match (self, context) {
            (Self::Stagger, TreeContext::Column { .. }) => Some(F::ColumnStagger),
            (Self::Spread, _) => Some(F::ColumnSpread),
            (Self::Splay, _) => Some(F::ColumnSplay),
            (Self::Offset, TreeContext::Row { .. }) => Some(F::RowOffsetX),
            (Self::Offset, _) => Some(F::ColumnOffsetX),
            (Self::Shift, TreeContext::Row { .. }) => Some(F::RowShiftX),
            (Self::Shift, TreeContext::Key { .. }) => Some(F::KeyShiftX),
            (Self::Bend, TreeContext::Row { .. }) => Some(F::RowBend),
            (Self::Bend, TreeContext::Key { .. }) => Some(F::KeyBend),
            (Self::Move, _) => Some(F::OriginX),
            (Self::Rotate, _) => Some(F::MatrixRotation),
            _ => None,
        }
    }
}

/// What choosing a menu tool does: the tool that is active afterwards and the selection kind to
/// switch to. Choosing the active tool again turns it off. Offset keeps a Row selection; Shift and
/// Bend keep a Row or Key; Move, Rotate and Anchor keep the selection; everything else becomes the
/// selection's Column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolActivation {
    pub active: Option<LayoutTransformTool>,
    pub selection_kind: Option<LayoutSelectionKind>,
}

pub fn activate_transform_tool(
    active: Option<LayoutTransformTool>,
    tool: LayoutTransformTool,
    context: &TreeContext,
) -> ToolActivation {
    use LayoutSelectionKind as Kind;
    use LayoutTransformTool as Tool;
    if active == Some(tool) {
        return ToolActivation {
            active: None,
            selection_kind: None,
        };
    }
    let selection_kind = match (tool, context) {
        (Tool::Move | Tool::Rotate | Tool::Anchor, _) => None,
        (Tool::Offset | Tool::Shift | Tool::Bend, TreeContext::Row { .. }) => Some(Kind::Row),
        (Tool::Shift | Tool::Bend, TreeContext::Key { .. }) => Some(Kind::Key),
        _ => Some(Kind::Column),
    };
    ToolActivation {
        active: Some(tool),
        selection_kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LayoutSelectionKind as Kind;
    use LayoutTransformTool as Tool;

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
        for (tool, selection, expected) in [
            // Offset keeps a Row and turns the rest into their Column.
            (Tool::Offset, "row", Kind::Row),
            (Tool::Offset, "key", Kind::Column),
            (Tool::Offset, "matrix", Kind::Column),
            // Shift and Bend keep a Row or a Key and turn the rest into their Column.
            (Tool::Shift, "row", Kind::Row),
            (Tool::Shift, "key", Kind::Key),
            (Tool::Bend, "column", Kind::Column),
            // Stagger is a column parameter, so every selection asks for a Column.
            (Tool::Stagger, "row", Kind::Column),
            (Tool::Splay, "row", Kind::Column),
            (Tool::Origin, "key", Kind::Column),
        ] {
            assert_eq!(
                activate_transform_tool(None, tool, &context_of(selection)),
                ToolActivation {
                    active: Some(tool),
                    selection_kind: Some(expected)
                },
                "{tool:?} on a {selection} selection"
            );
        }
        // Move, Rotate and Anchor edit or retarget the selection itself, so no selection change is requested.
        for tool in [Tool::Move, Tool::Rotate, Tool::Anchor] {
            for selection in ["row", "column", "key", "matrix"] {
                assert_eq!(
                    activate_transform_tool(None, tool, &context_of(selection)),
                    ToolActivation {
                        active: Some(tool),
                        selection_kind: None
                    }
                );
            }
        }
        // Choosing the active tool again turns it off; switching replaces it.
        assert_eq!(
            activate_transform_tool(Some(Tool::Offset), Tool::Offset, &context_of("row")),
            ToolActivation {
                active: None,
                selection_kind: None
            }
        );
        assert_eq!(
            activate_transform_tool(Some(Tool::Stagger), Tool::Offset, &context_of("row")).active,
            Some(Tool::Offset)
        );
    }

    #[test]
    fn rail_tokens_preview_the_inspector_field_with_the_same_movers() {
        use MatrixTransformField as Field;
        for (tool, selection, expected) in [
            (Tool::Stagger, "column", Some(Field::ColumnStagger)),
            (Tool::Stagger, "row", None),
            (Tool::Offset, "row", Some(Field::RowOffsetX)),
            (Tool::Offset, "key", Some(Field::ColumnOffsetX)),
            (Tool::Shift, "key", Some(Field::KeyShiftX)),
            (Tool::Shift, "column", None),
            (Tool::Bend, "row", Some(Field::RowBend)),
            (Tool::Move, "matrix", Some(Field::OriginX)),
            (Tool::Rotate, "matrix", Some(Field::MatrixRotation)),
            (Tool::Origin, "column", None),
            (Tool::Anchor, "matrix", None),
        ] {
            assert_eq!(
                tool.preview_field(&context_of(selection)),
                expected,
                "{tool:?} on a {selection} selection"
            );
        }
    }

    #[test]
    fn the_menu_lists_every_drag_tool_by_its_label() {
        for (tool, label) in Tool::MENU {
            assert_eq!(tool.label(), label);
        }
        assert_eq!(Tool::Anchor.label(), "Anchor");
    }
}
