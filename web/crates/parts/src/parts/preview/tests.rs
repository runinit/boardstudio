use super::*;
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_model::model::{Board, ProjectDoc};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::Element as DomElement;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
use boardstudio_model::model::KicadSource;

use crate::preview_logic::test_fixtures::definition;

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
