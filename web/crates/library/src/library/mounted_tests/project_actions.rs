use super::*;

#[wasm_bindgen_test]
async fn current_project_delete_sorts_remaining_records_by_stored_name() {
    let current = ProjectDoc::empty("delete-sort-current", "Current keyboard");
    let alpha = ProjectDoc::empty("delete-sort-alpha", "Alpha keyboard");
    let unnamed = ProjectDoc::empty("delete-sort-unnamed", "");
    let (runtime, root, _mounted) = seeded_menu(
        current.clone(),
        &[
            "delete-sort-current",
            "delete-sort-alpha",
            "delete-sort-unnamed",
        ],
        &[&current, &alpha, &unnamed],
        "delete-current-sort-regression",
        3,
    )
    .await;
    click(&root, "Delete Current keyboard");
    wait_until("the delete dialog opens", || delete_dialog_open(&root)).await;
    click(&root, "Delete keyboard");

    wait_until_step(
        "the raw blank name sorts before the displayed Untitled fallback",
        async || {
            runtime
                .model()
                .accepted
                .as_ref()
                .is_some_and(|accepted| accepted.document.id == "delete-sort-unnamed")
                && runtime
                    .store
                    .load_document("delete-sort-current".into())
                    .await
                    .unwrap()
                    .is_none()
        },
    )
    .await;
    assert!(
        runtime
            .store
            .load_document("delete-sort-alpha".into())
            .await
            .unwrap()
            .is_some()
    );
    clean_saved_projects(&runtime, &["delete-sort-alpha", "delete-sort-unnamed"]).await;
    let _ = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .remove_child(&root);
}

#[wasm_bindgen_test]
async fn failed_current_project_replacement_keeps_the_record_and_dialog_available() {
    remove_test_root("delete-current-failure-regression");
    let current = ProjectDoc::empty("delete-failure-current", "Current board");
    let candidate = ProjectDoc::empty("delete-failure-alpha", "Alpha board");
    let runtime = seeded_runtime(
        current.clone(),
        &["delete-failure-current", "delete-failure-alpha"],
        &[&current, &candidate],
    )
    .await;
    boardstudio_web_test_runtime::project_name_test_support::fail_next_persist(
        &runtime,
        "replacement storage failed",
    );
    let (root, _mounted) = mount_menu(runtime.clone(), "delete-current-failure-regression");
    wait_for_saved_cards(&root, 2).await;
    click(&root, "Delete Current board");
    wait_until("the delete dialog opens", || delete_dialog_open(&root)).await;
    click(&root, "Delete keyboard");

    wait_until(
        "a failed replacement stays actionable in the dialog",
        || {
            root.query_selector("[role=alertdialog] [role=alert]")
                .unwrap()
                .is_some()
        },
    )
    .await;
    assert!(delete_dialog_open(&root));
    assert_eq!(
        runtime.model().accepted.unwrap().document.id,
        "delete-failure-current"
    );
    assert_eq!(
        runtime.model().lifecycle,
        boardstudio_application::Lifecycle::Ready
    );
    assert!(
        runtime
            .store
            .load_document("delete-failure-current".into())
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        runtime
            .store
            .load_document("delete-failure-alpha".into())
            .await
            .unwrap()
            .is_some()
    );
    click(&root, "Delete keyboard");
    wait_until_step(
        "the still-open dialog can retry and finish deletion",
        async || {
            runtime
                .model()
                .accepted
                .as_ref()
                .is_some_and(|accepted| accepted.document.id == "delete-failure-alpha")
                && runtime
                    .store
                    .load_document("delete-failure-current".into())
                    .await
                    .unwrap()
                    .is_none()
        },
    )
    .await;
    clean_saved_projects(&runtime, &["delete-failure-alpha"]).await;
    let _ = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .remove_child(&root);
}

#[wasm_bindgen_test]
async fn noncurrent_delete_preserves_accepted_document_and_undo_history() {
    remove_test_root("delete-noncurrent-mounted-regression");
    let current = ProjectDoc::empty("delete-noncurrent-active", "Current board");
    let saved = ProjectDoc::empty("delete-noncurrent-target", "Other board");
    let runtime = seeded_runtime(
        current.clone(),
        &["delete-noncurrent-active", "delete-noncurrent-target"],
        &[&current, &saved],
    )
    .await;

    let initial = runtime.model().accepted.unwrap();
    let mut renamed = (*initial.document).clone();
    renamed.name = "Current board edited".into();
    let edit = submit(&runtime, |operation_id| {
        boardstudio_web_test_runtime::project_name_test_support::fixed_command_event(
            operation_id,
            EditCommand {
                base_revision: initial.document.revision,
                transaction_id: "delete-noncurrent-history-edit".into(),
                phase: EditPhase::Commit,
                target_ids: vec![renamed.id.clone()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(renamed),
                },
            },
        )
    });
    assert_eq!(
        wait_outcome(&runtime, &edit).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    let before_delete = runtime.model().accepted.unwrap();
    runtime
        .delete_saved_project("delete-noncurrent-target".into())
        .await
        .unwrap();
    let after_delete = runtime.model().accepted.unwrap();
    assert_eq!(after_delete.document, before_delete.document);
    assert_eq!(after_delete.session_epoch, before_delete.session_epoch);
    assert_eq!(after_delete.token, before_delete.token);
    assert!(
        runtime
            .store
            .load_document("delete-noncurrent-target".into())
            .await
            .unwrap()
            .is_none()
    );

    let undo = submit(&runtime, |operation_id| Event::Undo { operation_id });
    assert_eq!(
        wait_outcome(&runtime, &undo).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Current board"
    );
    clean_saved_projects(&runtime, &["delete-noncurrent-active"]).await;
}

#[wasm_bindgen_test]
async fn project_menu_name_draft_survives_unrelated_accepted_revision_and_commits_latest_document()
{
    let (session, core) = accepted(ProjectDoc::empty("menu-name", "Sofle v2"));
    let snapshot = session.read_model().accepted.as_ref().unwrap().clone();
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    let mut search_one = ProjectDoc::empty("menu-search-alpha", "Alpha Board");
    search_one.revision = 1;
    let mut search_two = ProjectDoc::empty("menu-search-ergonomic", "Ergonomic 75%");
    search_two.revision = 1;
    let mut search_three = ProjectDoc::empty("menu-search-untitled", "  ");
    search_three.revision = 1;
    save_all(&runtime, &[&search_one, &search_two, &search_three]).await;
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let seed = Rc::new(Seed {
        state: Rc::new(RefCell::new(None)),
        show_library: Rc::new(RefCell::new(None)),
    });
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    root.set_id("project-name-mounted-regression");
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed.clone());
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_for_saved_cards(&root, 4).await;

    let project_menu = root
        .query_selector("[aria-label='Project menu']")
        .unwrap()
        .expect("menu instance uses the dropdown composition");
    let heading = project_menu.first_element_child().expect("menu heading");
    let actions = heading.next_element_sibling().expect("project actions");
    let current = actions.next_element_sibling().expect("current project");
    let content = current.next_element_sibling().expect("saved keyboards");
    let footer = content.next_element_sibling().expect("menu footer");
    assert!(
        footer.next_element_sibling().is_none(),
        "unexpected direct menu child"
    );
    assert!(heading.text_content().unwrap().contains("Keyboards"));
    assert_eq!(
        current.get_attribute("aria-label").as_deref(),
        Some("Current project")
    );
    assert!(content.text_content().unwrap().contains("Saved keyboards"));
    let new_project = by_role_name(&actions, "button", "New project").expect("new project action");
    assert!(
        !new_project
            .get_attribute("class")
            .unwrap_or_default()
            .split_whitespace()
            .any(|class| class.starts_with("m1-")),
        "the project action does not depend on a dead legacy class"
    );
    assert!(by_role_name(&actions, "button", "Open project…").is_some());
    assert!(by_role_name(&footer, "button", "Setup guide").is_some());

    let search = search_field();
    assert_eq!(
        search.get_attribute("placeholder").as_deref(),
        Some("Search your keyboards")
    );
    let search_wrapper = search
        .closest("[data-slot=input-group]")
        .unwrap()
        .unwrap()
        .parent_element()
        .unwrap();
    search_wrapper.remove_attribute("class").unwrap();
    new_project.focus().unwrap();
    focus_library_search();
    wait_until(
        "search receives focus without a legacy wrapper class",
        || search.matches(":focus").unwrap(),
    )
    .await;
    type_value(&search, "  sOfLe  ");
    wait_until("the trimmed search leaves the current keyboard", || {
        saved_card_count(&root) == 1 && saved_card_title(&root).as_deref() == Some("Sofle v2")
    })
    .await;
    assert_eq!(
        saved_card_count(&root),
        1,
        "trimmed case-insensitive search includes the current keyboard"
    );
    assert_eq!(saved_card_title(&root).as_deref(), Some("Sofle v2"));
    type_value(&search, "alpha");
    wait_until("the alpha search shows Alpha Board", || {
        saved_card_count(&root) == 1 && saved_card_title(&root).as_deref() == Some("Alpha Board")
    })
    .await;
    assert_eq!(saved_card_count(&root), 1);
    assert_eq!(saved_card_title(&root).as_deref(), Some("Alpha Board"));
    type_value(&search, "untitled");
    wait_until("the untitled search shows the Untitled fallback", || {
        saved_card_count(&root) == 1
            && saved_card_title(&root).as_deref() == Some("Untitled keyboard")
    })
    .await;
    assert_eq!(saved_card_count(&root), 1);
    assert_eq!(
        saved_card_title(&root).as_deref(),
        Some("Untitled keyboard")
    );
    type_value(&search, "missing");
    wait_until("the missing search reports no matches", || {
        root.text_content()
            .unwrap()
            .contains("No keyboards match your search.")
    })
    .await;
    assert!(
        root.text_content()
            .unwrap()
            .contains("No keyboards match your search.")
    );
    let clear_search = by_role_name(&root, "button", "Clear search").unwrap();
    clear_search.click();
    wait_until("clearing the search restores all four saved cards", || {
        search.value().is_empty() && saved_card_count(&root) == 4
    })
    .await;
    assert_eq!(search.value(), "");
    assert_eq!(saved_card_count(&root), 4);

    assert_eq!(field().value(), "Sofle v2");
    let label = field().closest("[data-slot=label]").unwrap().unwrap();
    assert_eq!(label.text_content().unwrap().trim(), "Current project");
    type_value(&field(), "  My custom keyboard  ");

    let mut unrelated = snapshot.document.as_ref().clone();
    unrelated
        .parameters
        .insert("independent".into(), serde_json::json!(42));
    let unrelated_slot = submit(&runtime, |operation_id| {
        boardstudio_web_test_runtime::project_name_test_support::fixed_command_event(
            operation_id,
            EditCommand {
                base_revision: snapshot.document.revision,
                transaction_id: "project-name-unrelated-edit".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["independent".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(unrelated),
                },
            },
        )
    });
    assert_eq!(
        wait_outcome(&runtime, &unrelated_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    drain_and_yield(&runtime).await;

    assert_eq!(
        field().value(),
        "  My custom keyboard  ",
        "an unrelated accepted revision must not discard the menu's active name draft"
    );
    let latest = runtime.model().accepted.unwrap();
    let rename_slot =
        boardstudio_web_test_runtime::project_name_test_support::observe_next(&runtime);
    let _ = field().blur();
    assert_eq!(
        wait_outcome(&runtime, &rename_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    wait_until("the committed name renders in the field", || {
        field().value() == "My custom keyboard"
    })
    .await;
    let committed = runtime.model().accepted.unwrap();
    assert_eq!(committed.document.revision, latest.document.revision + 1);
    assert_eq!(committed.document.name, "My custom keyboard");
    assert_eq!(
        committed.document.parameters.get("independent"),
        Some(&serde_json::json!(42)),
        "the latest accepted unrelated field must survive the rename"
    );
    assert_eq!(field().value(), "My custom keyboard");
    let stored = runtime
        .store
        .load_document("menu-name".into())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        stored.name, "My custom keyboard",
        "the rename must reach durable browser storage"
    );
    assert_eq!(
        stored.parameters.get("independent"),
        Some(&serde_json::json!(42))
    );

    let undo_slot = submit(&runtime, |operation_id| Event::Undo { operation_id });
    assert_eq!(
        wait_outcome(&runtime, &undo_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    let undone = runtime.model().accepted.unwrap();
    assert_eq!(undone.document.name, "Sofle v2");
    assert_eq!(
        undone.document.parameters.get("independent"),
        Some(&serde_json::json!(42)),
        "Undoing the rename must leave the earlier unrelated edit accepted"
    );
    wait_until("the field follows Undo to the earlier name", || {
        field().value() == "Sofle v2"
    })
    .await;

    let redo_slot = submit(&runtime, |operation_id| Event::Redo { operation_id });
    assert_eq!(
        wait_outcome(&runtime, &redo_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    let redone = runtime.model().accepted.unwrap();
    assert_eq!(redone.document.name, "My custom keyboard");
    wait_until("the field follows Redo to the renamed keyboard", || {
        field().value() == "My custom keyboard"
    })
    .await;

    type_value(&field(), "Keyboard entered");
    let enter_slot =
        boardstudio_web_test_runtime::project_name_test_support::observe_next(&runtime);
    press(&field(), "Enter");
    assert_eq!(
        wait_outcome(&runtime, &enter_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    let entered = runtime.model().accepted.unwrap();
    assert_eq!(entered.document.name, "Keyboard entered");

    type_value(&field(), "Canceled draft");
    press(&field(), "Escape");
    wait_until("Escape restores the accepted name", || {
        field().value() == "Keyboard entered"
    })
    .await;
    let _ = field().blur();
    drain_and_yield(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.revision,
        entered.document.revision
    );

    type_value(&field(), "   ");
    let _ = field().blur();
    wait_until("a blank draft restores the accepted name", || {
        field().value() == "Keyboard entered"
    })
    .await;
    assert_eq!(field().value(), "Keyboard entered");
    assert_eq!(
        runtime.model().accepted.unwrap().document.revision,
        entered.document.revision
    );

    type_value(&field(), "Keyboard entered");
    let _ = field().blur();
    drain_and_yield(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.revision,
        entered.document.revision
    );

    type_value(&field(), "Draft before same-project reopen");
    let reopen_slot = submit(&runtime, |operation_id| Event::Open {
        operation_id,
        document: ProjectDoc::empty("menu-name", "Sofle v2"),
    });
    assert_eq!(
        wait_outcome(&runtime, &reopen_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    wait_until("the same-project reopen restores the accepted name", || {
        field().value() == "Sofle v2"
    })
    .await;
    type_value(&field(), "Stale same-name epoch draft");
    let same_name_reopen = submit(&runtime, |operation_id| Event::Open {
        operation_id,
        document: ProjectDoc::empty("menu-name", "Sofle v2"),
    });
    assert_eq!(
        wait_outcome(&runtime, &same_name_reopen).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    wait_until("the same-name reopen replaces the stale draft", || {
        field().value() == "Sofle v2"
    })
    .await;
    let _ = field().blur();
    drain_and_yield(&runtime).await;
    assert_eq!(runtime.model().accepted.unwrap().document.revision, 0);

    let reopen_saved = submit(&runtime, |operation_id| Event::Open {
        operation_id,
        document: stored.clone(),
    });
    assert_eq!(
        wait_outcome(&runtime, &reopen_saved).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    wait_until("the reopened field shows the saved name", || {
        field().value() == stored.name
    })
    .await;
    let reopened = runtime.model().accepted.unwrap();
    assert_eq!(reopened.document.name, stored.name);
    assert_eq!(
        reopened.document.parameters.get("independent"),
        Some(&serde_json::json!(42)),
        "reopening the saved renamed document restores its unrelated accepted data"
    );
    assert_eq!(field().value(), stored.name);
    let _ = runtime.store.delete_project("menu-name".into()).await;
    let _ = runtime
        .store
        .delete_project("menu-search-alpha".into())
        .await;
    let _ = runtime
        .store
        .delete_project("menu-search-ergonomic".into())
        .await;
    let _ = runtime
        .store
        .delete_project("menu-search-untitled".into())
        .await;
    let _ = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .remove_child(&root);
}

#[wasm_bindgen_test]
async fn retained_project_name_action_is_rejected_after_library_unmount() {
    let (session, core) = accepted(ProjectDoc::empty("menu-name-unmount", "Original"));
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let seed = Rc::new(Seed {
        state: Rc::new(RefCell::new(None)),
        show_library: Rc::new(RefCell::new(None)),
    });
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    root.set_id("project-name-unmount-regression");
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed.clone());
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("the mounted Library publishes its name action", || {
        PROJECT_NAME_ACTION_PROBE.with(|probe| probe.borrow().is_some())
            && seed.show_library.borrow().is_some()
    })
    .await;

    let retained_action = project_name_action();
    assert!(retained_action.mounted.get());
    let mut show_library = seed
        .show_library
        .borrow()
        .expect("host exposes its mounted state");
    show_library.set(false);
    wait_until(
        "Library cleanup retires the exact guard captured by its name action",
        || !retained_action.mounted.get(),
    )
    .await;

    let next_operation =
        boardstudio_web_test_runtime::project_name_test_support::observe_next(&runtime);
    retained_action.commit("Stale retained rename");
    drain_and_yield(&runtime).await;
    assert!(
        next_operation.borrow().is_none(),
        "the retained name action must not submit after unmount"
    );
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.name, "Original");
    assert_eq!(accepted.document.revision, 0);
    let _ = runtime
        .store
        .delete_project("menu-name-unmount".into())
        .await;
    let _ = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .remove_child(&root);
}

#[wasm_bindgen_test]
async fn pending_project_rename_preserves_typing_and_escape_does_not_commit() {
    let (runtime, root, _mounted) = mounted_name_field("menu-name-typing").await;

    use boardstudio_web_test_runtime::project_name_test_support as support;
    let (mut entered, release) = support::gate_next_core_reply(&runtime);
    type_value(&field(), "Committed");
    let _ = field().blur();
    wait_until_step("the rename reaches the gated Core reply", async || {
        support::drive_pending(&runtime);
        matches!(entered.try_recv(), Ok(Some(())))
    })
    .await;
    type_value(&field(), "Canceled");
    press(&field(), "Escape");
    let _ = field().blur();
    release.send(()).unwrap();
    wait_until_step("the gated rename lands as Committed", async || {
        support::run_pending(&runtime).await;
        runtime.model().accepted.unwrap().document.name == "Committed"
    })
    .await;
    // Escape and blur submit nothing after the landing: drain held work, then recheck.
    drain_and_yield(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Committed",
        "Escape and blur must not queue a rename of the old accepted name"
    );
    let (mut entered, release) = support::gate_next_core_reply(&runtime);
    type_value(&field(), "Next committed");
    let _ = field().blur();
    wait_until_step("the next rename reaches the gated Core reply", async || {
        support::drive_pending(&runtime);
        matches!(entered.try_recv(), Ok(Some(())))
    })
    .await;
    type_value(&field(), "Still typing");
    release.send(()).unwrap();
    wait_until_step(
        "the older landing is accepted as Next committed",
        async || {
            support::run_pending(&runtime).await;
            runtime.model().accepted.unwrap().document.name == "Next committed"
        },
    )
    .await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Next committed"
    );
    // The DOM is checked after the landing's Core state; no render signal proves the
    // menu re-rendered, so this is drain-protected only (recorded in the Outcome).
    drain_and_yield(&runtime).await;
    assert_eq!(
        field().value(),
        "Still typing",
        "older landing preserves the newer uncommitted draft"
    );
    let _ = field().blur();
    wait_until_step("the blur of Still typing lands", async || {
        support::run_pending(&runtime).await;
        runtime.model().accepted.unwrap().document.name == "Still typing"
    })
    .await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Still typing"
    );
    runtime
        .store
        .delete_project("menu-name-typing".into())
        .await
        .unwrap();
    root.remove();
}

#[wasm_bindgen_test]
async fn a_failed_unchanged_project_name_restores_the_accepted_name_with_an_inline_failure() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    let (runtime, root, _mounted) = mounted_name_field("menu-name-unchanged-failure").await;
    support::fail_next_core_reply(&runtime, "name executor failed");
    type_value(&field(), "Rejected");
    let _ = field().blur();
    wait_until_step(
        "the unchanged draft restores with its inline failure",
        async || {
            support::run_pending(&runtime).await;
            field().value() == "Original" && name_alert(&root).is_some()
        },
    )
    .await;
    assert_eq!(runtime.model().accepted.unwrap().document.name, "Original");
    assert_eq!(field().value(), "Original", "the unchanged draft restores");
    assert!(name_alert(&root).is_some_and(|message| message.contains("name executor failed")));
    runtime
        .store
        .delete_project("menu-name-unchanged-failure".into())
        .await
        .unwrap();
    root.remove();
}

#[wasm_bindgen_test]
async fn a_newer_project_name_draft_survives_an_older_failure_that_reports_inline() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    let (runtime, root, _mounted) = mounted_name_field("menu-name-newer-failure").await;
    support::fail_next_core_reply(&runtime, "name executor failed");
    type_value(&field(), "Rejected");
    let _ = field().blur();
    // The older rename has not settled when the user types the next draft.
    type_value(&field(), "Still typing");
    // The inline failure renders with the draft, so its presence is the render sentinel.
    wait_until_step(
        "the older failure reports beside the newer draft",
        async || {
            support::run_pending(&runtime).await;
            name_alert(&root).is_some()
        },
    )
    .await;
    assert_eq!(field().value(), "Still typing", "a newer draft survives");
    assert!(
        name_alert(&root).is_some_and(|message| message.contains("name executor failed")),
        "the older failure reports inline beside the newer text"
    );
    runtime
        .store
        .delete_project("menu-name-newer-failure".into())
        .await
        .unwrap();
    root.remove();
}

#[wasm_bindgen_test]
async fn two_project_name_commits_keep_the_latest_observation_and_both_undo_steps() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    let (runtime, root, _mounted) = mounted_name_field("menu-name-latest-observation").await;
    let (mut first_entered, first_release) = support::gate_next_core_reply(&runtime);
    type_value(&field(), "First name");
    let _ = field().blur();
    wait_until_step(
        "the first rename reaches its gated Core reply",
        async || {
            support::drive_pending(&runtime);
            matches!(first_entered.try_recv(), Ok(Some(())))
        },
    )
    .await;

    let (mut second_entered, second_release) = support::gate_next_core_reply(&runtime);
    type_value(&field(), "Latest name");
    let _ = field().blur();
    first_release.send(()).unwrap();
    wait_until_step(
        "the latest rename reaches its gated Core reply",
        async || {
            support::drive_pending(&runtime);
            matches!(second_entered.try_recv(), Ok(Some(())))
        },
    )
    .await;
    wait_until("the first landing is accepted as First name", || {
        runtime.model().accepted.unwrap().document.name == "First name"
    })
    .await;
    gloo_timers::future::TimeoutFuture::new(0).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "First name"
    );
    assert_eq!(
        field().value(),
        "Latest name",
        "the first landing cannot restore over the latest committed name"
    );
    assert!(name_alert(&root).is_none());

    second_release.send(()).unwrap();
    wait_until_step("the latest rename lands as Latest name", async || {
        support::run_pending(&runtime).await;
        runtime.model().accepted.unwrap().document.name == "Latest name"
    })
    .await;
    drain_and_yield(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Latest name"
    );
    assert_eq!(field().value(), "Latest name");
    assert!(name_alert(&root).is_none());
    for expected in ["First name", "Original"] {
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        wait_until_step("Undo lands and the field follows it", async || {
            support::run_pending(&runtime).await;
            runtime.model().accepted.unwrap().document.name == expected
                && field().value() == expected
        })
        .await;
        assert_eq!(runtime.model().accepted.unwrap().document.name, expected);
        assert_eq!(field().value(), expected, "the settled field follows Undo");
    }
    runtime
        .store
        .delete_project("menu-name-latest-observation".into())
        .await
        .unwrap();
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn delayed_rename_persist_after_owner_replacement_cannot_update_new_project() {
    let (runtime, root, _mounted) = mounted_name_field("menu-name-delayed").await;

    let (mut entered, release) =
        boardstudio_web_test_runtime::project_name_test_support::gate_next_persist(&runtime);
    type_value(&field(), "Delayed old rename");
    let slot = boardstudio_web_test_runtime::project_name_test_support::observe_next(&runtime);
    let _ = field().blur();
    wait_until_step("the actual Persist effect reaches the gate", async || {
        boardstudio_web_test_runtime::project_name_test_support::drive_pending(&runtime);
        matches!(entered.try_recv(), Ok(Some(())))
    })
    .await;
    let (replacement, _replacement_core) =
        accepted(ProjectDoc::empty("menu-name-replacement", "Replacement"));
    boardstudio_web_test_runtime::project_name_test_support::replace_session(&runtime, replacement);
    wait_until("the replacement session renders its name", || {
        field().value() == "Replacement"
    })
    .await;
    release.send(()).expect("release the captured old save");
    wait_until_step(
        "the released old save finishes before fixture cleanup",
        async || {
            runtime
                .store
                .load_document("menu-name-delayed".into())
                .await
                .unwrap()
                .is_some_and(|document| document.name == "Delayed old rename")
        },
    )
    .await;
    drain_and_yield(&runtime).await;
    assert!(
        slot.borrow().is_none(),
        "a completion owned by the prior session epoch is ignored"
    );
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Replacement"
    );
    assert!(!runtime.status().contains("Saved locally"));
    let _ = runtime
        .store
        .delete_project("menu-name-delayed".into())
        .await;
    let _ = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .remove_child(&root);
}
