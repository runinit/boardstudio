use super::*;
use boardstudio_application::Scope;
use boardstudio_model::model::{Board, ProjectDoc};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_role::by_role_name;
use futures_channel::oneshot;
use std::{cell::RefCell, collections::VecDeque};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

const DEFINITION_ID: &str = "generator:ceoloide/switch_mx";

struct CandidateRequest {
    definition: PartDefinition,
    schema: BTreeMap<String, Value>,
    edits: BTreeMap<String, Value>,
    complete: oneshot::Sender<Result<PartDefinition, String>>,
    returned: oneshot::Receiver<()>,
}

#[derive(Clone)]
struct GeneratorMountedFixture {
    snapshot: AcceptedSnapshot,
    scope: Scope,
    first: PartDefinition,
    second: PartDefinition,
    runtime: Rc<Runtime>,
    requests: Rc<RefCell<VecDeque<CandidateRequest>>>,
    next_runtime_change: Rc<RefCell<Option<oneshot::Sender<()>>>>,
}

#[derive(Clone, Copy)]
struct GeneratorMountedSignals {
    definition: Signal<PartDefinition>,
    selected: Signal<Option<(Option<Scope>, String)>>,
    selection_generation: Signal<u64>,
    store: Signal<Option<GeneratorPreviewDraft>>,
}

#[component]
fn GeneratorMountedHost() -> Element {
    let fixture = use_context::<Rc<GeneratorMountedFixture>>();
    let handles = use_context::<Rc<RefCell<Option<GeneratorMountedSignals>>>>();
    let definition = use_signal(|| fixture.first.clone());
    let selected = use_signal(|| Some((Some(fixture.scope.clone()), fixture.first.id.clone())));
    let selection_generation = use_signal(|| 1_u64);
    let scope_generation = use_signal(|| 1_u64);
    let workspace = use_signal(|| "Parts");
    let store = use_signal(|| None::<GeneratorPreviewDraft>);
    let selected_context = use_signal(|| None);
    let anchor_scope = use_signal(|| Some(fixture.scope.clone()));
    let adapter = use_hook(|| {
        super::super::super::selection::SelectionAdapter::new(
            selected_context,
            anchor_scope,
            scope_generation,
        )
    });
    use_context_provider(|| adapter.clone());
    use_context_provider(|| super::super::PartsSelectionGeneration(selection_generation));
    use_context_provider(|| super::super::GeneratorDraftStore(store));
    use_context_provider(|| super::super::super::WorkspaceState(workspace));
    let version = use_signal(|| 0_u64);
    use_context_provider(|| version);
    let _ = version();
    use_hook({
        let runtime = fixture.runtime.clone();
        let next_change = fixture.next_runtime_change.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                if let Some(wake) = next_change.borrow_mut().take() {
                    let _ = wake.send(());
                }
                let mut version = version;
                version += 1;
            }))
        }
    });
    let current = fixture.runtime.model().accepted.unwrap();
    let accepted_definition = current
        .document
        .definitions
        .iter()
        .find(|item| item.id == definition().id)
        .cloned()
        .unwrap_or_else(&*definition);
    *handles.borrow_mut() = Some(GeneratorMountedSignals {
        definition,
        selected,
        selection_generation,
        store,
    });
    rsx! {
        GeneratorSettingsEditor {
            snapshot: current,
            scope: Some(fixture.scope.clone()),
            selected,
            definition: accepted_definition,
        }
    }
}

fn controlled_candidate_provider(
    requests: Rc<RefCell<VecDeque<CandidateRequest>>>,
) -> GeneratorCandidateTestProvider {
    GeneratorCandidateTestProvider(Rc::new(move |definition, schema, edits| {
        let (complete, result) = oneshot::channel();
        let (returned, returned_signal) = oneshot::channel();
        requests.borrow_mut().push_back(CandidateRequest {
            definition,
            schema,
            edits,
            complete,
            returned: returned_signal,
        });
        Box::pin(async move {
            let result = result
                .await
                .unwrap_or_else(|_| Err("controlled preview was retired".into()));
            let _ = returned.send(());
            result
        })
    }))
}

async fn mount_generator_editor() -> (
    web_sys::Element,
    Rc<GeneratorMountedFixture>,
    Rc<RefCell<Option<GeneratorMountedSignals>>>,
    MountedFixture,
) {
    mount_generator_editor_with(definition()).await
}

async fn mount_generator_editor_with(
    first: PartDefinition,
) -> (
    web_sys::Element,
    Rc<GeneratorMountedFixture>,
    Rc<RefCell<Option<GeneratorMountedSignals>>>,
    MountedFixture,
) {
    let mut second = first.clone();
    second.id = "test:other-mx-switch".into();
    second.name = "Other MX switch".into();
    let mut document = ProjectDoc::empty("generator-merge", "Sofle v2");
    document.revision = 4;
    document.definitions = vec![first.clone(), second.clone()];
    document.boards.push(Board {
        id: "left-pcb".into(),
        name: "Left PCB".into(),
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
        .expect("the generator project is accepted");
    let scope = runtime
        .scope()
        .expect("the accepted generator project has a scope");
    let requests = Rc::new(RefCell::new(VecDeque::new()));
    let fixture = Rc::new(GeneratorMountedFixture {
        snapshot,
        scope,
        first,
        second,
        runtime: runtime.clone(),
        requests: requests.clone(),
        next_runtime_change: Default::default(),
    });
    let handles = Rc::new(RefCell::new(None));
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("parts-generator-owner-mounted-test");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(GeneratorMountedHost);
    dom.provide_root_context(fixture.clone());
    dom.provide_root_context(handles.clone());
    dom.provide_root_context(runtime);
    dom.provide_root_context(controlled_candidate_provider(requests));
    let mounted = MountedFixture::new(root.clone(), dom);
    wait_for_control(&root, "spinbutton", "Keycap Width").await;
    (root, fixture, handles, mounted)
}

async fn wait_for_control(root: &web_sys::Element, role: &str, name: &str) {
    for _ in 0..150 {
        if by_role_name(root, role, name).is_some() {
            return;
        }
        gloo_timers::future::TimeoutFuture::new(20).await;
    }
    panic!(
        "timed out waiting for mounted {role} {name}; mounted text was: {}",
        root.text_content().unwrap_or_default()
    );
}

async fn take_request(
    requests: &Rc<RefCell<VecDeque<CandidateRequest>>>,
    width: f64,
) -> CandidateRequest {
    for _ in 0..150 {
        let index = {
            let queue = requests.borrow();
            queue.iter().position(|request| {
                request
                    .edits
                    .get("keycap_width")
                    .and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok()))
                    == Some(width)
            })
        };
        if let Some(index) = index {
            return requests.borrow_mut().remove(index).unwrap();
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
    panic!("no controlled candidate request arrived for width {width}");
}

fn dispatch_width(root: &web_sys::Element, width: &str) {
    let input = by_role_name(root, "spinbutton", "Keycap Width")
        .expect("mounted keycap width field")
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    input.set_value(width);
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
        .unwrap();
}

fn candidate(request: &CandidateRequest) -> PartDefinition {
    apply_input(&request.definition, &request.schema, &request.edits)
        .expect("test width is valid against the retained parameter schema")
}

async fn finish_request(request: CandidateRequest, result: Result<PartDefinition, String>) {
    request.complete.send(result).unwrap();
    request.returned.await.expect("candidate provider returned");
}

async fn wait_for_text(root: &web_sys::Element, selector: &str, text: &str) {
    for _ in 0..150 {
        let found = root
            .query_selector(selector)
            .unwrap()
            .and_then(|element| element.text_content())
            .is_some_and(|content| content.contains(text));
        if found {
            return;
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
    panic!("timed out waiting for `{text}` in {selector}");
}

async fn take_initial_default_request(
    requests: &Rc<RefCell<VecDeque<CandidateRequest>>>,
) -> CandidateRequest {
    for _ in 0..150 {
        if let Some(index) = {
            let queue = requests.borrow();
            queue
                .iter()
                .position(|request| request.edits.get("side") == Some(&Value::String("F".into())))
        } {
            return requests.borrow_mut().remove(index).unwrap();
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
    panic!("initial mounted generator preview did not request the schema default side F");
}

fn definition() -> PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id": DEFINITION_ID,
        "name": "MX switch",
        "kind": "switch",
        "courtyard": [
            {"x": -7.0, "y": -4.0},
            {"x": 7.0, "y": -4.0},
            {"x": 7.0, "y": 4.0}
        ],
        "pads": [{
            "id": "pad-1", "number": "1", "at": {"x": 0.0, "y": 0.0},
            "size": {"x": 2.0, "y": 2.0}, "shape": "circle"
        }],
        "generator": {
            "source": "ceoloide/switch_mx", "version": "bundled-1",
            "parameters": {"keycap_width": 18.0, "keycap_height": 18}
        }
    }))
    .expect("generator definition")
}

#[wasm_bindgen_test]
async fn mounted_generator_provider_failure_is_visible_and_never_enables_apply() {
    let (root, fixture, _, _mounted) = mount_generator_editor().await;
    dispatch_width(&root, "19");
    wait_for_text(&root, "[role='status']", "Generating footprint preview").await;
    let request = take_request(&fixture.requests, 19.0).await;
    assert!(
        by_role_name(&root, "button", "Apply generator settings")
            .expect("apply action")
            .has_attribute("disabled")
    );

    finish_request(request, Err("controlled candidate provider failure".into())).await;
    wait_for_text(
        &root,
        "[role='alert']",
        "controlled candidate provider failure",
    )
    .await;
    assert!(
        by_role_name(&root, "button", "Apply generator settings")
            .expect("apply action")
            .has_attribute("disabled")
    );
    let accepted = fixture.runtime.model().accepted.unwrap();
    assert_eq!(accepted.document, fixture.snapshot.document);
    assert_eq!(
        accepted.document.revision,
        fixture.snapshot.document.revision
    );
    assert_eq!(
        accepted.document.definitions[0]
            .generator
            .as_ref()
            .unwrap()
            .parameters["keycap_width"],
        18.0
    );
    root.remove();
}

fn mounted_value(root: &web_sys::Element, role: &str, name: &str) -> String {
    by_role_name(root, role, name)
        .unwrap_or_else(|| panic!("mounted field {role} {name}"))
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
        .value()
}

fn mounted_textarea_value(root: &web_sys::Element, name: &str) -> String {
    by_role_name(root, "textbox", name)
        .unwrap_or_else(|| panic!("mounted textarea {name}"))
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .unwrap()
        .value()
}

fn mounted_checked(root: &web_sys::Element, name: &str) -> bool {
    by_role_name(root, "checkbox", name)
        .unwrap_or_else(|| panic!("mounted checkbox {name}"))
        .get_attribute("aria-checked")
        .as_deref()
        == Some("true")
}

#[wasm_bindgen_test]
async fn saved_null_and_missing_parameters_show_and_preview_declared_defaults() {
    for saved in [
        serde_json::json!({
            "keycap_width": null, "hotswap": null, "designator": null,
            "keycap_3dmodel_xyz_scale": null, "from": null
        }),
        serde_json::json!({}),
    ] {
        let mut saved_definition = definition();
        saved_definition.generator.as_mut().unwrap().parameters =
            serde_json::from_value(saved).unwrap();
        let (root, fixture, _, _mounted) = mount_generator_editor_with(saved_definition).await;
        let width = mounted_value(&root, "spinbutton", "Keycap Width");
        assert_eq!(width.parse::<f64>(), Ok(18.0));
        assert!(mounted_checked(&root, "hotswap"));
        assert_eq!(mounted_value(&root, "textbox", "designator"), "S");
        let scale = mounted_textarea_value(&root, "keycap_3dmodel_xyz_scale");
        assert_eq!(
            serde_json::from_str::<Value>(&scale).unwrap(),
            serde_json::json!([1, 1, 1])
        );
        assert_eq!(
            Some(mounted_value(&root, "textbox", "from")),
            Some(String::new())
        );

        let request = take_initial_default_request(&fixture.requests).await;
        assert_eq!(request.edits["keycap_width"], 18.0);
        assert_eq!(request.edits["hotswap"], true);
        assert_eq!(request.edits["designator"], "S");
        assert_eq!(
            request.edits["keycap_3dmodel_xyz_scale"],
            serde_json::json!([1, 1, 1])
        );
        assert!(
            !request.edits.contains_key("from"),
            "a net without a default stays absent"
        );
        let prepared = super::super::catalogue::normalize_generator_definition(candidate(&request))
            .await
            .expect("defaults prepare a candidate");
        let parameters = &prepared.generator.as_ref().unwrap().parameters;
        assert_eq!(parameters["hotswap"], true);
        assert!(parameters.get("from").is_none_or(Value::is_null));
        finish_request(request, Ok(prepared)).await;
        wait_for_text(
            &root,
            "[role='status']",
            "Current generator preview is ready",
        )
        .await;
        let accepted = fixture.runtime.model().accepted.unwrap();
        assert_eq!(accepted.document, fixture.snapshot.document);
        assert_eq!(
            accepted.document.revision,
            fixture.snapshot.document.revision
        );
        root.remove();
    }
}

#[wasm_bindgen_test]
async fn explicit_null_draft_selects_the_default_and_non_null_draft_wins() {
    let (root, fixture, _, _mounted) = mount_generator_editor().await;
    let request = take_initial_default_request(&fixture.requests).await;
    let mut saved = request.definition.clone();
    saved
        .generator
        .as_mut()
        .unwrap()
        .parameters
        .insert("keycap_width".into(), Value::from(25));
    let width = |edit: Value| {
        let edits = BTreeMap::from([("keycap_width".to_owned(), edit)]);
        apply_input(&saved, &request.schema, &edits)
            .unwrap()
            .generator
            .unwrap()
            .parameters["keycap_width"]
            .clone()
    };
    assert_eq!(width(Value::Null), 18.0);
    assert_eq!(width(Value::from(30)), 30.0);
    assert_eq!(width(Value::String("31".into())), 31.0);
    assert!(
        apply_input(
            &saved,
            &request.schema,
            &BTreeMap::from([("keycap_width".to_owned(), Value::String(" ".into()))])
        )
        .is_err()
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_generator_initial_schema_default_seeds_unapplied_preview() {
    let (root, fixture, handles, _mounted) = mount_generator_editor().await;
    let side = by_role_name(&root, "button", "side").expect("mounted Board side control");
    assert!(
        side.text_content().unwrap_or_default().contains("Front"),
        "the side picker shows the Front default"
    );

    let request = take_initial_default_request(&fixture.requests).await;
    assert_eq!(request.edits.get("side"), Some(&Value::String("F".into())));
    let candidate = super::super::catalogue::normalize_generator_definition(candidate(&request))
        .await
        .expect("packaged normalizer prepares the default-side draft");
    assert_eq!(
        candidate.pads.len(),
        7,
        "real MX hotswap geometry replaces the catalogue fixture"
    );
    let pad_one = candidate.pads.iter().find(|pad| pad.number == "1").unwrap();
    let pad_two = candidate.pads.iter().find(|pad| pad.number == "2").unwrap();
    assert!(
        pad_one.at.x > 0.0,
        "Front preview places pad 1 on the right"
    );
    assert!(pad_two.at.x < 0.0, "Front preview places pad 2 on the left");
    finish_request(request, Ok(candidate.clone())).await;
    wait_for_text(
        &root,
        "[role='status']",
        "Current generator preview is ready",
    )
    .await;
    let draft = (handles.borrow().as_ref().unwrap().store)().unwrap();
    assert_eq!(draft.status, GeneratorPreviewStatus::Ready);
    assert_eq!(draft.definition.as_ref(), Some(&candidate));
    assert_eq!(
        draft
            .definition
            .as_ref()
            .unwrap()
            .generator
            .as_ref()
            .unwrap()
            .parameters["side"],
        "F"
    );
    let accepted = fixture.runtime.model().accepted.unwrap();
    assert_eq!(accepted.document, fixture.snapshot.document);
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_generator_discards_superseded_edit_and_selection_results() {
    let (root, fixture, handles, _mounted) = mount_generator_editor().await;
    let initial_request = take_initial_default_request(&fixture.requests).await;
    dispatch_width(&root, "19");
    let older_edit = take_request(&fixture.requests, 19.0).await;
    dispatch_width(&root, "20");
    let newer_edit = take_request(&fixture.requests, 20.0).await;
    let newer_candidate = candidate(&newer_edit);
    finish_request(newer_edit, Ok(newer_candidate)).await;
    wait_for_text(
        &root,
        "[role='status']",
        "Current generator preview is ready",
    )
    .await;
    let ready = (handles.borrow().as_ref().unwrap().store)().unwrap();
    assert_eq!(
        ready
            .definition
            .as_ref()
            .unwrap()
            .generator
            .as_ref()
            .unwrap()
            .parameters["keycap_width"]
            .as_f64(),
        Some(20.0)
    );

    let older_candidate = candidate(&older_edit);
    finish_request(older_edit, Ok(older_candidate)).await;
    let initial_candidate = candidate(&initial_request);
    finish_request(initial_request, Ok(initial_candidate)).await;
    let still_ready = (handles.borrow().as_ref().unwrap().store)().unwrap();
    assert_eq!(
        still_ready
            .definition
            .as_ref()
            .unwrap()
            .generator
            .as_ref()
            .unwrap()
            .parameters["keycap_width"]
            .as_f64(),
        Some(20.0)
    );

    dispatch_width(&root, "21");
    let retired_selection = take_request(&fixture.requests, 21.0).await;
    let mut mounted = handles.borrow().as_ref().copied().unwrap();
    mounted.definition.set(fixture.second.clone());
    mounted.selected.set(Some((
        Some(fixture.scope.clone()),
        fixture.second.id.clone(),
    )));
    mounted
        .selection_generation
        .with_mut(|generation| *generation += 1);
    let new_selection = take_initial_default_request(&fixture.requests).await;
    assert_eq!(new_selection.definition.id, fixture.second.id);
    let pending = (handles.borrow().as_ref().unwrap().store)().unwrap();
    assert_eq!(pending.owner.definition_id, fixture.second.id);
    assert_eq!(pending.status, GeneratorPreviewStatus::Pending);
    let retired_candidate = candidate(&retired_selection);
    finish_request(retired_selection, Ok(retired_candidate)).await;
    let still_pending = (handles.borrow().as_ref().unwrap().store)().unwrap();
    assert_eq!(
        still_pending, pending,
        "retired selection cannot replace the new pending draft"
    );
    let new_candidate = candidate(&new_selection);
    finish_request(new_selection, Ok(new_candidate.clone())).await;
    wait_for_text(
        &root,
        "[role='status']",
        "Current generator preview is ready",
    )
    .await;
    let current = (handles.borrow().as_ref().unwrap().store)().unwrap();
    assert_eq!(current.owner.definition_id, fixture.second.id);
    assert_eq!(current.status, GeneratorPreviewStatus::Ready);
    assert_eq!(current.definition.as_ref(), Some(&new_candidate));
    assert_eq!(
        fixture.runtime.model().accepted.unwrap().document,
        fixture.snapshot.document
    );
    root.remove();
}
