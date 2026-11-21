use crate::footprint_forms::{Graphic, Shape};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{EnvelopeOrigin, Pad, PartDefinition, Side, Vec2};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PreviewLayer {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) group: &'static str,
}

#[derive(Clone, Debug)]
pub(crate) struct PreviewOutline {
    pub(crate) label: &'static str,
    pub(crate) points: Vec<Vec2>,
}

#[derive(Clone, Debug)]
pub(crate) struct PreviewGeometry {
    pub(crate) definition: PartDefinition,
    pub(crate) drawings: Rc<Vec<Graphic>>,
    pub(crate) outline: Option<PreviewOutline>,
    pub(crate) at: Vec2,
    pub(crate) rotation: f64,
    pub(crate) side: Side,
}

pub(crate) fn rectangle_points(size: Vec2) -> Vec<Vec2> {
    vec![
        Vec2 {
            x: -size.x / 2.0,
            y: -size.y / 2.0,
        },
        Vec2 {
            x: size.x / 2.0,
            y: -size.y / 2.0,
        },
        Vec2 {
            x: size.x / 2.0,
            y: size.y / 2.0,
        },
        Vec2 {
            x: -size.x / 2.0,
            y: size.y / 2.0,
        },
    ]
}

pub(crate) fn preview_layers(
    definition: &PartDefinition,
    drawings: &[Graphic],
    outline: Option<&PreviewOutline>,
    part_side: &Side,
) -> Vec<PreviewLayer> {
    let mut copper = BTreeSet::new();
    for pad in &definition.pads {
        copper.insert(copper_id(pad, definition, part_side));
    }
    let mut layers = Vec::new();
    for id in ["copper:F.Cu", "copper:B.Cu"] {
        if copper.contains(id) {
            layers.push(PreviewLayer {
                id: id.into(),
                label: id.trim_start_matches("copper:").into(),
                group: "Footprint",
            });
        }
    }
    let graphics = drawings
        .iter()
        .map(|graphic| graphic.layer.clone())
        .collect::<BTreeSet<_>>();
    for layer in graphics {
        layers.push(PreviewLayer {
            id: format!("graphics:{layer}"),
            label: layer,
            group: "Footprint",
        });
    }
    if let Some(outline) = outline {
        layers.push(PreviewLayer {
            id: format!("outline:{}", outline.label),
            label: outline.label.into(),
            group: "Footprint",
        });
    }
    if definition
        .pads
        .iter()
        .any(|pad| pad.drill.is_some_and(|drill| drill > 0.0))
    {
        layers.push(PreviewLayer {
            id: "drills".into(),
            label: "Drills".into(),
            group: "Footprint",
        });
    }
    if !definition.pads.is_empty() {
        layers.push(PreviewLayer {
            id: "pad-labels".into(),
            label: "Pad numbers".into(),
            group: "Footprint",
        });
    }
    layers.push(PreviewLayer {
        id: "part:0".into(),
        label: boardstudio_web_catalogue::catalogue::preferred_label(definition).into(),
        group: "Parts",
    });
    layers
}

pub(crate) fn copper_id(pad: &Pad, definition: &PartDefinition, part_side: &Side) -> String {
    let side = pad.side.as_ref().map_or_else(
        || {
            let configured_back = definition
                .generator
                .as_ref()
                .and_then(|generator| generator.parameters.get("side"))
                .and_then(serde_json::Value::as_str)
                == Some("B");
            if configured_back {
                "copper:B.Cu"
            } else {
                match part_side {
                    Side::Back => "copper:B.Cu",
                    Side::Front => "copper:F.Cu",
                }
            }
        },
        |side| match side {
            Side::Back => "copper:B.Cu",
            Side::Front => "copper:F.Cu",
        },
    );
    side.into()
}

pub(crate) fn recipe_view_box(members: &[PreviewGeometry]) -> Option<String> {
    let mut complete = Bounds::default();
    for member in members {
        let mut local = Bounds::default();
        if member
            .outline
            .as_ref()
            .is_some_and(|outline| outline.label == "Courtyard")
        {
            for point in &member.definition.courtyard {
                local.include(point.x, point.y);
            }
        }
        if let Some(outline) = member.outline.as_ref() {
            for point in &outline.points {
                local.include(point.x, point.y);
            }
        }
        for pad in &member.definition.pads {
            let angle = pad.rotation.unwrap_or(0.0).to_radians();
            let (sin, cos) = angle.sin_cos();
            for x in [-pad.size.x / 2.0, pad.size.x / 2.0] {
                for y in [-pad.size.y / 2.0, pad.size.y / 2.0] {
                    local.include(pad.at.x + x * cos - y * sin, pad.at.y + x * sin + y * cos);
                }
            }
        }
        for graphic in member.drawings.iter() {
            include_graphic_bounds(&mut local, graphic);
        }
        let Some((min_x, min_y, max_x, max_y)) = local.finish() else {
            continue;
        };
        let rotation = member.rotation.to_radians();
        let (sin, cos) = rotation.sin_cos();
        for x in [min_x, max_x] {
            for y in [min_y, max_y] {
                complete.include(
                    member.at.x + x * cos - y * sin,
                    member.at.y + x * sin + y * cos,
                );
            }
        }
    }
    // Match the reference's empty source-backed library surface. A zero-size
    // viewBox represents no geometry and deliberately adds no synthetic point.
    let Some(_) = complete.finish() else {
        return Some("0 0 0 0".into());
    };
    complete.include(0.0, 0.0);
    let (min_x, min_y, max_x, max_y) = complete.finish()?;
    let margin = 3.0;
    Some(format!(
        "{:.3} {:.3} {:.3} {:.3}",
        min_x - margin,
        -(max_y + margin),
        max_x - min_x + margin * 2.0,
        max_y - min_y + margin * 2.0
    ))
}

#[derive(Default)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    has_point: bool,
}

impl Bounds {
    fn include(&mut self, x: f64, y: f64) {
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        if !self.has_point {
            self.min_x = x;
            self.min_y = y;
            self.max_x = x;
            self.max_y = y;
            self.has_point = true;
            return;
        }
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn finish(&self) -> Option<(f64, f64, f64, f64)> {
        self.has_point
            .then_some((self.min_x, self.min_y, self.max_x, self.max_y))
    }
}

fn include_graphic_bounds(bounds: &mut Bounds, graphic: &Graphic) {
    let include = |bounds: &mut Bounds, point: &crate::footprint_forms::Point| {
        // `footprint_forms::point` already converts native Y into the preview
        // frame. GraphicElement consumes that value under the one outer flip;
        // use the same point here rather than applying a second conversion.
        bounds.include(point.0, point.1);
    };
    match &graphic.shape {
        Shape::Line(start, end) | Shape::Rect(start, end) => {
            include(bounds, start);
            include(bounds, end);
        }
        Shape::Arc(start, middle, end) => {
            include(bounds, start);
            include(bounds, middle);
            include(bounds, end);
        }
        Shape::Circle(center, radius) => {
            bounds.include(center.0 - radius, center.1 - radius);
            bounds.include(center.0 + radius, center.1 + radius);
        }
        Shape::Polygon(points, _) => {
            for point in points {
                include(bounds, point);
            }
        }
        Shape::Text(at, _) => include(bounds, at),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PreviewInput {
    pub(super) scope: Option<Scope>,
    pub(super) snapshot_token: SnapshotToken,
    pub(super) definition_id: String,
    pub(super) definition_json: String,
    pub(super) recipe_identity: String,
    pub(super) selection_generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PreviewOwner {
    pub(super) input: PreviewInput,
    pub(super) generation: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Visibility {
    pub(super) definition_id: String,
    pub(super) hidden: BTreeSet<String>,
}

pub(super) fn next_preview_owner(
    last_input: &Rc<RefCell<Option<PreviewInput>>>,
    generation_counter: &Rc<Cell<u64>>,
    input: PreviewInput,
) -> PreviewOwner {
    let mut previous = last_input.borrow_mut();
    if previous.as_ref() != Some(&input) {
        generation_counter.set(generation_counter.get().wrapping_add(1));
        *previous = Some(input.clone());
    }
    PreviewOwner {
        input,
        generation: generation_counter.get(),
    }
}

pub(super) fn valid_size(size: Vec2) -> Option<Vec2> {
    (size.x.is_finite() && size.y.is_finite() && size.x > 0.0 && size.y > 0.0).then_some(size)
}

pub(super) fn keycap_size(
    definition: &PartDefinition,
    defaults: (Option<f64>, Option<f64>, Option<bool>),
) -> Option<Vec2> {
    let saved = definition.keycap.and_then(valid_size);
    let authored = definition
        .envelope_source
        .as_ref()
        .is_none_or(|source| source.keycap == Some(EnvelopeOrigin::Authored));
    if authored && let Some(saved) = saved {
        return Some(saved);
    }

    let parameters = definition
        .generator
        .as_ref()
        .map(|generator| &generator.parameters);
    let width = parameters
        .and_then(|parameters| parameters.get("keycap_width"))
        .and_then(serde_json::Value::as_f64)
        .or(defaults.0);
    let height = parameters
        .and_then(|parameters| parameters.get("keycap_height"))
        .and_then(serde_json::Value::as_f64)
        .or(defaults.1);
    let generated = width
        .zip(height)
        .map(|(x, y)| Vec2 { x, y })
        .and_then(valid_size);
    generated.or(saved)
}

pub(super) fn include_keycap(
    definition: &PartDefinition,
    defaults: (Option<f64>, Option<f64>, Option<bool>),
) -> bool {
    definition
        .generator
        .as_ref()
        .and_then(|generator| generator.parameters.get("include_keycap"))
        .and_then(serde_json::Value::as_bool)
        .or(defaults.2)
        .unwrap_or(true)
}

pub(super) fn owner_is_current(captured: &PreviewOwner, current: &PreviewOwner) -> bool {
    captured == current
}

pub(super) fn visible_hidden(definition_id: &str, visibility: &Visibility) -> BTreeSet<String> {
    if visibility.definition_id == definition_id {
        visibility.hidden.clone()
    } else {
        BTreeSet::new()
    }
}

pub(super) fn drill_class(pad: &Pad) -> &'static str {
    if pad.plated == Some(false) {
        "fill-canvas stroke-geometry-outline stroke-[0.1] is-mechanical"
    } else {
        "fill-canvas stroke-geometry-outline stroke-[0.1]"
    }
}

#[cfg(test)]
mod tests {
    use super::test_fixtures::{definition, pad};
    use super::*;
    use boardstudio_application::SessionEpoch;
    use boardstudio_model::model::{EnvelopeSource, Side};
    #[test]
    fn stale_async_result_cannot_reclaim_same_definition_after_selection_round_trip() {
        let input_a = PreviewInput {
            selection_generation: 1,
            scope: Some(Scope {
                session_epoch: SessionEpoch(4),
                document_id: "doc-a".into(),
                board_id: "board-a".into(),
                instance_id: None,
            }),
            snapshot_token: SnapshotToken(7),
            definition_id: "generator:ceoloide/switch_mx".into(),
            definition_json: String::new(),
            recipe_identity: String::new(),
        };
        let last_input = Rc::new(RefCell::new(None));
        let generation_counter = Rc::new(Cell::new(0));
        let first_a = next_preview_owner(&last_input, &generation_counter, input_a.clone());
        assert_eq!(
            next_preview_owner(&last_input, &generation_counter, input_a.clone()),
            first_a,
            "an ordinary repaint of the same accepted resource keeps its generation"
        );
        let input_b = PreviewInput {
            definition_id: "generator:ceoloide/encoder".into(),
            ..input_a.clone()
        };
        let _b = next_preview_owner(&last_input, &generation_counter, input_b);
        let second_a = next_preview_owner(&last_input, &generation_counter, input_a.clone());
        assert_eq!(first_a.generation, 1);
        assert_eq!(second_a.generation, 3);
        assert!(!owner_is_current(&first_a, &second_a));

        let mut other_scope = input_a.clone();
        other_scope.scope.as_mut().unwrap().instance_id = Some("right".into());
        let scoped_owner = next_preview_owner(&last_input, &generation_counter, other_scope);
        let returned_a = next_preview_owner(&last_input, &generation_counter, input_a);
        assert_eq!(scoped_owner.generation, 4);
        assert_eq!(returned_a.generation, 5);
        assert!(!owner_is_current(&scoped_owner, &returned_a));
    }

    #[test]
    fn generated_keycap_uses_retained_dimensions_unless_the_envelope_is_authored() {
        let defaults = (Some(18.0), Some(18.0), Some(true));
        let mut definition = definition();
        definition.envelope_source = Some(EnvelopeSource {
            courtyard: None,
            keycap: Some(EnvelopeOrigin::Generated),
        });
        definition
            .generator
            .as_mut()
            .unwrap()
            .parameters
            .insert("keycap_width".into(), serde_json::Value::from(19.0));
        definition
            .generator
            .as_mut()
            .unwrap()
            .parameters
            .insert("keycap_height".into(), serde_json::Value::from(20.0));
        assert_eq!(
            keycap_size(&definition, defaults),
            Some(Vec2 { x: 19.0, y: 20.0 })
        );

        definition.keycap = Some(Vec2 { x: 21.0, y: 22.0 });
        definition.envelope_source.as_mut().unwrap().keycap = Some(EnvelopeOrigin::Authored);
        assert_eq!(
            keycap_size(&definition, defaults),
            Some(Vec2 { x: 21.0, y: 22.0 })
        );
    }

    #[test]
    fn keycap_visibility_prefers_generator_value_then_retained_default() {
        let defaults = (None, None, Some(false));
        let mut definition = definition();
        assert!(!include_keycap(&definition, defaults));
        definition
            .generator
            .as_mut()
            .unwrap()
            .parameters
            .insert("include_keycap".into(), serde_json::Value::Bool(true));
        assert!(include_keycap(&definition, defaults));
    }

    #[test]
    fn non_plated_drills_keep_the_mechanical_hole_class() {
        let mut mechanical = pad("NPTH", Side::Front, Some(1.0));
        mechanical.plated = Some(false);
        let plated = pad("PTH", Side::Front, Some(1.0));
        assert!(
            drill_class(&mechanical)
                .split_whitespace()
                .any(|class| class == "is-mechanical")
        );
        assert!(
            !drill_class(&plated)
                .split_whitespace()
                .any(|class| class == "is-mechanical")
        );
    }

    #[test]
    fn visibility_is_one_definition_pair_like_the_reference_workspace() {
        let stored_a = Visibility {
            definition_id: "a".into(),
            hidden: BTreeSet::from(["drills".into()]),
        };
        assert_eq!(
            visible_hidden("a", &stored_a),
            BTreeSet::from(["drills".into()])
        );
        assert!(visible_hidden("b", &stored_a).is_empty());
        let after_b_toggle = Visibility {
            definition_id: "b".into(),
            hidden: BTreeSet::from(["copper:F.Cu".into()]),
        };
        assert!(visible_hidden("a", &after_b_toggle).is_empty());
        assert_eq!(
            visible_hidden("b", &after_b_toggle),
            BTreeSet::from(["copper:F.Cu".into()])
        );
    }

    #[test]
    fn mx_preview_layer_options_follow_geometry_and_retained_source_layers() {
        let definition = definition();
        let drawings = vec![Graphic {
            layer: "B.SilkS".into(),
            shape: Shape::Line(
                crate::footprint_forms::Point(-2.0, -3.0),
                crate::footprint_forms::Point(2.0, -3.0),
            ),
        }];
        let outline = PreviewOutline {
            label: "Keycap",
            points: rectangle_points(Vec2 { x: 18.0, y: 18.0 }),
        };
        let layers = preview_layers(&definition, &drawings, Some(&outline), &Side::Front);
        assert_eq!(
            layers
                .iter()
                .map(|layer| (layer.id.as_str(), layer.label.as_str(), layer.group))
                .collect::<Vec<_>>(),
            [
                ("copper:F.Cu", "F.Cu", "Footprint"),
                ("copper:B.Cu", "B.Cu", "Footprint"),
                ("graphics:B.SilkS", "B.SilkS", "Footprint"),
                ("outline:Keycap", "Keycap", "Footprint"),
                ("drills", "Drills", "Footprint"),
                ("pad-labels", "Pad numbers", "Footprint"),
                ("part:0", "MX switch", "Parts"),
            ]
        );
    }

    #[test]
    fn mx_view_box_contains_the_source_envelope_and_rendered_graphics() {
        let definition = definition();
        let drawings = vec![Graphic {
            layer: "B.SilkS".into(),
            shape: Shape::Line(
                crate::footprint_forms::Point(-11.0, -14.0),
                crate::footprint_forms::Point(11.0, -14.0),
            ),
        }];
        let outline = PreviewOutline {
            label: "Keycap",
            points: rectangle_points(Vec2 { x: 18.0, y: 18.0 }),
        };
        let member = PreviewGeometry {
            definition,
            drawings: Rc::new(drawings),
            outline: Some(outline),
            at: Vec2::default(),
            rotation: 0.0,
            side: Side::Front,
        };
        assert_eq!(
            recipe_view_box(&[member]).as_deref(),
            Some("-14.000 -12.000 28.000 29.000")
        );
    }

    #[test]
    fn view_box_keeps_the_reference_origin_for_one_sided_graphics() {
        let mut definition = definition();
        definition.pads.clear();
        definition.courtyard.clear();
        let drawings = vec![Graphic {
            layer: "F.SilkS".into(),
            shape: Shape::Line(
                crate::footprint_forms::Point(5.0, 4.0),
                crate::footprint_forms::Point(8.0, 7.0),
            ),
        }];
        let member = PreviewGeometry {
            definition,
            drawings: Rc::new(drawings),
            outline: None,
            at: Vec2::default(),
            rotation: 0.0,
            side: Side::Front,
        };
        assert_eq!(
            recipe_view_box(&[member]).as_deref(),
            Some("-3.000 -10.000 14.000 13.000")
        );
    }

    #[test]
    fn layer_options_omit_source_shapes_that_cannot_be_toggled() {
        let mut definition = definition();
        definition.pads = vec![pad("1", Side::Front, None)];
        definition.keycap = None;
        definition.courtyard.clear();
        let drawings = vec![Graphic {
            layer: "B.SilkS".into(),
            shape: Shape::Line(
                crate::footprint_forms::Point(0.0, 0.0),
                crate::footprint_forms::Point(1.0, -1.0),
            ),
        }];
        let layers = preview_layers(&definition, &drawings, None, &Side::Front);
        let ids = layers
            .iter()
            .map(|layer| layer.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            ["copper:F.Cu", "graphics:B.SilkS", "pad-labels", "part:0"]
        );
    }
}

#[cfg(test)]
pub(crate) mod test_fixtures {
    use boardstudio_model::model::{
        Pad, PadShape, PartDefinition, PartGenerator, PartKind, Side, Vec2,
    };
    pub(crate) fn definition() -> PartDefinition {
        PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: "generator:ceoloide/switch_mx".into(),
            name: "switch mx".into(),
            kind: PartKind::Switch,
            keycap: Some(Vec2 { x: 18.0, y: 18.0 }),
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![
                Vec2 { x: -8.0, y: -8.0 },
                Vec2 { x: 8.0, y: -8.0 },
                Vec2 { x: 8.0, y: 8.0 },
                Vec2 { x: -8.0, y: 8.0 },
            ],
            pads: vec![pad("1", Side::Front, Some(1.5)), pad("2", Side::Back, None)],
            models: None,
            generator: Some(PartGenerator {
                source: "ceoloide/switch_mx".into(),
                version: "bundled-1".into(),
                parameters: Default::default(),
            }),
            mechanical_profile: None,
        }
    }

    pub(crate) fn pad(number: &str, side: Side, drill: Option<f64>) -> Pad {
        Pad {
            id: format!("pad-{number}"),
            number: number.into(),
            at: Vec2::default(),
            size: Vec2 { x: 2.0, y: 2.0 },
            shape: PadShape::Rect,
            drill,
            plated: Some(true),
            side: Some(side),
            rotation: None,
            net_id: None,
        }
    }
}
