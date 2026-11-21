use super::*;
use crate::runtime::Runtime;
use boardstudio_application::{Event as AppEvent, Scope};
use boardstudio_model::model::{PartDefinition, ProjectDoc};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_role::{all_by_role_name, by_role_name};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{Event as DomEvent, HtmlInputElement};

wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct State {
    snapshot: Signal<AcceptedSnapshot>,
    scope: Signal<Option<Scope>>,
    selection: Signal<Option<(Option<Scope>, String)>>,
    definition: Signal<PartDefinition>,
    version: Signal<u64>,
}

struct Seed {
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    selection: Option<(Option<Scope>, String)>,
    definition: PartDefinition,
    state: Rc<RefCell<Option<State>>>,
}

fn definition(id: &str, name: &str) -> PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": name,
        "kind": "custom",
        "courtyard": [],
        "pads": []
    }))
    .unwrap()
}

fn host() -> Element {
    let seed = use_context::<Rc<Seed>>();
    let snapshot = use_signal(|| seed.snapshot.clone());
    let scope = use_signal(|| seed.scope.clone());
    let selection = use_signal(|| seed.selection.clone());
    let definition = use_signal(|| seed.definition.clone());
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    *seed.state.borrow_mut() = Some(State {
        snapshot,
        scope,
        selection,
        definition,
        version,
    });
    rsx! {
        div { "data-snapshot-revision": "{snapshot().document.revision}",
            DefinitionNameEditor {
                snapshot: snapshot(),
                scope: scope(),
                selection,
                definition: definition(),
                children: rsx! {
                    crate::parts_custom_definition::CustomDefinitionFields {
                        snapshot: snapshot(),
                        scope: scope(),
                        selection,
                        definition: definition(),
                    }
                },
            }
        }
    }
}

async fn settle() {
    gloo_timers::future::TimeoutFuture::new(5).await;
}

/// Open `document` through the real Session and Core on a fresh Runtime and return what the
/// mounted editor is given: the accepted snapshot and its scope.
async fn opened(document: ProjectDoc) -> (Rc<Runtime>, AcceptedSnapshot, Option<Scope>) {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    let snapshot = runtime
        .model()
        .accepted
        .expect("the opened document is accepted");
    let scope = runtime.scope();
    (runtime, snapshot, scope)
}

fn input(root: &web_sys::Element) -> HtmlInputElement {
    by_role_name(root, "textbox", "Definition name")
        .expect("definition name field")
        .dyn_into()
        .unwrap()
}

fn section_trigger(root: &web_sys::Element) -> web_sys::HtmlElement {
    by_role_name(root, "button", "Edit footprint Custom geometry")
        .expect("edit footprint section trigger")
}

fn section_is_open(root: &web_sys::Element) -> bool {
    section_trigger(root)
        .get_attribute("aria-expanded")
        .as_deref()
        == Some("true")
}

fn pad_id_input(root: &web_sys::Element) -> HtmlInputElement {
    by_role_name(root, "textbox", "Pad 1 ID")
        .expect("pad ID field")
        .dyn_into()
        .unwrap()
}

fn pad_number_input(root: &web_sys::Element) -> HtmlInputElement {
    by_role_name(root, "textbox", "Pad 1 number")
        .expect("pad number field")
        .dyn_into()
        .unwrap()
}

fn courtyard_width_input(root: &web_sys::Element) -> HtmlInputElement {
    by_role_name(root, "spinbutton", "Courtyard width")
        .expect("courtyard width field")
        .dyn_into()
        .unwrap()
}

fn courtyard_height_input(root: &web_sys::Element) -> HtmlInputElement {
    by_role_name(root, "spinbutton", "Courtyard height")
        .expect("courtyard height field")
        .dyn_into()
        .unwrap()
}

fn pad_y_input(root: &web_sys::Element) -> HtmlInputElement {
    by_role_name(root, "spinbutton", "Pad 1 Y")
        .expect("pad Y field")
        .dyn_into()
        .unwrap()
}

#[wasm_bindgen_test]
async fn mounted_definition_owner_switch_and_pad_id_refresh_retain_and_retire_drafts() {
    crate::parts_custom_definition::clear_pad_number_draft_for_test();
    let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
    let mut first = definition("first", "First");
    let mut second = definition("second", "Second");
    for definition in [&mut first, &mut second] {
        definition.courtyard = vec![
            boardstudio_model::model::Vec2 { x: -5.0, y: -3.0 },
            boardstudio_model::model::Vec2 { x: 5.0, y: -3.0 },
            boardstudio_model::model::Vec2 { x: 5.0, y: 3.0 },
            boardstudio_model::model::Vec2 { x: -5.0, y: 3.0 },
        ];
        definition.pads = serde_json::from_value(serde_json::json!([
                {"id":"shared-pad","number":"1","at":{"x":0.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle"}
            ])).unwrap();
    }
    document.definitions = vec![first, second];
    let (runtime, snapshot, scope) = opened(document).await;
    let state = Rc::new(RefCell::new(None));
    let seed = Rc::new(Seed {
        snapshot: snapshot.clone(),
        scope: scope.clone(),
        selection: Some((scope.clone(), "first".into())),
        definition: snapshot.document.definitions[0].clone(),
        state: state.clone(),
    });
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    let root_index = NEXT_ROOT.with(|next| next.replace(next.get() + 1));
    root.set_id(&format!("parts-name-mounted-regression-{root_index}"));
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed);
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    settle().await;
    section_trigger(&root).click();
    settle().await;
    let pad_id = pad_id_input(&root);
    type_value(&pad_id, "");
    settle().await;
    let _ = input(&root).focus();
    settle().await;
    assert!(root.query_selector("[role='alert']").unwrap().is_some());

    let mut controls = state.borrow().as_ref().unwrap().clone();
    controls
        .selection
        .set(Some((scope.clone(), "second".into())));
    controls
        .definition
        .set(snapshot.document.definitions[1].clone());
    settle().await;
    section_trigger(&root).click();
    settle().await;

    assert_eq!(pad_id_input(&root).value(), "shared-pad");
    assert!(root.query_selector("[role='alert']").unwrap().is_none());

    // A real accepted pad-ID and Y-coordinate update must preserve a dirty
    // number draft in the same row. This catches rows keyed by mutable ID.
    assert!(crate::parts_custom_definition::set_pad_number_draft_for_test("7"));
    settle().await;
    assert_eq!(pad_number_input(&root).value(), "7");
    let mut refreshed_doc = runtime
        .model()
        .accepted
        .as_ref()
        .unwrap()
        .document
        .as_ref()
        .clone();
    refreshed_doc.definitions[1].pads[0].id = "renamed-pad".into();
    refreshed_doc.definitions[1].pads[0].at.y = 4.0;
    let refreshed = support::replace_document(
        &runtime,
        "parts04-mounted-refresh-911",
        "second",
        refreshed_doc,
    )
    .await;
    controls.snapshot.set(refreshed.clone());
    controls
        .definition
        .set(refreshed.document.definitions[1].clone());
    settle().await;
    assert_eq!(pad_y_input(&root).value(), "4");
    assert_eq!(pad_id_input(&root).value(), "renamed-pad");
    assert_eq!(pad_number_input(&root).value(), "7");
    let _ = pad_number_input(&root).focus();
    settle().await;
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .active_element()
            .unwrap()
            .is_same_node(Some(&pad_number_input(&root)))
    );
    let _ = pad_number_input(&root).blur();
    settle().await;
    support::run_pending(&runtime).await;
    let accepted = runtime.model().accepted.as_ref().unwrap().clone();
    assert_eq!(
        accepted.document.revision,
        refreshed.document.revision + 1,
        "dirty row draft submits one edit against the accepted renamed pad"
    );
    assert_eq!(accepted.document.definitions[1].pads[0].id, "renamed-pad");
    assert_eq!(accepted.document.definitions[1].pads[0].number, "7");
    assert_eq!(accepted.document.definitions[1].pads[0].at.y, 4.0);

    // Deleting a row must retire its draft, even when the next pad has
    // identical accepted scalar values and moves into that row's index.
    let mut with_survivor = accepted.document.as_ref().clone();
    let mut survivor = with_survivor.definitions[1].pads[0].clone();
    survivor.id = "surviving-pad".into();
    with_survivor.definitions[1].pads.push(survivor);
    let expanded = support::replace_document(
        &runtime,
        "parts04-mounted-refresh-912",
        "second",
        with_survivor,
    )
    .await;
    controls.snapshot.set(expanded.clone());
    controls
        .definition
        .set(expanded.document.definitions[1].clone());
    settle().await;
    assert!(crate::parts_custom_definition::set_pad_number_draft_for_test("9"));
    settle().await;
    assert_eq!(pad_number_input(&root).value(), "9");
    all_by_role_name(&root, "button", "Remove pad")
        .into_iter()
        .next()
        .expect("remove pad action")
        .click();
    settle().await;
    support::run_pending(&runtime).await;
    let accepted = runtime.model().accepted.as_ref().unwrap().clone();
    assert_eq!(
        accepted.document.revision,
        expanded.document.revision + 1,
        "production Remove pad emits one scoped edit"
    );
    assert_eq!(accepted.document.definitions[1].pads.len(), 1);
    assert_eq!(accepted.document.definitions[1].pads[0].id, "surviving-pad");
    controls.snapshot.set(accepted.clone());
    controls
        .definition
        .set(accepted.document.definitions[1].clone());
    settle().await;
    assert_eq!(pad_id_input(&root).value(), "surviving-pad");
    assert_eq!(
        pad_number_input(&root).value(),
        "7",
        "a removed pad's dirty draft cannot move into the surviving pad"
    );

    controls
        .selection
        .set(Some((scope.clone(), "first".into())));
    controls
        .definition
        .set(accepted.document.definitions[0].clone());
    controls.snapshot.set(accepted.clone());
    settle().await;
    assert_eq!(pad_id_input(&root).value(), "shared-pad");
    assert_eq!(pad_number_input(&root).value(), "1");
    let _ = pad_number_input(&root).blur();
    settle().await;
    assert!(
        support::take_held_effects(&runtime).is_empty(),
        "dirty values owned by the prior definition cannot commit into the next owner"
    );
    assert_eq!(
        runtime.model().accepted.unwrap().document.revision,
        accepted.document.revision
    );
    crate::parts_custom_definition::clear_pad_number_draft_for_test();
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_unchanged_blur_does_not_rewrite_empty_courtyard_or_create_history() {
    let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
    document.definitions = vec![definition("selected", "Empty courtyard")];
    let (runtime, snapshot, scope) = opened(document).await;
    let state = Rc::new(RefCell::new(None));
    let seed = Rc::new(Seed {
        snapshot: snapshot.clone(),
        scope: scope.clone(),
        selection: Some((scope.clone(), "selected".into())),
        definition: snapshot.document.definitions[0].clone(),
        state,
    });
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    let root_index = NEXT_ROOT.with(|next| next.replace(next.get() + 1));
    root.set_id(&format!("parts-name-mounted-regression-{root_index}"));
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed);
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    settle().await;
    section_trigger(&root).click();
    settle().await;
    let width = courtyard_width_input(&root);
    assert_eq!(width.value(), "10");
    let _ = width.focus();
    let _ = width.blur();
    settle().await;
    assert!(
        support::take_held_effects(&runtime).is_empty(),
        "unchanged DraftInput blur must not turn an empty courtyard into an authored rectangle"
    );
    let accepted = runtime.model().accepted.unwrap();
    assert!(accepted.document.definitions[0].courtyard.is_empty());
    assert_eq!(
        accepted.document.revision, snapshot.document.revision,
        "an unchanged blur creates no history"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_pad_id_action_remaps_every_matching_instance_through_the_runtime_edit() {
    let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
    let mut definition = definition("selected", "Footprint");
    definition.pads = serde_json::from_value(serde_json::json!([
        {"id":"old","number":"1","at":{"x":0.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle"},
        {"id":"keep","number":"2","at":{"x":1.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle"}
    ]))
    .unwrap();
    document.definitions = vec![definition.clone()];
    document.parts = serde_json::from_value(serde_json::json!([
            {"id":"instance-a","definitionId":"selected","reference":"U1","pose":{"at":{"x":0.0,"y":0.0},"rotation":0.0},"side":"front"},
            {"id":"instance-b","definitionId":"selected","reference":"U2","pose":{"at":{"x":1.0,"y":0.0},"rotation":0.0},"side":"front"}
        ])).unwrap();
    document.nets = serde_json::from_value(serde_json::json!([
            {"id":"net-a","name":"A","pins":[{"partId":"instance-a","padId":"old"},{"partId":"instance-b","padId":"old"}]},
            {"id":"net-b","name":"B","pins":[{"partId":"instance-a","padId":"keep"}]}
        ])).unwrap();
    let (runtime, snapshot, scope) = opened(document).await;
    let state = Rc::new(RefCell::new(None));
    let seed = Rc::new(Seed {
        snapshot: snapshot.clone(),
        scope: scope.clone(),
        selection: Some((scope.clone(), "selected".into())),
        definition: snapshot.document.definitions[0].clone(),
        state: state.clone(),
    });
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    let root_index = NEXT_ROOT.with(|next| next.replace(next.get() + 1));
    root.set_id(&format!("parts-name-mounted-regression-{root_index}"));
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed);
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    settle().await;
    section_trigger(&root).click();
    settle().await;
    let field = pad_id_input(&root);
    type_value(&field, "renamed");
    let _ = field.blur();
    settle().await;

    support::run_pending(&runtime).await;
    let accepted = runtime.model().accepted.as_ref().unwrap().clone();
    assert_eq!(
        accepted.document.revision,
        snapshot.document.revision + 1,
        "mounted definition field submits one Runtime edit"
    );
    assert_eq!(accepted.document.definitions[0].pads[0].id, "renamed");
    assert_eq!(accepted.document.nets[0].pins[0].pad_id, "renamed");
    assert_eq!(accepted.document.nets[0].pins[1].pad_id, "renamed");
    assert_eq!(accepted.document.nets[1].pins[0].pad_id, "keep");

    runtime.submit(AppEvent::Undo {
        operation_id: runtime.operation(),
    });
    support::run_pending(&runtime).await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .definitions[0]
            .pads[0]
            .id,
        "old"
    );
    root.remove();
}

fn type_value(input: &HtmlInputElement, value: &str) {
    let _ = input.focus();
    input.set_value(value);
    let event = DomEvent::new("input").unwrap();
    event.init_event_with_bubbles_and_cancelable("input", true, true);
    input.dispatch_event(&event).unwrap();
}

fn enter_keydown(input: &HtmlInputElement) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key("Enter");
    init.set_bubbles(true);
    let keydown = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("keydown event");
    input.dispatch_event(&keydown).unwrap();
}

thread_local! {
    static NEXT_ROOT: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
}

/// Mount the definition panel over `document` under its own root element, so one
/// test's leftovers cannot satisfy another test's selectors.
async fn mount_panel(
    document: ProjectDoc,
    open_section: bool,
) -> (
    Rc<Runtime>,
    AcceptedSnapshot,
    Option<Scope>,
    Rc<RefCell<Option<State>>>,
    web_sys::Element,
    MountedFixture,
) {
    let (runtime, snapshot, scope) = opened(document).await;
    let state = Rc::new(RefCell::new(None));
    let seed = Rc::new(Seed {
        snapshot: snapshot.clone(),
        scope: scope.clone(),
        selection: Some((scope.clone(), "selected".into())),
        definition: snapshot.document.definitions[0].clone(),
        state: state.clone(),
    });
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    let root_index = NEXT_ROOT.with(|next| next.replace(next.get() + 1));
    root.set_id(&format!("parts-name-mounted-regression-{root_index}"));
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed);
    dom.provide_root_context(runtime.clone());
    let mounted = MountedFixture::new(root.clone(), dom);
    settle().await;
    if open_section {
        section_trigger(&root).click();
        settle().await;
    }
    (runtime, snapshot, scope, state, root, mounted)
}

/// Drive a released gate's continuation and every pending effect to completion, then
/// push the accepted snapshot and definition back into the mounted host so the panel
/// settles its tickets against the final document.
async fn accept_edits(runtime: &Rc<Runtime>, controls: &State) {
    settle().await;
    support::run_pending(runtime).await;
    refresh_host(runtime, controls).await;
}

async fn refresh_host(runtime: &Rc<Runtime>, controls: &State) {
    let accepted = runtime.model().accepted.clone().unwrap();
    let mut snapshot = controls.snapshot;
    let mut definition_signal = controls.definition;
    snapshot.set(accepted.clone());
    if let Some(definition) = accepted
        .document
        .definitions
        .iter()
        .find(|definition| definition.id == "selected")
        .cloned()
    {
        definition_signal.set(definition);
    }
    // Wake the panel's settle pass the way the production workspace's runtime-change
    // version does, even when the accepted content did not change.
    let mut version = controls.version;
    version += 1;
    settle().await;
}

fn definition_bounds(document: &ProjectDoc) -> (f64, f64) {
    let (width, height, _) =
        crate::parts_custom_definition::courtyard_bounds(&document.definitions[0].courtyard);
    (width, height)
}

fn courtyard_document(name: &str) -> ProjectDoc {
    let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
    let mut selected = definition("selected", name);
    selected.courtyard = vec![
        boardstudio_model::model::Vec2 { x: -5.0, y: -3.0 },
        boardstudio_model::model::Vec2 { x: 5.0, y: -3.0 },
        boardstudio_model::model::Vec2 { x: 5.0, y: 3.0 },
        boardstudio_model::model::Vec2 { x: -5.0, y: 3.0 },
    ];
    document.definitions = vec![selected];
    document
}

#[wasm_bindgen_test]
async fn mounted_dirty_courtyard_width_commits_when_focus_moves_away() {
    // Ticket 07 saw a dirty courtyard width submit nothing when focus moved away.
    // The cause was the harness: a pad-less definition's custom section defaults
    // open, so the old test's summary click closed it and focus/blur no-opped on
    // hidden inputs. With the section open the blur commits and lands.
    let (runtime, snapshot, _scope, state, root, _mounted) =
        mount_panel(courtyard_document("Courtyard"), false).await;
    let controls = state.borrow().as_ref().unwrap().clone();
    assert!(
        section_is_open(&root),
        "a pad-less definition's custom section defaults open"
    );

    type_value(&courtyard_width_input(&root), "12");
    let _ = courtyard_width_input(&root).blur();
    accept_edits(&runtime, &controls).await;

    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(
        accepted.document.revision,
        snapshot.document.revision + 1,
        "a dirty courtyard width must commit when focus moves away"
    );
    assert_eq!(definition_bounds(&accepted.document), (12.0, 6.0));
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_rapid_width_height_keeps_both_and_undo_reverts_only_height() {
    let (runtime, snapshot, _scope, state, root, _mounted) =
        mount_panel(courtyard_document("Courtyard"), false).await;
    let controls = state.borrow().as_ref().unwrap().clone();
    let width = courtyard_width_input(&root);
    let height = courtyard_height_input(&root);

    // Hold the width edit's Core reply so the height edit queues behind it, exactly
    // as rapid Tab-entry does when the engine is busy.
    let (entered, release) = support::gate_next_core_reply(&runtime);
    type_value(&width, "12");
    let _ = height.focus();
    settle().await;
    support::drive_pending(&runtime);
    entered
        .await
        .expect("the width edit reached the in-process Core");

    type_value(&height, "8");
    enter_keydown(&height);
    settle().await;

    release.send(()).expect("release the held width reply");
    accept_edits(&runtime, &controls).await;

    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(
        accepted.document.revision,
        snapshot.document.revision + 2,
        "both queued courtyard edits land"
    );
    assert_eq!(
        definition_bounds(&accepted.document),
        (12.0, 8.0),
        "width, Tab, height keeps both values"
    );
    assert_eq!(courtyard_width_input(&root).value(), "12");
    assert_eq!(courtyard_height_input(&root).value(), "8");

    runtime.submit(AppEvent::Undo {
        operation_id: runtime.operation(),
    });
    accept_edits(&runtime, &controls).await;
    let undone = runtime.model().accepted.unwrap();
    assert_eq!(
        definition_bounds(&undone.document),
        (12.0, 6.0),
        "one Undo reverts only the height change"
    );
    assert_eq!(courtyard_height_input(&root).value(), "6");

    runtime.submit(AppEvent::Undo {
        operation_id: runtime.operation(),
    });
    accept_edits(&runtime, &controls).await;
    let undone_again = runtime.model().accepted.unwrap();
    assert_eq!(
        definition_bounds(&undone_again.document),
        (10.0, 6.0),
        "a second Undo reverts the width change: the edits undo in order"
    );
    assert_eq!(courtyard_width_input(&root).value(), "10");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_rename_then_field_edit_queued_behind_it_both_survive() {
    let (runtime, snapshot, _scope, state, root, _mounted) =
        mount_panel(courtyard_document("Original name"), false).await;
    let controls = state.borrow().as_ref().unwrap().clone();

    let (entered, release) = support::gate_next_core_reply(&runtime);
    type_value(&input(&root), "Renamed");
    let _ = courtyard_width_input(&root).focus();
    settle().await;
    support::drive_pending(&runtime);
    entered
        .await
        .expect("the rename reached the in-process Core");

    type_value(&courtyard_width_input(&root), "12");
    let _ = courtyard_width_input(&root).blur();
    settle().await;

    release.send(()).expect("release the held rename reply");
    accept_edits(&runtime, &controls).await;

    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(
        accepted.document.revision,
        snapshot.document.revision + 2,
        "the rename and the queued width edit both land"
    );
    assert_eq!(accepted.document.definitions[0].name, "Renamed");
    assert_eq!(definition_bounds(&accepted.document), (12.0, 6.0));

    runtime.submit(AppEvent::Undo {
        operation_id: runtime.operation(),
    });
    accept_edits(&runtime, &controls).await;
    let undone = runtime.model().accepted.unwrap();
    assert_eq!(
        undone.document.definitions[0].name, "Renamed",
        "one Undo reverts only the width change; the rename survives"
    );
    assert_eq!(definition_bounds(&undone.document), (10.0, 6.0));
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_pad_actions_refuse_retained_double_clicks_while_core_is_held() {
    let (runtime, snapshot, _scope, state, root, _mounted) =
        mount_panel(courtyard_document("Pad actions"), false).await;
    let controls = state.borrow().as_ref().unwrap().clone();
    let add = by_role_name(&root, "button", "+ Add pad").expect("add pad action");
    let (entered, release) = support::gate_next_core_reply(&runtime);

    // Both events reach the same retained handler before a disabled render.
    add.click();
    add.click();
    settle().await;
    support::drive_pending(&runtime);
    entered.await.expect("the first add reaches Core");
    assert!(add.has_attribute("disabled"));
    release.send(()).expect("release the held add reply");
    accept_edits(&runtime, &controls).await;
    let added = runtime.model().accepted.unwrap();
    assert_eq!(
        added.document.definitions[0].pads.len(),
        1,
        "the retained second click cannot add another pad"
    );
    assert_eq!(added.document.revision, snapshot.document.revision + 1);

    if !section_is_open(&root) {
        section_trigger(&root).click();
        settle().await;
    }
    let remove = by_role_name(&root, "button", "Remove pad").expect("remove pad action");
    let (entered, release) = support::gate_next_core_reply(&runtime);
    remove.click();
    remove.click();
    settle().await;
    support::drive_pending(&runtime);
    entered.await.expect("the first removal reaches Core");
    assert!(remove.has_attribute("disabled"));
    release.send(()).expect("release the held removal reply");
    accept_edits(&runtime, &controls).await;
    let removed = runtime.model().accepted.unwrap();
    assert!(removed.document.definitions[0].pads.is_empty());
    assert_eq!(removed.document.revision, snapshot.document.revision + 2);
    assert!(
        root.query_selector("[role='alert']").unwrap().is_none(),
        "the retained second removal must not queue a missing-pad failure"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_rejected_pad_number_keeps_newer_text_beside_the_failure() {
    let mut document = courtyard_document("Pad failure");
    document.definitions[0].pads = serde_json::from_value(serde_json::json!([
        {"id":"a","number":"1","at":{"x":0.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle"},
        {"id":"b","number":"2","at":{"x":3.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle"}
    ]))
    .unwrap();
    let (runtime, snapshot, _scope, state, root, _mounted) = mount_panel(document, true).await;
    let controls = state.borrow().as_ref().unwrap().clone();
    let (entered, release) = support::gate_next_core_reply(&runtime);
    type_value(&courtyard_width_input(&root), "12");
    let _ = courtyard_width_input(&root).blur();
    settle().await;
    support::drive_pending(&runtime);
    entered.await.expect("the width edit reaches Core");

    // Duplicate numbers are rejected by the resolver, without making the
    // Session enter persistence or Core recovery.
    type_value(&pad_number_input(&root), "2");
    let _ = pad_number_input(&root).blur();
    settle().await;
    type_value(&pad_number_input(&root), "9");
    settle().await;
    release.send(()).expect("release the held width reply");
    accept_edits(&runtime, &controls).await;

    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.revision, snapshot.document.revision + 1);
    assert_eq!(accepted.document.definitions[0].pads[0].number, "1");
    assert_eq!(pad_number_input(&root).value(), "9");
    let alert = root
        .query_selector("[role='alert']")
        .unwrap()
        .expect("the older failure remains inline")
        .text_content()
        .unwrap();
    assert!(
        alert.contains("Pad numbers must be unique"),
        "the failure explains duplicate numbers: {alert}"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_recommitted_name_and_width_baselines_win_over_pending_values() {
    let (runtime, snapshot, _scope, state, root, _mounted) =
        mount_panel(courtyard_document("Original name"), false).await;
    let controls = state.borrow().as_ref().unwrap().clone();
    let (entered, release) = support::gate_next_core_reply(&runtime);

    type_value(&input(&root), "Renamed");
    let _ = input(&root).blur();
    settle().await;
    support::drive_pending(&runtime);
    entered.await.expect("the initial rename reaches Core");
    type_value(&courtyard_width_input(&root), "12");
    let _ = courtyard_width_input(&root).blur();
    settle().await;

    // These are committed values, even though the accepted document still has
    // them: their earlier edits are waiting and must not become the final values.
    type_value(&input(&root), "Original name");
    let _ = input(&root).blur();
    settle().await;
    type_value(&courtyard_width_input(&root), "10");
    let _ = courtyard_width_input(&root).blur();
    settle().await;
    release.send(()).expect("release the held rename reply");
    accept_edits(&runtime, &controls).await;

    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(
        (
            accepted.document.definitions[0].name.as_str(),
            definition_bounds(&accepted.document).0
        ),
        ("Original name", 10.0),
        "the latest committed values win for both fields"
    );
    assert_eq!(accepted.document.revision, snapshot.document.revision + 4);
    assert_eq!(input(&root).value(), "Original name");
    assert_eq!(courtyard_width_input(&root).value(), "10");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_landed_courtyard_keeps_a_newer_draft_then_clean_undo_redo_syncs() {
    let (runtime, _snapshot, _scope, state, root, _mounted) =
        mount_panel(courtyard_document("Courtyard"), false).await;
    let controls = state.borrow().as_ref().unwrap().clone();
    let (entered, release) = support::gate_next_core_reply(&runtime);

    type_value(&courtyard_width_input(&root), "12");
    let _ = courtyard_width_input(&root).blur();
    settle().await;
    support::drive_pending(&runtime);
    entered.await.expect("the width edit reaches Core");
    type_value(&courtyard_width_input(&root), "10");
    settle().await;
    release.send(()).expect("release the held width reply");
    accept_edits(&runtime, &controls).await;

    assert_eq!(
        definition_bounds(&runtime.model().accepted.unwrap().document).0,
        12.0
    );
    assert_eq!(
        courtyard_width_input(&root).value(),
        "10",
        "the newer baseline draft survives landing"
    );

    let init = web_sys::KeyboardEventInit::new();
    init.set_key("Escape");
    init.set_bubbles(true);
    courtyard_width_input(&root)
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
    settle().await;
    assert_eq!(courtyard_width_input(&root).value(), "12");
    runtime.submit(AppEvent::Undo {
        operation_id: runtime.operation(),
    });
    accept_edits(&runtime, &controls).await;
    assert_eq!(
        courtyard_width_input(&root).value(),
        "10",
        "a clean field follows Undo"
    );
    runtime.submit(AppEvent::Redo {
        operation_id: runtime.operation(),
    });
    accept_edits(&runtime, &controls).await;
    assert_eq!(
        courtyard_width_input(&root).value(),
        "12",
        "a clean field follows Redo"
    );
    root.remove();
    let mut document = courtyard_document("Pad draft");
    document.definitions[0].pads = serde_json::from_value(serde_json::json!([
        {"id":"a","number":"1","at":{"x":0.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle"}
    ]))
    .unwrap();
    let (runtime, _snapshot, _scope, state, root, _mounted) = mount_panel(document, true).await;
    let controls = state.borrow().as_ref().unwrap().clone();
    let (entered, release) = support::gate_next_core_reply(&runtime);
    type_value(&pad_number_input(&root), "7");
    let _ = pad_number_input(&root).blur();
    settle().await;
    support::drive_pending(&runtime);
    entered.await.expect("the pad number edit reaches Core");

    type_value(&pad_number_input(&root), "1");
    settle().await;
    release.send(()).expect("release the held pad number reply");
    accept_edits(&runtime, &controls).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.definitions[0].pads[0].number,
        "7"
    );
    assert_eq!(
        pad_number_input(&root).value(),
        "1",
        "the newer pad draft survives its accepted refresh"
    );
    root.remove();
    let (runtime, _snapshot, _scope, state, root, _mounted) =
        mount_panel(courtyard_document("Original name"), false).await;
    let controls = state.borrow().as_ref().unwrap().clone();
    let (entered, release) = support::gate_next_core_reply(&runtime);

    type_value(&input(&root), "Renamed");
    let _ = input(&root).blur();
    settle().await;
    support::drive_pending(&runtime);
    entered
        .await
        .expect("the rename reaches Core before the newer draft");

    // Returning to the old accepted text is still a newer user draft.
    type_value(&input(&root), "Original name");
    settle().await;
    release.send(()).expect("release the held rename reply");
    accept_edits(&runtime, &controls).await;

    assert_eq!(
        runtime.model().accepted.unwrap().document.definitions[0].name,
        "Renamed"
    );
    assert_eq!(
        input(&root).value(),
        "Original name",
        "the accepted refresh preserves the newer draft"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_definition_switch_retires_old_name_and_courtyard_observations() {
    let mut document = courtyard_document("Original name");
    let mut other = document.definitions[0].clone();
    other.id = "other".into();
    other.name = "Other definition".into();
    document.definitions.push(other);
    let (runtime, snapshot, scope, state, root, _mounted) = mount_panel(document, false).await;
    let mut controls = state.borrow().as_ref().unwrap().clone();
    let (entered, release) = support::gate_next_core_reply(&runtime);

    // Target departure is a resolver rejection, so the Session remains usable.
    let mut removed = snapshot.document.as_ref().clone();
    removed.definitions.remove(0);
    support::submit_fixed_command(
        &runtime,
        boardstudio_model::model::EditCommand {
            base_revision: snapshot.document.revision,
            transaction_id: "definition-departs-before-owner-switch".into(),
            phase: boardstudio_model::model::EditPhase::Commit,
            target_ids: vec!["selected".into()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(removed),
            },
        },
    );
    support::drive_pending(&runtime);
    entered.await.expect("the departure reaches Core");

    type_value(&input(&root), "Too late");
    let _ = input(&root).blur();
    settle().await;
    type_value(&courtyard_width_input(&root), "12");
    let _ = courtyard_width_input(&root).blur();
    settle().await;
    controls.selection.set(Some((scope, "other".into())));
    controls
        .definition
        .set(snapshot.document.definitions[1].clone());
    settle().await;

    release.send(()).expect("release the held departure reply");
    accept_edits(&runtime, &controls).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.definitions[0].id,
        "other"
    );
    assert_eq!(input(&root).value(), "Other definition");
    assert_eq!(courtyard_width_input(&root).value(), "10");
    assert!(
        root.query_selector("[role='alert']").unwrap().is_none(),
        "old owner failures never reach the replacement definition"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_failed_save_returns_the_field_to_the_accepted_value_with_a_message() {
    let (runtime, snapshot, _scope, state, root, _mounted) =
        mount_panel(courtyard_document("Courtyard"), false).await;
    let controls = state.borrow().as_ref().unwrap().clone();
    support::fail_next_persist(&runtime, "injected durable write failure");

    type_value(&courtyard_width_input(&root), "12");
    let _ = courtyard_width_input(&root).blur();
    settle().await;
    support::run_pending(&runtime).await;
    refresh_host(&runtime, &controls).await;

    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(
        accepted.document.revision, snapshot.document.revision,
        "a failed save does not move the accepted document"
    );
    assert_eq!(
        courtyard_width_input(&root).value(),
        "10",
        "the failed field shows the accepted value again"
    );
    let alert = root
        .query_selector("[role='alert']")
        .unwrap()
        .expect("the failure is explained inline")
        .text_content()
        .unwrap();
    assert!(
        alert.contains("did not save") && alert.contains("injected durable write failure"),
        "the inline message names the failure: {alert}"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_definition_deletion_retires_the_queued_field_edit_with_a_reason() {
    let (runtime, snapshot, _scope, state, root, _mounted) =
        mount_panel(courtyard_document("Courtyard"), false).await;
    let controls = state.borrow().as_ref().unwrap().clone();

    // Hold a definition deletion's Core reply so the width edit queues behind it.
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let mut deleted = snapshot.document.as_ref().clone();
    deleted.definitions.clear();
    support::submit_fixed_command(
        &runtime,
        boardstudio_model::model::EditCommand {
            base_revision: snapshot.document.revision,
            transaction_id: "delete-definition-before-queued-edit".into(),
            phase: boardstudio_model::model::EditPhase::Commit,
            target_ids: vec!["selected".into()],
            operation: boardstudio_model::model::EditOperation::ReplaceDocument {
                document: Box::new(deleted),
            },
        },
    );
    support::drive_pending(&runtime);
    entered
        .await
        .expect("the deletion reached the in-process Core");

    type_value(&courtyard_width_input(&root), "12");
    let _ = courtyard_width_input(&root).blur();
    settle().await;

    release.send(()).expect("release the held deletion reply");
    accept_edits(&runtime, &controls).await;

    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(
        accepted.document.revision,
        snapshot.document.revision + 1,
        "only the deletion lands; the retired edit moves nothing"
    );
    assert!(
        accepted.document.definitions.is_empty(),
        "the deletion is accepted"
    );
    assert_eq!(
        courtyard_width_input(&root).value(),
        "10",
        "the retired field shows the accepted value again"
    );
    let alert = root
        .query_selector("[role='alert']")
        .unwrap()
        .expect("the retirement is explained inline")
        .text_content()
        .unwrap();
    assert!(
        alert.contains("no longer exists"),
        "the inline message explains why the edit retired: {alert}"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_name_commit_uses_refreshed_runtime_capture_and_definition_owner_defaults() {
    let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
    document.definitions = vec![
        definition("selected", "Original name"),
        definition("other", "Other definition"),
    ];
    let (runtime, snapshot, scope) = opened(document).await;
    assert!(
        scope.is_some(),
        "the mounted editor uses the accepting Session scope"
    );
    let state = Rc::new(RefCell::new(None));
    let seed = Rc::new(Seed {
        snapshot: snapshot.clone(),
        scope: scope.clone(),
        selection: Some((scope.clone(), "selected".into())),
        definition: snapshot.document.definitions[0].clone(),
        state: state.clone(),
    });
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    let root_index = NEXT_ROOT.with(|next| next.replace(next.get() + 1));
    root.set_id(&format!("parts-name-mounted-regression-{root_index}"));
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed);
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    settle().await;

    let field = input(&root);
    type_value(&field, "Dirty name draft");
    settle().await;
    assert_eq!(input(&root).value(), "Dirty name draft");

    let mut controls = state.borrow().as_ref().unwrap().clone();
    let initial_revision = snapshot.document.revision;
    let mut unrelated = snapshot.document.as_ref().clone();
    unrelated
        .parameters
        .insert("independent".into(), serde_json::json!(42));
    let latest = support::replace_document(
        &runtime,
        "parts-name-unrelated-edit",
        "unrelated",
        unrelated,
    )
    .await;
    assert!(latest.document.revision > initial_revision);
    controls.snapshot.set(latest.clone());
    settle().await;
    assert_eq!(
        input(&root).value(),
        "Dirty name draft",
        "an unrelated accepted revision must refresh admission capture without clearing the local field draft"
    );

    let _ = input(&root).blur();
    support::run_pending(&runtime).await;
    let committed = runtime.model().accepted.as_ref().unwrap().clone();
    assert_eq!(
        committed.document.revision,
        latest.document.revision + 1,
        "the mounted blur callback submits through Runtime"
    );
    assert_eq!(
        committed.document.definitions[0].name, "Dirty name draft",
        "the mounted blur must submit using the refreshed accepted capture"
    );
    assert_eq!(
        committed.document.parameters.get("independent"),
        Some(&serde_json::json!(42)),
        "the production commit must preserve the unrelated accepted edit"
    );

    let mut renamed = committed.document.as_ref().clone();
    renamed.definitions[0].name = "Accepted external name".into();
    let latest =
        support::replace_document(&runtime, "parts-name-external-rename", "selected", renamed)
            .await;
    controls.snapshot.set(latest.clone());
    controls
        .definition
        .set(latest.document.definitions[0].clone());
    settle().await;
    assert_eq!(
        input(&root).value(),
        "Accepted external name",
        "an accepted Name change must synchronize the draft"
    );

    assert!(section_is_open(&root));
    section_trigger(&root).click();
    settle().await;
    assert!(!section_is_open(&root));

    controls
        .selection
        .set(Some((scope.clone(), "other".into())));
    controls
        .definition
        .set(latest.document.definitions[1].clone());
    settle().await;
    assert_eq!(input(&root).value(), "Other definition");
    assert!(
        section_is_open(&root),
        "a newly selected empty definition uses its own default-open state"
    );

    let changed_scope = Some(Scope {
        session_epoch: snapshot.session_epoch,
        document_id: snapshot.document.id.clone(),
        board_id: "another-board".into(),
        instance_id: None,
    });
    controls.scope.set(changed_scope.clone());
    controls
        .selection
        .set(Some((changed_scope, "selected".into())));
    controls
        .definition
        .set(definition("selected", "Scoped target"));
    settle().await;
    assert_eq!(input(&root).value(), "Scoped target");
    root.remove();
}
