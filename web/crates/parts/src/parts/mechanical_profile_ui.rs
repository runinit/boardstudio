use super::super::{WorkspaceState, selection::SelectionAdapter};
use super::GeneratorPreviewDraft;
use super::PartsPreviewPanel;
use super::PartsSelectionGeneration;
use super::{
    AcceptedProfileOwner, CurrentProfileScope, DetachedProfileSpawner, ManualProfileEditor,
    ManualProfileEditorPorts, MechanicalExtractionFuture, MechanicalExtractionRequester,
    StandardProfileFuture, StandardProfileRequester,
};
use crate::parts_mechanical_profile::{
    ProfileDefinitionSource, ProfileEditOwner, mechanical_profile_resolver,
};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::model::{MechanicalPartProfile, PartDefinition};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonVariant},
};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

/// This panel's one bounded key: the mechanical profile save.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProfileKey {
    Save,
}

/// The profile the latest save was submitted with, so a reopened editor starts from
/// the submitted draft; the shared helper owns the observation itself.
#[derive(Clone)]
struct SubmittedProfile {
    owner: ProfileEditOwner,
    scope_generation: u64,
    selection_generation: u64,
    profile: MechanicalPartProfile,
}

#[component]
pub fn PartsMechanicalProfileWorkspace(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    mut selection: Signal<Option<(Option<Scope>, String)>>,
    definition: PartDefinition,
    preview_definition: Option<PartDefinition>,
    generator_draft: Option<GeneratorPreviewDraft>,
    recipe: Vec<crate::parts_preview::PartsPreviewRecipeMember>,
    recipe_error: Option<String>,
    recipe_pending: bool,
    recipe_identity: String,
    preview_title: Option<String>,
    source: ProfileDefinitionSource,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let view_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation()
    });
    let owner = ProfileEditOwner::new(view_id, &snapshot, scope.clone(), source, &definition);
    let selection_generation = use_context::<PartsSelectionGeneration>().0;
    let selection_adapter = use_context::<SelectionAdapter>();
    let workspace = use_context::<WorkspaceState>().0;
    let runtime_version = use_context::<Signal<u64>>();
    let _ = runtime_version();
    let editing = use_signal(|| false);
    let pending_edits = use_hook(PendingEditSignals::<ProfileKey>::new);
    let save_disabled = use_signal(|| false);
    pending_edits.bind_one_shot(ProfileKey::Save, save_disabled);
    use_drop({
        let pending_edits = pending_edits.clone();
        move || pending_edits.unbind_one_shot(&ProfileKey::Save)
    });
    let mut pending = use_signal(|| None::<SubmittedProfile>);
    let mut owner_seen = use_signal(|| None::<(ProfileEditOwner, u64, u64)>);
    let scope_generation_at_render = (selection_adapter.generation)();
    let selection_generation_at_render = selection_generation();
    let mut error = use_signal(String::new);
    use_effect(use_reactive((&owner,), move |_| error.set(String::new())));
    // The panel's saves share one editor owner: a change to the owner or its
    // generations retires every observation silently before the next save begins.
    {
        let current_owner = Some((
            owner.clone(),
            scope_generation_at_render,
            selection_generation_at_render,
        ));
        if owner_seen.read().as_ref() != current_owner.as_ref() {
            if owner_seen.read().is_some() {
                let _ = pending_edits.settle(false, |_| String::new());
                pending.set(None);
            }
            owner_seen.set(current_owner);
        } else if pending.peek().is_some() {
            let live = workspace() == "Parts"
                && selection() == Some((owner.scope.clone(), owner.definition_id.clone()))
                && runtime.scope() == owner.scope
                && runtime.model().accepted.as_ref().is_some_and(|current| {
                    current.session_epoch == owner.session_epoch
                        && current.document.id == owner.document_id
                });
            for result in pending_edits.settle(live, |_| String::new()) {
                if let PendingEditResult::Failed { message, .. } = result {
                    error.set(message);
                }
                pending.set(None);
            }
        }
    }
    let start_editing = {
        let mut editing = editing;
        let mut error = error;
        let scope = scope.clone();
        let definition = definition.clone();
        move |_| {
            selection.set(Some((scope.clone(), definition.id.clone())));
            editing.set(true);
            error.set(String::new());
        }
    };
    let close_editor = {
        let mut editing = editing;
        move |_| {
            editing.set(false);
        }
    };
    let save_profile = {
        let runtime = runtime.clone();
        let owner = owner.clone();
        let definition = definition.clone();
        let pending_edits = pending_edits.clone();
        let mut pending = pending;
        let mut error = error;
        move |profile: MechanicalPartProfile| {
            // Admission: this view's owner is still the current Parts project and the
            // profile belongs to the definition being edited. Every document-dependent
            // check runs in the resolver at execution.
            if pending_edits.is_pending(&ProfileKey::Save)
                || workspace() != "Parts"
                || selection() != Some((owner.scope.clone(), owner.definition_id.clone()))
                || selection_generation() != selection_generation_at_render
                || (selection_adapter.generation)() != scope_generation_at_render
                || runtime.scope() != owner.scope
                || !runtime.model().accepted.as_ref().is_some_and(|current| {
                    current.session_epoch == owner.session_epoch
                        && current.document.id == owner.document_id
                })
                || profile.definition_id != definition.id
            {
                return;
            }
            let resolver = mechanical_profile_resolver(
                source,
                definition.id.clone(),
                definition.clone(),
                profile.clone(),
            );
            pending_edits.begin_one_shot(
                &runtime,
                ProfileKey::Save,
                "parts-mechanical-profile",
                Some("mechanical profile".into()),
                resolver,
            );
            pending.set(Some(SubmittedProfile {
                owner: owner.clone(),
                scope_generation: scope_generation_at_render,
                selection_generation: selection_generation_at_render,
                profile,
            }));
            error.set(String::new());
        }
    };

    if editing() {
        let initial = pending
            .peek()
            .clone()
            .filter(|save| {
                save.owner == owner
                    && save.scope_generation == scope_generation_at_render
                    && save.selection_generation == selection_generation_at_render
            })
            .map(|save| save.profile)
            .or_else(|| definition.mechanical_profile.clone());
        let editor_key = format!("{owner:?}");
        let request_runtime = runtime.clone();
        let request_standard_profile: StandardProfileRequester =
            Rc::new(move |definition_id, family, plate_to_pcb| {
                let operation_id = request_runtime.operation();
                let runtime = request_runtime.clone();
                let future: StandardProfileFuture = Box::pin(async move {
                    runtime
                        .standard_switch_profile(operation_id, definition_id, family, plate_to_pcb)
                        .await
                });
                (operation_id, future)
            });
        let extraction_runtime = runtime.clone();
        let request_mechanical_extraction: MechanicalExtractionRequester =
            Rc::new(move |source, mappings| {
                let operation_id = extraction_runtime.operation();
                let runtime = extraction_runtime.clone();
                let request_id = operation_id.0.to_string();
                let future: MechanicalExtractionFuture = Box::pin(async move {
                    runtime
                        .extract_mechanical_profile(request_id, source, mappings, 0.005)
                        .await
                });
                (operation_id, future)
            });
        let spawn_detached: DetachedProfileSpawner = Rc::new(spawn_local);
        let current_runtime_scope = runtime.clone();
        let current_scope: CurrentProfileScope = Rc::new(move || current_runtime_scope.scope());
        let owner_runtime = runtime.clone();
        let accepted_owner_is_current: AcceptedProfileOwner = Rc::new(move |owner| {
            let model = owner_runtime.model();
            model.accepted.as_ref().is_some_and(|accepted| {
                accepted.session_epoch == owner.session_epoch
                    && accepted.document.id == owner.document_id
            })
        });
        // Ticket 012 fifteenth slice: expose this view's Save one-shot to the
        // fit editor's draft-owner registration (same-key in-flight exemption).
        // The shell owns the one-shot; the editor owns the draft.
        let save_pending_edits = pending_edits.clone();
        let save_is_pending: Rc<dyn Fn() -> bool> =
            Rc::new(move || save_pending_edits.is_pending(&ProfileKey::Save));
        let save_pending = pending;
        let submitted_save: Rc<dyn Fn() -> Option<MechanicalPartProfile>> =
            Rc::new(move || save_pending.peek().clone().map(|save| save.profile));
        let ports = ManualProfileEditorPorts {
            request_standard_profile,
            request_mechanical_extraction,
            spawn_detached,
            current_scope,
            accepted_owner_is_current,
            save_is_pending,
            submitted_save,
        };
        return rsx! {
            ManualProfileEditor {
                key: "{editor_key}",
                definition: definition.clone(),
                initial,
                owner: owner.clone(),
                snapshot: snapshot.clone(),
                selection,
                selection_generation,
                scope_generation: selection_adapter.generation,
                workspace,
                on_save: save_profile,
                on_close: close_editor,
                ports,
            }
        };
    }
    let profile_defined = definition.mechanical_profile.is_some();
    rsx! {
            section { class: "grid gap-2", "aria-label": "Mechanical fit profile",
                div {
                    h3 { "Mechanical fit" }
                    p {
                        if profile_defined { "Defined with this part and inherited by layouts and cases." }
                        else { "Define this part’s fit once so every layout and case uses the same profile." }
                    }
                }
                Button {
                    variant: ButtonVariant::Secondary,
                    class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: save_disabled(),
                    onclick: start_editing,
                    if profile_defined { "Edit profile" } else { "Define profile" }
                }
                if !error().is_empty() {
                    Alert { variant: AlertVariant::Destructive, "{error()}" }
                }
            }
        PartsPreviewPanel {
            definition: Some(Rc::new(preview_definition.unwrap_or_else(|| definition.clone()))),
            recipe,
            recipe_error,
            recipe_pending,
            recipe_identity,
            preview_title,
            scope,
            snapshot_token: snapshot.token,
            generator_draft,
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    use super::*;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_application::TerminalOutcome;
    use boardstudio_core::model::{Board, ProjectDoc};
    use boardstudio_web_host::host::{UpdateSignals, UpdateWatch};
    use boardstudio_web_host::offline::{AdoptError, UpdateSnapshot};
    use boardstudio_web_runtime::update_safety::{UpdateBlocker, UpdateDialog, UpdateOpener};
    use boardstudio_web_ui_shared::test_role::by_role_name;
    use boardstudio_web_ui_shared::test_wait::{wait_until, wait_until_future, wait_until_step};
    use std::cell::{Cell, RefCell};
    use wasm_bindgen_test::wasm_bindgen_test;
    use web_sys::{Element as DomElement, HtmlElement};

    #[derive(Clone)]
    struct Fixture {
        definition: PartDefinition,
        generation: Rc<RefCell<Option<Signal<u64>>>>,
    }

    fn mounted_profile_host() -> Element {
        let fixture = use_context::<Fixture>();
        let runtime = use_context::<Rc<Runtime>>();
        let selection = use_signal(|| None::<(Option<Scope>, String)>);
        let selected_context = use_signal(|| None);
        let anchor_scope = use_signal(|| None::<Scope>);
        let generation = use_signal(|| 1_u64);
        *fixture.generation.borrow_mut() = Some(generation);
        let adapter =
            use_hook(|| SelectionAdapter::new(selected_context, anchor_scope, generation));
        use_context_provider(|| adapter);
        use_context_provider(|| PartsSelectionGeneration(generation));
        let activation = use_signal(|| 0_u64);
        use_context_provider(|| super::super::PartsPreviewActivation(activation));
        let workspace = use_signal(|| "Parts");
        use_context_provider(|| WorkspaceState(workspace));
        let theme = use_memo(|| "light");
        use_context_provider(|| super::super::super::ResolvedAppearance(theme));
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        let _ = version();
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let snapshot = runtime.model().accepted.unwrap();
        let definition = snapshot
            .document
            .definitions
            .iter()
            .find(|definition| definition.id == fixture.definition.id)
            .cloned()
            .unwrap_or(fixture.definition);
        rsx! {
            PartsMechanicalProfileWorkspace {
                snapshot,
                scope: runtime.scope(),
                selection,
                definition: definition.clone(),
                preview_definition: Some(definition),
                generator_draft: None,
                recipe: Vec::new(),
                recipe_error: None,
                recipe_pending: false,
                recipe_identity: "mechanical-one-shot-test".to_owned(),
                preview_title: None,
                source: ProfileDefinitionSource::Project,
            }
        }
    }

    async fn mount() -> (Rc<Runtime>, DomElement, Fixture) {
        let definition: PartDefinition = serde_json::from_value(serde_json::json!({
            "id": "mechanical-one-shot-part", "name": "Mechanical test part",
            "kind": "custom", "courtyard": [], "pads": []
        }))
        .unwrap();
        let mut project = ProjectDoc::empty("mechanical-one-shot-project", "Mechanical test");
        project.definitions.push(definition.clone());
        project.boards.push(Board {
            id: "mechanical-one-shot-board".into(),
            name: "Mechanical test board".into(),
            outline_ids: Vec::new(),
            part_ids: Vec::new(),
            net_ids: Vec::new(),
            thickness: 1.6,
            traces: Vec::new(),
            vias: Vec::new(),
        });
        let runtime = support::new_runtime();
        support::open_document(&runtime, project).await;
        let fixture = Fixture {
            definition,
            generation: Rc::new(RefCell::new(None)),
        };
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(mounted_profile_host);
        dom.provide_root_context(runtime.clone());
        dom.provide_root_context(fixture.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("mechanical profile entry mounts", || {
            by_role_name(&root, "button", "Define profile")
                .or_else(|| by_role_name(&root, "button", "Edit profile"))
                .is_some()
        })
        .await;
        (runtime, root, fixture)
    }

    fn entry(root: &DomElement) -> HtmlElement {
        by_role_name(root, "button", "Define profile")
            .or_else(|| by_role_name(root, "button", "Edit profile"))
            .expect("mechanical profile entry")
    }

    fn panel_alert(root: &DomElement) -> Option<web_sys::Element> {
        let section = root
            .query_selector("section[aria-label='Mechanical fit profile']")
            .unwrap()?;
        section.query_selector("[role=alert]").unwrap()
    }

    async fn open_editor(root: &DomElement) -> HtmlElement {
        entry(root).click();
        wait_until("mechanical profile editor opens", || {
            by_role_name(root, "button", "Save fit profile").is_some()
        })
        .await;
        by_role_name(root, "button", "Save fit profile").expect("save fit profile action")
    }

    async fn settle_until(runtime: &Rc<Runtime>, what: &str, done: impl Fn() -> bool) {
        wait_until_step(what, async || {
            support::run_pending(runtime).await;
            done()
        })
        .await;
    }

    #[wasm_bindgen_test]
    async fn mechanical_profile_save_disables_entry_silently_and_rejects_retained_duplicate() {
        let (runtime, root, _) = mount().await;
        let save = open_editor(&root).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let first = support::observe_next(&runtime);
        save.click();
        let duplicate = support::observe_next(&runtime);
        // The old rendered Save callback remains available until the close rerender.
        // Invoke it again after admission, before the browser can unmount the editor.
        save.click();
        support::drive_pending(&runtime);
        wait_until_future("mechanical profile save enters Core", entered)
            .await
            .unwrap();
        assert!(
            by_role_name(&root, "button", "Save fit profile").is_none(),
            "saving closes the manual editor"
        );
        assert!(
            entry(&root).has_attribute("disabled"),
            "the parent entry stays disabled while the save is pending"
        );
        assert!(
            !root.text_content().unwrap().contains("Saving fit profile"),
            "one-shot saves have no pending message"
        );
        release.send(()).unwrap();
        settle_until(&runtime, "mechanical profile save lands", || {
            matches!(*first.borrow(), Some(TerminalOutcome::Completed))
                && runtime.model().accepted.unwrap().document.definitions[0]
                    .mechanical_profile
                    .is_some()
                && !entry(&root).has_attribute("disabled")
        })
        .await;
        assert!(matches!(*first.borrow(), Some(TerminalOutcome::Completed)));
        assert!(
            duplicate.borrow().is_none(),
            "the retained duplicate callback cannot queue another save"
        );
        assert!(
            runtime.model().accepted.unwrap().document.definitions[0]
                .mechanical_profile
                .is_some()
        );
        assert!(!entry(&root).has_attribute("disabled"));
        assert!(root.query_selector("[role=alert]").unwrap().is_none());
        assert!(!root.text_content().unwrap().contains("Saved"));
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mechanical_profile_save_failure_reenables_entry_with_inline_error() {
        let (runtime, root, _) = mount().await;
        let save = open_editor(&root).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        support::fail_next_persist(&runtime, "mechanical profile disk failure");
        save.click();
        support::drive_pending(&runtime);
        wait_until_future("mechanical profile save enters Core", entered)
            .await
            .unwrap();
        assert!(entry(&root).has_attribute("disabled"));
        release.send(()).unwrap();
        settle_until(&runtime, "mechanical profile save error appears", || {
            !entry(&root).has_attribute("disabled") && panel_alert(&root).is_some()
        })
        .await;
        let alert = panel_alert(&root).expect("mechanical profile save error");
        assert!(
            alert
                .text_content()
                .unwrap()
                .contains("mechanical profile disk failure")
        );
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mechanical_profile_save_owner_retirement_reenables_entry_silently() {
        let (runtime, root, fixture) = mount().await;
        let save = open_editor(&root).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        support::fail_next_persist(&runtime, "departed mechanical profile disk failure");
        save.click();
        support::drive_pending(&runtime);
        wait_until_future("mechanical profile save enters Core", entered)
            .await
            .unwrap();
        assert!(entry(&root).has_attribute("disabled"));
        let mut generation = fixture.generation.borrow().unwrap();
        generation += 1;
        wait_until("retired mechanical profile save releases its entry", || {
            !entry(&root).has_attribute("disabled")
        })
        .await;
        assert!(
            !entry(&root).has_attribute("disabled"),
            "retiring the observation re-enables the live parent control"
        );
        release.send(()).unwrap();
        settle_until(&runtime, "retired mechanical profile save settles", || {
            matches!(
                runtime.model().durability,
                boardstudio_application::Durability::Failed { .. }
            ) && panel_alert(&root).is_none()
        })
        .await;
        assert!(panel_alert(&root).is_none());
        let text = root.text_content().unwrap();
        assert!(!text.contains("departed mechanical profile disk failure"));
        assert!(!text.contains("Saving fit profile"));
        assert!(!text.contains("Saved"));
        runtime.unsubscribe();
        root.remove();
    }

    // ---- Ticket 012 fifteenth slice: fit profile Save draft defers Update Now. ----
    //
    // RED expected: no fit-profile adapter exists yet, so the gate adopts instead of
    // deferring. Smallest coherent mounted regression through the real Runtime gate
    // with a live fit-profile owner: a real valid unapplied cutout draft at the toast
    // entry. Oracle: deferral state, per-(owner, key) area/remedy copy, unchanged
    // draft, zero adoptions. Header, invalid, lifecycle and retention follow separately.

    struct FakeUpdateSignals012 {
        snapshot: Rc<RefCell<UpdateSnapshot>>,
        adoptions: Rc<Cell<u32>>,
    }

    impl FakeUpdateSignals012 {
        fn new(snapshot: UpdateSnapshot) -> Self {
            Self {
                snapshot: Rc::new(RefCell::new(snapshot)),
                adoptions: Rc::new(Cell::new(0)),
            }
        }
    }

    impl UpdateSignals for FakeUpdateSignals012 {
        fn snapshot(&self) -> UpdateSnapshot {
            *self.snapshot.borrow()
        }

        fn watch(&self, _on_change: Rc<dyn Fn()>) -> Box<dyn UpdateWatch> {
            struct Stop;
            impl UpdateWatch for Stop {
                fn stop(&mut self) {}
            }
            Box::new(Stop)
        }

        fn adopt_waiting(
            &self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), AdoptError>>>> {
            let adoptions = self.adoptions.clone();
            Box::pin(async move {
                adoptions.set(adoptions.get() + 1);
                Ok(())
            })
        }

        fn reload(&self) {}
    }

    fn waiting_update_012() -> UpdateSnapshot {
        UpdateSnapshot {
            waiting: true,
            installing: false,
            controlled: true,
        }
    }

    #[wasm_bindgen_test]
    async fn mounted_fit_profile_valid_save_draft_defers_update_now_at_the_toast_012() {
        let (runtime, root, _) = mount().await;
        let _save = open_editor(&root).await;
        let signals = Rc::new(FakeUpdateSignals012::new(waiting_update_012()));
        runtime.observe_update_waiting(signals.clone());
        // Baseline holds no cutouts; a real valid cutout enters via the production Add button.
        let add: HtmlElement =
            by_role_name(&root, "button", "Add cutout").expect("add cutout action");
        add.click();
        wait_until("cutout enters the draft", || {
            root.text_content()
                .is_some_and(|text| text.contains("Contour 1"))
        })
        .await;
        runtime.begin_update(UpdateOpener::Toast);
        match runtime.update_dialog() {
            Some(UpdateDialog::Deferred { blockers }) => {
                assert_eq!(
                    blockers.len(),
                    1,
                    "only the fit profile draft defers: {blockers:?}"
                );
                match &blockers[0] {
                    UpdateBlocker::UnresolvedDraft { area, remedy } => {
                        assert_eq!(area, "Fit profile");
                        assert_eq!(remedy, "Save the fit profile, or restore its saved values");
                    }
                    other => panic!("expected the fit profile draft blocker, got {other:?}"),
                }
            }
            other => panic!("expected a deferred update dialog, got {other:?}"),
        }
        assert_eq!(signals.adoptions.get(), 0, "the draft never adopts");
        let text = root.text_content().unwrap();
        assert!(
            text.contains("Contour 1"),
            "deferral preserves the exact draft contour: {text}"
        );
        assert!(
            text.contains("Vertex 1"),
            "deferral preserves the draft vertices: {text}"
        );
        runtime.unsubscribe();
        root.remove();
    }

    // ---- Ticket 012 sixteenth slice (a): fit profile Save draft defers at the header. ----
    //
    // Same real valid unapplied cutout draft as the toast case, via UpdateOpener::Header.
    // Distinct failure story is header-checkpoint wiring for this owner. Existing
    // Save registration polls at both checkpoints via the same is_unresolved, so
    // GREEN as proof gap is expected with no production change.
    #[wasm_bindgen_test]
    async fn mounted_fit_profile_valid_save_draft_defers_update_now_at_the_header_012() {
        let (runtime, root, _) = mount().await;
        let _save = open_editor(&root).await;
        let signals = Rc::new(FakeUpdateSignals012::new(waiting_update_012()));
        runtime.observe_update_waiting(signals.clone());
        // Baseline holds no cutouts; a real valid cutout enters via the production Add button.
        let add: HtmlElement =
            by_role_name(&root, "button", "Add cutout").expect("add cutout action");
        add.click();
        wait_until("cutout enters the draft", || {
            root.text_content()
                .is_some_and(|text| text.contains("Contour 1"))
        })
        .await;
        runtime.begin_update(UpdateOpener::Header);
        match runtime.update_dialog() {
            Some(UpdateDialog::Deferred { blockers }) => {
                assert_eq!(
                    blockers.len(),
                    1,
                    "only the fit profile draft defers: {blockers:?}"
                );
                match &blockers[0] {
                    UpdateBlocker::UnresolvedDraft { area, remedy } => {
                        assert_eq!(area, "Fit profile");
                        assert_eq!(remedy, "Save the fit profile, or restore its saved values");
                    }
                    other => panic!("expected the fit profile draft blocker, got {other:?}"),
                }
            }
            other => panic!("expected a deferred update dialog, got {other:?}"),
        }
        assert_eq!(signals.adoptions.get(), 0, "the draft never adopts");
        let text = root.text_content().unwrap();
        assert!(
            text.contains("Contour 1"),
            "deferral preserves the exact draft contour: {text}"
        );
        assert!(
            text.contains("Vertex 1"),
            "deferral preserves the draft vertices: {text}"
        );
        runtime.unsubscribe();
        root.remove();
    }

    // ---- Ticket 012 seventeenth slice (a): Save landing re-baselines. ----
    //
    // Fit Save closes the editor (on_save + on_close), retiring the registration;
    // reopening after landing proves re-baseline (draft == accepted), not just
    // retirement. Valid cutout → Save → land (Completed + accepted 1 cutout
    // asserted before gate) → reopen (Contour 1 from accepted) → Update Now
    // PROCEEDS. Oracle: adoptions==1, dialog None. Distinct story: landing
    // re-baselines.
    #[wasm_bindgen_test]
    async fn mounted_fit_profile_save_landing_rebaselines_update_now_at_the_toast_012() {
        let (runtime, root, _) = mount().await;
        let save = open_editor(&root).await;
        let signals = Rc::new(FakeUpdateSignals012::new(waiting_update_012()));
        runtime.observe_update_waiting(signals.clone());
        // Baseline holds no cutouts; a real valid cutout enters via the production Add button.
        let add: HtmlElement =
            by_role_name(&root, "button", "Add cutout").expect("add cutout action");
        add.click();
        wait_until("cutout enters the draft", || {
            root.text_content()
                .is_some_and(|text| text.contains("Contour 1"))
        })
        .await;
        // Submit Save and land it; assert landing before gate poll.
        let landed = support::observe_next(&runtime);
        save.click();
        settle_until(&runtime, "fit Save lands", || {
            matches!(*landed.borrow(), Some(TerminalOutcome::Completed))
                && runtime.model().accepted.as_ref().is_some_and(|snapshot| {
                    snapshot.document.definitions.first().is_some_and(|def| {
                        def.mechanical_profile
                            .as_ref()
                            .is_some_and(|profile| profile.cutouts.len() == 1)
                    })
                })
        })
        .await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.definitions[0]
                .mechanical_profile
                .as_ref()
                .unwrap()
                .cutouts
                .len(),
            1,
            "the fit Save landed before the gate polls"
        );
        // Save closed the editor; reopen to prove re-baseline (draft == accepted).
        assert!(
            by_role_name(&root, "button", "Save fit profile").is_none(),
            "saving closes the manual editor"
        );
        let _save = open_editor(&root).await;
        assert!(
            root.text_content()
                .is_some_and(|text| text.contains("Contour 1")),
            "reopened editor shows the landed cutout"
        );
        // Update Now proceeds: no deferral from this key.
        runtime.begin_update(UpdateOpener::Toast);
        wait_until("update adopts after landing", || {
            signals.adoptions.get() == 1
        })
        .await;
        assert_eq!(signals.adoptions.get(), 1, "landing lets the update adopt");
        assert!(
            runtime.update_dialog().is_none(),
            "no deferral remains after landing"
        );
        runtime.unsubscribe();
        root.remove();
    }

    // ---- Ticket 012 seventeenth slice (c): revert-to-baseline clears. ----
    //
    // Valid cutout → defers (asserted 1 blocker Save, 0 adoptions) → cancel →
    // revert via production Remove contour 1 (wait baseline restored) → Update
    // Now PROCEEDS. Oracle: adoptions==1, dialog None. Distinct story: revert
    // clears via canonical equality.
    #[wasm_bindgen_test]
    async fn mounted_fit_profile_revert_to_baseline_clears_save_draft_012() {
        let (runtime, root, _) = mount().await;
        let _save = open_editor(&root).await;
        let signals = Rc::new(FakeUpdateSignals012::new(waiting_update_012()));
        runtime.observe_update_waiting(signals.clone());
        // Baseline holds no cutouts; a real valid cutout enters via the production Add button.
        let add: HtmlElement =
            by_role_name(&root, "button", "Add cutout").expect("add cutout action");
        add.click();
        wait_until("cutout enters the draft", || {
            root.text_content()
                .is_some_and(|text| text.contains("Contour 1"))
        })
        .await;
        // First Update Now defers (asserted setup).
        runtime.begin_update(UpdateOpener::Toast);
        match runtime.update_dialog() {
            Some(UpdateDialog::Deferred { blockers }) => {
                assert_eq!(
                    blockers.len(),
                    1,
                    "only the fit profile draft defers: {blockers:?}"
                );
                match &blockers[0] {
                    UpdateBlocker::UnresolvedDraft { area, remedy } => {
                        assert_eq!(area, "Fit profile");
                        assert_eq!(remedy, "Save the fit profile, or restore its saved values");
                    }
                    other => panic!("expected the fit profile draft blocker, got {other:?}"),
                }
            }
            other => panic!("expected a deferred update dialog, got {other:?}"),
        }
        assert_eq!(
            signals.adoptions.get(),
            0,
            "the draft never adopts before revert"
        );
        runtime.cancel_update_dialog();
        // Revert to baseline through the production Remove path.
        by_role_name(&root, "button", "Remove contour 1")
            .expect("remove contour action")
            .click();
        wait_until("revert restores the baseline", || {
            root.text_content().is_some_and(|text| {
                text.contains("No plate cutouts defined") && !text.contains("Contour 1")
            })
        })
        .await;
        // Second Update Now proceeds: no Deferred from this owner.
        runtime.begin_update(UpdateOpener::Toast);
        wait_until("update adopts after revert", || {
            signals.adoptions.get() == 1
        })
        .await;
        assert_eq!(signals.adoptions.get(), 1, "revert lets the update adopt");
        assert!(
            runtime.update_dialog().is_none(),
            "no deferral remains after revert"
        );
        runtime.unsubscribe();
        root.remove();
    }

    // ---- Ticket 012 eighteenth slice P4 (a): hidden but mounted still defers. ----
    //
    // Valid unapplied cutout → hide/keep-mounted via display:none (established
    // generator/module P4 pattern; production mechanism is workspace switch
    // keeping signals alive) → Update Now toast → STILL DEFERS with Save
    // remedy, draft preserved. Distinct story: retention wiring (hidden keeps
    // blocking); shared lifetime mechanics reuse 008/011.
    #[wasm_bindgen_test]
    async fn mounted_fit_profile_hidden_save_draft_still_defers_update_now_at_the_toast_012() {
        let (runtime, root, _) = mount().await;
        let _save = open_editor(&root).await;
        let signals = Rc::new(FakeUpdateSignals012::new(waiting_update_012()));
        runtime.observe_update_waiting(signals.clone());
        // Baseline holds no cutouts; a real valid cutout enters via the production Add button.
        let add: HtmlElement =
            by_role_name(&root, "button", "Add cutout").expect("add cutout action");
        add.click();
        wait_until("cutout enters the draft", || {
            root.text_content()
                .is_some_and(|text| text.contains("Contour 1"))
        })
        .await;
        // Hidden but mounted: display:none keeps signals alive, out of sight.
        root.set_attribute("style", "display: none")
            .expect("hide the mounted fit profile owner");
        assert!(
            by_role_name(&root, "button", "Save fit profile").is_some(),
            "the hidden owner stays mounted"
        );
        runtime.begin_update(UpdateOpener::Toast);
        match runtime.update_dialog() {
            Some(UpdateDialog::Deferred { blockers }) => {
                assert_eq!(
                    blockers.len(),
                    1,
                    "only the fit profile draft defers: {blockers:?}"
                );
                match &blockers[0] {
                    UpdateBlocker::UnresolvedDraft { area, remedy } => {
                        assert_eq!(area, "Fit profile");
                        assert_eq!(remedy, "Save the fit profile, or restore its saved values");
                    }
                    other => panic!("expected the fit profile draft blocker, got {other:?}"),
                }
            }
            other => panic!("expected a deferred update dialog, got {other:?}"),
        }
        assert_eq!(signals.adoptions.get(), 0, "the hidden draft never adopts");
        let text = root.text_content().unwrap();
        assert!(
            text.contains("Contour 1"),
            "deferral preserves the exact draft contour: {text}"
        );
        assert!(
            text.contains("Vertex 1"),
            "deferral preserves the draft vertices: {text}"
        );
        runtime.unsubscribe();
        root.remove();
    }

    #[derive(Clone, Copy)]
    struct FitProfileMountedSignals012 {
        definition: Signal<PartDefinition>,
        selection: Signal<Option<(Option<Scope>, String)>>,
        selection_generation: Signal<u64>,
    }

    fn mounted_profile_host_with_selection_012() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let handles = use_context::<Rc<RefCell<Option<FitProfileMountedSignals012>>>>();
        let first = runtime.model().accepted.unwrap().document.definitions[0].clone();
        let definition = use_signal(|| first.clone());
        let selection = use_signal(|| None::<(Option<Scope>, String)>);
        let selection_generation = use_signal(|| 1_u64);
        let selected_context = use_signal(|| None);
        let anchor_scope = use_signal(|| None::<Scope>);
        let adapter = use_hook(|| {
            SelectionAdapter::new(selected_context, anchor_scope, selection_generation)
        });
        use_context_provider(|| adapter);
        use_context_provider(|| PartsSelectionGeneration(selection_generation));
        let activation = use_signal(|| 0_u64);
        use_context_provider(|| super::super::PartsPreviewActivation(activation));
        let workspace = use_signal(|| "Parts");
        use_context_provider(|| WorkspaceState(workspace));
        let theme = use_memo(|| "light");
        use_context_provider(|| super::super::super::ResolvedAppearance(theme));
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        let _ = version();
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let current = runtime.model().accepted.unwrap();
        let accepted_definition = current
            .document
            .definitions
            .iter()
            .find(|item| item.id == definition().id)
            .cloned()
            .unwrap_or_else(&*definition);
        *handles.borrow_mut() = Some(FitProfileMountedSignals012 {
            definition,
            selection,
            selection_generation,
        });
        rsx! {
            PartsMechanicalProfileWorkspace {
                snapshot: current,
                scope: runtime.scope(),
                selection,
                definition: accepted_definition.clone(),
                preview_definition: Some(accepted_definition),
                generator_draft: None,
                recipe: Vec::new(),
                recipe_error: None,
                recipe_pending: false,
                recipe_identity: "mechanical-one-shot-test".to_owned(),
                preview_title: None,
                source: ProfileDefinitionSource::Project,
            }
        }
    }

    async fn mount_with_selection_012() -> (
        Rc<Runtime>,
        DomElement,
        Rc<RefCell<Option<FitProfileMountedSignals012>>>,
        PartDefinition,
    ) {
        let first: PartDefinition = serde_json::from_value(serde_json::json!({
            "id": "mechanical-one-shot-part", "name": "Mechanical test part",
            "kind": "custom", "courtyard": [], "pads": []
        }))
        .unwrap();
        let second: PartDefinition = serde_json::from_value(serde_json::json!({
            "id": "mechanical-one-shot-part-second", "name": "Mechanical second part",
            "kind": "custom", "courtyard": [], "pads": []
        }))
        .unwrap();
        let mut project = ProjectDoc::empty("mechanical-one-shot-project", "Mechanical test");
        project.definitions.push(first.clone());
        project.definitions.push(second.clone());
        project.boards.push(Board {
            id: "mechanical-one-shot-board".into(),
            name: "Mechanical test board".into(),
            outline_ids: Vec::new(),
            part_ids: Vec::new(),
            net_ids: Vec::new(),
            thickness: 1.6,
            traces: Vec::new(),
            vias: Vec::new(),
        });
        let runtime = support::new_runtime();
        support::open_document(&runtime, project).await;
        let second_accepted = runtime.model().accepted.unwrap().document.definitions[1].clone();
        assert_eq!(second_accepted.id, "mechanical-one-shot-part-second");
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let handles: Rc<RefCell<Option<FitProfileMountedSignals012>>> = Rc::new(RefCell::new(None));
        let dom = VirtualDom::new(mounted_profile_host_with_selection_012);
        dom.provide_root_context(runtime.clone());
        dom.provide_root_context(handles.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("mechanical profile entry mounts", || {
            by_role_name(&root, "button", "Define profile")
                .or_else(|| by_role_name(&root, "button", "Edit profile"))
                .is_some()
        })
        .await;
        (runtime, root, handles, second_accepted)
    }

    // ---- Ticket 012 eighteenth slice P4 (b): owner change retires the draft. ----
    //
    // Valid unapplied cutout → defers (asserted 1 blocker Save, 0 adoptions) →
    // cancel → owner change to a second definition via production selection
    // signals (definition + selection + generation, as generator/module P4 do)
    // → switch proof (new baseline, no Contour 1) → Update Now PROCEEDS with
    // no UnresolvedDraft from fit. Oracle: adoptions==1, dialog None. Distinct
    // story: retire wiring (effect rotation + use_drop on editor remount);
    // shared lifetime mechanics reuse 008/011.
    #[wasm_bindgen_test]
    async fn mounted_fit_profile_owner_change_retires_save_draft_012() {
        let (runtime, root, handles, second) = mount_with_selection_012().await;
        let _save = open_editor(&root).await;
        let signals = Rc::new(FakeUpdateSignals012::new(waiting_update_012()));
        runtime.observe_update_waiting(signals.clone());
        // Baseline holds no cutouts; a real valid cutout enters via the production Add button.
        let add: HtmlElement =
            by_role_name(&root, "button", "Add cutout").expect("add cutout action");
        add.click();
        wait_until("cutout enters the draft", || {
            root.text_content()
                .is_some_and(|text| text.contains("Contour 1"))
        })
        .await;
        // First Update Now defers (asserted setup).
        runtime.begin_update(UpdateOpener::Toast);
        match runtime.update_dialog() {
            Some(UpdateDialog::Deferred { blockers }) => {
                assert_eq!(
                    blockers.len(),
                    1,
                    "only the fit profile draft defers: {blockers:?}"
                );
                match &blockers[0] {
                    UpdateBlocker::UnresolvedDraft { area, remedy } => {
                        assert_eq!(area, "Fit profile");
                        assert_eq!(remedy, "Save the fit profile, or restore its saved values");
                    }
                    other => panic!("expected the fit profile draft blocker, got {other:?}"),
                }
            }
            other => panic!("expected a deferred update dialog, got {other:?}"),
        }
        assert_eq!(
            signals.adoptions.get(),
            0,
            "the draft never adopts before retire"
        );
        runtime.cancel_update_dialog();
        // Owner change via production selection signals (as generator/module P4 do).
        let mut mounted = handles.borrow().as_ref().copied().expect("mounted handles");
        mounted.definition.set(second.clone());
        mounted
            .selection
            .set(Some((runtime.scope(), second.id.clone())));
        mounted
            .selection_generation
            .with_mut(|generation| *generation += 1);
        // The new owner's clean baseline proves the switch was processed.
        wait_until("owner change resets to the new baseline", || {
            root.text_content().is_some_and(|text| {
                text.contains("No plate cutouts defined") && !text.contains("Contour 1")
            })
        })
        .await;
        // Second Update Now proceeds: no UnresolvedDraft from the retired entry.
        runtime.begin_update(UpdateOpener::Toast);
        wait_until("update adopts after retire", || {
            signals.adoptions.get() == 1
        })
        .await;
        assert_eq!(signals.adoptions.get(), 1, "retire lets the update adopt");
        assert!(
            runtime.update_dialog().is_none(),
            "no deferral remains after owner change"
        );
        runtime.unsubscribe();
        root.remove();
    }
}
