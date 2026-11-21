use super::*;

#[wasm_bindgen_test]
async fn palette_project_tools_consume_once_and_reject_rename_after_project_switch() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    use boardstudio_web_ui_shared::test_role::by_role_name;
    let runtime = support::new_runtime();
    let (session, core) = accepted(ProjectDoc::empty("palette-tools-a", "Project A"));
    support::install(&runtime, session, core);
    let seed = Rc::new(Seed {
        state: Rc::new(RefCell::new(None)),
        show_library: Rc::new(RefCell::new(None)),
    });
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(project_tools_host);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(seed.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    let status = |name| by_role_name(&root, "status", name).map(|e| e.text_content().unwrap());
    let click = |name| by_role_name(&root, "button", name).unwrap().click();
    wait_until("header tools registered", || {
        status("Registered tools").is_some_and(|s| s == "7")
    })
    .await;
    click("Capture Rename project");
    let mut version = seed.state.borrow().unwrap();
    version += 1;
    wait_until("same project refreshed", || {
        status("Header version").as_deref() == Some("1")
    })
    .await;
    click("Run captured tool");
    wait_until("rename queued before Library mount", || {
        status("Tool pending").as_deref() == Some("true")
    })
    .await;
    let (session, core) = accepted(ProjectDoc::empty("palette-tools-b", "Project B"));
    support::install(&runtime, session, core);
    let mut version = seed.state.borrow().unwrap();
    version += 1;
    click("Toggle Library owner");
    wait_until("old project request consumed", || {
        status("Tool pending").as_deref() == Some("false")
            && by_role_name(&root, "button", "Setup guide").is_some()
    })
    .await;
    assert!(
        by_role_name(&root, "textbox", "Project name").is_none(),
        "stale rename must not select the new project"
    );
    click("Run captured tool");
    assert_eq!(
        status("Tool pending").as_deref(),
        Some("false"),
        "old project callback must remain stale"
    );
    click("Palette Rename project");
    wait_until("current project rename focused", || {
        by_role_name(&root, "textbox", "Project name")
            .is_some_and(|e| document.active_element().as_ref() == Some(e.as_ref()))
    })
    .await;
    assert_eq!(field().value(), "Project B");
    assert_eq!(status("Tool pending").as_deref(), Some("false"));
    click("Toggle Library owner");
    wait_until("Library unmounted", || {
        by_role_name(&root, "textbox", "Project name").is_none()
    })
    .await;
    click("Toggle Library owner");
    wait_until("Library remounted", || {
        by_role_name(&root, "button", "Setup guide").is_some()
    })
    .await;
    assert!(
        by_role_name(&root, "textbox", "Project name").is_none(),
        "consumed rename must not replay"
    );
    click("Capture Open demo");
    version += 1;
    wait_until("same project refreshed before demo", || {
        status("Header version").as_deref() == Some("3")
    })
    .await;
    click("Run captured tool");
    wait_until("demo tool focused", || {
        document
            .active_element()
            .is_some_and(|e| e.text_content().as_deref() == Some("Open demo"))
    })
    .await;
    let demo = document.active_element().unwrap();
    assert_eq!(demo.get_attribute("aria-expanded").as_deref(), Some("true"));
    assert_eq!(status("Tool pending").as_deref(), Some("false"));
    click("Toggle Library owner");
    wait_until("demo owner unmounted", || !root.contains(Some(&demo))).await;
    click("Toggle Library owner");
    wait_until("demo owner remounted", || {
        by_role_name(&root, "button", "Setup guide").is_some()
    })
    .await;
    assert!(
        by_role_name(&root, "button", "Open demo")
            .unwrap()
            .get_attribute("aria-expanded")
            .as_deref()
            == Some("false"),
        "consumed demo must not replay"
    );
    assert_eq!(
        runtime.model().accepted.unwrap().document.id,
        "palette-tools-b"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn unreadable_record_keeps_healthy_card_and_requires_confirmation_to_delete() {
    PROJECT_LIST_TEST_RESULTS.with(|results| results.borrow_mut().clear());
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    let healthy = ProjectDoc::empty("partial-list-healthy", "Healthy saved keyboard");
    let mut future = ProjectDoc::empty("partial-list-future", "Future saved keyboard");
    future.format_version = 999;
    save_all(&runtime, &[&healthy, &future]).await;
    let (session, core) = accepted(healthy.clone());
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let (root, _mounted) = mount_menu(runtime.clone(), "partial-list-mounted-regression");
    wait_until(
        "healthy card and raw recovery actions render together",
        || {
            root.text_content()
                .unwrap_or_default()
                .contains("Healthy saved keyboard")
                && by_role_name(&root, "button", "Download raw saved data (JSON only)").is_some()
                && by_role_name(&root, "button", "Delete unreadable keyboard…").is_some()
        },
    )
    .await;
    for name in [
        "Download raw saved data (JSON only)",
        "Delete unreadable keyboard…",
    ] {
        let action = by_role_name(&root, "button", name).expect("recovery action is named");
        assert!(
            !action
                .get_attribute("class")
                .unwrap_or_default()
                .split_whitespace()
                .any(|class| class.starts_with("m1-")),
            "recovery actions do not depend on removed legacy class names"
        );
    }
    assert!(root.text_content().unwrap().contains("partial-list-future"));
    click(&root, "Delete unreadable keyboard…");
    wait_until("unreadable deletion exposes an Alert Dialog", || {
        by_role_name(&root, "alertdialog", "Delete unreadable keyboard?").is_some()
    })
    .await;
    let dialog = by_role_name(&root, "alertdialog", "Delete unreadable keyboard?").unwrap();
    assert!(
        dialog.text_content().unwrap().contains(&future.id),
        "delete confirmation must name its target"
    );
    click(&dialog, "Cancel");
    wait_until("Cancel keeps the unreadable record", || {
        !delete_dialog_open(&root)
    })
    .await;
    drain_and_yield(&runtime).await;
    assert!(
        runtime
            .store
            .raw_project(&wasm_bindgen::JsValue::from_str(&future.id))
            .await
            .unwrap()
            .is_some()
    );
    click(&root, "Delete unreadable keyboard…");
    wait_until("unreadable deletion confirmation reopens", || {
        by_role_name(&root, "alertdialog", "Delete unreadable keyboard?").is_some()
    })
    .await;
    let dialog = by_role_name(&root, "alertdialog", "Delete unreadable keyboard?").unwrap();
    click(&dialog, "Delete unreadable keyboard");
    wait_until_step("confirmed unreadable record is deleted", async || {
        runtime
            .store
            .raw_project(&wasm_bindgen::JsValue::from_str(&future.id))
            .await
            .unwrap()
            .is_none()
    })
    .await;
    assert_eq!(
        runtime
            .store
            .load_document(healthy.id.clone())
            .await
            .unwrap()
            .unwrap(),
        healthy
    );
    clean_saved_projects(&runtime, &[&healthy.id]).await;
    remove_test_root("partial-list-mounted-regression");
}

#[wasm_bindgen_test]
async fn tokenless_unreadable_record_stays_deletable_by_explicit_confirm() {
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    let healthy = ProjectDoc::empty("tokenless-healthy", "Healthy board");
    runtime
        .store
        .save_document(&healthy, &Default::default())
        .await
        .unwrap();
    let raw = js_sys::Object::new();
    js_sys::Reflect::set(
        &raw,
        &"id".into(),
        &wasm_bindgen::JsValue::from_str("tokenless-unreadable"),
    )
    .unwrap();
    let huge = js_sys::BigInt::new(&wasm_bindgen::JsValue::from_str(
        "1267650600228229401496703205376",
    ))
    .unwrap();
    js_sys::Reflect::set(&raw, &"marker".into(), &huge).unwrap();
    runtime
        .store
        .put_raw_record_for_test(raw.into())
        .await
        .unwrap();
    let (session, core) = accepted(healthy.clone());
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let (root, _mounted) = mount_menu(runtime.clone(), "tokenless-unreadable-mounted-regression");
    wait_until(
        "tokenless unreadable record renders with a delete action",
        || by_role_name(&root, "button", "Delete unreadable keyboard…").is_some(),
    )
    .await;
    let button = by_role_name(&root, "button", "Delete unreadable keyboard…").unwrap();
    assert!(
        button.get_attribute("disabled").is_none(),
        "records we cannot read stay deletable without a token"
    );
    click(&root, "Delete unreadable keyboard…");
    wait_until("tokenless deletion exposes an Alert Dialog", || {
        by_role_name(&root, "alertdialog", "Delete unreadable keyboard?").is_some()
    })
    .await;
    let dialog = by_role_name(&root, "alertdialog", "Delete unreadable keyboard?").unwrap();
    let message = dialog.text_content().unwrap();
    assert!(
        message
            .to_lowercase()
            .contains("permanently delete what we cannot read")
    );
    assert!(
        message.contains("tokenless-unreadable"),
        "delete confirmation must name its target"
    );
    click(&dialog, "Delete unreadable keyboard");
    wait_until_step("tokenless unreadable record is deleted", async || {
        runtime
            .store
            .list_projects()
            .await
            .unwrap()
            .unreadable
            .is_empty()
    })
    .await;
    assert_eq!(
        runtime
            .store
            .load_document(healthy.id.clone())
            .await
            .unwrap()
            .unwrap(),
        healthy
    );
    clean_saved_projects(&runtime, &[&healthy.id]).await;
    remove_test_root("tokenless-unreadable-mounted-regression");
}

#[wasm_bindgen_test]
async fn damaged_saved_preview_keeps_open_action_and_healthy_card_available() {
    let mut damaged = ProjectDoc::empty("damaged-preview", "Damaged preview keyboard");
    damaged.definitions = vec![
        serde_json::from_value(serde_json::json!({
            "id": "switch",
            "name": "Switch",
            "kind": "switch",
            "courtyard": [],
            "pads": []
        }))
        .unwrap(),
    ];
    damaged.parts.push(
        serde_json::from_value(serde_json::json!({
            "id": "switch-1",
            "definitionId": "switch",
            "reference": "SW1",
            "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 },
            "side": "front",
            "keycap": { "x": 0.0, "y": 18.0 }
        }))
        .unwrap(),
    );
    let healthy = ProjectDoc::empty("healthy-preview", "Healthy keyboard");
    PROJECT_LIST_TEST_RESULTS.with(|results| {
        let mut results = results.borrow_mut();
        results.clear();
        results.push_back(Ok(vec![damaged, healthy]));
    });

    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    let (root, _mounted) = mount_menu(runtime, "damaged-preview-fallback-mounted-regression");
    wait_for_project_cards(&root, 2).await;

    let damaged_card = by_role_name(&root, "button", "Open Damaged preview keyboard")
        .expect("damaged preview remains an accessible Open action");
    assert!(damaged_card.get_attribute("disabled").is_none());
    assert!(
        damaged_card
            .text_content()
            .unwrap()
            .contains("Preview unavailable")
    );
    assert!(
        damaged_card
            .text_content()
            .unwrap()
            .contains("Open to check this keyboard")
    );
    assert!(
        by_role_name(&root, "button", "Open Healthy keyboard").is_some(),
        "one damaged preview does not hide a healthy saved card"
    );

    PROJECT_LIST_TEST_RESULTS.with(|results| results.borrow_mut().clear());
    remove_test_root("damaged-preview-fallback-mounted-regression");
}

#[wasm_bindgen_test]
async fn saved_project_list_loading_error_and_retry_recover_in_mounted_library() {
    let (session, core) = accepted(ProjectDoc::empty("list-retry-current", "Current keyboard"));
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    PROJECT_LIST_TEST_RESULTS.with(|results| {
        let mut results = results.borrow_mut();
        results.clear();
        results.extend([
            Err(boardstudio_web_host::host::PersistError(
                "fixture list failure".into(),
            )),
            Ok(vec![ProjectDoc::empty(
                "list-retry-recovered",
                "Recovered keyboard",
            )]),
        ]);
    });

    let (root, _mounted) = mount_menu(runtime, "saved-project-list-retry-mounted-regression");
    // The test-only hold on this listing keeps the Loading status on screen for 120 ms.
    wait_until("the initial loading status renders", || {
        root.query_selector("[role='status']")
            .unwrap()
            .is_some_and(|status| {
                status
                    .text_content()
                    .unwrap()
                    .contains("Loading saved keyboards")
            })
    })
    .await;
    assert!(
        root.query_selector("[role='status']")
            .unwrap()
            .is_some_and(|status| status
                .text_content()
                .unwrap()
                .contains("Loading saved keyboards")),
        "initial list request renders its distinct loading status"
    );
    let alert = wait_for_list_error(&root).await;
    assert!(
        alert
            .text_content()
            .unwrap()
            .contains("could not be loaded")
    );
    assert!(
        alert.text_content().unwrap().contains("Try again"),
        "failed listing exposes retry"
    );
    assert_eq!(
        saved_card_count(&root),
        1,
        "the current project remains available while listing fails"
    );

    click(&root, "Try again");
    wait_for_project_cards(&root, 2).await;
    assert!(root.query_selector("[role='alert']").unwrap().is_none());
    assert!(root.text_content().unwrap().contains("Recovered keyboard"));
    assert!(
        root.query_selector("[data-project-id] [aria-current='true']")
            .unwrap()
            .is_some(),
        "retry keeps the current-project marker"
    );
    PROJECT_LIST_TEST_RESULTS.with(|results| results.borrow_mut().clear());
    remove_test_root("saved-project-list-retry-mounted-regression");
}

#[wasm_bindgen_test]
async fn import_waits_for_storage_initialization() {
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::set_storage_initializing(
        &runtime, true,
    );
    let (root, _mounted) = mount_library(runtime.clone(), "import-readiness", false);
    wait_until("import button renders", || {
        by_role_name(&root, "button", "Import project").is_some()
    })
    .await;
    let input = by_role_name(&root, "button", "Import project").unwrap();
    assert!(
        input.has_attribute("disabled"),
        "import must be disabled while storage initializes"
    );
    boardstudio_web_test_runtime::project_name_test_support::set_storage_initializing(
        &runtime, false,
    );
    wait_until(
        "import becomes enabled after storage initialization",
        || !input.has_attribute("disabled"),
    )
    .await;
    boardstudio_web_test_runtime::project_name_test_support::fail_storage_initialization(
        &runtime,
        "injected failure",
    );
    runtime.report("another status");
    wait_until(
        "storage failure remains visible and import is disabled",
        || {
            input.has_attribute("disabled")
                && root
                    .text_content()
                    .is_some_and(|text| text.contains("injected failure"))
        },
    )
    .await;
    root.remove();
    let (menu, _menu_mounted) = mount_menu(runtime, "import-readiness-menu");
    wait_until("menu import renders disabled after storage failure", || {
        by_role_name(&menu, "button", "Open project…")
            .is_some_and(|button| button.has_attribute("disabled"))
            && menu
                .text_content()
                .is_some_and(|text| text.contains("injected failure"))
    })
    .await;
    menu.remove();
}

#[wasm_bindgen_test]
async fn browser_selection_previews_without_opening_and_focuses_the_handoff() {
    let (session, core) = accepted(ProjectDoc::empty(
        "browser-selection-current",
        "Current keyboard",
    ));
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    PROJECT_LIST_TEST_RESULTS.with(|results| {
        results.borrow_mut().clear();
        results.borrow_mut().push_back(Ok(vec![ProjectDoc::empty(
            "browser-selection-other",
            "Other keyboard",
        )]));
    });
    let (root, _mounted) = mount_library(runtime.clone(), "browser-selection-test-root", false);
    wait_for_saved_cards(&root, 2).await;
    click(&root, "Preview Other keyboard");
    wait_until("selection reveals and focuses Open keyboard", || {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .active_element()
            .is_some_and(|element| element.id() == "m1-browser-open-keyboard")
    })
    .await;
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.id,
        "browser-selection-current"
    );
    assert!(
        root.query_selector("[aria-label='Keyboard details and starting guidance']")
            .unwrap()
            .unwrap()
            .text_content()
            .unwrap()
            .contains("Other keyboard")
    );
    click(&root, "Preview Current keyboard");
    wait_until("current keyboard preview is selected", || {
        root.query_selector("[aria-label='Keyboard details and starting guidance'] > h2")
            .unwrap()
            .and_then(|heading| heading.text_content())
            .as_deref()
            == Some("Current keyboard")
    })
    .await;
    click(&root, "Rename project");
    type_value(&field(), "Renamed current");
    let rename_slot =
        boardstudio_web_test_runtime::project_name_test_support::observe_next(&runtime);
    let _ = field().blur();
    assert_eq!(
        wait_outcome(&runtime, &rename_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    wait_until("selected preview follows the accepted rename", || {
        root.query_selector("[aria-label='Keyboard details and starting guidance'] > h2")
            .unwrap()
            .and_then(|heading| heading.text_content())
            .as_deref()
            == Some("Renamed current")
    })
    .await;
    PROJECT_LIST_TEST_RESULTS.with(|results| results.borrow_mut().clear());
    root.remove();
}

#[wasm_bindgen_test]
async fn project_menu_exposes_the_whole_project_copy_action() {
    remove_test_root("project-copy-mounted-regression");
    let (session, core) = accepted(ProjectDoc::empty(
        "project-copy-ui",
        "Project copy keyboard",
    ));
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let (root, _mounted) = mount_menu(runtime, "project-copy-mounted-regression");
    wait_until("the Project menu renders its copy action", || {
        by_role_name(&root, "button", "Save project copy…").is_some()
    })
    .await;

    let copy = by_role_name(&root, "button", "Save project copy…")
        .expect("the Project menu exposes Save project copy");
    let trigger = by_role_name(&root, "button", "Project").expect("project menu trigger");
    assert_eq!(copy.text_content().unwrap().trim(), "Save project copy…");
    assert!(
        !copy
            .get_attribute("class")
            .unwrap_or_default()
            .split_whitespace()
            .any(|class| class.starts_with("m1-"))
    );

    copy.dyn_into::<web_sys::HtmlElement>().unwrap().click();
    // The menu starts open, so "closed" is false before the click and proves its effect.
    wait_until("starting a project copy closes the menu", || {
        root.query_selector("[aria-label='Project menu']")
            .unwrap()
            .and_then(|menu| menu.closest("details").unwrap())
            .is_some_and(|menu| !menu.has_attribute("open"))
    })
    .await;
    let menu = root
        .query_selector("[aria-label='Project menu']")
        .unwrap()
        .unwrap()
        .closest("details")
        .unwrap()
        .unwrap();
    assert!(
        !menu.has_attribute("open"),
        "starting a project copy closes the menu"
    );
    assert_eq!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .active_element(),
        Some(trigger.into()),
        "closing the project menu returns focus to its trigger"
    );
    remove_test_root("project-copy-mounted-regression");
}

#[wasm_bindgen_test]
async fn project_delete_alert_cancel_keeps_the_named_project() {
    use boardstudio_web_ui_shared::test_role::by_role_name;
    let document = ProjectDoc::empty("alert-safe-project", "Safe project");
    let runtime = seeded_runtime(document.clone(), &[], &[&document]).await;
    let (root, _mounted) = mount_library(runtime.clone(), "alert-safe-root", false);
    wait_for_saved_cards(&root, 1).await;
    let opener = by_role_name(&root, "button", "Delete Safe project").unwrap();
    opener.focus().unwrap();
    opener.click();
    wait_until("the named Alert Dialog opens", || {
        by_role_name(&root, "alertdialog", "Delete “Safe project”?").is_some()
    })
    .await;
    let dialog = by_role_name(&root, "alertdialog", "Delete “Safe project”?").unwrap();
    assert_eq!(dialog.get_attribute("aria-modal").as_deref(), Some("true"));
    let confirm = by_role_name(&dialog, "button", "Delete keyboard").unwrap();
    assert!(
        !confirm
            .get_attribute("class")
            .unwrap_or_default()
            .split_whitespace()
            .any(|class| class.starts_with("m1-"))
    );
    let cancel = by_role_name(&dialog, "button", "Cancel").unwrap();
    cancel.click();
    wait_until("Cancel closes the named Alert Dialog", || {
        by_role_name(&root, "alertdialog", "Delete “Safe project”?").is_none()
    })
    .await;
    assert!(
        runtime
            .store
            .load_document(document.id.clone())
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(runtime.model().accepted.unwrap().document.id, document.id);
    runtime.store.delete_project(document.id).await.unwrap();
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn current_project_delete_cancel_and_confirm_use_the_observed_replacement_path() {
    remove_test_root("delete-current-mounted-regression");
    remove_test_root("delete-current-failure-regression");
    let document = ProjectDoc::empty("delete-current-ui", "Current board");
    let runtime = seeded_runtime(
        document.clone(),
        &[
            "delete-current-ui",
            "delete-failure-current",
            "delete-failure-alpha",
        ],
        &[&document],
    )
    .await;
    let (root, _mounted) = mount_menu(runtime.clone(), "delete-current-mounted-regression");
    wait_for_saved_cards(&root, 1).await;

    let trigger = by_role_name(&root, "button", "Delete Current board").unwrap();
    trigger.focus().unwrap();
    trigger.click();
    wait_until("the delete dialog opens", || delete_dialog_open(&root)).await;
    let dialog = by_role_name(&root, "alertdialog", "Delete “Current board”?").unwrap();
    assert_eq!(dialog.get_attribute("aria-modal").as_deref(), Some("true"));
    click(&dialog, "Cancel");
    wait_until("Cancel closes the delete dialog", || {
        !delete_dialog_open(&root)
    })
    .await;
    // Cancel submits nothing: drain held work, then the store must still hold the record.
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&runtime).await;
    assert!(
        runtime
            .store
            .load_document("delete-current-ui".into())
            .await
            .unwrap()
            .is_some()
    );
    // Trusted focus trapping and restoration run in the packaged workflow, which serves the primitive asset.
    trigger.click();
    wait_until("the delete dialog reopens", || delete_dialog_open(&root)).await;
    click(&root, "Delete keyboard");
    let replacement = RefCell::new(None::<String>);
    wait_until_step(
        "the durable fallback is active and deletion closes its dialog",
        async || {
            let accepted = runtime.model().accepted;
            if let Some(accepted) = accepted
                && accepted.document.id != "delete-current-ui"
                && runtime.store.active_project_id("").unwrap() == accepted.document.id
                && !delete_dialog_open(&root)
                && runtime
                    .store
                    .load_document("delete-current-ui".into())
                    .await
                    .unwrap()
                    .is_none()
            {
                *replacement.borrow_mut() = Some(accepted.document.id.clone());
                return true;
            }
            false
        },
    )
    .await;
    let replacement_id = replacement
        .into_inner()
        .expect("the durable fallback must precede deletion");
    assert_eq!(runtime.store.active_project_id("").unwrap(), replacement_id);
    assert!(!delete_dialog_open(&root));
    clean_saved_projects(&runtime, &[&replacement_id]).await;
    let _ = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .remove_child(&root);
}
