use super::super::{WorkspaceState, selection::SelectionAdapter};
use super::GeneratorPreviewDraft;
use super::PartsPreviewPanel;
use super::PartsSelectionGeneration;
use super::{
    AcceptedProfileOwner, CurrentProfileScope, DetachedProfileSpawner, ManualProfileEditor,
    ManualProfileEditorPorts, StandardProfileFuture, StandardProfileRequester,
};
use crate::parts_mechanical_profile::{
    PendingProfileEdit, ProfileDefinitionSource, ProfileEditCapture, ProfileEditContext,
    ProfileEditOwner, prepare_profile_edit,
};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::model::{MechanicalPartProfile, PartDefinition};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

#[component]
pub(crate) fn PartsMechanicalProfileWorkspace(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    mut selection: Signal<Option<(Option<Scope>, String)>>,
    definition: PartDefinition,
    preview_definition: Option<PartDefinition>,
    generator_draft: Option<GeneratorPreviewDraft>,
    recipe: Vec<crate::parts_preview::PartsPreviewRecipeMember>,
    recipe_error: Option<String>,
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
    let capture = use_signal(|| None::<ProfileEditCapture>);
    let pending = use_signal(|| None::<PendingProfileEdit>);
    let capture_owner = owner.clone();
    use_effect(use_reactive((&owner,), {
        let mut capture = capture;
        let definition = definition.clone();
        move |(_owner,)| {
            if editing() {
                capture.set(Some(ProfileEditCapture::new(
                    capture_owner.clone(),
                    definition.clone(),
                )));
            }
        }
    }));
    use_effect(use_reactive((&owner, &runtime_version()), {
        let runtime = runtime.clone();
        let mut pending = pending;
        move |(owner, _)| {
            if pending
                .read()
                .as_ref()
                .is_some_and(|operation| operation.should_retire(&owner, &runtime.scope()))
            {
                pending.set(None);
            }
        }
    }));
    let start_editing = {
        let mut editing = editing;
        let mut capture = capture;
        let owner = owner.clone();
        let scope = scope.clone();
        let definition = definition.clone();
        move |_| {
            selection.set(Some((scope.clone(), definition.id.clone())));
            capture.set(Some(ProfileEditCapture::new(
                owner.clone(),
                definition.clone(),
            )));
            editing.set(true);
        }
    };
    let close_editor = {
        let mut editing = editing;
        let mut capture = capture;
        move |_| {
            editing.set(false);
            capture.set(None);
        }
    };
    let save_profile = {
        let runtime = runtime.clone();
        let selection = selection;
        let mut pending = pending;
        let owner = owner.clone();
        let definition = definition.clone();
        move |profile: MechanicalPartProfile| {
            let Some(capture) = capture() else {
                return;
            };
            let model = runtime.model();
            let Some(current) = model.accepted else {
                return;
            };
            let operation_id = runtime.operation();
            let Some(event) = prepare_profile_edit(
                ProfileEditContext {
                    snapshot: &current,
                    owner: &owner,
                    runtime_scope: runtime.scope(),
                    selection: selection(),
                    definition: &definition,
                },
                &capture,
                profile,
                operation_id,
            ) else {
                return;
            };
            let outcome = runtime.observe_operation(operation_id);
            pending.set(Some(PendingProfileEdit::new(owner.clone(), outcome)));
            runtime.submit(event);
        }
    };

    if editing() {
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
            spawn_detached,
            current_scope,
            accepted_owner_is_current,
        };
        return rsx! {
            ManualProfileEditor {
                key: "{editor_key}",
                definition: definition.clone(),
                initial: definition.mechanical_profile.clone(),
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
                onclick: start_editing,
                if profile_defined { "Edit profile" } else { "Define profile" }
            }
        }
        PartsPreviewPanel {
            definition: Some(Rc::new(preview_definition.unwrap_or_else(|| definition.clone()))),
            recipe,
            recipe_error,
            preview_title,
            scope,
            snapshot_token: snapshot.token,
            generator_draft,
        }
    }
}
