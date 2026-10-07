//! Private Case authoring adapter. The child owns drafts; Runtime remains authoritative.
use super::case_bodies::{
    CaseBoardSummary, CaseBodies, CaseBodyEdit, CaseBodyEditFeedback, CaseBodyEditState,
    CaseBodyRequest, CaseMismatch,
};
use crate::runtime::Runtime;
#[cfg(test)]
use boardstudio_application::Event;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Lifecycle, Resolution, Scope};
#[cfg(test)]
use boardstudio_core::model::ProjectDoc;
use boardstudio_core::model::{
    CaseBody, CaseKind, EditCommand, EditOperation, EditPhase, Mount, MountKind, Vec2,
};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
struct BodyEditTicket {
    request: CaseBodyRequest,
    ticket: EditTicket,
    created_body_id: Option<String>,
}

/// Mount beside the Case preview in the Inspector slot. The shared page passes
/// its already scope-guarded configured-board navigation callback.
#[component]
pub fn CaseBodyInspector(on_show_configured_board: EventHandler<String>) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let instance_selection = use_context::<super::InstanceSelection>();
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let mut body_edit_portal = use_context::<super::case_viewer::CaseSelection>().body_edit_portal;
    let request_sequence = use_signal(|| 0_u64);
    let pending = use_signal(|| Vec::<BodyEditTicket>::new());
    let feedback = use_signal(|| Vec::<CaseBodyEditFeedback>::new());
    let body_edit_dispatch = use_hook({
        let runtime = runtime.clone();
        move || {
            let portal = body_edit_portal;
            let runtime = runtime.clone();
            Rc::new(
                move |edit: CaseBodyEdit,
                      scope: boardstudio_application::Scope,
                      snapshot_token: boardstudio_application::SnapshotToken,
                      revision: u64| {
                    let mut request_sequence = request_sequence;
                    let mut pending = pending;
                    let mut feedback = feedback;
                    if !(portal.editable)() {
                        return;
                    }
                    let Some(request_id) = request_sequence().checked_add(1) else {
                        return;
                    };
                    request_sequence.set(request_id);
                    let body_id = edit.body_id();
                    let mount_id = match &edit {
                        CaseBodyEdit::SetMountPosition { mount_id, .. } => Some(mount_id.as_str()),
                        _ => None,
                    };
                    submit_body_edit(
                        &runtime,
                        instance_selection,
                        editor_instance_id,
                        &mut pending,
                        &mut feedback,
                        CaseBodyRequest {
                            editor_instance_id,
                            request_id,
                            field_id: body_id.zip(mount_id).map(|(body, mount)| {
                                format!("case-body:{body}:mount:{mount}:position")
                            }),
                            scope,
                            snapshot_token,
                            revision,
                            edit,
                        },
                    );
                },
            ) as super::case_viewer::CaseBodyEditDispatch
        }
    });
    {
        let current = body_edit_portal.dispatch.read().clone();
        if current
            .as_ref()
            .is_none_or(|current| !Rc::ptr_eq(current, &body_edit_dispatch))
        {
            body_edit_portal
                .dispatch
                .set(Some(body_edit_dispatch.clone()));
        }
    }
    use_drop({
        let mut portal = body_edit_portal;
        let dispatch = body_edit_dispatch.clone();
        move || {
            if portal
                .dispatch
                .read()
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, &dispatch))
            {
                portal.dispatch.set(None);
                portal.editable.set(false);
            }
        }
    });

    let model = runtime.model();
    let snapshot_token = model.accepted.as_ref().map(|snapshot| snapshot.token);
    let scope = runtime.scope();
    let projected_case = use_memo(use_reactive((&snapshot_token, &scope), {
        let runtime = runtime.clone();
        move |(token, scope)| {
            let token = token?;
            let scope = scope.as_ref()?;
            let snapshot = runtime.model().accepted?;
            if snapshot.token != token || snapshot.document.id != scope.document_id {
                return None;
            }
            Some(
                boardstudio_web_host::cad_jobs::captured_case_document(&snapshot, scope)
                    .map(Rc::new)
                    .map_err(|error| format!("{error:?}")),
            )
        }
    }));

    use_effect(use_reactive((&version(),), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |_| {
            let scope = runtime.scope();
            let mut remaining = Vec::new();
            let mut next_feedback = feedback.peek().clone();
            for waiting in pending.peek().iter() {
                match waiting
                    .ticket
                    .settlement(scope.as_ref() == Some(&waiting.request.scope))
                {
                    Settlement::Pending => remaining.push(waiting.clone()),
                    Settlement::Landed { .. } => record_feedback(
                        &mut next_feedback,
                        feedback_for(waiting, CaseBodyEditState::Saved, None),
                    ),
                    Settlement::Failed { message } => record_feedback(
                        &mut next_feedback,
                        feedback_for(waiting, CaseBodyEditState::Failed, Some(message)),
                    ),
                    Settlement::Retired => {
                        next_feedback.retain(|entry| entry.request_id != waiting.request.request_id)
                    }
                }
            }
            if remaining.len() != pending.peek().len() {
                pending.set(remaining);
                feedback.set(next_feedback);
            }
        }
    }));

    let on_edit = {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |request: CaseBodyRequest| {
            submit_body_edit(
                &runtime,
                instance_selection,
                editor_instance_id,
                &mut pending,
                &mut feedback,
                request,
            );
        }
    };

    let Some(snapshot) = model.accepted.as_ref() else {
        body_edit_portal.editable.set(false);
        return rsx! { p { role: "status", "Open a saved keyboard to edit its case bodies." } };
    };
    let Some(scope) = scope else {
        body_edit_portal.editable.set(false);
        return rsx! { p { role: "alert", "The active Case scope is unavailable." } };
    };
    if scope.board_id != model.active_board_id || scope.document_id != snapshot.document.id {
        body_edit_portal.editable.set(false);
        return rsx! { p { role: "alert", "The active Case scope changed. Reopen the inspector to continue." } };
    }
    let Some(projected_case) = projected_case.read().clone() else {
        body_edit_portal.editable.set(false);
        return rsx! { p { role: "alert", "The active Case projection is not current." } };
    };
    let effective = match projected_case {
        Ok(document) => document,
        Err(error) => {
            body_edit_portal.editable.set(false);
            return rsx! { p { role: "alert", "Could not read the active Case projection: {error}" } };
        }
    };
    let selected_board = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id);
    let board = selected_board.map(|board| CaseBoardSummary {
        id: board.id.clone(),
        name: board.name.clone(),
    });
    let bodies = snapshot
        .document
        .case_bodies
        .iter()
        .filter(|body| body.board_id == scope.board_id)
        .cloned()
        .collect();
    let configured = effective.mechanical.as_ref();
    let mismatch = configured
        .filter(|configuration| configuration.board_id != scope.board_id)
        .map(|configuration| CaseMismatch {
            board_id: configuration.board_id.clone(),
            board_name: snapshot
                .document
                .boards
                .iter()
                .find(|board| board.id == configuration.board_id)
                .map_or_else(
                    || configuration.board_id.clone(),
                    |board| board.name.clone(),
                ),
        });
    let generated_stack =
        configured.is_some_and(|configuration| configuration.board_id == scope.board_id);
    let editable = matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) && model.display_preview.is_none()
        && model.gesture.is_none()
        && !generated_stack;
    if (body_edit_portal.editable)() != editable {
        body_edit_portal.editable.set(editable);
    }
    let editor_scope_key = format!(
        "{}:{}:{}:{}:{:?}",
        editor_instance_id,
        scope.session_epoch.0,
        scope.document_id,
        scope.board_id,
        scope.instance_id,
    );

    rsx! {
        CaseBodies {
            key: "{editor_scope_key}",
            board,
            bodies,
            scope,
            editor_instance_id,
            request_sequence,
            snapshot_token: snapshot.token,
            revision: snapshot.document.revision,
            generated_stack,
            mismatch,
            editable,
            feedback: feedback.read().clone(),
            on_edit,
            on_show_configured_board,
        }
    }
}

fn feedback_for(
    pending: &BodyEditTicket,
    state: CaseBodyEditState,
    message: Option<String>,
) -> CaseBodyEditFeedback {
    CaseBodyEditFeedback {
        editor_instance_id: pending.request.editor_instance_id,
        scope: pending.request.scope.clone(),
        snapshot_token: pending.request.snapshot_token,
        revision: pending.request.revision,
        request_id: pending.request.request_id,
        field_id: pending.request.field_id.clone(),
        state,
        message,
        created_body_id: pending.created_body_id.clone(),
    }
}

fn record_feedback(entries: &mut Vec<CaseBodyEditFeedback>, feedback: CaseBodyEditFeedback) {
    if let Some(current) = entries
        .iter_mut()
        .find(|entry| entry.field_id == feedback.field_id)
    {
        if current.request_id <= feedback.request_id {
            *current = feedback;
        }
    } else {
        entries.push(feedback);
    }
}

fn submit_body_edit(
    runtime: &Rc<Runtime>,
    instance_selection: super::InstanceSelection,
    editor_instance_id: u64,
    pending: &mut Signal<Vec<BodyEditTicket>>,
    feedback: &mut Signal<Vec<CaseBodyEditFeedback>>,
    request: CaseBodyRequest,
) {
    let model = runtime.model();
    if request.editor_instance_id != editor_instance_id
        || !instance_selection.is_current(&model)
        || runtime.scope().as_ref() != Some(&request.scope)
        || !matches!(
            model.lifecycle,
            Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
        )
        || model.display_preview.is_some()
        || model.gesture.is_some()
    {
        return;
    }
    let Some(snapshot) = model.accepted.as_ref() else {
        return;
    };
    if snapshot.session_epoch != request.scope.session_epoch
        || snapshot.document.id != request.scope.document_id
    {
        return;
    }
    if let Some(action_id) = request.edit.action_id()
        && pending
            .peek()
            .iter()
            .any(|entry| entry.request.edit.action_id().as_ref() == Some(&action_id))
    {
        return;
    }
    let seed = runtime.operation().0;
    let resolver = body_resolver(request.scope.clone(), request.edit.clone(), seed);
    let created_body_id = if matches!(request.edit, CaseBodyEdit::AddBody) {
        match resolver.resolve(snapshot) {
            Resolution::Submit(EditCommand {
                operation: EditOperation::SetCase { body },
                ..
            }) => Some(body.id),
            _ => None,
        }
    } else {
        None
    };
    let ticket = EditTicket::begin(runtime, "case-body", Some("case body".into()), resolver);
    let waiting = BodyEditTicket {
        request,
        ticket,
        created_body_id,
    };
    record_feedback(
        &mut feedback.write(),
        feedback_for(&waiting, CaseBodyEditState::Pending, None),
    );
    pending.write().push(waiting);
}

fn body_resolver(scope: Scope, edit: CaseBodyEdit, seed: u64) -> EditResolver {
    EditResolver::new("case-body", move |accepted: &AcceptedSnapshot| {
        let document = &accepted.document;
        let effective =
            match boardstudio_web_host::cad_jobs::captured_case_document(accepted, &scope) {
                Ok(value) => value,
                Err(_) => return Resolution::Retire("The Case scope no longer exists.".into()),
            };
        if effective
            .mechanical
            .as_ref()
            .is_some_and(|configuration| configuration.board_id == scope.board_id)
        {
            return Resolution::Retire(
                "Disable the generated mechanical stack before editing authored case bodies."
                    .into(),
            );
        }
        let mut body = match &edit {
            CaseBodyEdit::AddBody => match new_case_body(seed, document, &scope.board_id) {
                Ok(body) => body,
                Err(message) => return Resolution::Retire(message),
            },
            _ => match document.case_bodies.iter().find(|body| {
                Some(body.id.as_str()) == edit.body_id() && body.board_id == scope.board_id
            }) {
                Some(body) => body.clone(),
                None => {
                    return Resolution::Retire(
                        "The selected case body no longer belongs to this board.".into(),
                    );
                }
            },
        };
        if let Err(message) = apply_body_edit(seed, document, &mut body, &edit) {
            return Resolution::Retire(message);
        }
        if document
            .case_bodies
            .iter()
            .any(|accepted| accepted == &body)
        {
            return Resolution::Unchanged;
        }
        Resolution::Submit(EditCommand {
            base_revision: 0,
            transaction_id: String::new(),
            phase: EditPhase::Commit,
            target_ids: vec![body.id.clone()],
            operation: EditOperation::SetCase { body },
        })
    })
}

fn new_case_body(
    seed: u64,
    document: &boardstudio_core::model::ProjectDoc,
    board_id: &str,
) -> Result<CaseBody, String> {
    let board = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or("The selected board is unavailable.")?;
    let existing = document
        .case_bodies
        .iter()
        .map(|body| body.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let id = unique_id(seed, "case-body", |candidate| existing.contains(candidate));
    let material_id = crate::case_generation_admission::default_case_material(document);
    Ok(CaseBody {
        features: None,
        openings: None,
        id,
        name: format!("{} plate", board.name),
        board_id: board.id.clone(),
        kind: CaseKind::Plate,
        thickness: 3.0,
        clearance: 0.5,
        material_id,
        z: Some(0.0),
        wall_height: Some(14.0),
        wall_thickness: Some(2.0),
        mounts: Some(vec![]),
        gasket: None,
    })
}

fn apply_body_edit(
    seed: u64,
    document: &boardstudio_core::model::ProjectDoc,
    body: &mut CaseBody,
    edit: &CaseBodyEdit,
) -> Result<(), String> {
    use CaseBodyEdit as Edit;
    match edit {
        Edit::AddBody => {}
        Edit::SetKind { body_id, kind } if body.id == *body_id => {
            body.kind = kind.clone();
            body.wall_height.get_or_insert(14.0);
            body.wall_thickness.get_or_insert(2.0);
        }
        Edit::SetThickness { body_id, value } if body.id == *body_id => {
            require_positive(*value, "Thickness")?;
            body.thickness = *value;
        }
        Edit::SetClearance { body_id, value } if body.id == *body_id => {
            require_nonnegative(*value, "Clearance")?;
            body.clearance = *value;
        }
        Edit::SetZ { body_id, value } if body.id == *body_id => {
            require_finite(*value, "Z offset")?;
            body.z = Some(*value);
        }
        Edit::SetWallHeight { body_id, value } if body.id == *body_id => {
            require_positive(*value, "Wall height")?;
            body.wall_height = Some(*value);
        }
        Edit::SetWallThickness { body_id, value } if body.id == *body_id => {
            require_positive(*value, "Wall thickness")?;
            body.wall_thickness = Some(*value);
        }
        Edit::AddMount { body_id } if body.id == *body_id => {
            let existing = document
                .case_bodies
                .iter()
                .flat_map(|body| body.mounts.iter().flatten())
                .map(|mount| mount.id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            let id = unique_id(seed, "case-mount", |candidate| existing.contains(candidate));
            body.mounts.get_or_insert_with(Vec::new).push(Mount {
                id,
                at: Vec2::default(),
                kind: MountKind::Hole,
                hole_diameter: 2.5,
                boss_diameter: Some(5.0),
                height: Some(5.0),
            });
        }
        Edit::SetMountKind {
            body_id,
            mount_id,
            kind,
        } if body.id == *body_id => {
            find_mount_mut(body, mount_id)?.kind = kind.clone();
        }
        Edit::SetMountX {
            body_id,
            mount_id,
            value,
        } if body.id == *body_id => {
            require_finite(*value, "Mount X position")?;
            find_mount_mut(body, mount_id)?.at.x = *value;
        }
        Edit::SetMountY {
            body_id,
            mount_id,
            value,
        } if body.id == *body_id => {
            require_finite(*value, "Mount Y position")?;
            find_mount_mut(body, mount_id)?.at.y = *value;
        }
        Edit::SetMountPosition {
            body_id,
            mount_id,
            at,
        } if body.id == *body_id => {
            require_finite(at.x, "Mount X position")?;
            require_finite(at.y, "Mount Y position")?;
            find_mount_mut(body, mount_id)?.at = *at;
        }
        Edit::SetMountHoleDiameter {
            body_id,
            mount_id,
            value,
        } if body.id == *body_id => {
            require_positive(*value, "Hole diameter")?;
            find_mount_mut(body, mount_id)?.hole_diameter = *value;
        }
        Edit::SetMountBossDiameter {
            body_id,
            mount_id,
            value,
        } if body.id == *body_id => {
            require_positive(*value, "Boss diameter")?;
            find_mount_mut(body, mount_id)?.boss_diameter = Some(*value);
        }
        Edit::SetMountHeight {
            body_id,
            mount_id,
            value,
        } if body.id == *body_id => {
            require_positive(*value, "Mount height")?;
            find_mount_mut(body, mount_id)?.height = Some(*value);
        }
        Edit::RemoveMount { body_id, mount_id } if body.id == *body_id => {
            let mounts = body
                .mounts
                .as_mut()
                .ok_or("Mount is no longer available.")?;
            let before = mounts.len();
            mounts.retain(|mount| mount.id != *mount_id);
            if mounts.len() == before {
                return Err("Mount is no longer available.".into());
            }
        }
        Edit::SetGasket { body_id, gasket } if body.id == *body_id => {
            if let Some(gasket) = gasket {
                require_nonnegative(gasket.inset, "Gasket inset")?;
                require_positive(gasket.width, "Gasket width")?;
                require_positive(gasket.depth, "Gasket depth")?;
            }
            body.gasket = gasket.clone();
        }
        _ => return Err("The Case edit target changed before it could be applied.".into()),
    }
    Ok(())
}

fn find_mount_mut<'a>(body: &'a mut CaseBody, mount_id: &str) -> Result<&'a mut Mount, String> {
    body.mounts
        .as_mut()
        .and_then(|mounts| mounts.iter_mut().find(|mount| mount.id == mount_id))
        .ok_or_else(|| "Mount is no longer available.".into())
}

fn unique_id(seed: u64, prefix: &str, mut already_exists: impl FnMut(&str) -> bool) -> String {
    for suffix in 0_u64.. {
        let candidate = format!("{prefix}-{seed}-{suffix}");
        if !already_exists(&candidate) {
            return candidate;
        }
    }
    unreachable!("identifier space exhausted")
}

fn require_finite(value: f64, label: &str) -> Result<(), String> {
    value
        .is_finite()
        .then_some(())
        .ok_or_else(|| format!("{label} must be finite."))
}

fn require_positive(value: f64, label: &str) -> Result<(), String> {
    (value.is_finite() && value > 0.0)
        .then_some(())
        .ok_or_else(|| format!("{label} must be greater than zero."))
}

fn require_nonnegative(value: f64, label: &str) -> Result<(), String> {
    (value.is_finite() && value >= 0.0)
        .then_some(())
        .ok_or_else(|| format!("{label} must be zero or greater."))
}

impl CaseBodyEdit {
    fn body_id(&self) -> Option<&str> {
        use CaseBodyEdit as Edit;
        match self {
            Edit::AddBody => None,
            Edit::SetKind { body_id, .. }
            | Edit::SetThickness { body_id, .. }
            | Edit::SetClearance { body_id, .. }
            | Edit::SetZ { body_id, .. }
            | Edit::SetWallHeight { body_id, .. }
            | Edit::SetWallThickness { body_id, .. }
            | Edit::AddMount { body_id }
            | Edit::SetMountKind { body_id, .. }
            | Edit::SetMountX { body_id, .. }
            | Edit::SetMountY { body_id, .. }
            | Edit::SetMountPosition { body_id, .. }
            | Edit::SetMountHoleDiameter { body_id, .. }
            | Edit::SetMountBossDiameter { body_id, .. }
            | Edit::SetMountHeight { body_id, .. }
            | Edit::RemoveMount { body_id, .. }
            | Edit::SetGasket { body_id, .. } => Some(body_id),
        }
    }
}

#[cfg(test)]
mod queued_body_tests {
    use super::*;
    use crate::runtime::project_name_test_support as support;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    fn host() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        crate::use_empty_test_instance_selection();
        crate::test_contexts::use_case_viewer_test_contexts();
        use_hook(move || {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }))
        });
        rsx! { CaseBodyInspector { on_show_configured_board: |_| {} } }
    }

    #[wasm_bindgen_test]
    async fn mounted_body_fields_queue_and_undo_independently() {
        let runtime = support::new_runtime();
        let mut document = ProjectDoc::empty("queued-bodies", "Bodies");
        document.boards.push(
            serde_json::from_value(serde_json::json!({
                "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
                "netIds": [], "thickness": 1.6, "traces": [], "vias": []
            }))
            .unwrap(),
        );
        document
            .case_bodies
            .push(new_case_body(1, &document, "board").unwrap());
        support::open_document(&runtime, document).await;
        let dom_document = web_sys::window().unwrap().document().unwrap();
        let root = dom_document.create_element("div").unwrap();
        dom_document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(50).await;
        let commit = |index, value: &str| {
            let input = root
                .query_selector_all(".m1-case-measures input")
                .unwrap()
                .item(index)
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
            input.set_value(value);
            let event = web_sys::EventInit::new();
            event.set_bubbles(true);
            input
                .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
                .unwrap();
            let enter = web_sys::KeyboardEventInit::new();
            enter.set_key("Enter");
            enter.set_bubbles(true);
            input
                .dispatch_event(
                    &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter)
                        .unwrap(),
                )
                .unwrap();
        };
        let (entered, release) = support::gate_next_core_reply(&runtime);
        commit(0, "4");
        support::drive_pending(&runtime);
        entered.await.unwrap();
        commit(1, "0.8");
        support::drive_pending(&runtime);
        release.send(()).unwrap();
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(accepted.document.case_bodies[0].thickness, 4.0);
        assert_eq!(accepted.document.case_bodies[0].clearance, 0.8);
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.case_bodies[0].thickness,
            4.0
        );
        assert_eq!(
            runtime.model().accepted.unwrap().document.case_bodies[0].clearance,
            0.5
        );
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.case_bodies[0].thickness,
            3.0
        );
        runtime.unsubscribe();
        root.remove();
    }
}
