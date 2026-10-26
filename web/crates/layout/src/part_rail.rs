//! Ticket 07: the pure part-rail tool list for an independent part.
//!
//! The canvas rail for a selected part depends only on three facts the mount
//! site reads from the accepted document: whether the part is anchored (and,
//! when it is, its anchor column), and whether it is locked. Matrix members
//! and companions never reach this list: the mount site passes no facts for
//! them, so they keep their matrix's tools. This module also compiles
//! natively, unlike the overlay, so the list rules are pinned by `cargo test`.

/// The rail tools an independent part can offer, in rail order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartRailTool {
    Move,
    Rotate,
    Spread,
    Stagger,
}

/// The facts a part rail depends on: anchored versus detached (with the
/// anchor column for the column-0 Spread rule), locked, and whether the part
/// is a matrix member or companion (those keep their matrix's tools, never a
/// part rail).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartRailFacts {
    pub anchored: bool,
    pub anchor_column: u32,
    pub locked: bool,
    pub member_or_companion: bool,
}

/// The rail tools for one independent part: an anchored part edits its local
/// Offset and Rotation plus the Spread and Stagger carried from its anchor
/// key; a detached part moves and rotates through the shipped part-drag
/// path. Locked parts and matrix members/companions show no part tokens.
/// Column 0 carries no Spread step, so an anchor there offers no Spread
/// token, matching the matrix column-0 rule.
pub fn part_rail_tools(facts: PartRailFacts) -> Vec<PartRailTool> {
    if facts.locked || facts.member_or_companion {
        return Vec::new();
    }
    if !facts.anchored {
        return vec![PartRailTool::Move, PartRailTool::Rotate];
    }
    let mut tools = vec![PartRailTool::Move, PartRailTool::Rotate];
    if facts.anchor_column != 0 {
        tools.push(PartRailTool::Spread);
    }
    tools.push(PartRailTool::Stagger);
    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(
        anchored: bool,
        anchor_column: u32,
        locked: bool,
        member_or_companion: bool,
    ) -> PartRailFacts {
        PartRailFacts {
            anchored,
            anchor_column,
            locked,
            member_or_companion,
        }
    }

    #[test]
    fn anchored_part_lists_move_rotate_spread_stagger() {
        assert_eq!(
            part_rail_tools(facts(true, 1, false, false)),
            vec![
                PartRailTool::Move,
                PartRailTool::Rotate,
                PartRailTool::Spread,
                PartRailTool::Stagger,
            ]
        );
    }

    #[test]
    fn anchor_on_column_zero_lists_no_spread() {
        assert_eq!(
            part_rail_tools(facts(true, 0, false, false)),
            vec![
                PartRailTool::Move,
                PartRailTool::Rotate,
                PartRailTool::Stagger,
            ]
        );
    }

    #[test]
    fn detached_part_lists_move_and_rotate() {
        assert_eq!(
            part_rail_tools(facts(false, 0, false, false)),
            vec![PartRailTool::Move, PartRailTool::Rotate]
        );
    }

    #[test]
    fn locked_parts_show_no_tokens() {
        assert_eq!(part_rail_tools(facts(true, 1, true, false)), Vec::new());
        assert_eq!(part_rail_tools(facts(false, 0, true, false)), Vec::new());
    }

    #[test]
    fn members_and_companions_keep_their_matrix_tools() {
        assert_eq!(part_rail_tools(facts(true, 1, false, true)), Vec::new());
    }
}
