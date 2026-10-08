//! Private Case authoring adapter. The child owns drafts; Runtime remains authoritative.
use super::case_bodies::{
    CaseBoardSummary, CaseBodies, CaseBodyEdit, CaseBodyEditFeedback, CaseBodyRequest, CaseMismatch,
};
use crate::owned_edits::OwnedEdits;
use crate::runtime::Runtime;
#[cfg(test)]
use boardstudio_application::Event;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Lifecycle, Resolution, Scope};
#[cfg(test)]
use boardstudio_core::model::ProjectDoc;
use boardstudio_core::model::{
    CaseBody, CaseKind, EditCommand, EditOperation, Mount, MountKind, Vec2,
};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use dioxus::prelude::*;
use std::rc::Rc;

struct BodyEditMeta {
    request: CaseBodyRequest,
    created_body_id: Option<String>,
}

/// What a body edit is logically: a numeric field, or another edit kind on a body (an
/// action such as adding a mount, or a kind change). Edits of different kinds never share
/// a key, so one cannot replace the observation of another.
#[derive(Clone, PartialEq)]
enum BodyEditLogical {
    Field(String),
    Edit(std::mem::Discriminant<CaseBodyEdit>, Option<String>),
}

/// A body edit's logical identity: the latest edit for it replaces the earlier one. The key
/// carries its request and created body for follow-ups; keys compare by logical identity only.
#[derive(Clone)]
struct BodyEditKey {
    logical: BodyEditLogical,
    meta: Option<Rc<BodyEditMeta>>,
}

impl PartialEq for BodyEditKey {
    fn eq(&self, other: &Self) -> bool {
        self.logical == other.logical
    }
}

type BodyEditPending = OwnedEdits<BodyEditKey>;

impl BodyEditKey {
    /// A key without a request, for binding a field's Signals.
    fn bound(field_id: &str) -> Self {
        Self {
            logical: BodyEditLogical::Field(field_id.to_owned()),
            meta: None,
        }
    }

    fn of(waiting: BodyEditMeta) -> Self {
        let logical = match &waiting.request.field_id {
            Some(field_id) => BodyEditLogical::Field(field_id.clone()),
            None => BodyEditLogical::Edit(
                std::mem::discriminant(&waiting.request.edit),
                waiting.request.edit.body_id().map(str::to_owned),
            ),
        };
        Self {
            logical,
            meta: Some(Rc::new(waiting)),
        }
    }
}

struct CaseFieldEntry {
    field_id: String,
    draft: Signal<String>,
    /// The text the accepted document shows for the field, refreshed on every render.
    accepted: String,
}

/// The controller's helper and the mounted number fields it is bound to. Fields register
/// while mounted so the controller reads the draft a commit submits and the accepted text a
/// settlement restores, and releases their Signals when they leave.
#[derive(Clone)]
pub(crate) struct CaseEditsContext {
    pending: Signal<BodyEditPending>,
    fields: Rc<std::cell::RefCell<Vec<CaseFieldEntry>>>,
}

impl CaseEditsContext {
    fn accepted_text(&self, key: &BodyEditKey) -> String {
        self.fields
            .borrow()
            .iter()
            .find(|entry| BodyEditLogical::Field(entry.field_id.clone()) == key.logical)
            .map(|entry| entry.accepted.clone())
            .unwrap_or_default()
    }

    fn draft_text(&self, field_id: Option<&String>) -> String {
        self.fields
            .borrow()
            .iter()
            .find(|entry| Some(&entry.field_id) == field_id)
            .map(|entry| entry.draft.peek().clone())
            .unwrap_or_default()
    }
}

/// Bind a number field's draft and failure Signals to the controller's helper while the
/// calling component is mounted; they are released when it unmounts. Return whether
/// the helper is still observing a pending edit for this field.
pub(crate) fn use_bound_case_field(
    field_id: &str,
    accepted: String,
    draft: Signal<String>,
    failure: Signal<Option<String>>,
) -> bool {
    let context = try_consume_context::<CaseEditsContext>();
    let bound = use_hook(|| Rc::new(std::cell::RefCell::new(None::<String>)));
    if let Some(context) = context.as_ref() {
        let mut previous = bound.borrow_mut();
        if let Some(old) = previous.as_ref()
            && old != field_id
        {
            context
                .pending
                .peek()
                .unbind_field(&BodyEditKey::bound(old));
            context
                .fields
                .borrow_mut()
                .retain(|entry| entry.field_id != *old);
        }
        context
            .pending
            .peek()
            .bind_field(BodyEditKey::bound(field_id), draft, failure);
        let mut fields = context.fields.borrow_mut();
        match fields.iter_mut().find(|entry| entry.field_id == field_id) {
            Some(entry) => {
                entry.draft = draft;
                entry.accepted = accepted;
            }
            None => fields.push(CaseFieldEntry {
                field_id: field_id.to_owned(),
                draft,
                accepted,
            }),
        }
        *previous = Some(field_id.to_owned());
    }
    let pending = context.as_ref().is_some_and(|context| {
        context
            .pending
            .read()
            .is_pending(&BodyEditKey::bound(field_id))
    });
    use_drop({
        let bound = bound.clone();
        move || {
            if let (Some(context), Some(field_id)) = (context.as_ref(), bound.borrow_mut().take()) {
                context
                    .pending
                    .peek()
                    .unbind_field(&BodyEditKey::bound(&field_id));
                context
                    .fields
                    .borrow_mut()
                    .retain(|entry| entry.field_id != field_id);
            }
        }
    });
    pending
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
    let pending = use_signal(BodyEditPending::default);
    let edits_context = CaseEditsContext {
        pending,
        fields: use_hook(|| Rc::new(std::cell::RefCell::new(Vec::new()))),
    };
    use_context_provider(|| edits_context.clone());
    let feedback = use_signal(|| Vec::<CaseBodyEditFeedback>::new());
    let body_edit_dispatch = use_hook({
        let runtime = runtime.clone();
        let edits_context = edits_context.clone();
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
                        &edits_context,
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
        let edits_context = edits_context.clone();
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |_| {
            if !pending.peek().has_terminal() {
                return;
            }
            let scope = runtime.scope();
            let results = pending
                .peek()
                .helper
                .settle(true, |key| edits_context.accepted_text(key));
            let mut next_feedback = feedback.peek().clone();
            for result in results {
                let (PendingEditResult::Landed { key, .. }
                | PendingEditResult::Failed { key, .. }
                | PendingEditResult::Retired { key }) = &result;
                let Some(waiting) = key.meta.as_deref() else {
                    continue;
                };
                let live = scope.as_ref() == Some(&waiting.request.scope);
                match &result {
                    PendingEditResult::Landed { .. } if live => {
                        record_feedback(&mut next_feedback, feedback_for(waiting, None));
                    }
                    PendingEditResult::Failed { message, .. } if live => record_feedback(
                        &mut next_feedback,
                        feedback_for(waiting, Some(message.clone())),
                    ),
                    _ => {
                        next_feedback.retain(|entry| entry.request_id != waiting.request.request_id)
                    }
                }
            }
            feedback.set(next_feedback);
            pending.write().prune();
        }
    }));

    let on_edit = {
        let runtime = runtime.clone();
        let edits_context = edits_context.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |request: CaseBodyRequest| {
            submit_body_edit(
                &runtime,
                instance_selection,
                editor_instance_id,
                &mut pending,
                &mut feedback,
                &edits_context,
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

fn feedback_for(pending: &BodyEditMeta, failure: Option<String>) -> CaseBodyEditFeedback {
    CaseBodyEditFeedback {
        editor_instance_id: pending.request.editor_instance_id,
        scope: pending.request.scope.clone(),
        snapshot_token: pending.request.snapshot_token,
        revision: pending.request.revision,
        request_id: pending.request.request_id,
        field_id: pending.request.field_id.clone(),
        pending: false,
        failure,
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
    pending: &mut Signal<BodyEditPending>,
    feedback: &mut Signal<Vec<CaseBodyEditFeedback>>,
    edits: &CaseEditsContext,
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
            .pending()
            .filter_map(|entry| entry.meta.as_deref())
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
    let scope = request.scope.clone();
    let draft_text = edits.draft_text(request.field_id.as_ref());
    let waiting = BodyEditMeta {
        request,
        created_body_id,
    };
    let mut submitted = feedback_for(&waiting, None);
    submitted.pending = true;
    record_feedback(&mut feedback.write(), submitted);
    let key = BodyEditKey::of(waiting);
    if pending.peek().owner_changed(Some(&scope), 0) {
        pending.write().follow_owner(Some(&scope), 0);
    }
    pending.peek().helper.begin_field(
        runtime,
        key.clone(),
        "case-body",
        Some("case body".into()),
        resolver,
        &draft_text,
    );
    pending.write().remember(key);
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
        Resolution::submit(vec![body.id.clone()], EditOperation::SetCase { body })
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
        Edit::SetGasketInset { body_id, value } if body.id == *body_id => {
            require_nonnegative(*value, "Gasket inset")?;
            body.gasket
                .as_mut()
                .ok_or("Gasket is no longer available.")?
                .inset = *value;
        }
        Edit::SetGasketWidth { body_id, value } if body.id == *body_id => {
            require_positive(*value, "Gasket width")?;
            body.gasket
                .as_mut()
                .ok_or("Gasket is no longer available.")?
                .width = *value;
        }
        Edit::SetGasketDepth { body_id, value } if body.id == *body_id => {
            require_positive(*value, "Gasket depth")?;
            body.gasket
                .as_mut()
                .ok_or("Gasket is no longer available.")?
                .depth = *value;
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
            | Edit::SetGasketInset { body_id, .. }
            | Edit::SetGasketWidth { body_id, .. }
            | Edit::SetGasketDepth { body_id, .. }
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

    #[wasm_bindgen_test]
    async fn mounted_gasket_fields_queue_and_undo_independently() {
        let runtime = support::new_runtime();
        let mut document = ProjectDoc::empty("queued-gaskets", "Bodies");
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
        document.case_bodies[0].gasket = Some(boardstudio_core::model::Gasket {
            inset: 2.0,
            width: 2.0,
            depth: 1.5,
        });
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
                .query_selector_all(".m1-case-gasket-measures input")
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
        commit(0, "3");
        support::drive_pending(&runtime);
        entered.await.unwrap();
        let inset = root
            .query_selector(".m1-case-gasket-measures input")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        inset.set_value("5");
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        inset
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        commit(1, "4");
        support::drive_pending(&runtime);
        release.send(()).unwrap();
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            accepted.document.case_bodies[0]
                .gasket
                .as_ref()
                .unwrap()
                .inset,
            3.0
        );
        assert_eq!(
            accepted.document.case_bodies[0]
                .gasket
                .as_ref()
                .unwrap()
                .width,
            4.0
        );
        assert_eq!(
            inset.value(),
            "5",
            "older gasket landing preserves newer active typing"
        );
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.case_bodies[0]
                .gasket
                .as_ref()
                .unwrap()
                .inset,
            3.0
        );
        assert_eq!(
            runtime.model().accepted.unwrap().document.case_bodies[0]
                .gasket
                .as_ref()
                .unwrap()
                .width,
            2.0
        );
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.case_bodies[0]
                .gasket
                .as_ref()
                .unwrap()
                .inset,
            2.0
        );
        runtime.unsubscribe();
        root.remove();
    }

    // ---- Settlement of a bound body field through the real number input.

    async fn mounted_two_bodies() -> (Rc<Runtime>, web_sys::Element) {
        let runtime = support::new_runtime();
        let mut document = ProjectDoc::empty("body-settlement", "Bodies");
        document.boards.push(
            serde_json::from_value(serde_json::json!({
                "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
                "netIds": [], "thickness": 1.6, "traces": [], "vias": []
            }))
            .unwrap(),
        );
        for seed in [1, 2] {
            let body = new_case_body(seed, &document, "board").unwrap();
            document.case_bodies.push(body);
        }
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
        (runtime, root)
    }

    fn select_body(root: &web_sys::Element, index: u32) {
        root.query_selector_all(".m1-case-body-tab")
            .unwrap()
            .item(index)
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
    }

    fn thickness_input(root: &web_sys::Element) -> web_sys::HtmlInputElement {
        root.query_selector_all(".m1-case-measures input")
            .unwrap()
            .item(0)
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap()
    }

    fn type_into(input: &web_sys::HtmlInputElement, value: &str) {
        input.set_value(value);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
    }

    fn press_enter(input: &web_sys::HtmlInputElement) {
        let enter = web_sys::KeyboardEventInit::new();
        enter.set_key("Enter");
        enter.set_bubbles(true);
        input
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter)
                    .unwrap(),
            )
            .unwrap();
    }

    async fn settle_body(runtime: &Rc<Runtime>) {
        for _ in 0..20 {
            support::run_pending(runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
    }

    fn inline_failure(root: &web_sys::Element) -> Option<String> {
        root.query_selector(".m1-case-measures small.m1-case-field-error[role='alert']")
            .unwrap()
            .and_then(|alert| alert.text_content())
    }

    fn thickness(runtime: &Rc<Runtime>) -> f64 {
        runtime.model().accepted.unwrap().document.case_bodies[0].thickness
    }

    #[wasm_bindgen_test]
    async fn a_failed_unchanged_body_draft_restores_the_accepted_value_with_an_inline_failure() {
        let (runtime, root) = mounted_two_bodies().await;
        support::fail_next_core_reply(&runtime, "body executor failed");
        let input = thickness_input(&root);
        type_into(&input, "4");
        press_enter(&input);
        settle_body(&runtime).await;
        assert_eq!(thickness(&runtime), 3.0);
        assert_eq!(thickness_input(&root).value(), "3", "the draft restores");
        assert!(
            inline_failure(&root).is_some_and(|message| message.contains("body executor failed"))
        );
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_newer_body_draft_survives_an_older_failure_that_reports_inline() {
        let (runtime, root) = mounted_two_bodies().await;
        support::fail_next_core_reply(&runtime, "body executor failed");
        let input = thickness_input(&root);
        type_into(&input, "4");
        press_enter(&input);
        // The older edit has not settled when the user types the next draft.
        type_into(&input, "5");
        settle_body(&runtime).await;
        assert_eq!(
            thickness_input(&root).value(),
            "5",
            "a newer draft survives"
        );
        assert!(
            inline_failure(&root).is_some_and(|message| message.contains("body executor failed")),
            "the older failure still reports inline beside the newer text"
        );
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_second_edit_to_one_body_field_replaces_the_observation_and_both_edits_undo() {
        let (runtime, root) = mounted_two_bodies().await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let input = thickness_input(&root);
        type_into(&input, "4");
        press_enter(&input);
        support::drive_pending(&runtime);
        entered.await.unwrap();
        type_into(&input, "5");
        press_enter(&input);
        support::drive_pending(&runtime);
        release.send(()).unwrap();
        settle_body(&runtime).await;
        assert_eq!(thickness(&runtime), 5.0);
        assert_eq!(thickness_input(&root).value(), "5");
        assert!(inline_failure(&root).is_none());
        for expected in [4.0, 3.0] {
            runtime.submit(Event::Undo {
                operation_id: runtime.operation(),
            });
            support::run_pending(&runtime).await;
            assert_eq!(thickness(&runtime), expected, "each edit has an Undo step");
        }
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn repeated_enter_and_native_blur_commit_one_body_edit_then_the_field_follows_undo() {
        let (runtime, root) = mounted_two_bodies().await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let input = thickness_input(&root);
        let _ = input.focus();
        type_into(&input, "4");
        press_enter(&input);
        support::drive_pending(&runtime);
        entered.await.unwrap();
        let duplicate = support::observe_next(&runtime);
        press_enter(&input);
        let _ = input.blur();
        support::drive_pending(&runtime);
        release.send(()).unwrap();
        settle_body(&runtime).await;
        assert_eq!(thickness(&runtime), 4.0);
        assert!(
            duplicate.borrow().is_none(),
            "unchanged Enter and blur admit no second edit"
        );
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        settle_body(&runtime).await;
        assert_eq!(thickness(&runtime), 3.0, "one input creates one Undo step");
        assert_eq!(thickness_input(&root).value(), "3");
        assert!(inline_failure(&root).is_none());
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_body_field_that_leaves_with_its_body_tab_keeps_its_held_edit() {
        let (runtime, root) = mounted_two_bodies().await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let input = thickness_input(&root);
        type_into(&input, "4");
        press_enter(&input);
        support::drive_pending(&runtime);
        entered.await.unwrap();
        // The other body's tab replaces the editor while the first body's edit is held.
        select_body(&root, 1);
        gloo_timers::future::TimeoutFuture::new(50).await;
        release.send(()).unwrap();
        settle_body(&runtime).await;
        let document = runtime.model().accepted.unwrap().document;
        assert_eq!(
            document.case_bodies[0].thickness, 4.0,
            "the queued edit kept executing"
        );
        assert_eq!(document.case_bodies[1].thickness, 3.0);
        assert!(inline_failure(&root).is_none());
        select_body(&root, 0);
        gloo_timers::future::TimeoutFuture::new(50).await;
        assert_eq!(
            thickness_input(&root).value(),
            "4",
            "the remounted field shows the accepted value"
        );
        assert!(inline_failure(&root).is_none());
        runtime.unsubscribe();
        root.remove();
    }
}
