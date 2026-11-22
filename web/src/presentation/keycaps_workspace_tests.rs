use super::*;
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_core::model::{KeycapBoardSettings, Pose2, Vec2};
use boardstudio_web_ui_shared::test_role::by_role_name;
use boardstudio_web_ui_shared::test_wait::wait_until;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use web_sys::Element as DomElement;

wasm_bindgen_test_configure!(run_in_browser);

fn view(assigned_count: usize) -> Rc<KeycapsView> {
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
        assigned_count,
    })
}

fn mounted_inspector() -> Element {
    let mut projected_view = use_signal(|| view(0));
    let mut selection = use_signal(|| Some("key-a".to_owned()));
    let selected_key_id = selection();
    let view = projected_view();
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
    rsx! {
        button {
            id: "keycaps-test-accept-binding",
            onclick: move |_| {
                let mut next = (*projected_view.peek()).as_ref().clone();
                next.assigned_count = 1;
                projected_view.set(Rc::new(next));
            },
            "Accept binding projection"
        }
        button { onclick: move |_| selection.set(Some("key-a".into())), "Select fixture A1" }
        button { onclick: move |_| selection.set(Some("key-b".into())), "Select fixture B2" }
        button { onclick: move |_| selection.set(None), "Clear fixture selection" }
        {inspector(InspectorInput {
            view: Some(view),
            document: Rc::new(boardstudio_core::model::ProjectDoc::empty(
                "document", "Fixture",
            )),
            selected_key_id,
            on_select_key,
            settings_editor,
            settings_actions: None,
            fit_state: None,
            mechanical_layer_ids: Rc::from([]),
            fit_retry: EventHandler::new(|()| {}),
            fit_navigate: EventHandler::new(|_| {}),
        })}
    }
}

#[wasm_bindgen_test]
async fn inspector_header_shows_accepted_assigned_and_total_key_count() {
    let root = mount_inspector();
    wait_until("the Keycaps header renders", || {
        by_role_name(&root, "region", "Keycaps inspector").is_some_and(|panel| {
            panel
                .text_content()
                .unwrap_or_default()
                .contains("0/2 assigned")
        })
    })
    .await;

    assert!(by_role_name(&root, "heading", "Keycaps").is_some());
    assert!(
        !by_role_name(&root, "region", "Keycaps inspector")
            .unwrap()
            .text_content()
            .unwrap_or_default()
            .contains("Export keycap STEP")
    );

    by_role_name(&root, "button", "Accept binding projection")
        .unwrap()
        .click();
    wait_until("the accepted binding count updates", || {
        by_role_name(&root, "region", "Keycaps inspector").is_some_and(|panel| {
            panel
                .text_content()
                .unwrap_or_default()
                .contains("1/2 assigned")
        })
    })
    .await;

    assert!(
        !by_role_name(&root, "region", "Keycaps inspector")
            .unwrap()
            .text_content()
            .unwrap_or_default()
            .contains("0/2 assigned"),
        "a replaced accepted Keycaps projection refreshes the summary"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn selected_key_disclosure_tracks_selection_without_resetting_user_state() {
    let root = mount_inspector();
    wait_until("the selected key disclosure renders", || {
        by_role_name(&root, "button", "A1 · key").is_some()
    })
    .await;
    let disclosure = by_role_name(&root, "button", "A1 · key").unwrap();
    assert_eq!(
        disclosure.get_attribute("aria-expanded").as_deref(),
        Some("true")
    );
    assert!(element("[aria-label='Keycaps matrices']").is_err());
    assert!(element("[aria-label='Selected keycap']").is_err());
    assert!(by_role_name(&root, "searchbox", "Find a key").is_some());
    assert!(by_role_name(&root, "combobox", "Selected key").is_some());
    assert!(by_role_name(&root, "textbox", "Legend for A1").is_some());
    assert!(element("[aria-label='Keycap color for A1']").is_ok());
    disclosure.click();
    wait_until("the selected key disclosure collapses", || {
        disclosure.get_attribute("aria-expanded").as_deref() == Some("false")
    })
    .await;
    by_role_name(&root, "button", "Select fixture B2")
        .unwrap()
        .click();
    wait_until("selecting B2 updates the disclosure", || {
        by_role_name(&root, "button", "B2 · key").is_some()
    })
    .await;
    let updated = by_role_name(&root, "button", "B2 · key").unwrap();
    assert_eq!(
        updated.get_attribute("aria-expanded").as_deref(),
        Some("false"),
        "changing keys preserves the user's collapsed choice"
    );
    assert!(by_role_name(&root, "textbox", "Legend for B2").is_some());
    assert!(element("[aria-label='Keycap color for B2']").is_ok());
    by_role_name(&root, "button", "Clear fixture selection")
        .unwrap()
        .click();
    wait_until("clearing key selection updates the disclosure", || {
        by_role_name(&root, "button", "Select a key").is_some()
    })
    .await;
    let empty = by_role_name(&root, "button", "Select a key").unwrap();
    assert_eq!(
        empty.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );
    empty.click();
    wait_until("the empty key disclosure opens", || {
        empty.get_attribute("aria-expanded").as_deref() == Some("true")
    })
    .await;
    by_role_name(&root, "button", "Select fixture A1")
        .unwrap()
        .click();
    wait_until("selecting A1 updates the disclosure", || {
        by_role_name(&root, "button", "A1 · key").is_some()
    })
    .await;
    let reopened = by_role_name(&root, "button", "A1 · key").unwrap();
    assert_eq!(
        reopened.get_attribute("aria-expanded").as_deref(),
        Some("true"),
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

fn element(selector: &str) -> Result<DomElement, JsValue> {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#keycaps-workspace-test-root {selector}"))?
        .ok_or_else(|| JsValue::from_str("expected Keycaps Inspector element"))
}
