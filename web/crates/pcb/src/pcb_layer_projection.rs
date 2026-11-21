//! Pure PCB layer visibility projection.
use boardstudio_model::model::{Pad, Side};
use boardstudio_web_ui_shared::board_layer::resolve_board_layer;
use std::collections::BTreeSet;

pub(crate) fn pad_copper_layers(pad: &Pad, part_side: &Side) -> Vec<String> {
    if pad.drill.is_some() {
        return vec!["F.Cu".into(), "B.Cu".into()];
    }
    let layer = if matches!(pad.side.as_ref(), Some(Side::Back)) {
        "B.Cu"
    } else {
        "F.Cu"
    };
    vec![resolve_board_layer(layer, part_side)]
}

pub fn pad_is_visible(pad: &Pad, part_side: &Side, hidden: &BTreeSet<String>) -> bool {
    if pad.plated == Some(false) || hidden.contains("Pads") {
        return false;
    }
    if pad.drill.is_some() {
        !hidden.contains("F.Cu") || !hidden.contains("B.Cu")
    } else {
        let layer = if matches!(pad.side.as_ref(), Some(Side::Back)) {
            "B.Cu"
        } else {
            "F.Cu"
        };
        !hidden.contains(&resolve_board_layer(layer, part_side))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_model::model::{PadShape, Vec2};

    fn pad(side: Option<Side>, drill: Option<f64>) -> Pad {
        Pad {
            id: "pad".into(),
            number: "1".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 2.0, y: 2.0 },
            shape: PadShape::Rect,
            drill,
            plated: Some(true),
            side,
            rotation: None,
            net_id: None,
        }
    }

    #[test]
    fn pad_layers_follow_pad_and_part_sides_and_through_holes_span_both_copper_faces() {
        assert_eq!(
            pad_copper_layers(&pad(Some(Side::Front), None), &Side::Front),
            ["F.Cu"]
        );
        assert_eq!(
            pad_copper_layers(&pad(Some(Side::Back), None), &Side::Front),
            ["B.Cu"]
        );
        assert_eq!(
            pad_copper_layers(&pad(Some(Side::Front), None), &Side::Back),
            ["B.Cu"]
        );
        assert_eq!(
            pad_copper_layers(&pad(Some(Side::Back), None), &Side::Back),
            ["F.Cu"]
        );
        assert_eq!(
            pad_copper_layers(&pad(None, Some(0.8)), &Side::Front),
            ["F.Cu", "B.Cu"]
        );
    }

    #[test]
    fn through_hole_pad_remains_visible_until_both_copper_faces_or_pads_are_hidden() {
        let through_hole = pad(Some(Side::Front), Some(0.8));
        let mut hidden = BTreeSet::from(["F.Cu".to_owned()]);
        assert!(pad_is_visible(&through_hole, &Side::Front, &hidden));
        hidden.insert("B.Cu".into());
        assert!(!pad_is_visible(&through_hole, &Side::Front, &hidden));
        hidden.remove("F.Cu");
        hidden.remove("B.Cu");
        hidden.insert("Pads".into());
        assert!(!pad_is_visible(&through_hole, &Side::Front, &hidden));
    }

    #[test]
    fn back_graphics_swap_only_front_and_back_layer_prefixes() {
        assert_eq!(resolve_board_layer("F.SilkS", &Side::Back), "B.SilkS");
        assert_eq!(resolve_board_layer("B.Cu", &Side::Back), "F.Cu");
        assert_eq!(resolve_board_layer("Dwgs.User", &Side::Back), "Dwgs.User");
    }
}
