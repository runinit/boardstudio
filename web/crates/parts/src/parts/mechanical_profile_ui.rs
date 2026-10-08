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
    let pending_edits = use_hook(|| PendingEditSignals::<ProfileKey>::new());
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
        let ports = ManualProfileEditorPorts {
            request_standard_profile,
            request_mechanical_extraction,
            spawn_detached,
            current_scope,
            accepted_owner_is_current,
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
            section { class: "m1-parts-mechanical-fit", "aria-label": "Mechanical fit profile",
                div {
                    h3 { "Mechanical fit" }
                    p {
                        if profile_defined { "Defined with this part and inherited by layouts and cases." }
                        else { "Define this part’s fit once so every layout and case uses the same profile." }
                    }
                }
                button {
                    class: "m1-secondary",
                    r#type: "button",
                    disabled: save_disabled(),
                    onclick: start_editing,
                    if profile_defined { "Edit profile" } else { "Define profile" }
                }
                if !error().is_empty() {
                    p { class: "m1-parts-load-error", role: "alert", "{error()}" }
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
    use std::cell::RefCell;
    use wasm_bindgen::JsCast;
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
        use_context_provider(|| super::super::super::ResolvedTheme(theme));
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
        tick().await;
        (runtime, root, fixture)
    }

    async fn tick() {
        gloo_timers::future::TimeoutFuture::new(30).await;
    }

    fn entry(root: &DomElement) -> HtmlElement {
        root.query_selector(".m1-parts-mechanical-fit button")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    async fn open_editor(root: &DomElement) -> HtmlElement {
        entry(root).click();
        tick().await;
        root.query_selector("button[aria-label='Save fit profile']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    async fn wait_for_core(mut entered: futures_channel::oneshot::Receiver<()>) {
        for _ in 0..100 {
            if entered.try_recv().unwrap().is_some() {
                return;
            }
            tick().await;
        }
        panic!("mechanical profile save did not enter the real Core gate");
    }

    async fn drain(runtime: &Rc<Runtime>) {
        for _ in 0..8 {
            support::run_pending(runtime).await;
            tick().await;
        }
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
        tick().await;
        support::drive_pending(&runtime);
        wait_for_core(entered).await;
        assert!(
            root.query_selector("button[aria-label='Save fit profile']")
                .unwrap()
                .is_none(),
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
        drain(&runtime).await;
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
        tick().await;
        support::drive_pending(&runtime);
        wait_for_core(entered).await;
        assert!(entry(&root).has_attribute("disabled"));
        release.send(()).unwrap();
        drain(&runtime).await;
        assert!(!entry(&root).has_attribute("disabled"));
        let alert = root
            .query_selector(".m1-parts-mechanical-fit [role=alert]")
            .unwrap()
            .unwrap();
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
        tick().await;
        support::drive_pending(&runtime);
        wait_for_core(entered).await;
        assert!(entry(&root).has_attribute("disabled"));
        let mut generation = fixture.generation.borrow().unwrap();
        generation += 1;
        tick().await;
        assert!(
            !entry(&root).has_attribute("disabled"),
            "retiring the observation re-enables the live parent control"
        );
        release.send(()).unwrap();
        drain(&runtime).await;
        assert!(
            root.query_selector(".m1-parts-mechanical-fit [role=alert]")
                .unwrap()
                .is_none()
        );
        let text = root.text_content().unwrap();
        assert!(!text.contains("departed mechanical profile disk failure"));
        assert!(!text.contains("Saving fit profile"));
        assert!(!text.contains("Saved"));
        runtime.unsubscribe();
        root.remove();
    }
}
