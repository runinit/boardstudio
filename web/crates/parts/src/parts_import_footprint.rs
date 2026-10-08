//! Parts-owned file import and acceptance reconciliation for source-owned KiCad footprints.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope};
use boardstudio_core::model::{CompiledFootprint, EditOperation};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportCapture {
    scope: Scope,
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    view_generation: u64,
    scope_generation: u64,
    selection: Option<(Option<Scope>, String)>,
    definition_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportOwner {
    scope: Option<Scope>,
    selection: Option<(Option<Scope>, String)>,
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    view_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopedMessage {
    owner: ImportOwner,
    message: String,
}

impl ImportOwner {
    fn new(
        current: &AcceptedSnapshot,
        scope: Option<Scope>,
        selection: Option<(Option<Scope>, String)>,
        view_generation: u64,
        scope_generation: u64,
        workspace: &'static str,
    ) -> Self {
        Self {
            scope,
            selection,
            session_epoch: current.session_epoch,
            document_id: current.document.id.clone(),
            view_generation,
            scope_generation,
            workspace,
        }
    }
}

impl ImportCapture {
    fn new(
        snapshot: &AcceptedSnapshot,
        scope: Scope,
        view_generation: u64,
        scope_generation: u64,
        selection: Option<(Option<Scope>, String)>,
        definition_id: String,
    ) -> Self {
        Self {
            scope,
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            view_generation,
            scope_generation,
            selection,
            definition_id,
        }
    }
}

fn owner_matches(capture: &ImportCapture, owner: &ImportOwner) -> bool {
    owner.scope.as_ref() == Some(&capture.scope)
        && owner.selection == capture.selection
        && owner.session_epoch == capture.session_epoch
        && owner.document_id == capture.document_id
        && capture.scope.document_id == capture.document_id
        && capture.scope.session_epoch == capture.session_epoch
        && owner.view_generation == capture.view_generation
        && owner.scope_generation == capture.scope_generation
        && owner.workspace == "Parts"
}

fn capture_matches(
    capture: &ImportCapture,
    _current: &AcceptedSnapshot,
    owner: &ImportOwner,
) -> bool {
    owner_matches(capture, owner)
}

fn import_resolver(capture: ImportCapture, imported: CompiledFootprint) -> EditResolver {
    EditResolver::new(
        "parts-import-footprint",
        move |accepted: &AcceptedSnapshot| {
            if accepted.document.id != capture.document_id {
                return Resolution::Retire("The Parts project changed during import.".into());
            }
            if imported.definition.id != capture.definition_id {
                return Resolution::Retire(
                    "The importer returned a different definition identity.".into(),
                );
            }
            let mut definition = imported.definition.clone();
            let mut suffix = 0_u64;
            while accepted
                .document
                .definitions
                .iter()
                .any(|existing| existing.id == definition.id)
            {
                suffix += 1;
                definition.id = format!("{}-{suffix}", capture.definition_id);
            }
            definition.pads = imported.geometry.pads.clone();
            definition.courtyard = imported.geometry.courtyard.clone();
            let id = definition.id.clone();
            let mut document = accepted.document.as_ref().clone();
            document.definitions.push(definition);
            crate::parts_custom_definition::replacement_commit(
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
                vec![id],
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{ImportCapture, ImportOwner, owner_matches};
    use boardstudio_application::{Scope, SessionEpoch};

    #[test]
    fn import_feedback_is_limited_to_the_captured_owner_but_survives_revision_advance() {
        let scope = Scope {
            session_epoch: SessionEpoch(4),
            document_id: "project-1".into(),
            board_id: "board-1".into(),
            instance_id: None,
        };
        let capture = ImportCapture {
            scope: scope.clone(),
            session_epoch: SessionEpoch(4),
            document_id: "project-1".into(),
            view_generation: 3,
            scope_generation: 5,
            selection: Some((Some(scope.clone()), "existing-footprint".into())),
            definition_id: "imported-1".into(),
        };
        let owner = ImportOwner {
            scope: Some(scope.clone()),
            selection: capture.selection.clone(),
            session_epoch: SessionEpoch(4),
            document_id: "project-1".into(),
            view_generation: 3,
            scope_generation: 5,
            workspace: "Parts",
        };

        // Persistence may advance the accepted revision before its failure is reported.
        // Feedback still belongs to this selection as long as the project/view owner holds.
        assert!(owner_matches(&capture, &owner));

        let replacement_selection = ImportOwner {
            selection: Some((Some(scope), "replacement-footprint".into())),
            ..owner
        };
        assert!(!owner_matches(&capture, &replacement_selection));
    }
}

#[cfg(target_arch = "wasm32")]
mod ui {
    use super::{ImportCapture, ImportOwner, ScopedMessage, import_resolver, owner_matches};
    use crate::runtime::Runtime;
    use boardstudio_application::{AcceptedSnapshot, Scope};
    use boardstudio_web_runtime::pending_edits::PendingEditResult;
    use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
    use dioxus::prelude::*;
    use dioxus_web::WebEventExt;
    use js_sys::{Date, Function, Reflect};
    use std::{cell::Cell, rc::Rc};
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::{JsFuture, spawn_local};
    use web_sys::HtmlInputElement;

    /// This action's one bounded key: the helper keeps one observation for the import
    /// edit; preparation (file reading and compiling) happens before it begins.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ImportKey {
        Footprint,
    }

    /// The import this panel tracks: preparation is cancellable; once the resolver is
    /// admitted the helper owns the observation until it settles.
    #[derive(Clone)]
    enum ImportPhase {
        Preparing {
            filename: String,
        },
        Committed {
            capture: ImportCapture,
            filename: String,
            diagnostics: Vec<String>,
        },
    }

    fn current_owner(
        runtime: &Runtime,
        selected: Signal<Option<(Option<Scope>, String)>>,
        view_generation: Signal<u64>,
        scope_generation: Signal<u64>,
        workspace: Signal<&'static str>,
    ) -> Option<ImportOwner> {
        let snapshot = runtime.model().accepted?;
        Some(ImportOwner::new(
            &snapshot,
            runtime.scope(),
            selected(),
            view_generation(),
            scope_generation(),
            workspace(),
        ))
    }

    fn publish_message(
        target: &mut Signal<Option<ScopedMessage>>,
        owner: Option<ImportOwner>,
        message: impl Into<String>,
    ) {
        if let Some(owner) = owner {
            target.set(Some(ScopedMessage {
                owner,
                message: message.into(),
            }));
        }
    }

    /// The upload action stays mounted independently from catalogue success/loading state.
    #[component]
    pub fn ImportKiCadFootprintAction(
        scope: Option<Scope>,
        view_generation: Signal<u64>,
        scope_generation: Signal<u64>,
        workspace: Signal<&'static str>,
        selected: Signal<Option<(Option<Scope>, String)>>,
        query: Signal<String>,
        on_select: EventHandler<()>,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_context::<Signal<u64>>();
        let pending = use_signal(|| None::<ImportPhase>);
        let edits = use_hook(|| PendingEditSignals::<ImportKey>::new());
        let mut error = use_signal(|| None::<ScopedMessage>);
        let mut notice = use_signal(|| None::<ScopedMessage>);
        let owner_is_mounted = use_hook(|| Rc::new(Cell::new(true)));
        let request_epoch = use_hook(|| Rc::new(Cell::new(0_u64)));
        use_drop({
            let owner_is_mounted = owner_is_mounted.clone();
            let request_epoch = request_epoch.clone();
            move || {
                owner_is_mounted.set(false);
                request_epoch.set(request_epoch.get().wrapping_add(1));
            }
        });

        use_effect(use_reactive((&version(),), {
            let runtime = runtime.clone();
            let owner_is_mounted = owner_is_mounted.clone();
            let edits = edits.clone();
            let mut pending = pending;
            let mut selected = selected;
            let mut query = query;
            move |_| {
                let Some(ImportPhase::Committed {
                    capture,
                    filename,
                    diagnostics,
                }) = pending.read().clone()
                else {
                    return;
                };
                let model = runtime.model();
                let Some(snapshot) = model.accepted.as_ref() else {
                    return;
                };
                let owner = ImportOwner::new(
                    snapshot,
                    runtime.scope(),
                    selected(),
                    view_generation(),
                    scope_generation(),
                    workspace(),
                );
                let live = owner_is_mounted.get() && owner_matches(&capture, &owner);
                let mut landed = false;
                for result in edits.settle(live, |_| String::new()) {
                    match result {
                        PendingEditResult::Landed { .. } => {
                            landed = true;
                            pending.set(None);
                        }
                        PendingEditResult::Failed { message, .. } => {
                            pending.set(None);
                            publish_message(&mut error, Some(owner), message);
                            return;
                        }
                        PendingEditResult::Retired { .. } => {
                            pending.set(None);
                            return;
                        }
                    }
                }
                if !landed {
                    return;
                }
                // The resolver appends the newly allocated definition. Read its accepted
                // identity only after the edit confirms landing.
                let Some(created) = snapshot
                    .document
                    .definitions
                    .iter()
                    .rev()
                    .find(|definition| {
                        definition.id == capture.definition_id
                            || definition
                                .id
                                .strip_prefix(&format!("{}-", capture.definition_id))
                                .is_some_and(|suffix| suffix.parse::<u64>().is_ok())
                    })
                else {
                    return;
                };
                selected.set(Some((Some(capture.scope.clone()), created.id.clone())));
                query.set(String::new());
                error.set(None);
                notice.set((!diagnostics.is_empty()).then(|| ScopedMessage {
                    owner: ImportOwner::new(
                        snapshot,
                        runtime.scope(),
                        selected(),
                        view_generation(),
                        scope_generation(),
                        workspace(),
                    ),
                    message: format!("Imported with notes: {}", diagnostics.join(" ")),
                }));
                let _ = filename;
                on_select.call(());
            }
        }));

        let cancel = {
            let request_epoch = request_epoch.clone();
            let mut pending = pending;
            let mut error = error;
            let mut notice = notice;
            move |_| {
                request_epoch.set(request_epoch.get().wrapping_add(1));
                pending.set(None);
                error.set(None);
                notice.set(None);
            }
        };
        let onchange = {
            let runtime = runtime.clone();
            let owner_is_mounted = owner_is_mounted.clone();
            let request_epoch = request_epoch.clone();
            let mut pending = pending;
            let mut error = error;
            let mut notice = notice;
            let selected = selected;
            let scope = scope.clone();
            move |event: FormEvent| {
                let edits = edits.clone();
                let Some(input) = event
                    .data()
                    .try_as_web_event()
                    .and_then(|event| event.target())
                    .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                else {
                    return;
                };
                let Some(file) = input.files().and_then(|files| files.get(0)) else {
                    return;
                };
                input.set_value("");
                error.set(None);
                notice.set(None);
                if !file.name().to_lowercase().ends_with(".kicad_mod") {
                    publish_message(
                        &mut error,
                        current_owner(
                            &runtime,
                            selected,
                            view_generation,
                            scope_generation,
                            workspace,
                        ),
                        "Select a .kicad_mod footprint.",
                    );
                    return;
                }
                if pending.read().is_some() || !owner_is_mounted.get() || workspace() != "Parts" {
                    return;
                }

                let model = runtime.model();
                let Some(snapshot) = model.accepted.as_ref().cloned() else {
                    return;
                };
                let Some(current_scope) = runtime.scope() else {
                    return;
                };
                if scope.as_ref() != Some(&current_scope)
                    || snapshot.document.id != current_scope.document_id
                    || snapshot.session_epoch != current_scope.session_epoch
                {
                    publish_message(
                        &mut error,
                        Some(ImportOwner::new(
                            &snapshot,
                            Some(current_scope),
                            selected(),
                            view_generation(),
                            scope_generation(),
                            workspace(),
                        )),
                        "The active Parts project changed. Choose the file again.",
                    );
                    return;
                }
                let definition_id = match unique_definition_id(&snapshot) {
                    Ok(id) => id,
                    Err(message) => {
                        publish_message(
                            &mut error,
                            current_owner(
                                &runtime,
                                selected,
                                view_generation,
                                scope_generation,
                                workspace,
                            ),
                            message,
                        );
                        return;
                    }
                };
                let capture = ImportCapture::new(
                    &snapshot,
                    current_scope,
                    view_generation(),
                    scope_generation(),
                    selected(),
                    definition_id,
                );
                let epoch = request_epoch.get().wrapping_add(1);
                request_epoch.set(epoch);
                let filename = file.name();
                pending.set(Some(ImportPhase::Preparing {
                    filename: filename.clone(),
                }));
                let runtime = runtime.clone();
                let owner_is_mounted = owner_is_mounted.clone();
                let request_epoch = request_epoch.clone();
                let mut pending = pending;
                let mut error = error;
                let selected = selected;
                let scope = scope.clone();
                spawn_local(async move {
                    let still_owned = || owner_is_mounted.get() && request_epoch.get() == epoch;
                    let feedback_is_current = || {
                        current_owner(
                            &runtime,
                            selected,
                            view_generation,
                            scope_generation,
                            workspace,
                        )
                        .is_some_and(|owner| owner_matches(&capture, &owner))
                    };
                    let source_is_current = || {
                        runtime.model().accepted.as_ref().is_some_and(|current| {
                            super::capture_matches(
                                &capture,
                                current,
                                &ImportOwner::new(
                                    current,
                                    runtime.scope(),
                                    selected(),
                                    view_generation(),
                                    scope_generation(),
                                    workspace(),
                                ),
                            )
                        })
                    };
                    let source = match JsFuture::from(file.text()).await {
                        Ok(value) => match value.as_string() {
                            Some(source) => source,
                            None => {
                                if still_owned() {
                                    pending.set(None);
                                    if feedback_is_current() {
                                        publish_message(
                                            &mut error,
                                            current_owner(
                                                &runtime,
                                                selected,
                                                view_generation,
                                                scope_generation,
                                                workspace,
                                            ),
                                            "The selected footprint could not be read as text.",
                                        );
                                    }
                                }
                                return;
                            }
                        },
                        Err(cause) => {
                            if still_owned() {
                                pending.set(None);
                                if feedback_is_current() {
                                    publish_message(
                                        &mut error,
                                        current_owner(
                                            &runtime,
                                            selected,
                                            view_generation,
                                            scope_generation,
                                            workspace,
                                        ),
                                        format!("Could not read the selected footprint: {cause:?}"),
                                    );
                                }
                            }
                            return;
                        }
                    };
                    if !still_owned() {
                        return;
                    }
                    if !source_is_current() {
                        pending.set(None);
                        if feedback_is_current() {
                            publish_message(
                                &mut error,
                                current_owner(
                                    &runtime,
                                    selected,
                                    view_generation,
                                    scope_generation,
                                    workspace,
                                ),
                                "The Parts project or selection changed during file reading. Choose the file again.",
                            );
                        }
                        return;
                    }
                    let request_id = format!("parts-import-footprint-{}", runtime.operation().0);
                    let compiled = match runtime
                        .import_footprint(request_id, capture.definition_id.clone(), source)
                        .await
                    {
                        Ok(compiled) => compiled,
                        Err(message) => {
                            if still_owned() {
                                pending.set(None);
                                if feedback_is_current() {
                                    publish_message(
                                        &mut error,
                                        current_owner(
                                            &runtime,
                                            selected,
                                            view_generation,
                                            scope_generation,
                                            workspace,
                                        ),
                                        message,
                                    );
                                }
                            }
                            return;
                        }
                    };
                    if !still_owned() {
                        return;
                    }
                    if !source_is_current() {
                        pending.set(None);
                        if feedback_is_current() {
                            publish_message(
                                &mut error,
                                current_owner(
                                    &runtime,
                                    selected,
                                    view_generation,
                                    scope_generation,
                                    workspace,
                                ),
                                "The Parts project or selection changed during import. Choose the file again.",
                            );
                        }
                        return;
                    }
                    let diagnostics = compiled
                        .diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.message.clone())
                        .collect::<Vec<_>>();
                    if scope.as_ref() != runtime.scope().as_ref() || !still_owned() {
                        pending.set(None);
                        if feedback_is_current() {
                            publish_message(
                                &mut error,
                                current_owner(
                                    &runtime,
                                    selected,
                                    view_generation,
                                    scope_generation,
                                    workspace,
                                ),
                                "The Parts project changed during import. Choose the file again.",
                            );
                        }
                        return;
                    }
                    edits.begin_one_shot(
                        &runtime,
                        ImportKey::Footprint,
                        "parts-import-footprint",
                        Some("KiCad footprint".into()),
                        import_resolver(capture.clone(), compiled),
                    );
                    pending.set(Some(ImportPhase::Committed {
                        capture,
                        filename,
                        diagnostics,
                    }));
                });
            }
        };

        let busy = pending.read().is_some();
        let current_owner = current_owner(
            &runtime,
            selected,
            view_generation,
            scope_generation,
            workspace,
        );
        let visible_error = error()
            .filter(|feedback| current_owner.as_ref() == Some(&feedback.owner))
            .map(|feedback| feedback.message);
        let visible_notice = notice()
            .filter(|feedback| current_owner.as_ref() == Some(&feedback.owner))
            .map(|feedback| feedback.message);
        rsx! {
            div { class: "m1-parts-import-control",
                label {
                    "Import KiCad footprint"
                    input {
                        r#type: "file",
                        accept: ".kicad_mod",
                        aria_label: "Import KiCad footprint",
                        disabled: busy || scope.is_none(),
                        onchange,
                    }
                }
            }
            if let Some(ImportPhase::Preparing { filename }) = pending.read().as_ref() {
                p { class: "m1-parts-loading", role: "status",
                    "Reading and importing {filename}…"
                    button { type: "button", onclick: cancel, "Cancel import" }
                }
            }
            if let Some(message) = visible_error {
                p { class: "m1-parts-load-error", role: "alert", "{message}" }
            }
            if let Some(message) = visible_notice {
                p { class: "m1-parts-import-notice", role: "status", "{message}" }
            }
        }
    }

    fn unique_definition_id(snapshot: &AcceptedSnapshot) -> Result<String, String> {
        let base = browser_identity().unwrap_or_else(date_identity);
        (0_u64..)
            .map(|suffix| {
                if suffix == 0 {
                    format!("imported-{base}")
                } else {
                    format!("imported-{base}-{suffix}")
                }
            })
            .find(|candidate| {
                snapshot
                    .document
                    .definitions
                    .iter()
                    .all(|definition| definition.id != *candidate)
            })
            .ok_or_else(|| "Could not allocate a unique imported footprint identity.".into())
    }

    fn browser_identity() -> Option<String> {
        let crypto = Reflect::get(&js_sys::global(), &JsValue::from_str("crypto")).ok()?;
        let random_uuid = Reflect::get(&crypto, &JsValue::from_str("randomUUID"))
            .ok()?
            .dyn_into::<Function>()
            .ok()?;
        random_uuid.call0(&crypto).ok()?.as_string()
    }

    fn date_identity() -> String {
        let mut value = Date::now().max(0.0) as u64;
        const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let mut digits = Vec::new();
        while value > 0 {
            digits.push(DIGITS[(value % 36) as usize]);
            value /= 36;
        }
        if digits.is_empty() {
            digits.push(b'0');
        }
        digits.reverse();
        String::from_utf8(digits).expect("base-36 digits are valid UTF-8")
    }
}

#[cfg(target_arch = "wasm32")]
pub use ui::ImportKiCadFootprintAction;

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    use super::*;
    use boardstudio_core::model::ProjectDoc;
    use boardstudio_web_runtime::{
        runtime::Runtime, runtime::project_name_test_support as support,
    };
    use dioxus::prelude::*;
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_test::*;

    type SelectionProbe = Rc<RefCell<Option<Signal<Option<(Option<Scope>, String)>>>>>;
    fn host() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let probe = use_context::<SelectionProbe>();
        let selected = use_signal(|| None);
        *probe.borrow_mut() = Some(selected);
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let view_generation = use_signal(|| 1);
        let scope_generation = use_signal(|| 1);
        let workspace = use_signal(|| "Parts");
        let query = use_signal(String::new);
        rsx! { super::ui::ImportKiCadFootprintAction { scope: runtime.scope(), view_generation, scope_generation, workspace, query, selected, on_select: |_| {} } }
    }
    #[wasm_bindgen_test]
    async fn import_selects_only_after_persistence_lands() {
        let runtime = support::new_runtime();
        let mut document = ProjectDoc::empty("import-selection", "Import selection");
        document.boards.push(serde_json::from_value(serde_json::json!({"id":"board","name":"Board","outlineIds":[],"partIds":[],"netIds":[],"thickness":1.6,"traces":[],"vias":[]})).unwrap());
        support::open_document(&runtime, document).await;
        let probe: SelectionProbe = Rc::new(RefCell::new(None));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(30).await;
        let input = root
            .query_selector("input[type=file]")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        let source = r#"(footprint "Imported" (version 20241229) (generator "pcbnew") (layer "F.Cu") (attr smd) (pad "1" smd rect (at 0 0) (size 1 1) (layers "F.Cu" "F.Paste" "F.Mask")))"#;
        let file = web_sys::File::new_with_str_sequence(
            &js_sys::Array::of1(&JsValue::from_str(source)),
            "Imported.kicad_mod",
        )
        .unwrap();
        let descriptor = js_sys::Object::new();
        js_sys::Reflect::set(&descriptor, &"value".into(), &js_sys::Array::of1(&file)).unwrap();
        js_sys::Object::define_property(
            input.unchecked_ref::<js_sys::Object>(),
            &"files".into(),
            &descriptor,
        );
        let (mut entered, release) = support::gate_next_persist(&runtime);
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &init).unwrap())
            .unwrap();
        let mut held = false;
        for _ in 0..100 {
            support::drive_pending(&runtime);
            gloo_timers::future::TimeoutFuture::new(20).await;
            if entered.try_recv().unwrap().is_some() {
                held = true;
                break;
            }
        }
        assert!(
            held,
            "import reached persistence: {}",
            root.text_content().unwrap_or_default()
        );
        assert!(input.disabled());
        assert!(probe.borrow().as_ref().unwrap()().is_none());
        assert_eq!(
            runtime.model().accepted.unwrap().document.definitions.len(),
            0
        );
        release.send(()).unwrap();
        gloo_timers::future::TimeoutFuture::new(40).await;
        support::run_pending(&runtime).await;
        gloo_timers::future::TimeoutFuture::new(30).await;
        let selected = probe.borrow().as_ref().unwrap()().unwrap();
        assert_eq!(selected.0, runtime.scope());
        assert_eq!(
            runtime.model().accepted.unwrap().document.definitions[0].id,
            selected.1
        );
        assert!(!input.disabled());
        runtime.unsubscribe();
        root.remove();
    }
}
