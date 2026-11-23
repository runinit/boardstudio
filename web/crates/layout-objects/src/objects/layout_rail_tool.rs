//! The rail tools and their activation rules. Plain logic: compiled natively for tests.
use super::layout_selection_kind::LayoutSelectionKind;
use crate::{matrix_transform_operation::MatrixTransformField, part_rail::PartRailFacts};
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

/// Ticket 07: `part` carries the mount site's part-rail facts for an
/// independent part selection (a `Component` with a part id and no matrix).
/// Matrix members and companions name themselves in the facts and keep their
/// matrix's tools.
pub fn transform_tools(
    context: &TreeContext,
    active: Option<LayoutTransformTool>,
    handles_visible: bool,
    part: Option<PartRailFacts>,
) -> Vec<LayoutTransformTool> {
    use crate::part_rail::part_rail_tools;
    use LayoutTransformTool as Tool;
    let part_tools = |facts: PartRailFacts| {
        part_rail_tools(facts)
            .into_iter()
            .map(Tool::from)
            .collect::<Vec<_>>()
    };
    if !handles_visible {
        // View-only: every passive handle hides, but the active tool keeps
        // the handle its gesture requires. Origin and Splay expose their
        // origin through the active overlay itself, so no extra overlay is
        // needed. With no active tool nothing draws.
        return match (context, active) {
            (TreeContext::Column { .. }, Some(tool))
            | (TreeContext::Row { .. }, Some(tool @ (Tool::Offset | Tool::Shift | Tool::Bend)))
            | (TreeContext::Key { .. }, Some(tool @ (Tool::Shift | Tool::Bend)))
            | (TreeContext::Matrix { .. }, Some(tool @ (Tool::Move | Tool::Rotate))) => vec![tool],
            // Ticket 07: a hidden part rail keeps only its active tool.
            (TreeContext::Component { .. }, Some(tool)) => part
                .filter(|facts| part_tools(*facts).contains(&tool))
                .map_or_else(Vec::new, |_| vec![tool]),
            _ => Vec::new(),
        };
    }
    match context {
        TreeContext::Column { .. } => match active {
            Some(Tool::Offset) => vec![Tool::Offset, Tool::Splay],
            Some(Tool::Origin) => vec![Tool::Stagger, Tool::Origin],
            Some(Tool::Spread) => vec![Tool::Spread, Tool::Splay],
            _ => vec![Tool::Stagger, Tool::Splay],
        },
        TreeContext::Row { .. } => vec![Tool::Offset, Tool::Shift, Tool::Bend],
        TreeContext::Key { .. } => vec![Tool::Shift, Tool::Bend],
        // Ticket 08: the rail of a matrix ends in the Anchor token. The
        // mount site only mounts it while the matrix is anchored (the
        // projection arms no pick without an anchor to retarget).
        TreeContext::Matrix { .. } => vec![Tool::Move, Tool::Rotate, Tool::Anchor],
        // Ticket 07: an independent part gets its own rail; without facts
        // (a member or companion) nothing draws.
        TreeContext::Component { .. } => part.map_or_else(Vec::new, part_tools),
        _ => vec![],
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
            "part" => TreeContext::Component {
                part_id: Some("mcu".into()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
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
    fn every_drag_tool_has_its_menu_label() {
        for (tool, label) in [
            (Tool::Stagger, "Stagger"),
            (Tool::Spread, "Spread"),
            (Tool::Offset, "Offset"),
            (Tool::Shift, "Shift"),
            (Tool::Bend, "Bend"),
            (Tool::Splay, "Splay"),
            (Tool::Origin, "Origin"),
            (Tool::Move, "Move"),
            (Tool::Rotate, "Rotate"),
        ] {
            assert_eq!(tool.label(), label);
        }
        assert_eq!(Tool::Anchor.label(), "Anchor");
    }

    #[test]
    fn explicit_offset_and_origin_activate_the_named_canvas_handle() {
        use LayoutTransformTool as Tool;
        let column = context_of("column");
        assert_eq!(
            transform_tools(&column, Some(Tool::Offset), true, None),
            vec![Tool::Offset, Tool::Splay]
        );
        assert_eq!(
            transform_tools(&column, Some(Tool::Origin), true, None),
            vec![Tool::Stagger, Tool::Origin]
        );
        assert_eq!(
            transform_tools(&column, None, true, None),
            vec![Tool::Stagger, Tool::Splay]
        );
        // Ticket 05: a Row keeps its shipped Offset next to Shift and Bend;
        // a Key offers Shift and Bend.
        assert_eq!(
            transform_tools(&context_of("row"), Some(Tool::Stagger), true, None,),
            vec![Tool::Offset, Tool::Shift, Tool::Bend]
        );
        assert_eq!(
            transform_tools(&context_of("row"), None, true, None,),
            vec![Tool::Offset, Tool::Shift, Tool::Bend]
        );
        assert_eq!(
            transform_tools(&context_of("key"), None, true, None,),
            vec![Tool::Shift, Tool::Bend]
        );
        assert_eq!(
            transform_tools(&context_of("matrix"), None, true, None,),
            vec![Tool::Move, Tool::Rotate, Tool::Anchor]
        );
        assert_eq!(
            transform_tools(&context_of("matrix"), Some(Tool::Move), true, None,),
            vec![Tool::Move, Tool::Rotate, Tool::Anchor]
        );
        assert_eq!(
            transform_tools(&context_of("matrix"), Some(Tool::Move), false, None,),
            vec![Tool::Move]
        );
        // Ticket 07: an independent part gets its own rail from the mount
        // site's facts; without facts (a member or companion) nothing draws.
        let part = context_of("part");
        let anchored = || {
            Some(PartRailFacts {
                anchored: true,
                anchor_column: 1,
                locked: false,
                member_or_companion: false,
            })
        };
        assert_eq!(
            transform_tools(&part, None, true, anchored()),
            vec![
                Tool::Move,
                Tool::Rotate,
                Tool::Spread,
                Tool::Stagger,
                Tool::Anchor,
            ]
        );
        assert_eq!(
            transform_tools(
                &part,
                None,
                true,
                Some(PartRailFacts {
                    anchored: true,
                    anchor_column: 0,
                    locked: false,
                    member_or_companion: false,
                }),
            ),
            vec![Tool::Move, Tool::Rotate, Tool::Stagger, Tool::Anchor]
        );
        assert_eq!(
            transform_tools(
                &part,
                None,
                true,
                Some(PartRailFacts {
                    anchored: false,
                    anchor_column: 0,
                    locked: false,
                    member_or_companion: false,
                }),
            ),
            vec![Tool::Move, Tool::Rotate, Tool::Anchor]
        );
        assert_eq!(
            transform_tools(
                &part,
                None,
                true,
                Some(PartRailFacts {
                    anchored: true,
                    anchor_column: 1,
                    locked: true,
                    member_or_companion: false,
                }),
            ),
            Vec::new()
        );
        assert_eq!(transform_tools(&part, None, true, None), Vec::new());
        // A matrix member or companion names itself in the facts and draws no
        // part rail.
        assert_eq!(
            transform_tools(
                &part,
                None,
                true,
                Some(PartRailFacts {
                    anchored: true,
                    anchor_column: 1,
                    locked: false,
                    member_or_companion: true,
                }),
            ),
            Vec::new()
        );
        // A matrix member keeps its matrix's key tools, never the part rail.
        let member_key = context_of("key");
        assert_eq!(
            transform_tools(&member_key, None, true, anchored()),
            vec![Tool::Shift, Tool::Bend]
        );
    }

    /// Ticket 23: the Handles toggle is view-only. Hidden handles drop every
    /// passive overlay but keep the active tool's required handle; with no
    /// active tool nothing draws.
    #[test]
    fn hidden_handles_keep_only_the_active_tool() {
        use LayoutTransformTool as Tool;
        let column = context_of("column");
        let row = context_of("row");
        let key = context_of("key");
        assert_eq!(transform_tools(&column, None, false, None), Vec::new());
        assert_eq!(transform_tools(&row, None, false, None), Vec::new());
        assert_eq!(transform_tools(&key, None, false, None), Vec::new());
        assert_eq!(
            transform_tools(&column, Some(Tool::Stagger), false, None),
            vec![Tool::Stagger]
        );
        assert_eq!(
            transform_tools(&column, Some(Tool::Spread), false, None),
            vec![Tool::Spread]
        );
        assert_eq!(
            transform_tools(&column, Some(Tool::Origin), false, None),
            vec![Tool::Origin]
        );
        assert_eq!(
            transform_tools(&row, Some(Tool::Offset), false, None),
            vec![Tool::Offset]
        );
        assert_eq!(
            transform_tools(&row, Some(Tool::Shift), false, None),
            vec![Tool::Shift]
        );
        assert_eq!(
            transform_tools(&row, Some(Tool::Bend), false, None),
            vec![Tool::Bend]
        );
        // A Row selection never draws a Stagger handle, even hidden.
        assert_eq!(
            transform_tools(&row, Some(Tool::Stagger), false, None),
            Vec::new()
        );
        assert_eq!(
            transform_tools(&key, Some(Tool::Shift), false, None),
            vec![Tool::Shift]
        );
        assert_eq!(
            transform_tools(&key, Some(Tool::Bend), false, None),
            vec![Tool::Bend]
        );
        // A Key selection never draws a Stagger handle, even hidden.
        assert_eq!(
            transform_tools(&key, Some(Tool::Stagger), false, None),
            Vec::new()
        );
        // Ticket 07: a hidden part rail keeps only its active tool.
        let part = context_of("part");
        let anchored = || {
            Some(PartRailFacts {
                anchored: true,
                anchor_column: 1,
                locked: false,
                member_or_companion: false,
            })
        };
        let detached = || {
            Some(PartRailFacts {
                anchored: false,
                anchor_column: 0,
                locked: false,
                member_or_companion: false,
            })
        };
        assert_eq!(transform_tools(&part, None, false, anchored()), Vec::new());
        assert_eq!(
            transform_tools(&part, Some(Tool::Spread), false, anchored()),
            vec![Tool::Spread]
        );
        assert_eq!(
            transform_tools(&part, Some(Tool::Move), false, detached()),
            vec![Tool::Move]
        );
        // Spread is not in the detached rail, even hidden.
        assert_eq!(
            transform_tools(&part, Some(Tool::Spread), false, detached()),
            Vec::new()
        );
    }
}
