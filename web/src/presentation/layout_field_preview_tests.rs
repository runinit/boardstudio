//! Ticket 16: hovering or focusing an Inspector number field previews, in
//! amber, the keys a change to that field would move, and the canvas hint
//! reads "Moves N keys (n per half)". The preview clears on leave, blur or
//! selection change and never writes the document.
use super::*;
use crate::runtime::project_name_test_support as support;
use boardstudio_core::model::ProjectDoc;
use boardstudio_web_ui_shared::test_wait::wait_until;
use serde_json::json;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

const ROWS: u32 = 2;
const COLUMNS: u32 = 4;

#[derive(Clone)]
struct Probe {
    runtime: Rc<crate::runtime::Runtime>,
    root_id: &'static str,
    adapter: Rc<RefCell<Option<SelectionAdapter>>>,
    version: Rc<RefCell<Option<Signal<u64>>>>,
    hidden: Rc<RefCell<Option<Signal<BTreeSet<String>>>>>,
    workspace: Rc<RefCell<Option<Signal<&'static str>>>>,
}

impl Probe {
    fn select(&self, context: objects::TreeContext) {
        let adapter = self.adapter.borrow().clone().expect("mounted adapter");
        selection::submit_context(
            &self.runtime,
            &adapter,
            selection::ContextRequest {
                scope: self.runtime.scope().expect("opened project scope"),
                context,
                mode: boardstudio_application::SelectionMode::Replace,
            },
        );
    }

    fn revision(&self) -> u64 {
        self.runtime
            .model()
            .accepted
            .expect("accepted document")
            .document
            .revision
    }

    fn element(&self, selector: &str) -> web_sys::Element {
        query(self.root_id, selector).unwrap_or_else(|| panic!("no element for {selector}"))
    }
}

/// A mirrored pair opened directly, following the ticket-14 split-document
/// pattern: the pair link (not an edit) is what the preview reads for the
/// mirrored half. The right origin is the reflection of the left (-20)
/// about the pair axis (10).
fn mirrored_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("field-preview", "Field preview");
    document
        .definitions
        .push(serde_json::from_value(definition("switch", "switch")).unwrap());
    // Materialized key parts with canonical member ids, following the
    // ticket-14 split-document pattern: scenes resolve members against
    // these, so the preview names both halves.
    let mut board_parts = Vec::new();
    let mut matrices = Vec::new();
    for (id, origin_x, prefix) in [("left-matrix", -20.0, "L"), ("right-matrix", 40.0, "R")] {
        let mut part_ids = Vec::new();
        let mut cells = Vec::new();
        for row in 0..ROWS {
            for column in 0..COLUMNS {
                let part_id = format!("matrix/{id}/r{row}c{column}");
                part_ids.push(part_id.clone());
                board_parts.push(part_id.clone());
                document.parts.push(
                    serde_json::from_value(json!({
                        "id": part_id, "definitionId": "switch",
                        "reference": format!("{prefix}{row}{column}"),
                        "pose": {"at": {"x": origin_x + column as f64 * 19.0,
                                                "y": row as f64 * 19.0},
                                         "rotation": 0},
                        "side": "front"
                    }))
                    .unwrap(),
                );
                cells.push(json!({"row": row, "column": column, "enabled": true}));
            }
        }
        matrices.push(json!({
            "id": id, "rows": ROWS, "columns": COLUMNS,
            "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": origin_x, "y": 0.0 },
            "definitionId": "switch", "boardId": "board", "partIds": part_ids,
            "cells": cells
        }));
    }
    for matrix in matrices {
        document
            .matrices
            .push(serde_json::from_value(matrix).unwrap());
    }
    document.boards.push(
        serde_json::from_value(json!({
            "id": "board", "name": "Board", "outlineIds": [],
            "partIds": board_parts, "netIds": [], "thickness": 1.6
        }))
        .unwrap(),
    );
    // Layout records go through JSON so additive optional link fields
    // (half-frame contract work in flight) cannot break this fixture.
    document.layouts.push(
        serde_json::from_value(json!({
            "id": "source-layout", "name": "Left half", "boardId": "board",
            "matrixIds": ["left-matrix"], "partIds": []
        }))
        .unwrap(),
    );
    document.layouts.push(
        serde_json::from_value(json!({
            "id": "paired-layout", "name": "Right half", "boardId": "board",
            "matrixIds": ["right-matrix"], "partIds": [],
            "mirrorLink": {
                "sourceId": "source-layout", "axisX": 10.0,
                "correspondences": [
                    {"kind": "matrix", "sourceId": "left-matrix", "mirrorId": "right-matrix"}
                ],
                "exceptions": []
            }
        }))
        .unwrap(),
    );
    document
}

fn definition(id: &str, kind: &str) -> serde_json::Value {
    json!({
        "id": id, "name": id, "kind": kind,
        "courtyard": [
            { "x": -7.0, "y": -7.0 }, { "x": 7.0, "y": 7.0 },
            { "x": 7.0, "y": -7.0 }, { "x": -7.0, "y": 7.0 }
        ],
        "pads": [{ "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
                   "size": { "x": 2.0, "y": 2.0 }, "shape": "rect" }],
    })
}

fn query(root_id: &str, selector: &str) -> Option<web_sys::Element> {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{root_id} {selector}"))
        .unwrap()
}

fn count(root_id: &str, selector: &str) -> u32 {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector_all(&format!("#{root_id} {selector}"))
        .unwrap()
        .length()
}

fn dispatch(element: &web_sys::Element, kind: &str) {
    let event = web_sys::PointerEvent::new(kind).unwrap();
    element.dispatch_event(event.as_ref()).unwrap();
}

fn host() -> Element {
    let probe = use_context::<Probe>();
    let workspace = use_signal(|| "Layout");
    *probe.workspace.borrow_mut() = Some(workspace);
    use_context_provider(|| WorkspaceState(workspace));
    use_context_provider(|| ExportReturnWorkspace(workspace));
    let version = use_signal(|| 0u64);
    *probe.version.borrow_mut() = Some(version);
    // Like the page host, deliver accepted runtime changes to the mounted Editor.
    // Hover-only cases previously advanced this signal manually; Create must settle too.
    use_hook({
        let runtime = probe.runtime.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }))
        }
    });
    use_drop({
        let runtime = probe.runtime.clone();
        move || runtime.unsubscribe()
    });
    use_context_provider(|| version);
    let created = use_signal(|| None::<SetupGuideRequest>);
    use_context_provider(|| created);
    let objects_open = use_signal(|| true);
    let inspector_open = use_signal(|| true);
    use_context_provider(|| CompactPanelState {
        objects_open,
        inspector_open,
    });
    let warning = use_signal(|| false);
    use_context_provider(|| PreferenceStorageWarning(warning));
    let theme = use_signal(|| "light");
    use_context_provider(|| ThemeState(theme));
    let resolved = use_memo(|| "light");
    use_context_provider(|| ResolvedAppearance(resolved));
    let selected = use_signal(|| None);
    let anchor = use_signal(|| None);
    let generation = use_signal(|| 0u64);
    let adapter = SelectionAdapter::new(selected, anchor, generation);
    *probe.adapter.borrow_mut() = Some(adapter.clone());
    use_context_provider(|| adapter);
    let hidden = use_signal(BTreeSet::new);
    let modules_hidden = use_signal(BTreeSet::new);
    let footprints = use_signal(|| false);
    *probe.hidden.borrow_mut() = Some(hidden);
    use_context_provider(|| LayerVisibility {
        hidden,
        modules_hidden,
        footprints,
    });
    boardstudio_web_ui_model::state::use_test_case_generation_state();
    rsx! {
        // Canvas visibility needs production layout CSS, not only the previous DOM markers.
        style { {include_str!("../../assets/workbench-base.css")} }
        style { {include_str!("../../assets/tailwind.css")} }
        style { {include_str!("../../assets/shell.css")} }
        div { class: "flex h-[900px] flex-col",
            boardstudio_web_ui_shared::ui::toast::ToastProvider { Editor {} }
        }
    }
}

/// Open the mirrored pair through the Session and Core and mount the real
/// Layout editor, without waiting: the tests below wait for their own state.
/// Column 2 of the left half is selected: Spread carries columns 2-4 on both
/// halves, 12 keys, "Moves 12 keys (6 per half)".
async fn mount_bare(root_id: &'static str) -> Probe {
    let runtime = support::new_runtime();
    support::open_document(&runtime, mirrored_document()).await;
    let probe = Probe {
        runtime: runtime.clone(),
        root_id,
        adapter: Rc::default(),
        version: Rc::default(),
        hidden: Rc::default(),
        workspace: Rc::default(),
    };
    let dom_document = web_sys::window().unwrap().document().unwrap();
    let root = dom_document.create_element("div").unwrap();
    root.set_id(root_id);
    dom_document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    probe
}

async fn mount(root_id: &'static str) -> Probe {
    let probe = mount_bare(root_id).await;
    wait_until("the Layout canvas draws the mirrored keys", || {
        count(root_id, "[data-matrix-id][data-row][data-column]") == ROWS * COLUMNS * 2
    })
    .await;
    probe.select(objects::TreeContext::Column {
        matrix_id: "left-matrix".into(),
        column: 1,
    });
    wait_until("the Inspector shows the column Spread field", || {
        query(root_id, "input[aria-label='Spread']").is_some()
    })
    .await;
    probe
}

fn spread_field(probe: &Probe) -> web_sys::HtmlElement {
    probe
        .element("input[aria-label='Spread']")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
}

/// The `(row, column)` of every amber field-preview canvas key.
fn previewed_keys(root_id: &str) -> Vec<(String, String, String)> {
    let list = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector_all(&format!(
            "#{root_id} [data-matrix-id][data-field-preview='true']"
        ))
        .unwrap();
    (0..list.length())
        .map(|index| {
            let key = list
                .item(index)
                .unwrap()
                .dyn_into::<web_sys::Element>()
                .unwrap();
            (
                key.get_attribute("data-matrix-id").unwrap_or_default(),
                key.get_attribute("data-row").unwrap_or_default(),
                key.get_attribute("data-column").unwrap_or_default(),
            )
        })
        .collect()
}

fn hint_text(root_id: &str) -> Option<String> {
    query(root_id, "[data-field-preview-hint]")
        .map(|element| element.text_content().unwrap_or_default())
}

// The guide can cover the entire narrow canvas independently of Add's floating form.
#[wasm_bindgen_test]
async fn new_matrix_canvas_preview_avoids_the_resized_setup_guide() {
    let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
    let panel_preferences = ["objects", "inspector"].map(|side| {
        let key = format!("boardstudio:v2:panel:{side}");
        let value = storage.get_item(&key).unwrap();
        (key, value)
    });
    let preferences = setup_guide::read_preferences("field-preview");
    setup_guide::write_preferences(&setup_guide::SetupGuidePreferences {
        project_id: "field-preview".into(),
        open: true,
        current_stage: SetupGuideStage::Layout,
    });
    let root_id = "new-matrix-preview-guide";
    let probe = mount_bare(root_id).await;
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(root_id)
        .unwrap();
    root.set_attribute("style", "width:390px;height:844px")
        .unwrap();
    wait_until("the real setup guide offers another matrix", || {
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Add Thumb cluster")
            .is_some()
    })
    .await;
    probe
        .element("div.flex.h-\\[900px\\]")
        .set_attribute("style", "height:844px")
        .unwrap();
    boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Add Thumb cluster")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    let ghost = "#m1-workspace-panel svg[role='group'] g[aria-label='New matrix preview']";
    wait_until("the guide publishes the live matrix footprint", || {
        count(root_id, &format!("{ghost} rect")) == 30
    })
    .await;
    let before = probe.runtime.model();
    let unobscured = || {
        let canvas = probe
            .element("#m1-workspace-panel svg[role='group']")
            .get_bounding_client_rect();
        let guide = probe
            .element("[data-setup-guide-overlay]")
            .get_bounding_client_rect();
        let preview = probe.element(ghost).get_bounding_client_rect();
        preview.width() >= 80.0
            && preview.height() >= 60.0
            && preview.left() >= canvas.left()
            && preview.right() <= canvas.right()
            && preview.top() >= canvas.top()
            && preview.bottom() <= canvas.bottom()
            && (preview.left() >= guide.right() || preview.top() >= guide.bottom())
    };
    wait_until(
        "the 390px guide leaves a visible main-canvas matrix, not covered space",
        unobscured,
    )
    .await;
    assert_eq!(
        probe.runtime.model(),
        before,
        "reserving paint space writes no accepted state"
    );
    root.set_attribute("style", "width:820px;height:844px")
        .unwrap();
    wait_until("the wider guide leaves the footprint beside it", || {
        let guide = probe
            .element("[data-setup-guide-overlay]")
            .get_bounding_client_rect();
        probe.element(ghost).get_bounding_client_rect().left() >= guide.right() && unobscured()
    })
    .await;
    // Content/dock geometry can change without resizing the SVG. Observe the guide itself.
    probe
        .element("[data-setup-guide-overlay]")
        .set_attribute("style", "width:600px;height:65%;bottom:auto")
        .unwrap();
    wait_until(
        "changed guide dimensions move the footprint below, outside its actual bounds",
        || {
            let guide = probe
                .element("[data-setup-guide-overlay]")
                .get_bounding_client_rect();
            probe.element(ghost).get_bounding_client_rect().top() >= guide.bottom() && unobscured()
        },
    )
    .await;
    // A second settled change cannot borrow a late callback from the width resize above.
    probe
        .element("[data-setup-guide-overlay]")
        .set_attribute("style", "width:500px;height:100%;bottom:auto")
        .unwrap();
    wait_until(
        "a subsequent guide-only resize returns the footprint beside it",
        || {
            let guide = probe
                .element("[data-setup-guide-overlay]")
                .get_bounding_client_rect();
            probe.element(ghost).get_bounding_client_rect().left() >= guide.right() && unobscured()
        },
    )
    .await;
    assert_eq!(
        probe.runtime.model(),
        before,
        "responsive painting does not change camera or geometry"
    );
    boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Hide guide")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    wait_until(
        "hiding the guide hands setup back to its floating form",
        || {
            query(root_id, "[data-setup-guide-overlay]").is_none()
                && query(root_id, "[aria-label='New matrix']").is_some()
                && count(root_id, &format!("{ghost} rect")) == 30
        },
    )
    .await;
    // This fixture retains the desktop viewport. Collapse its real docks before
    // narrowing the container, rather than squeezing pinned docks into a zero-width canvas.
    for label in ["Collapse objects", "Collapse inspector"] {
        if let Some(button) =
            boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", label)
        {
            button.dyn_into::<web_sys::HtmlElement>().unwrap().click();
        }
    }
    root.set_attribute("style", "width:390px;height:844px")
        .unwrap();
    wait_until(
        "the floating setup also leaves its ghost clear of stationary view controls",
        || {
            let Some(preview) = query(root_id, ghost) else {
                return false;
            };
            let preview = preview.get_bounding_client_rect();
            let controls = probe
                .element("[aria-label='Design view']")
                .get_bounding_client_rect();
            let canvas = probe
                .element("#m1-workspace-panel svg[role='group']")
                .get_bounding_client_rect();
            canvas.width() >= 200.0
                && preview.bottom() <= controls.top()
                && preview.right() <= canvas.right()
                && preview.left() >= canvas.left()
                && preview.top() >= canvas.top()
        },
    )
    .await;
    assert_eq!(probe.runtime.model(), before);
    boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Cancel")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    root.set_attribute("style", "width:1440px;height:844px")
        .unwrap();
    for label in ["Show objects", "Show inspector"] {
        if let Some(button) =
            boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", label)
        {
            button.dyn_into::<web_sys::HtmlElement>().unwrap().click();
        }
    }
    wait_until("restore the fixture's dock owners before cleanup", || {
        query(root_id, "#m1-objects-panel[data-mode='pinned']").is_some()
            && query(root_id, "#m1-inspector-panel[data-mode='pinned']").is_some()
            && query(root_id, "[aria-label='New matrix']").is_none()
    })
    .await;
    setup_guide::write_preferences(&preferences);
    root.remove();
    // Dock controls persist browser preferences; don't leak the narrow fixture into its siblings.
    for (key, value) in panel_preferences {
        if let Some(value) = value {
            storage.set_item(&key, &value).unwrap();
        } else {
            storage.remove_item(&key).unwrap();
        }
    }
}

// D2 uses the real setup owner and main canvas, not a hand-built ghost projection.
#[wasm_bindgen_test]
async fn new_matrix_sliders_preview_on_canvas_without_changing_accepted_state() {
    let root_id = "new-matrix-canvas-preview";
    let probe = mount(root_id).await;
    wait_until("the accepted board renders before adding a matrix", || {
        count(root_id, "[data-matrix-id][data-row][data-column]") == ROWS * COLUMNS * 2
    })
    .await;
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(root_id)
        .unwrap();
    let click = |name: &str| {
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", name)
            .unwrap_or_else(|| panic!("missing {name}"))
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
    };
    let open_setup = || async {
        click("Add object");
        wait_until("the real Add menu opens", || {
            query(root_id, "[aria-label='Add']").is_some()
        })
        .await;
        click("Matrix…Single or mirrored-pair rows, columns & key assemblies");
        wait_until("New matrix owns the live sliders", || {
            query(root_id, "[role='slider'][aria-label='Rows']").is_some()
        })
        .await;
    };
    open_setup().await;
    let before = probe.runtime.model();
    let ghost = "#m1-workspace-panel svg[role='group'] g[aria-label='New matrix preview']";
    wait_until(
        "New matrix draws 30 candidate footprints on the main canvas before Create",
        || count(root_id, &format!("{ghost} rect")) == 30,
    )
    .await;
    let canvas_view_box = probe
        .element("#m1-workspace-panel svg[role='group']")
        .get_attribute("viewBox");
    let mut previous_bounds = probe.element(ghost).get_bounding_client_rect();
    let form_bounds = probe
        .element("[aria-label='New matrix']")
        .get_bounding_client_rect();
    assert!(
        previous_bounds.left() >= form_bounds.right(),
        "the main-canvas ghost must be visible beside, not behind, the setup form"
    );
    for (label, expected, keys) in [("Rows", "6", 36), ("Columns", "7", 42)] {
        let slider = probe.element(&format!("[role='slider'][aria-label='{label}']"));
        let init = web_sys::KeyboardEventInit::new();
        init.set_key("ArrowRight");
        init.set_bubbles(true);
        slider
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap(),
            )
            .unwrap();
        wait_until("the slider updates the main-canvas footprint live", || {
            slider.get_attribute("aria-valuenow").as_deref() == Some(expected)
                && count(root_id, &format!("{ghost} rect")) == keys
        })
        .await;
        let bounds = probe.element(ghost).get_bounding_client_rect();
        if label == "Rows" {
            assert!(bounds.height() > previous_bounds.height());
            assert!((bounds.width() - previous_bounds.width()).abs() < 0.01);
        } else {
            assert!(bounds.width() > previous_bounds.width());
            assert!((bounds.height() - previous_bounds.height()).abs() < 0.01);
        }
        previous_bounds = bounds;
        assert_eq!(
            probe
                .element("#m1-workspace-panel svg[role='group']")
                .get_attribute("viewBox"),
            canvas_view_box
        );
        assert_eq!(
            count(root_id, "[data-matrix-id][data-row][data-column]"),
            ROWS * COLUMNS * 2
        );
        assert_eq!(
            probe.runtime.model(),
            before,
            "setup painting must not alter accepted document, scene, revision, preview, camera or selection"
        );
    }
    click("Cancel");
    wait_until("Cancel retires the setup and its canvas ghost", || {
        query(root_id, "[aria-label='New matrix']").is_none() && query(root_id, ghost).is_none()
    })
    .await;
    assert_eq!(
        probe.runtime.model(),
        before,
        "Cancel writes no accepted state"
    );
    open_setup().await;
    wait_until("reopening starts a fresh 5 by 6 ghost", || {
        count(root_id, &format!("{ghost} rect")) == 30
    })
    .await;
    let mut workspace = probe.workspace.borrow().expect("mounted workspace");
    workspace.set("PCB");
    wait_until("leaving Layout retires the form and ghost", || {
        query(root_id, "[aria-label='New matrix']").is_none() && query(root_id, ghost).is_none()
    })
    .await;
    assert_eq!(
        probe.runtime.model(),
        before,
        "stage exit writes no accepted state"
    );
    workspace.set("Layout");
    wait_until(
        "Layout returns without resurrecting the setup ghost",
        || {
            query(
                root_id,
                "#m1-workspace-panel svg[aria-label^='Keyboard layout;']",
            )
            .is_some()
                && query(root_id, ghost).is_none()
        },
    )
    .await;
    open_setup().await;
    wait_until("Create begins with a visible draft ghost", || {
        count(root_id, &format!("{ghost} rect")) == 30
    })
    .await;
    click("Create matrix");
    wait_until("normal Create lands and retires the ghost", || {
        support::drive_pending(&probe.runtime);
        let feedback = query(root_id, "[aria-label='New matrix'] [role='alert']")
            .and_then(|alert| alert.text_content());
        assert!(feedback.is_none(), "Create refused: {feedback:?}");
        probe.revision() == before.accepted.as_ref().unwrap().document.revision + 1
            && query(root_id, "[aria-label='New matrix']").is_none()
            && query(root_id, ghost).is_none()
    })
    .await;
    let after = probe.runtime.model();
    let document = &after.accepted.as_ref().unwrap().document;
    assert_eq!(
        document.matrices.len(),
        before.accepted.as_ref().unwrap().document.matrices.len() + 1
    );
    let created = document
        .matrices
        .iter()
        .find(|matrix| matrix.id != "left-matrix" && matrix.id != "right-matrix")
        .unwrap();
    assert_eq!((created.rows, created.columns), (5, 6));
    assert_eq!(after.camera, before.camera);
    assert!(after.display_preview.is_none());
    assert!(
        matches!(probe.adapter.borrow().as_ref().unwrap().selected_context.peek().as_ref(), Some(selected) if matches!(&selected.context, objects::TreeContext::Matrix { matrix_id, .. } if matrix_id == &created.id))
    );
    assert_eq!(
        count(root_id, "[data-matrix-id][data-row][data-column]"),
        ROWS * COLUMNS * 2 + 30
    );
    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    let undone = probe.runtime.model().accepted.unwrap();
    assert_eq!(undone.document.revision, document.revision + 1);
    assert_eq!(
        undone.document.matrices,
        before.accepted.as_ref().unwrap().document.matrices,
        "one Undo removes exactly the normal Create"
    );
    assert_eq!(
        undone.document.parts,
        before.accepted.as_ref().unwrap().document.parts
    );
    assert!(query(root_id, ghost).is_none());
    root.remove();
}

#[wasm_bindgen_test]
async fn hovering_a_column_field_previews_its_keys_on_both_halves() {
    let root_id = "field-preview-hover";
    let probe = mount(root_id).await;
    let before = probe.revision();
    let field = spread_field(&probe);
    dispatch(&field, "pointerenter");
    wait_until(
        "hovering Spread previews columns 2-4 on both halves",
        || previewed_keys(root_id).len() == (COLUMNS - 1) as usize * ROWS as usize * 2,
    )
    .await;
    let previewed = previewed_keys(root_id);
    assert_eq!(
        previewed.len(),
        12,
        "Spread carries 6 own + 6 mirrored keys"
    );
    assert!(
        previewed
            .iter()
            .all(|(_, _, column)| ["1", "2", "3"].contains(&column.as_str())),
        "only columns 2-4 move: {previewed:?}"
    );
    assert!(
        previewed
            .iter()
            .filter(|(matrix, _, _)| matrix == "left-matrix")
            .count()
            == 6
            && previewed
                .iter()
                .filter(|(matrix, _, _)| matrix == "right-matrix")
                .count()
                == 6,
        "both halves move: {previewed:?}"
    );
    assert!(
        previewed
            .iter()
            .all(|(matrix, _, _)| matrix == "left-matrix" || matrix == "right-matrix"),
        "no other matrix moves: {previewed:?}"
    );
    assert_eq!(
        count(
            root_id,
            "[data-matrix-id][data-field-preview='true'][data-relation-highlight='true']"
        ),
        0,
        "the amber preview never uses the violet reference treatment"
    );
    assert_eq!(
        count(root_id, "[data-matrix-id][data-relation-highlight='true']"),
        0,
        "no transient relation highlight remains"
    );
    assert_eq!(
        count(root_id, "[data-matrix-id][data-anchor-target='true']"),
        2,
        "only the persistent anchor target stays violet"
    );
    assert_eq!(
        count(
            root_id,
            "[data-matrix-id='left-matrix'][data-anchor-target='true'][data-column='0']"
        ),
        2,
        "the previous column is the anchor target"
    );
    assert_eq!(
        hint_text(root_id).as_deref(),
        Some("Moves 12 keys (6 per half)"),
        "the canvas hint names the move"
    );
    assert_eq!(probe.revision(), before, "previewing writes no revision");
    dispatch(&field, "pointerleave");
    wait_until("leaving the field clears its preview", || {
        previewed_keys(root_id).is_empty() && hint_text(root_id).is_none()
    })
    .await;
    assert!(previewed_keys(root_id).is_empty());
    assert_eq!(hint_text(root_id), None);
    assert_eq!(probe.revision(), before, "clearing writes no revision");
}

#[wasm_bindgen_test]
async fn keyboard_focus_previews_and_blur_clears() {
    let root_id = "field-preview-focus";
    let probe = mount(root_id).await;
    let field = spread_field(&probe);
    field.focus().unwrap();
    wait_until("keyboard focus previews the same keys as hover", || {
        previewed_keys(root_id).len() == 12
    })
    .await;
    assert_eq!(
        hint_text(root_id).as_deref(),
        Some("Moves 12 keys (6 per half)"),
        "focus names the move like hover does"
    );
    field.blur().unwrap();
    wait_until("blurring the field clears its preview", || {
        previewed_keys(root_id).is_empty() && hint_text(root_id).is_none()
    })
    .await;
    assert!(previewed_keys(root_id).is_empty());
}

#[wasm_bindgen_test]
async fn changing_the_selection_clears_the_field_preview() {
    let root_id = "field-preview-selection";
    let probe = mount(root_id).await;
    dispatch(&spread_field(&probe), "pointerenter");
    wait_until("hovering the field previews every carried key", || {
        previewed_keys(root_id).len() == 12
    })
    .await;
    probe.select(objects::TreeContext::Key {
        matrix_id: "left-matrix".into(),
        row: 0,
        column: 0,
    });
    wait_until("a selection change clears the field preview", || {
        previewed_keys(root_id).is_empty() && hint_text(root_id).is_none()
    })
    .await;
    assert!(previewed_keys(root_id).is_empty());
}
