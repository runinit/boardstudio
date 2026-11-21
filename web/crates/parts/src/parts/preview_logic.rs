use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{EnvelopeOrigin, Pad, PartDefinition, Vec2};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};

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
