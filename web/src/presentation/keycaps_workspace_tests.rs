use super::*;
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_core::model::{KeycapBoardSettings, Pose2, Vec2};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;
use web_sys::{Element as DomElement, Event, HtmlElement};

wasm_bindgen_test_configure!(run_in_browser);

fn view() -> Rc<KeycapsView> {
    let key = |id: &'static str, reference: &'static str| super::super::keycaps_scene::KeycapsKey {
        id: Rc::from(id),
        reference: Rc::from(reference),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        size: Vec2 { x: 18.2, y: 18.2 },
        color: Rc::from("#e8e4dc"),
        legend: Rc::from(""),
        legend_source: super::super::keycaps_scene::LegendSource::Binding,
        binding_label: Rc::from(""),
        search_text: reference.to_lowercase(),
    };
    Rc::new(KeycapsView {
        board_id: Rc::from("board"),
        board_settings: KeycapBoardSettings::default(),
        keys: vec![key("key-a", "A1"), key("key-b", "B2")],
        matrices: Vec::new(),
        assigned_count: 0,
    })
}

fn mounted_inspector() -> Element {
    let mut selection = use_signal(|| Some("key-a".to_owned()));
    let selected_key_id = selection();
    let view = view();
    let on_select_key = EventHandler::new(move |id: String| {
        selection.set((!id.is_empty()).then_some(id));
    });
    let request_sequence = use_signal(|| 0);
    let scope = Scope {
        session_epoch: SessionEpoch(1),
        document_id: "document".to_owned(),
        board_id: "board".to_owned(),
        instance_id: None,
    };
    let settings_editor = selected_key_id.as_deref().and_then(|selected_id| {
        let key = view
            .keys
            .iter()
            .find(|key| key.id.as_ref() == selected_id)?
            .clone();
        let accepted_settings = boardstudio_core::model::KeycapKeySettings::default();
        let selected = SelectedKeySettings {
            key,
            settings: accepted_settings.clone(),
            board_color: KeycapBoardSettings::default().color,
        };
        let actions = KeycapsSettingsActions {
            editor_instance_id: 1,
            scope: scope.clone(),
            scope_generation: 1,
            selection_generation: 1,
            token: SnapshotToken(1),
            revision: 0,
            selected_key_id: selected_id.to_owned(),
            accepted_settings,
            request_sequence,
            feedback: None,
            retry_drafts: Rc::from([]),
            on_change: EventHandler::new(
                |_: super::super::keycaps_settings::KeycapsEditRequest| {},
            ),
            on_retry: EventHandler::new(
                |_: super::super::keycaps_settings::KeycapsRetryIdentity| {},
            ),
            on_discard: EventHandler::new(
                |_: super::super::keycaps_settings::KeycapsRetryIdentity| {},
            ),
        };
        Some((selected, actions))
    });
    inspector(InspectorInput {
        view: Some(view),
        document: Rc::new(boardstudio_core::model::ProjectDoc::empty(
            "document", "Fixture",
        )),
        selected_key_id,
        on_select_key,
        settings_editor,
        settings_actions: None,
        fit_state: None,
        fit_retry: EventHandler::new(|()| {}),
        fit_navigate: EventHandler::new(|_| {}),
    })
}

#[wasm_bindgen_test]
async fn selected_key_disclosure_tracks_selection_without_resetting_user_state() {
    let root = mount_inspector();
    settle().await;

    let disclosure = element("details.m1-keycaps-selected-key").unwrap();
    assert!(is_open(&disclosure), "the Selected key section starts open");
    assert_eq!(
        summary(&disclosure).text_content().as_deref(),
        Some("A1 · key")
    );
    assert!(element(".m1-keycaps-matrix-list").is_err());
    assert!(element(".m1-keycaps-selected-summary").is_err());
    assert!(element("details.m1-keycaps-selected-key input[aria-label='Find a key']").is_ok());
    assert!(element("details.m1-keycaps-selected-key select[aria-label='Selected key']").is_ok());
    assert!(element("details.m1-keycaps-selected-key input[aria-label='Legend for A1']").is_ok());
    assert!(
        element("details.m1-keycaps-selected-key input[aria-label='Keycap color for A1']").is_ok()
    );

    summary(&disclosure).click();
    settle().await;
    assert!(!is_open(&disclosure), "the user can collapse the section");

    select_key("key-b");
    settle().await;
    let updated = element("details.m1-keycaps-selected-key").unwrap();
    assert_eq!(
        summary(&updated).text_content().as_deref(),
        Some("B2 · key")
    );
    assert!(
        !is_open(&updated),
        "changing keys preserves the user's collapsed choice"
    );
    assert!(element("input[aria-label='Legend for B2']").is_ok());
    assert!(element("input[aria-label='Keycap color for B2']").is_ok());

    select_key("");
    settle().await;
    let empty = element("details.m1-keycaps-selected-key").unwrap();
    assert_eq!(
        summary(&empty).text_content().as_deref(),
        Some("Select a key")
    );
    assert!(!is_open(&empty));

    summary(&empty).click();
    settle().await;
    assert!(is_open(
        &element("details.m1-keycaps-selected-key").unwrap()
    ));
    select_key("key-a");
    settle().await;
    let reopened = element("details.m1-keycaps-selected-key").unwrap();
    assert_eq!(
        summary(&reopened).text_content().as_deref(),
        Some("A1 · key")
    );
    assert!(
        is_open(&reopened),
        "changing keys preserves the user's open choice"
    );
    root.remove();
}

fn mount_inspector() -> DomElement {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("keycaps-workspace-test-root");
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(mounted_inspector),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    root
}

fn select_key(id: &str) {
    let select = element("select[aria-label='Selected key']").unwrap();
    js_sys::Reflect::set(select.as_ref(), &"value".into(), &id.into()).unwrap();
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    select
        .dispatch_event(&Event::new_with_event_init_dict("change", &init).unwrap())
        .unwrap();
}

async fn settle() {
    gloo_timers::future::TimeoutFuture::new(40).await;
}

fn element(selector: &str) -> Result<DomElement, JsValue> {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#keycaps-workspace-test-root {selector}"))?
        .ok_or_else(|| JsValue::from_str("expected Keycaps Inspector element"))
}

fn summary(disclosure: &DomElement) -> HtmlElement {
    disclosure
        .query_selector("summary")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap()
}

fn is_open(disclosure: &DomElement) -> bool {
    js_sys::Reflect::get(disclosure.as_ref(), &"open".into())
        .unwrap()
        .as_bool()
        .unwrap_or(false)
}
