use super::*;
use boardstudio_application::{AcceptedSnapshot, Scope, SessionEpoch};
use boardstudio_model::model::{Board, ProjectDoc};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::Element as DomElement;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
use boardstudio_model::model::{EnvelopeSource, KicadSource, PartGenerator, PartKind};

fn definition() -> PartDefinition {
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

fn pad(number: &str, side: Side, drill: Option<f64>) -> Pad {
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

#[derive(Clone)]
struct PartsMountedFixture {
    snapshot: AcceptedSnapshot,
    scope: Scope,
    definition: PartDefinition,
}

#[component]
fn parts_renderer_failure_host() -> Element {
    let fixture = use_context::<PartsMountedFixture>();
    let selection = use_signal(|| None::<(Option<Scope>, String)>);
    let selected_context = use_signal(|| None);
    let anchor_scope = use_signal(|| None::<Scope>);
    let generation = use_signal(|| 1_u64);
    let adapter = use_hook(|| {
        super::super::super::selection::SelectionAdapter::new(
            selected_context,
            anchor_scope,
            generation,
        )
    });
    use_context_provider(|| adapter.clone());
    use_context_provider(|| super::super::PartsSelectionGeneration(generation));
    let activation = use_signal(|| 0_u64);
    use_context_provider(|| super::super::PartsPreviewActivation(activation));
    let workspace = use_signal(|| "Parts");
    use_context_provider(|| super::super::super::WorkspaceState(workspace));
    let theme = use_memo(|| "light");
    use_context_provider(|| super::super::super::ResolvedAppearance(theme));
    let _: Signal<u64> = use_context_provider(|| Signal::new(0_u64));

    rsx! {
        super::super::mechanical_profile_ui::PartsMechanicalProfileWorkspace {
            snapshot: fixture.snapshot.clone(),
            scope: Some(fixture.scope.clone()),
            selection,
            definition: fixture.definition.clone(),
            preview_definition: Some(fixture.definition.clone()),
            generator_draft: None,
            recipe: Vec::new(),
            recipe_error: None,
            recipe_pending: false,
            recipe_identity: "parts-renderer-init-test".to_owned(),
            preview_title: None,
            source: crate::parts_mechanical_profile::ProfileDefinitionSource::Generator,
        }
    }
}

async fn accepted_parts_fixture() -> (Rc<crate::runtime::Runtime>, AcceptedSnapshot, Scope) {
    let mut document = ProjectDoc::empty("parts-renderer-init-project", "Parts test");
    document.revision = 7;
    document.boards.push(Board {
        id: "parts-renderer-init-board".into(),
        name: "Parts test board".into(),
        outline_ids: Vec::new(),
        part_ids: Vec::new(),
        net_ids: Vec::new(),
        thickness: 1.6,
        traces: Vec::new(),
        vias: Vec::new(),
    });
    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    let snapshot = runtime
        .model()
        .accepted
        .expect("the parts project is accepted");
    let scope = runtime
        .scope()
        .expect("the accepted parts project has a scope");
    (runtime, snapshot, scope)
}

fn click_button(root: &DomElement, label: &str) {
    boardstudio_web_ui_shared::test_role::by_role_name(root, "button", label)
        .unwrap_or_else(|| panic!("missing mounted Parts button: {label}"))
        .click();
}

fn has_button(root: &DomElement, label: &str) -> bool {
    boardstudio_web_ui_shared::test_role::by_role_name(root, "button", label).is_some()
}

async fn wait_for(root: &DomElement, selector: &str) -> DomElement {
    for _ in 0..500 {
        if let Some(element) = root.query_selector(selector).unwrap() {
            return element;
        }
        gloo_timers::future::TimeoutFuture::new(20).await;
    }
    let text = root.text_content().unwrap_or_default();
    let summary = text.chars().take(800).collect::<String>();
    panic!("timed out waiting for Parts preview element: {selector}; rendered text: {summary}");
}

async fn wait_for_button(root: &DomElement, label: &str) {
    for _ in 0..100 {
        if has_button(root, label) {
            return;
        }
        gloo_timers::future::TimeoutFuture::new(20).await;
    }
    panic!("timed out waiting for mounted Parts button: {label}");
}

#[component]
fn preview_controls_host() -> Element {
    let generation = use_signal(|| 0_u64);
    let activation = use_signal(|| 0_u64);
    use_context_provider(|| super::super::PartsSelectionGeneration(generation));
    use_context_provider(|| super::super::PartsPreviewActivation(activation));
    rsx! {
        PartsPreviewPanel {
            definition: Some(Rc::new(definition())), recipe: Vec::new(),
            recipe_error: None, recipe_pending: false, recipe_identity: "keyboard-floor",
            preview_title: None, scope: None, snapshot_token: SnapshotToken(1),
            generator_draft: None,
        }
    }
}

#[wasm_bindgen_test]
async fn parts_preview_mode_arrows_rove_without_changing_the_local_view() {
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(preview_controls_host);
    dom.provide_root_context(support::new_runtime());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("the actual Parts preview mode controls mount", || {
        by_role_name(&root, "button", "2D footprint").is_some()
    })
    .await;
    let footprint = by_role_name(&root, "button", "2D footprint").unwrap();
    let model = by_role_name(&root, "button", "3D model").unwrap();
    footprint.focus().unwrap();
    let key = web_sys::KeyboardEventInit::new();
    key.set_key("ArrowRight");
    key.set_bubbles(true);
    key.set_cancelable(true);
    footprint
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &key).unwrap(),
        )
        .unwrap();
    wait_until("ArrowRight roves to the actual 3D mode control", || {
        document.active_element() == Some(model.clone().unchecked_into())
    })
    .await;
    assert_eq!(
        footprint.get_attribute("aria-pressed").as_deref(),
        Some("true")
    );
    assert_eq!(
        model.get_attribute("aria-pressed").as_deref(),
        Some("false")
    );
    model.click();
    wait_until("the Parts-owned view changes after activation", || {
        model.get_attribute("aria-pressed").as_deref() == Some("true")
    })
    .await;
    footprint.click();
    wait_until("the same owner returns to the footprint", || {
        footprint.get_attribute("aria-pressed").as_deref() == Some("true")
    })
    .await;
    root.remove();
}

#[component]
fn parts_layers_host() -> Element {
    rsx! {
        PartsPreviewLayers {
            layers: vec![PreviewLayer {
                id: "F.Cu".into(), label: "Front copper".into(), group: "Copper",
            }],
            hidden: BTreeSet::new(),
            on_toggle: move |_| {},
        }
    }
}

#[wasm_bindgen_test]
async fn parts_layers_escape_closes_and_restores_trigger_focus() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("parts-layers-escape-test");
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(parts_layers_host));
    boardstudio_web_ui_shared::test_wait::wait_until("the Parts layers trigger mounts", || {
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Preview layers")
            .is_some()
    })
    .await;
    let trigger =
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Preview layers")
            .unwrap();
    assert_eq!(
        trigger.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );
    trigger.click();
    wait_for(
        &root,
        "#m1-parts-preview-layers-trigger[aria-expanded='true']",
    )
    .await;
    let layer =
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Hide Front copper")
            .unwrap();
    layer.focus().unwrap();
    assert_eq!(
        document.active_element(),
        Some(layer.clone().unchecked_into())
    );
    let key = web_sys::KeyboardEventInit::new();
    key.set_key("Escape");
    key.set_bubbles(true);
    key.set_cancelable(true);
    layer
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &key).unwrap(),
        )
        .unwrap();
    wait_for(
        &root,
        "#m1-parts-preview-layers-trigger[aria-expanded='false']",
    )
    .await;
    assert_eq!(document.active_element(), Some(trigger.unchecked_into()));
    root.remove();
}

#[wasm_bindgen_test]
async fn parts_renderer_initialization_failure_keeps_form_and_returns_to_2d() {
    let (runtime, snapshot, scope) = accepted_parts_fixture().await;
    let mut definition = definition();
    definition.generator = None;
    definition.kicad_source = Some(KicadSource {
        format_version: 1,
        source: "(footprint \"Parts test switch\" (version 20240108) (generator \"BoardStudio\"))"
            .into(),
    });
    let fixture = PartsMountedFixture {
        snapshot: snapshot.clone(),
        scope,
        definition,
    };
    let before = runtime
        .model()
        .accepted
        .expect("fixture project is current");

    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("parts-renderer-init-failure-test");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = dioxus::prelude::VirtualDom::new(parts_renderer_failure_host);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(fixture);
    let _failure = super::super::super::shared_viewer::fail_next_renderer_mount_for_test(
        "injected renderer initialization failure",
    );
    let _preview = super::seed_parts_preview_for_test(boardstudio_model::model::PcbPreview {
        revision: snapshot.document.revision,
        thickness: 1.6,
        contours: Vec::new(),
        surfaces: Vec::new(),
        holes: Vec::new(),
        models: Vec::new(),
        diagnostics: Vec::new(),
    });
    let _mounted = MountedFixture::new(root.clone(), dom);

    wait_for_button(&root, "3D model").await;
    click_button(&root, "3D model");
    let alert = wait_for(&root, "[role=alert]").await;
    assert!(
        alert
            .text_content()
            .unwrap_or_default()
            .contains("injected renderer initialization failure"),
        "renderer mount failure must travel through SharedViewer's actual error status"
    );
    assert!(
        root.text_content()
            .unwrap_or_default()
            .contains("Mechanical fit")
    );
    assert!(
        root.text_content()
            .unwrap_or_default()
            .contains("Define profile")
    );
    assert!(has_button(&root, "2D footprint"));
    assert!(has_button(&root, "3D model"));

    click_button(&root, "2D footprint");
    wait_for(&root, "svg[aria-label='switch mx footprint preview']").await;
    assert!(root.query_selector("canvas").unwrap().is_none());
    assert!(
        root.text_content()
            .unwrap_or_default()
            .contains("Define profile")
    );
    let after = runtime
        .model()
        .accepted
        .expect("fixture project remains current");
    assert_eq!(after.token, before.token);
    assert_eq!(after.document.revision, before.document.revision);
    assert_eq!(after.document.id, before.document.id);
    assert!(Arc::ptr_eq(&after.document, &before.document));
    assert!(
        support::take_held_effects(&runtime).is_empty(),
        "the mounted renderer failure and 2D return must not submit a project edit"
    );
    root.remove();
}

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
fn generated_keycap_uses_retained_dimensions_unless_the_envelope_is_authored() {
    let defaults = footprint_graphics::GeneratorPreviewDefaults {
        keycap_width: Some(18.0),
        keycap_height: Some(18.0),
        include_keycap: Some(true),
    };
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

#[wasm_bindgen_test]
fn keycap_visibility_prefers_generator_value_then_retained_default() {
    let defaults = footprint_graphics::GeneratorPreviewDefaults {
        include_keycap: Some(false),
        ..Default::default()
    };
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

#[wasm_bindgen_test]
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

#[wasm_bindgen_test]
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
