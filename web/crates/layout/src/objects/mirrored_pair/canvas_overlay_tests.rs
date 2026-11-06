//! Browser coverage for the mounted canvas composition and form ownership.
use super::*;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn owner(open_id: u64) -> MirroredPairOwner {
    MirroredPairOwner {
        editor_instance_id: 7,
        open_id,
        scope_generation: 11,
        scope: boardstudio_application::Scope {
            session_epoch: boardstudio_application::SessionEpoch(13),
            document_id: "mirror-overlay-fixture".into(),
            board_id: "board-current".into(),
            instance_id: None,
        },
        board_id: "board-current".into(),
        snapshot_token: boardstudio_application::SnapshotToken(17),
        revision: 19,
    }
}

fn projection(
    owner: MirroredPairOwner,
    values: MirroredPairFormValues,
) -> MirroredPairFormProjection {
    MirroredPairFormProjection {
        owner,
        values,
        editable: true,
        error: None,
        status: None,
    }
}

fn snapshot() -> boardstudio_application::AcceptedSnapshot {
    let mut document = boardstudio_core::model::ProjectDoc::empty(
        "mirror-overlay-fixture",
        "Mirror overlay fixture",
    );
    document.revision = 19;
    document.boards.push(boardstudio_core::model::Board {
        id: "board-current".into(),
        name: "Current board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    boardstudio_application::AcceptedSnapshot {
        token: boardstudio_application::SnapshotToken(17),
        session_epoch: boardstudio_application::SessionEpoch(13),
        document: std::sync::Arc::new(document),
        scene: std::sync::Arc::new(boardstudio_core::model::SceneDelta {
            module_scenes: vec![],
            revision: 19,
            transaction_id: "mirror-overlay-test".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: boardstudio_core::model::Readiness {
                layout: false,
                outline: false,
                pcb: false,
                case_ready: false,
            },
        }),
    }
}

fn composition() -> Element {
    let mut active = use_signal(|| true);
    let menu_open = use_signal(|| false);
    let mut current = use_signal(|| projection(owner(1), MirroredPairFormValues::default()));
    let mut previewed = use_signal(|| None::<MirroredPairRequest>);
    let mut cancelled = use_signal(|| None::<MirroredPairOwner>);
    rsx! {
        aside { id: "mirror-overlay-objects",
            super::super::AddObjectEntry {
                menu_open,
                show_trigger: true,
                on_select: EventHandler::default(),
                on_open_geometry_scripts: EventHandler::default(),
                snapshot: super::super::AcceptedSource(snapshot()),
                scope: Some(owner(1).scope),
                on_place_component: EventHandler::default(),
                layout_target: Signal::new(None),
                parts_query: Signal::new(String::new()),
                on_browse_parts: EventHandler::default(),
                matrix_setup: None,
                mirrored_pair: Some(MirroredPairMount {
                    form: Some(current()),
                    placement: None,
                    can_open: true,
                    owns_canvas: false,
                    on_open: EventHandler::default(),
                    on_cancel: EventHandler::default(),
                    on_preview: EventHandler::default(),
                    on_move: EventHandler::default(),
                    on_commit: EventHandler::default(),
                    on_created: EventHandler::default(),
                }),
                existing_half: super::super::ExistingHalfMount {
                    projection: None,
                    visible: false,
                    can_open: false,
                    on_open: EventHandler::default(),
                    on_cancel: EventHandler::default(),
                    on_create: EventHandler::default(),
                },
            }
        }
        section { id: "mirror-overlay-workspace", class: "relative min-h-[320px]",
            div { id: "mirror-overlay-canvas", "Layout canvas" }
            if active() {
                MirroredPairCanvasOverlay {
                    projection: current(),
                    on_cancel: move |owner| { cancelled.set(Some(owner)); active.set(false); },
                    on_preview: move |request: MirroredPairRequest| {
                        current.set(projection(request.owner.clone(), MirroredPairFormValues {
                            left_name: request.left_name.clone(),
                            right_name: request.right_name.clone(),
                            rows: request.rows.to_string(),
                            columns: request.columns.to_string(),
                            preset: request.preset,
                            gap_mm: request.gap_mm.to_string(),
                        }));
                        previewed.set(Some(request));
                        active.set(false);
                    },
                }
            }
            button { id: "mirror-overlay-return", onclick: move |_| active.set(true), "Return to form" }
            button {
                id: "mirror-overlay-new-owner",
                onclick: move |_| {
                    current.set(projection(owner(2), MirroredPairFormValues::default()));
                    active.set(true);
                },
                "Open new form",
            }
            if let Some(request) = previewed() {
                output {
                    id: "mirror-overlay-preview-result",
                    "data-owner": "{request.owner.open_id}:{request.owner.scope_generation}:{request.owner.revision}",
                    "data-values": "{request.left_name}|{request.rows}|{request.columns}|{request.gap_mm}",
                }
            }
            if let Some(owner) = cancelled() {
                output {
                    id: "mirror-overlay-cancel-result",
                    "data-owner": "{owner.open_id}:{owner.scope_generation}:{owner.revision}:{owner.scope.document_id}:{owner.board_id}",
                }
            }
        }
    }
}

fn element(selector: &str) -> web_sys::HtmlElement {
    let found = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(selector)
        .unwrap();
    let found =
        found.unwrap_or_else(|| panic!("missing mounted test element for selector: {selector}"));
    found.dyn_into().unwrap()
}

async fn rendered() {
    gloo_timers::future::TimeoutFuture::new(10).await;
}

fn control(role: &str, name: &str) -> web_sys::HtmlElement {
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("mirror-overlay-test-root")
        .unwrap();
    by_role_name(&root, role, name).unwrap_or_else(|| panic!("missing {role}: {name}"))
}

fn set_input(role: &str, name: &str, value: &str) {
    let input = control(role, name);
    input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap()
        .set_value(value);
    let event = web_sys::Event::new("input").unwrap();
    event.init_event_with_bubbles("input", true);
    input.dispatch_event(&event).unwrap();
}

#[wasm_bindgen_test]
async fn canvas_overlay_preserves_preview_values_and_routes_owner_cancel_escape() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("mirror-overlay-test-root");
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(composition),
        dioxus_web::Config::new().rootnode(root.into()),
    );
    rendered().await;

    let overlay = element("#mirror-overlay-workspace [aria-label='New mirrored pair']")
        .parent_element()
        .expect("the form is mounted in its canvas overlay");
    let in_workspace = overlay
        .parent_element()
        .is_some_and(|parent| parent.is_same_node(Some(&element("#mirror-overlay-workspace"))));
    let absent_from_objects = document
        .query_selector("#mirror-overlay-objects [aria-label='New mirrored pair']")
        .unwrap()
        .is_none();
    let one_form_in_workspace = document
        .query_selector_all("#mirror-overlay-workspace [aria-label='New mirrored pair']")
        .unwrap()
        .length()
        == 1;
    let paired_fields = [
        ("textbox", "Left layout name"),
        ("textbox", "Right layout name"),
        ("spinbutton", "Rows per half"),
        ("spinbutton", "Columns per half"),
    ]
    .into_iter()
    .all(|(role, name)| {
        control(role, name)
            .closest("[role='group'][aria-label='Mirrored pair dimensions']")
            .unwrap()
            .is_some()
    });
    control("button", "Key assembly").click();
    wait_until("the real mirrored-pair assembly choices open", || {
        let root = document
            .get_element_by_id("mirror-overlay-test-root")
            .unwrap();
        by_role_name(&root, "listbox", "Key assembly").is_some()
    })
    .await;
    let choices = control("listbox", "Key assembly");
    let assembly_options = choices.text_content().unwrap();
    let escape_options = web_sys::KeyboardEventInit::new();
    escape_options.set_bubbles(true);
    escape_options.set_key("Escape");
    document
        .active_element()
        .unwrap()
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape_options)
                .unwrap(),
        )
        .unwrap();
    wait_until("the mirrored-pair choice popup returns focus", || {
        control("button", "Key assembly")
            .get_attribute("aria-expanded")
            .as_deref()
            == Some("false")
            && document.active_element() == Some(control("button", "Key assembly").into())
    })
    .await;
    set_input("textbox", "Left layout name", "Kept left draft");
    rendered().await;
    set_input("spinbutton", "Rows per half", "4");
    rendered().await;
    set_input("spinbutton", "Columns per half", "6");
    rendered().await;
    control("button", "Preview placement").click();
    rendered().await;
    let preview = element("#mirror-overlay-preview-result");
    let preview_owner = preview.get_attribute("data-owner").unwrap();
    let preview_values = preview.get_attribute("data-values").unwrap();

    control("button", "Return to form").click();
    rendered().await;
    let retained_left = control("textbox", "Left layout name")
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
        .value();
    let retained_rows = control("spinbutton", "Rows per half")
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
        .value();

    let left = control("textbox", "Left layout name");
    let escape_options = web_sys::KeyboardEventInit::new();
    escape_options.set_bubbles(true);
    escape_options.set_key("Escape");
    let escape =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape_options)
            .unwrap();
    left.dispatch_event(&escape).unwrap();
    rendered().await;
    let escaped_owner = element("#mirror-overlay-cancel-result")
        .get_attribute("data-owner")
        .unwrap();
    let hidden_after_escape = !overlay.is_connected();

    control("button", "Open new form").click();
    rendered().await;
    let reset_left = control("textbox", "Left layout name")
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
        .value();
    control("button", "Cancel").click();
    rendered().await;
    let clicked_cancel_owner = element("#mirror-overlay-cancel-result")
        .get_attribute("data-owner")
        .unwrap();

    assert_eq!(
        (in_workspace, absent_from_objects, one_form_in_workspace),
        (true, true, true)
    );
    assert!(
        paired_fields,
        "the four name and dimension controls must remain mounted in the explicit paired field group"
    );
    assert!(assembly_options.contains("MX Solder"));
    assert!(assembly_options.contains("MX Hotswap"));
    assert!(assembly_options.contains("MX Hotswap RGB"));
    assert_eq!(preview_owner, "1:11:19");
    assert_eq!(preview_values, "Kept left draft|4|6|40");
    assert_eq!(
        (retained_left.as_str(), retained_rows.as_str()),
        ("Kept left draft", "4")
    );
    assert!(
        hidden_after_escape,
        "Escape from a focused form field must cancel the current setup owner"
    );
    assert_eq!(
        escaped_owner,
        "1:11:19:mirror-overlay-fixture:board-current"
    );
    assert_eq!(
        reset_left, "Left half",
        "a new open_id must remount the keyed form and reset its local draft"
    );
    assert_eq!(
        clicked_cancel_owner,
        "2:11:19:mirror-overlay-fixture:board-current"
    );
}
