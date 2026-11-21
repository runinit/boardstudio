use super::*;

pub(super) fn keep_retry_draft(
    drafts: &mut Signal<BTreeMap<(KeycapsEditTarget, KeycapEditField), KeycapsRetryDraft>>,
    request: &KeycapsEditRequest,
    message: String,
) {
    let mut values = drafts.write();
    store_retry_draft(&mut values, request, message);
}

pub(super) fn store_retry_draft(
    drafts: &mut BTreeMap<(KeycapsEditTarget, KeycapEditField), KeycapsRetryDraft>,
    request: &KeycapsEditRequest,
    message: String,
) {
    let field = request.change.field();
    let key = (request.target.clone(), field.clone());
    let replace = drafts
        .get(&key)
        .is_none_or(|existing| existing.request_id <= request.request_id);
    if replace {
        drafts.insert(
            key,
            KeycapsRetryDraft {
                editor_instance_id: request.editor_instance_id,
                scope: request.scope.clone(),
                scope_generation: request.scope_generation,
                selection_generation: request.selection_generation,
                target: request.target.clone(),
                request_id: request.request_id,
                field,
                change: request.change.clone(),
                message,
            },
        );
    }
}

pub(super) fn remove_retry_draft_through(
    drafts: &mut Signal<BTreeMap<(KeycapsEditTarget, KeycapEditField), KeycapsRetryDraft>>,
    target: &KeycapsEditTarget,
    field: &KeycapEditField,
    request_id: u64,
) {
    let mut values = drafts.write();
    remove_retry_draft_through_map(&mut values, target, field, request_id);
}

pub(super) fn remove_retry_draft_through_map(
    drafts: &mut BTreeMap<(KeycapsEditTarget, KeycapEditField), KeycapsRetryDraft>,
    target: &KeycapsEditTarget,
    field: &KeycapEditField,
    request_id: u64,
) {
    if drafts
        .get(&(target.clone(), field.clone()))
        .is_some_and(|draft| draft.request_id <= request_id)
    {
        drafts.remove(&(target.clone(), field.clone()));
    }
}

pub(super) fn draft_owner_mismatch(
    draft: &KeycapsRetryDraft,
    editor_instance_id: u64,
    scope: &Scope,
    scope_generation: u64,
    selection_generation: u64,
    selected_key_id: Option<&str>,
    workspace: &'static str,
) -> bool {
    // Drafts are cleared only when the current Editor owner changes. Their exact owner is stored
    // separately from field identity so an accepted-legend input remount cannot drop another field.
    draft.editor_instance_id != editor_instance_id
        || &draft.scope != scope
        || draft.scope_generation != scope_generation
        || workspace != "Keycaps"
        || (matches!(&draft.target, KeycapsEditTarget::Key(_))
            && (draft.selection_generation != selection_generation
                || selected_key_id
                    != match &draft.target {
                        KeycapsEditTarget::Key(id) => Some(id.as_str()),
                        _ => None,
                    }))
}

pub(super) fn retry_identity(draft: &KeycapsRetryDraft) -> KeycapsRetryIdentity {
    KeycapsRetryIdentity {
        editor_instance_id: draft.editor_instance_id,
        scope: draft.scope.clone(),
        scope_generation: draft.scope_generation,
        selection_generation: draft.selection_generation,
        target: draft.target.clone(),
        request_id: draft.request_id,
        field: draft.field.clone(),
    }
}

pub(super) fn retry_identity_matches(
    draft: &KeycapsRetryDraft,
    identity: &KeycapsRetryIdentity,
) -> bool {
    retry_identity(draft) == *identity
}

pub(super) fn retry_identity_is_current(
    identity: &KeycapsRetryIdentity,
    editor_instance_id: u64,
    scope: &Scope,
    scope_generation: u64,
    selection_generation: u64,
    selected_key_id: Option<&str>,
    workspace: &'static str,
) -> bool {
    workspace == "Keycaps"
        && identity.editor_instance_id == editor_instance_id
        && &identity.scope == scope
        && identity.scope_generation == scope_generation
        && (matches!(
            &identity.target,
            KeycapsEditTarget::Board | KeycapsEditTarget::Matrix(_)
        ) || (identity.selection_generation == selection_generation
            && selected_key_id
                == match &identity.target {
                    KeycapsEditTarget::Key(id) => Some(id.as_str()),
                    _ => None,
                }))
}

pub(super) fn submit_keycap_edit(
    runtime: &Rc<Runtime>,
    edits: &mut Signal<KeycapsEdits>,
    request: KeycapsEditRequest,
) {
    let intent = request.clone();
    let resolver = EditResolver::new("keycaps", move |accepted: &AcceptedSnapshot| {
        if !valid_edit_target(accepted, &intent.scope, &intent.target) {
            return Resolution::Retire("This keycap settings target no longer exists.".into());
        }
        let Some(operation) = edit_operation(
            &accepted.document,
            &intent.scope,
            &intent.target,
            &intent.change,
        ) else {
            return Resolution::Retire("This keycap setting is no longer available.".into());
        };
        if change_is_noop(
            &accepted.document,
            &intent.scope,
            &intent.target,
            &intent.change,
        ) {
            return Resolution::Unchanged;
        }
        Resolution::submit(vec![target_id(&intent.target, &intent.scope)], operation)
    });
    let key = KeycapsPendingKey::of(&request);
    let submitted = match &request.change {
        KeycapEditChange::Legend(value) => value.clone().unwrap_or_default(),
        change => change_label(change),
    };
    edits.write().helper(&request.target).begin_field(
        runtime,
        key,
        "keycaps",
        Some("keycap settings".into()),
        resolver,
        &submitted,
    );
}
