//! Editor-owned keycap settings edits and exact-operation feedback.
use super::keycaps_scene::{self, KeycapsKey, KeycapsView};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_core::model::{
    EditOperation, KeycapBoardChange, KeycapBoardSettings, KeycapKeyChange, KeycapKeySettings,
    KeycapMatrixChange, KeycapMatrixSettings, KeycapMount, KeycapProfile, ProjectDoc, Vec2,
};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use dioxus::prelude::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum KeycapEditField {
    BoardColor,
    BoardLegendColor,
    BoardClearance,
    MatrixProfile,
    MatrixMount,
    MatrixFirstRow,
    MatrixWallThickness,
    Legend,
    Color,
    Profile,
    Mount,
    Row,
    UnitsWidth,
    UnitsDepth,
}

#[derive(Clone, Debug, PartialEq)]
pub enum KeycapEditChange {
    BoardColor(String),
    BoardLegendColor(String),
    BoardClearance(f64),
    MatrixProfile(Option<KeycapProfile>),
    MatrixMount(Option<KeycapMount>),
    MatrixFirstRow(u8),
    MatrixWallThickness(f64),
    Legend(Option<String>),
    Color(Option<String>),
    Profile(Option<KeycapProfile>),
    Mount(Option<KeycapMount>),
    Row(Option<u8>),
    UnitsWidth(f64),
    UnitsDepth(f64),
    ClearUnits(KeycapEditField),
}

impl KeycapEditChange {
    fn field(&self) -> KeycapEditField {
        match self {
            Self::BoardColor(_) => KeycapEditField::BoardColor,
            Self::BoardLegendColor(_) => KeycapEditField::BoardLegendColor,
            Self::BoardClearance(_) => KeycapEditField::BoardClearance,
            Self::MatrixProfile(_) => KeycapEditField::MatrixProfile,
            Self::MatrixMount(_) => KeycapEditField::MatrixMount,
            Self::MatrixFirstRow(_) => KeycapEditField::MatrixFirstRow,
            Self::MatrixWallThickness(_) => KeycapEditField::MatrixWallThickness,
            Self::Legend(_) => KeycapEditField::Legend,
            Self::Color(_) => KeycapEditField::Color,
            Self::Profile(_) => KeycapEditField::Profile,
            Self::Mount(_) => KeycapEditField::Mount,
            Self::Row(_) => KeycapEditField::Row,
            Self::UnitsWidth(_) => KeycapEditField::UnitsWidth,
            Self::UnitsDepth(_) => KeycapEditField::UnitsDepth,
            Self::ClearUnits(field) => field.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum KeycapsEditTarget {
    Board,
    Matrix(String),
    Key(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum KeycapsEditBaseline {
    Board(KeycapBoardSettings),
    Matrix(KeycapMatrixSettings),
    Key(KeycapKeySettings),
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapsEditSource {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapsEditRequest {
    pub scope: Scope,
    pub scope_generation: u64,
    pub selection_generation: u64,
    pub editor_instance_id: u64,
    pub request_id: u64,
    pub admission_token: SnapshotToken,
    pub admission_revision: u64,
    pub target: KeycapsEditTarget,
    pub baseline: KeycapsEditBaseline,
    pub change: KeycapEditChange,
}

/// The newest keycap edit still pending for the visible owner. Failures are reported
/// inline at their field and as retry drafts; there is no Saved or Failed status here.
#[derive(Clone, Debug, PartialEq)]
pub struct KeycapsEditFeedback {
    pub editor_instance_id: u64,
    pub scope: Scope,
    pub scope_generation: u64,
    pub selection_generation: u64,
    pub target: KeycapsEditTarget,
    pub field: KeycapEditField,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapsRetryDraft {
    pub editor_instance_id: u64,
    pub scope: Scope,
    pub scope_generation: u64,
    pub selection_generation: u64,
    pub target: KeycapsEditTarget,
    pub request_id: u64,
    pub field: KeycapEditField,
    pub change: KeycapEditChange,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapsRetryIdentity {
    pub editor_instance_id: u64,
    pub scope: Scope,
    pub scope_generation: u64,
    pub selection_generation: u64,
    pub target: KeycapsEditTarget,
    pub request_id: u64,
    pub field: KeycapEditField,
}

#[derive(Props, Clone, PartialEq)]
pub struct KeycapsSettingsActions {
    pub editor_instance_id: u64,
    pub scope: Scope,
    pub scope_generation: u64,
    pub selection_generation: u64,
    pub token: SnapshotToken,
    pub revision: u64,
    pub selected_key_id: String,
    pub accepted_settings: KeycapKeySettings,
    pub request_sequence: Signal<u64>,
    pub feedback: Option<KeycapsEditFeedback>,
    pub retry_drafts: Rc<[KeycapsRetryDraft]>,
    pub on_change: EventHandler<KeycapsEditRequest>,
    pub on_retry: EventHandler<KeycapsRetryIdentity>,
    pub on_discard: EventHandler<KeycapsRetryIdentity>,
}

/// A field's logical identity: the latest edit for it replaces the earlier observation.
/// The key carries its request for follow-ups; keys compare by target and field only.
#[derive(Clone)]
struct KeycapsPendingKey {
    target: KeycapsEditTarget,
    field: KeycapEditField,
    request: Option<Rc<KeycapsEditRequest>>,
}

impl PartialEq for KeycapsPendingKey {
    fn eq(&self, other: &Self) -> bool {
        self.target == other.target && self.field == other.field
    }
}

impl KeycapsPendingKey {
    fn of(request: &KeycapsEditRequest) -> Self {
        Self {
            target: request.target.clone(),
            field: request.change.field(),
            request: Some(Rc::new(request.clone())),
        }
    }

    fn bound(target: KeycapsEditTarget, field: KeycapEditField) -> Self {
        Self {
            target,
            field,
            request: None,
        }
    }
}

type KeycapsHelper = PendingEditSignals<KeycapsPendingKey>;

/// The edit observers, one per owner lifetime. Board and Matrix edits live while the Editor
/// owns the scope in the Keycaps workspace; a Key edit also needs its selection. A changed
/// owner replaces the helper, silently retiring the observation; the Session edit still runs.
struct KeycapsEdits {
    shared: KeycapsHelper,
    shared_owner: Option<(Option<Scope>, u64, &'static str)>,
    key: KeycapsHelper,
    key_owner: u64,
    /// Caller projection memory: the requests whose edits are still observed.
    requests: Vec<KeycapsPendingKey>,
}

impl Default for KeycapsEdits {
    fn default() -> Self {
        Self {
            shared: KeycapsHelper::new(),
            shared_owner: None,
            key: KeycapsHelper::new(),
            key_owner: 0,
            requests: Vec::new(),
        }
    }
}

impl KeycapsEdits {
    fn helper(&self, target: &KeycapsEditTarget) -> &KeycapsHelper {
        match target {
            KeycapsEditTarget::Key(_) => &self.key,
            _ => &self.shared,
        }
    }

    /// Replace a helper whose owner lifetime ended. Returns whether anything was dropped.
    fn follow_owner(
        &mut self,
        scope: Option<&Scope>,
        scope_generation: u64,
        workspace: &'static str,
        owner_generation: u64,
    ) -> bool {
        let shared_owner = (scope.cloned(), scope_generation, workspace);
        let mut dropped = false;
        if self.shared_owner.as_ref() != Some(&shared_owner) {
            if self.shared_owner.is_some() {
                self.shared = KeycapsHelper::new();
                dropped = true;
            }
            self.shared_owner = Some(shared_owner);
        }
        if self.key_owner != owner_generation {
            if self.key_owner != 0 {
                self.key = KeycapsHelper::new();
                dropped = true;
            }
            self.key_owner = owner_generation;
        }
        dropped
    }

    fn owners_changed(
        &self,
        scope: Option<&Scope>,
        scope_generation: u64,
        workspace: &'static str,
        owner_generation: u64,
    ) -> bool {
        self.shared_owner.as_ref() != Some(&(scope.cloned(), scope_generation, workspace))
            || self.key_owner != owner_generation
    }

    /// Bind the selected key's legend Signals to the key helper.
    fn bind_legend(
        &self,
        selected_key_id: Option<&str>,
        draft: Signal<String>,
        failure: Signal<Option<String>>,
    ) {
        if let Some(id) = selected_key_id {
            self.key.bind_field(
                KeycapsPendingKey::bound(
                    KeycapsEditTarget::Key(id.to_owned()),
                    KeycapEditField::Legend,
                ),
                draft,
                failure,
            );
        }
    }

    fn has_terminal(&self) -> bool {
        self.requests.iter().any(|entry| !self.is_pending(entry))
    }

    fn prune(&mut self) {
        let (shared, key) = (&self.shared, &self.key);
        self.requests.retain(|entry| match entry.target {
            KeycapsEditTarget::Key(_) => key.is_pending(entry),
            _ => shared.is_pending(entry),
        });
    }

    fn is_pending(&self, key: &KeycapsPendingKey) -> bool {
        self.helper(&key.target).is_pending(key)
    }
}

#[derive(Clone, Copy)]
struct KeycapsEditsContext {
    edits: Signal<KeycapsEdits>,
    legend_draft: Signal<String>,
    legend_failure: Signal<Option<String>>,
}

// These values belong only to outstanding field edits. They are presentation drafts,
// never used to construct a replacement document or as the accepted baseline.
fn pending_changes(
    actions: &KeycapsSettingsActions,
    target: &KeycapsEditTarget,
) -> Vec<KeycapEditChange> {
    let Some(context) = try_consume_context::<KeycapsEditsContext>() else {
        return vec![];
    };
    let edits = context.edits.read();
    edits
        .requests
        .iter()
        .filter_map(|entry| {
            let request = entry.request.as_ref()?;
            (request.scope == actions.scope
                && request.scope_generation == actions.scope_generation
                && request.selection_generation == actions.selection_generation
                && request.target == *target
                && edits.is_pending(entry))
            .then(|| request.change.clone())
        })
        .collect()
}

fn field_pending(
    actions: &KeycapsSettingsActions,
    target: &KeycapsEditTarget,
    field: KeycapEditField,
) -> bool {
    pending_changes(actions, target)
        .iter()
        .any(|change| change.field() == field)
}

/// The legend text an accepted keycap shows; none is the empty draft.
fn accepted_legend_text(document: &ProjectDoc, key_id: &str) -> String {
    settings_for_key(document, key_id)
        .legend
        .unwrap_or_default()
}

#[derive(Default)]
struct OwnerTracker {
    owner: Option<(Option<Scope>, u64, &'static str, Option<String>)>,
    generation: u64,
}

impl OwnerTracker {
    fn observe(
        &mut self,
        scope: Option<&Scope>,
        scope_generation: u64,
        workspace: &'static str,
        selected_key_id: Option<&str>,
    ) -> u64 {
        let next = (
            scope.cloned(),
            scope_generation,
            workspace,
            selected_key_id.map(str::to_owned),
        );
        if self.owner.as_ref() != Some(&next) {
            self.generation = self.generation.saturating_add(1);
            self.owner = Some(next);
        }
        self.generation
    }
}

/// Owns keycap edit admission for the Editor lifetime, including while the Keycaps workspace is
/// hidden. Root calls this unconditionally and mounts the returned view actions in its Inspector.
pub fn use_keycaps_settings_actions(
    runtime: Rc<Runtime>,
    source: Option<KeycapsEditSource>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
) -> Option<KeycapsSettingsActions> {
    let version = use_context::<Signal<u64>>()();
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let request_sequence = use_signal(|| 0_u64);
    let last_request_id = use_signal(|| 0_u64);
    let edits = use_signal(KeycapsEdits::default);
    let legend_draft = use_signal(String::new);
    let legend_failure = use_signal(|| None::<String>);
    use_context_provider(|| KeycapsEditsContext {
        edits,
        legend_draft,
        legend_failure,
    });
    let mut retry_drafts =
        use_signal(BTreeMap::<(KeycapsEditTarget, KeycapEditField), KeycapsRetryDraft>::new);
    let owner_tracker = use_hook(|| Rc::new(RefCell::new(OwnerTracker::default())));
    let selected_key_id = live_selected_key_id(&runtime);
    let current_scope = runtime.scope();
    let current_scope_generation = scope_generation();
    let current_workspace = workspace();
    let selection_generation = owner_tracker.borrow_mut().observe(
        current_scope.as_ref(),
        current_scope_generation,
        current_workspace,
        selected_key_id.as_deref(),
    );
    let obsolete_retry_fields: Vec<_> = {
        let drafts = retry_drafts.read();
        if let Some(scope) = current_scope.as_ref() {
            drafts
                .iter()
                .filter(|(_, draft)| {
                    draft_owner_mismatch(
                        draft,
                        editor_instance_id,
                        scope,
                        current_scope_generation,
                        selection_generation,
                        selected_key_id.as_deref(),
                        current_workspace,
                    )
                })
                .map(|(key, _)| key.clone())
                .collect()
        } else {
            drafts.keys().cloned().collect()
        }
    };
    if !obsolete_retry_fields.is_empty() {
        let mut drafts = retry_drafts.write();
        for field in obsolete_retry_fields {
            drafts.remove(&field);
        }
    }
    let active_source = source.clone();
    let accepted_settings = runtime
        .model()
        .accepted
        .map(|snapshot| {
            settings_for_key(
                &snapshot.document,
                selected_key_id.as_deref().unwrap_or_default(),
            )
        })
        .unwrap_or_default();

    use_effect(use_reactive((&version,), {
        let runtime = runtime.clone();
        let mut edits = edits;
        let mut retry_drafts = retry_drafts;
        let owner_tracker = owner_tracker.clone();
        move |_| {
            let live_key_id = live_selected_key_id(&runtime);
            let live_scope = runtime.scope();
            let generation = scope_generation();
            let live_workspace = workspace();
            let owner_generation = owner_tracker.borrow_mut().observe(
                live_scope.as_ref(),
                generation,
                live_workspace,
                live_key_id.as_deref(),
            );
            if edits.peek().owners_changed(
                live_scope.as_ref(),
                generation,
                live_workspace,
                owner_generation,
            ) {
                let mut guard = edits.write();
                guard.follow_owner(
                    live_scope.as_ref(),
                    generation,
                    live_workspace,
                    owner_generation,
                );
                guard.prune();
                guard.bind_legend(live_key_id.as_deref(), legend_draft, legend_failure);
            }
            if !edits.peek().has_terminal() {
                return;
            }
            let document = runtime.model().accepted.map(|snapshot| snapshot.document);
            let results = {
                let guard = edits.peek();
                let accepted = |key: &KeycapsPendingKey| match (&key.target, &document) {
                    (KeycapsEditTarget::Key(id), Some(document)) => {
                        accepted_legend_text(document, id)
                    }
                    _ => String::new(),
                };
                let mut results = guard.shared.settle(true, accepted);
                results.extend(guard.key.settle(true, accepted));
                results
            };
            for result in results {
                let (PendingEditResult::Landed { key, .. }
                | PendingEditResult::Failed { key, .. }
                | PendingEditResult::Retired { key }) = &result;
                let Some(request) = key.request.as_deref() else {
                    continue;
                };
                let live = request_owner_is_current(
                    request,
                    editor_instance_id,
                    live_scope.as_ref(),
                    generation,
                    owner_generation,
                    live_key_id.as_deref(),
                    live_workspace,
                );
                match &result {
                    PendingEditResult::Landed { .. } if live => remove_retry_draft_through(
                        &mut retry_drafts,
                        &request.target,
                        &request.change.field(),
                        request.request_id,
                    ),
                    PendingEditResult::Failed { message, .. } if live => {
                        keep_retry_draft(&mut retry_drafts, request, message.clone());
                    }
                    _ => {}
                }
            }
            edits.write().prune();
        }
    }));

    let visible_feedback = {
        let live_key_id = live_selected_key_id(&runtime);
        let live_scope = runtime.scope();
        let live_scope_generation = scope_generation();
        let owner_generation = owner_tracker.borrow_mut().observe(
            live_scope.as_ref(),
            live_scope_generation,
            workspace(),
            live_key_id.as_deref(),
        );
        let guard = edits.read();
        guard
            .requests
            .iter()
            .filter(|key| guard.is_pending(key))
            .filter_map(|key| key.request.as_deref())
            .filter(|request| {
                request_owner_is_current(
                    request,
                    editor_instance_id,
                    live_scope.as_ref(),
                    live_scope_generation,
                    owner_generation,
                    live_key_id.as_deref(),
                    workspace(),
                )
            })
            .max_by_key(|request| request.request_id)
            .map(|request| KeycapsEditFeedback {
                editor_instance_id: request.editor_instance_id,
                scope: request.scope.clone(),
                scope_generation: request.scope_generation,
                selection_generation: request.selection_generation,
                target: request.target.clone(),
                field: request.change.field(),
            })
    };

    let on_change = use_callback({
        let runtime = runtime.clone();
        let source = active_source.clone();
        let owner_tracker = owner_tracker.clone();
        let mut edits = edits;
        let mut retry_drafts = retry_drafts;
        let mut last_request_id = last_request_id;
        move |request: KeycapsEditRequest| {
            if request.request_id <= last_request_id() {
                return;
            }
            last_request_id.set(request.request_id);
            let live_key_id = live_selected_key_id(&runtime);
            let live_scope = runtime.scope();
            let live_scope_generation = scope_generation();
            let live_workspace = workspace();
            let live_selection_generation = owner_tracker.borrow_mut().observe(
                live_scope.as_ref(),
                live_scope_generation,
                live_workspace,
                live_key_id.as_deref(),
            );
            if !request_owner_is_current(
                &request,
                editor_instance_id,
                live_scope.as_ref(),
                live_scope_generation,
                live_selection_generation,
                live_key_id.as_deref(),
                live_workspace,
            ) {
                return;
            }
            let Some(snapshot) = current_saved_snapshot(
                &runtime,
                source.as_ref(),
                &request,
                live_workspace,
                live_selection_generation,
            ) else {
                return;
            };
            if !valid_edit_target(&snapshot, &request.scope, &request.target) {
                return;
            }
            if edit_operation(
                &snapshot.document,
                &request.scope,
                &request.target,
                &request.change,
            )
            .is_none()
            {
                return;
            }
            remove_retry_draft_through(
                &mut retry_drafts,
                &request.target,
                &request.change.field(),
                request.request_id,
            );
            if edits.peek().owners_changed(
                live_scope.as_ref(),
                live_scope_generation,
                live_workspace,
                live_selection_generation,
            ) {
                let mut guard = edits.write();
                guard.follow_owner(
                    live_scope.as_ref(),
                    live_scope_generation,
                    live_workspace,
                    live_selection_generation,
                );
                guard.prune();
                guard.bind_legend(live_key_id.as_deref(), legend_draft, legend_failure);
            }
            submit_keycap_edit(&runtime, &mut edits, request);
        }
    });

    let on_retry = use_callback({
        let mut request_sequence = request_sequence;
        let retry_drafts = retry_drafts;
        let source = active_source.clone();
        let owner_tracker = owner_tracker.clone();
        let runtime = runtime.clone();
        move |identity: KeycapsRetryIdentity| {
            let Some(draft) = retry_drafts
                .read()
                .get(&(identity.target.clone(), identity.field.clone()))
                .cloned()
            else {
                return;
            };
            if !retry_identity_matches(&draft, &identity) {
                return;
            }
            let live_key_id = live_selected_key_id(&runtime);
            let live_scope = runtime.scope();
            let live_scope_generation = scope_generation();
            let live_workspace = workspace();
            let live_selection_generation = owner_tracker.borrow_mut().observe(
                live_scope.as_ref(),
                live_scope_generation,
                live_workspace,
                live_key_id.as_deref(),
            );
            let (Some(scope), Some(source)) = (live_scope, source.as_ref()) else {
                return;
            };
            if live_workspace != "Keycaps"
                || source.scope != scope
                || draft_owner_mismatch(
                    &draft,
                    editor_instance_id,
                    &scope,
                    live_scope_generation,
                    live_selection_generation,
                    live_key_id.as_deref(),
                    live_workspace,
                )
            {
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted else {
                return;
            };
            if !valid_edit_target(&snapshot, &scope, &draft.target) {
                return;
            }
            let Some(request_id) = request_sequence().checked_add(1) else {
                return;
            };
            request_sequence.set(request_id);
            let baseline = baseline_for_target(&snapshot.document, &scope, &draft.target);
            on_change.call(KeycapsEditRequest {
                scope: scope.clone(),
                scope_generation: live_scope_generation,
                selection_generation: live_selection_generation,
                editor_instance_id,
                request_id,
                admission_token: snapshot.token,
                admission_revision: snapshot.document.revision,
                target: draft.target.clone(),
                baseline,
                change: draft.change,
            });
        }
    });

    let on_discard = use_callback({
        let mut retry_drafts = retry_drafts;
        let mut legend_failure = legend_failure;
        let runtime = runtime.clone();
        let owner_tracker = owner_tracker.clone();
        move |identity: KeycapsRetryIdentity| {
            let live_key_id = live_selected_key_id(&runtime);
            let live_scope = runtime.scope();
            let live_scope_generation = scope_generation();
            let live_selection_generation = owner_tracker.borrow_mut().observe(
                live_scope.as_ref(),
                live_scope_generation,
                workspace(),
                live_key_id.as_deref(),
            );
            let Some(scope) = live_scope else {
                return;
            };
            if !retry_identity_is_current(
                &identity,
                editor_instance_id,
                &scope,
                live_scope_generation,
                live_selection_generation,
                live_key_id.as_deref(),
                workspace(),
            ) {
                return;
            }
            let mut drafts = retry_drafts.write();
            if drafts
                .get(&(identity.target.clone(), identity.field.clone()))
                .is_some_and(|draft| retry_identity_matches(draft, &identity))
            {
                drafts.remove(&(identity.target.clone(), identity.field.clone()));
            }
            drop(drafts);
            if identity.field == KeycapEditField::Legend {
                legend_failure.set(None);
            }
        }
    });

    let source = source.filter(|source| {
        current_workspace == "Keycaps" && current_scope.as_ref() == Some(&source.scope)
    })?;
    let drafts: Vec<_> = retry_drafts.read().values().cloned().collect();
    Some(KeycapsSettingsActions {
        editor_instance_id,
        scope: source.scope,
        scope_generation: current_scope_generation,
        selection_generation,
        token: source.token,
        revision: source.revision,
        selected_key_id: selected_key_id.unwrap_or_default(),
        accepted_settings,
        request_sequence,
        feedback: visible_feedback,
        retry_drafts: Rc::from(drafts),
        on_change,
        on_retry,
        on_discard,
    })
}

fn live_selected_key_id(runtime: &Runtime) -> Option<String> {
    runtime.model().selected_part_ids.first().cloned()
}

fn settings_for_key(document: &ProjectDoc, key_id: &str) -> KeycapKeySettings {
    document
        .keycaps
        .as_ref()
        .and_then(|keycaps| keycaps.keys.get(key_id))
        .cloned()
        .unwrap_or_default()
}

fn baseline_for_target(
    document: &ProjectDoc,
    scope: &Scope,
    target: &KeycapsEditTarget,
) -> KeycapsEditBaseline {
    match target {
        KeycapsEditTarget::Board => KeycapsEditBaseline::Board(
            document
                .keycaps
                .as_ref()
                .and_then(|k| k.boards.get(&scope.board_id))
                .cloned()
                .unwrap_or_default(),
        ),
        KeycapsEditTarget::Matrix(id) => KeycapsEditBaseline::Matrix(
            document
                .keycaps
                .as_ref()
                .and_then(|k| k.matrices.get(id))
                .cloned()
                .unwrap_or_default(),
        ),
        KeycapsEditTarget::Key(id) => KeycapsEditBaseline::Key(settings_for_key(document, id)),
    }
}

fn target_id(target: &KeycapsEditTarget, scope: &Scope) -> String {
    match target {
        KeycapsEditTarget::Board => scope.board_id.clone(),
        KeycapsEditTarget::Matrix(id) | KeycapsEditTarget::Key(id) => id.clone(),
    }
}

fn request_owner_is_current(
    request: &KeycapsEditRequest,
    editor_instance_id: u64,
    scope: Option<&Scope>,
    scope_generation: u64,
    selection_generation: u64,
    selected_key_id: Option<&str>,
    workspace: &'static str,
) -> bool {
    let target_is_current = match &request.target {
        KeycapsEditTarget::Board => {
            request.scope.board_id == scope.map_or("", |s| s.board_id.as_str())
        }
        KeycapsEditTarget::Matrix(_) => {
            request.scope.board_id == scope.map_or("", |s| s.board_id.as_str())
        }
        KeycapsEditTarget::Key(key_id) => {
            request.selection_generation == selection_generation
                && selected_key_id == Some(key_id.as_str())
        }
    };
    workspace == "Keycaps"
        && request.editor_instance_id == editor_instance_id
        && scope == Some(&request.scope)
        && request.scope_generation == scope_generation
        && target_is_current
}

fn current_saved_snapshot(
    runtime: &Runtime,
    expected_source: Option<&KeycapsEditSource>,
    request: &KeycapsEditRequest,
    workspace: &'static str,
    selection_generation: u64,
) -> Option<AcceptedSnapshot> {
    if workspace != "Keycaps"
        || (matches!(request.target, KeycapsEditTarget::Key(_))
            && request.selection_generation != selection_generation)
    {
        return None;
    }
    let expected = expected_source?;
    let model = runtime.model();
    let snapshot = model.accepted?;
    (runtime.scope().as_ref() == Some(&request.scope)
        && request.scope == expected.scope
        && request.scope.session_epoch == snapshot.session_epoch
        && request.scope.document_id == snapshot.document.id
        && request.scope.board_id == model.active_board_id
        && request.scope.instance_id == model.active_instance_id
        && snapshot.token == request.admission_token
        && snapshot.document.revision == request.admission_revision
        && snapshot.token == expected.token
        && snapshot.document.revision == expected.revision
        && matches!(
            model.lifecycle,
            Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
        )
        && matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
        && model.display_preview.is_none()
        && model.gesture.is_none())
    .then_some(snapshot)
}

fn valid_key_member(snapshot: &AcceptedSnapshot, scope: &Scope, key_id: &str) -> bool {
    keycaps_scene::project(snapshot, scope, &scope.board_id)
        .is_some_and(|view| view.keys.iter().any(|key| key.id.as_ref() == key_id))
}

fn valid_edit_target(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
    target: &KeycapsEditTarget,
) -> bool {
    match target {
        KeycapsEditTarget::Board => snapshot
            .document
            .boards
            .iter()
            .any(|b| b.id == scope.board_id),
        KeycapsEditTarget::Matrix(matrix_id) => {
            keycaps_scene::project(snapshot, scope, &scope.board_id).is_some_and(|view| {
                view.matrices
                    .iter()
                    .any(|matrix| matrix.id.as_ref() == matrix_id)
            })
        }
        KeycapsEditTarget::Key(key_id) => valid_key_member(snapshot, scope, key_id),
    }
}

fn edit_operation(
    document: &ProjectDoc,
    scope: &Scope,
    target: &KeycapsEditTarget,
    change: &KeycapEditChange,
) -> Option<EditOperation> {
    match (target, change) {
        (KeycapsEditTarget::Board, KeycapEditChange::BoardColor(value)) => {
            Some(EditOperation::SetKeycapBoard {
                board_id: scope.board_id.clone(),
                change: KeycapBoardChange::Color {
                    value: value.clone(),
                },
            })
        }
        (KeycapsEditTarget::Board, KeycapEditChange::BoardLegendColor(value)) => {
            Some(EditOperation::SetKeycapBoard {
                board_id: scope.board_id.clone(),
                change: KeycapBoardChange::LegendColor {
                    value: value.clone(),
                },
            })
        }
        (KeycapsEditTarget::Board, KeycapEditChange::BoardClearance(value)) => {
            Some(EditOperation::SetKeycapBoard {
                board_id: scope.board_id.clone(),
                change: KeycapBoardChange::Clearance { value: *value },
            })
        }
        (KeycapsEditTarget::Matrix(matrix_id), KeycapEditChange::MatrixProfile(value)) => {
            Some(EditOperation::SetMatrixKeycaps {
                matrix_id: matrix_id.clone(),
                change: KeycapMatrixChange::Profile { value: *value },
            })
        }
        (KeycapsEditTarget::Matrix(matrix_id), KeycapEditChange::MatrixMount(value)) => {
            Some(EditOperation::SetMatrixKeycaps {
                matrix_id: matrix_id.clone(),
                change: KeycapMatrixChange::Mount { value: *value },
            })
        }
        (KeycapsEditTarget::Matrix(matrix_id), KeycapEditChange::MatrixFirstRow(value)) => {
            Some(EditOperation::SetMatrixKeycaps {
                matrix_id: matrix_id.clone(),
                change: KeycapMatrixChange::FirstRow { value: *value },
            })
        }
        (KeycapsEditTarget::Matrix(matrix_id), KeycapEditChange::MatrixWallThickness(value)) => {
            Some(EditOperation::SetMatrixKeycaps {
                matrix_id: matrix_id.clone(),
                change: KeycapMatrixChange::WallThickness { value: *value },
            })
        }
        (KeycapsEditTarget::Key(key_id), change) => {
            let current = settings_for_key(document, key_id);
            let change = match change {
                KeycapEditChange::Legend(value) => KeycapKeyChange::Legend {
                    value: value.clone(),
                },
                KeycapEditChange::Color(value) => KeycapKeyChange::Color {
                    value: value.clone(),
                },
                KeycapEditChange::Profile(value) => KeycapKeyChange::Profile { value: *value },
                KeycapEditChange::Mount(value) => KeycapKeyChange::Mount { value: *value },
                KeycapEditChange::Row(value) => KeycapKeyChange::Row { value: *value },
                KeycapEditChange::UnitsWidth(value) => KeycapKeyChange::Units {
                    value: Some(Vec2 {
                        x: *value,
                        y: current.units.map_or(1.0, |units| units.y),
                    }),
                },
                KeycapEditChange::UnitsDepth(value) => KeycapKeyChange::Units {
                    value: Some(Vec2 {
                        x: current.units.map_or(1.0, |units| units.x),
                        y: *value,
                    }),
                },
                KeycapEditChange::ClearUnits(_) => KeycapKeyChange::Units { value: None },
                _ => return None,
            };
            Some(EditOperation::SetKeycapKey {
                key_id: key_id.clone(),
                change,
            })
        }
        _ => None,
    }
}

fn change_is_noop(
    document: &ProjectDoc,
    scope: &Scope,
    target: &KeycapsEditTarget,
    change: &KeycapEditChange,
) -> bool {
    match (target, change) {
        (KeycapsEditTarget::Board, KeycapEditChange::BoardColor(value)) => {
            document
                .keycaps
                .as_ref()
                .and_then(|k| k.boards.get(&scope.board_id))
                .cloned()
                .unwrap_or_default()
                .color
                == *value
        }
        (KeycapsEditTarget::Board, KeycapEditChange::BoardLegendColor(value)) => {
            document
                .keycaps
                .as_ref()
                .and_then(|k| k.boards.get(&scope.board_id))
                .cloned()
                .unwrap_or_default()
                .legend_color
                == *value
        }
        (KeycapsEditTarget::Board, KeycapEditChange::BoardClearance(value)) => {
            document
                .keycaps
                .as_ref()
                .and_then(|k| k.boards.get(&scope.board_id))
                .cloned()
                .unwrap_or_default()
                .clearance
                == *value
        }
        (KeycapsEditTarget::Matrix(id), KeycapEditChange::MatrixProfile(value)) => {
            document
                .keycaps
                .as_ref()
                .and_then(|k| k.matrices.get(id))
                .cloned()
                .unwrap_or_default()
                .profile
                == *value
        }
        (KeycapsEditTarget::Matrix(id), KeycapEditChange::MatrixMount(value)) => {
            document
                .keycaps
                .as_ref()
                .and_then(|k| k.matrices.get(id))
                .cloned()
                .unwrap_or_default()
                .mount
                == *value
        }
        (KeycapsEditTarget::Matrix(id), KeycapEditChange::MatrixFirstRow(value)) => {
            document
                .keycaps
                .as_ref()
                .and_then(|k| k.matrices.get(id))
                .cloned()
                .unwrap_or_default()
                .first_row
                == *value
        }
        (KeycapsEditTarget::Matrix(id), KeycapEditChange::MatrixWallThickness(value)) => {
            document
                .keycaps
                .as_ref()
                .and_then(|k| k.matrices.get(id))
                .cloned()
                .unwrap_or_default()
                .wall_thickness
                == *value
        }
        (KeycapsEditTarget::Key(id), change) => {
            let current = settings_for_key(document, id);
            match change {
                KeycapEditChange::Legend(value) => current.legend == *value,
                KeycapEditChange::Color(value) => current.color == *value,
                KeycapEditChange::Profile(value) => current.profile == *value,
                KeycapEditChange::Mount(value) => current.mount == *value,
                KeycapEditChange::Row(value) => current.row == *value,
                KeycapEditChange::UnitsWidth(value) => {
                    current.units.is_some_and(|units| units.x == *value)
                }
                KeycapEditChange::UnitsDepth(value) => {
                    current.units.is_some_and(|units| units.y == *value)
                }
                KeycapEditChange::ClearUnits(_) => current.units.is_none(),
                _ => false,
            }
        }
        _ => false,
    }
}

fn keep_retry_draft(
    drafts: &mut Signal<BTreeMap<(KeycapsEditTarget, KeycapEditField), KeycapsRetryDraft>>,
    request: &KeycapsEditRequest,
    message: String,
) {
    let mut values = drafts.write();
    store_retry_draft(&mut values, request, message);
}

fn store_retry_draft(
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

fn remove_retry_draft_through(
    drafts: &mut Signal<BTreeMap<(KeycapsEditTarget, KeycapEditField), KeycapsRetryDraft>>,
    target: &KeycapsEditTarget,
    field: &KeycapEditField,
    request_id: u64,
) {
    let mut values = drafts.write();
    remove_retry_draft_through_map(&mut values, target, field, request_id);
}

fn remove_retry_draft_through_map(
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

fn draft_owner_mismatch(
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

fn retry_identity(draft: &KeycapsRetryDraft) -> KeycapsRetryIdentity {
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

fn retry_identity_matches(draft: &KeycapsRetryDraft, identity: &KeycapsRetryIdentity) -> bool {
    retry_identity(draft) == *identity
}

fn retry_identity_is_current(
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

fn submit_keycap_edit(
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
    let mut guard = edits.write();
    guard.helper(&key.target).begin_field(
        runtime,
        key.clone(),
        "keycaps",
        Some("keycap settings".into()),
        resolver,
        &submitted,
    );
    guard.requests.retain(|existing| *existing != key);
    guard.requests.push(key);
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectedKeySettings {
    pub key: KeycapsKey,
    pub settings: KeycapKeySettings,
    pub board_color: String,
}

pub fn project_selected_key(
    document: &ProjectDoc,
    view: &KeycapsView,
    selected_key_id: Option<&str>,
) -> Option<SelectedKeySettings> {
    let key_id = selected_key_id?;
    let key = view
        .keys
        .iter()
        .find(|key| key.id.as_ref() == key_id)?
        .clone();
    let settings = document
        .keycaps
        .as_ref()
        .and_then(|keycaps| keycaps.keys.get(key_id))
        .cloned()
        .unwrap_or_default();
    let board_color = document
        .keycaps
        .as_ref()
        .and_then(|keycaps| keycaps.boards.get(view.board_id.as_ref()))
        .map_or_else(
            || boardstudio_core::model::KeycapBoardSettings::default().color,
            |settings| settings.color.clone(),
        );
    Some(SelectedKeySettings {
        key,
        settings,
        board_color,
    })
}

fn emit_settings_change(
    actions: &KeycapsSettingsActions,
    target: KeycapsEditTarget,
    baseline: KeycapsEditBaseline,
    change: KeycapEditChange,
) {
    let mut sequence = actions.request_sequence;
    let Some(request_id) = sequence().checked_add(1) else {
        return;
    };
    sequence.set(request_id);
    actions.on_change.call(KeycapsEditRequest {
        scope: actions.scope.clone(),
        scope_generation: actions.scope_generation,
        selection_generation: actions.selection_generation,
        editor_instance_id: actions.editor_instance_id,
        request_id,
        admission_token: actions.token,
        admission_revision: actions.revision,
        target,
        baseline,
        change,
    });
}

#[derive(Props, Clone, PartialEq)]
pub struct KeycapsBoardSettingsEditorProps {
    pub settings: KeycapBoardSettings,
    pub actions: KeycapsSettingsActions,
}

#[component]
pub fn KeycapsBoardSettingsEditor(props: KeycapsBoardSettingsEditorProps) -> Element {
    let mut settings = props.settings;
    let actions = props.actions;
    let target = KeycapsEditTarget::Board;
    let baseline = KeycapsEditBaseline::Board(settings.clone());
    for change in pending_changes(&actions, &target) {
        match change {
            KeycapEditChange::BoardColor(value) => settings.color = value,
            KeycapEditChange::BoardLegendColor(value) => settings.legend_color = value,
            KeycapEditChange::BoardClearance(value) => settings.clearance = value,
            _ => {}
        }
    }
    let feedback = actions.feedback.as_ref().filter(|f| f.target == target);
    let retries = actions
        .retry_drafts
        .iter()
        .filter(|draft| draft.target == target)
        .cloned()
        .collect::<Vec<_>>();
    let color_actions = actions.clone();
    let color_target = target.clone();
    let color_baseline = baseline.clone();
    let legend_actions = actions.clone();
    let legend_target = target.clone();
    let legend_baseline = baseline.clone();
    let clearance_actions = actions.clone();
    let clearance_target = target.clone();
    let clearance_baseline = baseline.clone();
    rsx! {
        details { class: "m1-keycaps-disclosure", open: true,
            summary { "Board colors" }
            section { class: "m1-keycaps-settings", aria_label: "Board colors",
                label { "Keycap color",
                    input { r#type: "color", aria_label: "Board keycap color", value: "{settings.color}",
                        oninput: move |event| emit_settings_change(&color_actions, color_target.clone(), color_baseline.clone(), KeycapEditChange::BoardColor(event.value())) }
                }
                label { "Legend color",
                    input { r#type: "color", aria_label: "Board legend color", value: "{settings.legend_color}",
                        oninput: move |event| emit_settings_change(&legend_actions, legend_target.clone(), legend_baseline.clone(), KeycapEditChange::BoardLegendColor(event.value())) }
                }
                label { "Minimum clearance (mm)",
                    input { r#type: "number", aria_label: "Keycap clearance", min: "0", max: "5", step: "0.1", value: "{settings.clearance}",
                        oninput: move |event| if let Ok(value) = event.value().parse::<f64>() { emit_settings_change(&clearance_actions, clearance_target.clone(), clearance_baseline.clone(), KeycapEditChange::BoardClearance(value)); } }
                }
                {settings_feedback(feedback, retries, &actions)}
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct KeycapsMatrixSettingsEditorProps {
    pub matrix_id: String,
    pub matrix_name: String,
    pub settings: KeycapMatrixSettings,
    pub actions: KeycapsSettingsActions,
}

#[component]
pub fn KeycapsMatrixSettingsEditor(props: KeycapsMatrixSettingsEditorProps) -> Element {
    let target = KeycapsEditTarget::Matrix(props.matrix_id);
    let mut settings = props.settings;
    let actions = props.actions;
    let baseline = KeycapsEditBaseline::Matrix(settings.clone());
    for change in pending_changes(&actions, &target) {
        match change {
            KeycapEditChange::MatrixProfile(value) => settings.profile = value,
            KeycapEditChange::MatrixMount(value) => settings.mount = value,
            KeycapEditChange::MatrixFirstRow(value) => settings.first_row = value,
            KeycapEditChange::MatrixWallThickness(value) => settings.wall_thickness = value,
            _ => {}
        }
    }
    let feedback = actions.feedback.as_ref().filter(|f| f.target == target);
    let retries = actions
        .retry_drafts
        .iter()
        .filter(|draft| draft.target == target)
        .cloned()
        .collect::<Vec<_>>();
    let profile = settings.profile.map_or("", profile_value);
    let mount = settings.mount.map_or("", mount_value);
    let profile_actions = actions.clone();
    let profile_target = target.clone();
    let profile_baseline = baseline.clone();
    let row_actions = actions.clone();
    let row_target = target.clone();
    let row_baseline = baseline.clone();
    let wall_actions = actions.clone();
    let wall_target = target.clone();
    let wall_baseline = baseline.clone();
    let mount_actions = actions.clone();
    let mount_target = target.clone();
    let mount_baseline = baseline.clone();
    rsx! {
        fieldset { class: "m1-keycaps-matrix-settings",
            legend { "{props.matrix_name}" }
            label { "Profile",
                select { aria_label: "Keycap profile for {props.matrix_name}", value: "{profile}",
                    onchange: move |event| emit_settings_change(&profile_actions, profile_target.clone(), profile_baseline.clone(), KeycapEditChange::MatrixProfile(parse_profile(&event.value()))),
                    option { value: "", "No generated keycap" }
                    option { value: "cherry", "Cherry" } option { value: "oem", "OEM" }
                    option { value: "dcs", "DCS" } option { value: "dsa", "DSA" }
                    option { value: "sa", "SA" } option { value: "hi-pro", "Hi-Pro" }
                    option { value: "g20", "G20" } option { value: "choc", "Choc" }
                }
            }
            details {
                summary { "Profile dimensions & socket" }
                label { "First profile row",
                    input { r#type: "number", aria_label: "First profile row for {props.matrix_name}", min: "1", max: "5", step: "1", value: "{settings.first_row}",
                        oninput: move |event| if let Ok(value) = event.value().parse::<u8>() { emit_settings_change(&row_actions, row_target.clone(), row_baseline.clone(), KeycapEditChange::MatrixFirstRow(value)); } }
                }
                label { "Wall thickness (mm)",
                    input { r#type: "number", aria_label: "Wall thickness for {props.matrix_name}", min: "0.8", max: "2", step: "0.1", value: "{settings.wall_thickness}",
                        oninput: move |event| if let Ok(value) = event.value().parse::<f64>() { emit_settings_change(&wall_actions, wall_target.clone(), wall_baseline.clone(), KeycapEditChange::MatrixWallThickness(value)); } }
                }
                label { "Socket",
                    select { aria_label: "Keycap socket for {props.matrix_name}", value: "{mount}",
                        onchange: move |event| emit_settings_change(&mount_actions, mount_target.clone(), mount_baseline.clone(), KeycapEditChange::MatrixMount(parse_mount(&event.value()))),
                        option { value: "", "From switch profile" } option { value: "mx", "MX cross" }
                        option { value: "choc-v1", "Choc v1" } option { value: "choc-v2", "Choc v2 cross" } option { value: "alps", "Alps" }
                    }
                }
            }
            {settings_feedback(feedback, retries, &actions)}
        }
    }
}

fn settings_feedback(
    feedback: Option<&KeycapsEditFeedback>,
    retries: Vec<KeycapsRetryDraft>,
    actions: &KeycapsSettingsActions,
) -> Element {
    let on_retry = actions.on_retry;
    let on_discard = actions.on_discard;
    rsx! {
        if feedback.is_some() {
            p { role: "status", "Saving keycap settings…" }
        }
        for draft in retries {
            div { class: "m1-keycaps-retry-draft",
                p { role: "status", "Unaccepted {field_label(&draft.field)} change: {change_label(&draft.change)}. {draft.message}" }
                button { onclick: { let id = retry_identity(&draft); move |_| on_retry.call(id.clone()) }, "Retry" }
                button { onclick: { let id = retry_identity(&draft); move |_| on_discard.call(id.clone()) }, "Discard" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct KeycapsSettingsEditorProps {
    pub selected: SelectedKeySettings,
    pub actions: KeycapsSettingsActions,
}

#[component]
pub fn KeycapsSettingsEditor(props: KeycapsSettingsEditorProps) -> Element {
    let selected = props.selected;
    let actions = props.actions;
    let mut settings = selected.settings;
    let key = selected.key;
    for change in pending_changes(&actions, &KeycapsEditTarget::Key(key.id.to_string())) {
        match change {
            KeycapEditChange::Color(value) => settings.color = value,
            KeycapEditChange::Profile(value) => settings.profile = value,
            KeycapEditChange::Mount(value) => settings.mount = value,
            KeycapEditChange::Row(value) => settings.row = value,
            KeycapEditChange::UnitsWidth(value) => {
                settings.units.get_or_insert(Vec2 { x: 1.0, y: 1.0 }).x = value
            }
            KeycapEditChange::UnitsDepth(value) => {
                settings.units.get_or_insert(Vec2 { x: 1.0, y: 1.0 }).y = value
            }
            KeycapEditChange::ClearUnits(_) => settings.units = None,
            _ => {}
        }
    }
    let accepted_legend = settings.legend.clone();
    let accepted_color = settings.color.clone();
    let accepted_profile = settings.profile;
    let accepted_mount = settings.mount;
    let accepted_row = settings.row;
    let accepted_units = settings.units;
    // The controller provides the legend Signals; an editor mounted without one keeps its own.
    let own_draft = use_signal(String::new);
    let own_failure = use_signal(|| None::<String>);
    let edits_context = try_consume_context::<KeycapsEditsContext>();
    let mut legend_draft = edits_context.map_or(own_draft, |context| context.legend_draft);
    let mut legend_failure = edits_context.map_or(own_failure, |context| context.legend_failure);
    let mut legend_dirty = use_signal(|| false);
    let legend_pending = field_pending(
        &actions,
        &KeycapsEditTarget::Key(key.id.to_string()),
        KeycapEditField::Legend,
    );
    let legend_owner = (
        actions.editor_instance_id,
        actions.scope.clone(),
        actions.scope_generation,
        actions.selection_generation,
        key.id.to_string(),
    );
    use_effect(use_reactive((&legend_owner,), {
        let mut legend_draft = legend_draft;
        let legend = accepted_legend.clone().unwrap_or_default();
        move |_| {
            legend_dirty.set(false);
            legend_draft.set(legend.clone());
        }
    }));
    use_effect(use_reactive(
        (&accepted_legend, &legend_pending),
        move |(accepted, pending)| {
            if !legend_dirty() && !pending {
                legend_draft.set(accepted.unwrap_or_default());
            }
        },
    ));
    let request = |change: KeycapEditChange,
                   request_sequence: &mut Signal<u64>,
                   on_change: EventHandler<KeycapsEditRequest>,
                   actions: &KeycapsSettingsActions| {
        let Some(request_id) = request_sequence().checked_add(1) else {
            return;
        };
        request_sequence.set(request_id);
        on_change.call(KeycapsEditRequest {
            scope: actions.scope.clone(),
            scope_generation: actions.scope_generation,
            selection_generation: actions.selection_generation,
            editor_instance_id: actions.editor_instance_id,
            request_id,
            admission_token: actions.token,
            admission_revision: actions.revision,
            target: KeycapsEditTarget::Key(actions.selected_key_id.clone()),
            baseline: KeycapsEditBaseline::Key(actions.accepted_settings.clone()),
            change,
        });
    };
    let profile = accepted_profile.map_or("", profile_value);
    let mount = accepted_mount.map_or("", mount_value);
    let width = accepted_units.map_or_else(String::new, |units| units.x.to_string());
    let depth = accepted_units.map_or_else(String::new, |units| units.y.to_string());
    let feedback = actions.feedback.as_ref().filter(|feedback| {
        feedback.target == KeycapsEditTarget::Key(key.id.to_string())
            && feedback.selection_generation == actions.selection_generation
    });
    let retry_drafts = actions.retry_drafts.clone();
    let legend_retry_pending = retry_drafts.iter().any(|draft| {
        draft.field == KeycapEditField::Legend
            && draft.editor_instance_id == actions.editor_instance_id
            && draft.scope == actions.scope
            && draft.scope_generation == actions.scope_generation
            && draft.selection_generation == actions.selection_generation
            && draft.target == KeycapsEditTarget::Key(actions.selected_key_id.clone())
    });
    let on_retry = actions.on_retry;
    let on_discard = actions.on_discard;
    let mut legend_sequence = actions.request_sequence;
    let mut inherit_sequence = actions.request_sequence;
    let mut blank_sequence = actions.request_sequence;
    let mut color_sequence = actions.request_sequence;
    let mut reset_color_sequence = actions.request_sequence;
    let mut profile_sequence = actions.request_sequence;
    let mut mount_sequence = actions.request_sequence;
    let mut row_sequence = actions.request_sequence;
    let mut width_sequence = actions.request_sequence;
    let mut depth_sequence = actions.request_sequence;
    let legend_actions = actions.clone();
    let inherit_actions = actions.clone();
    let blank_actions = actions.clone();
    let color_actions = actions.clone();
    let reset_color_actions = actions.clone();
    let profile_actions = actions.clone();
    let mount_actions = actions.clone();
    let row_actions = actions.clone();
    let width_actions = actions.clone();
    let depth_actions = actions.clone();
    let legend_change = actions.on_change;
    let inherit_change = actions.on_change;
    let blank_change = actions.on_change;
    let color_change = actions.on_change;
    let reset_color_change = actions.on_change;
    let profile_change = actions.on_change;
    let mount_change = actions.on_change;
    let row_change = actions.on_change;
    let width_change = actions.on_change;
    let depth_change = actions.on_change;
    rsx! {
        section { class: "m1-keycaps-settings", aria_label: "Keycap overrides for {key.reference}",
            label { "Legend",
                input {
                    aria_label: "Legend for {key.reference}",
                    maxlength: "12",
                    value: "{legend_draft()}",
                    placeholder: if key.binding_label.is_empty() { "From binding" } else { key.binding_label.as_ref() },
                    oninput: move |event| { legend_dirty.set(true); legend_failure.set(None); legend_draft.set(event.value()); },
                    onblur: move |_| {
                        let value = legend_draft();
                        if legend_dirty() || legend_blur_requires_reconciliation(&value, accepted_legend.as_deref(), legend_retry_pending) {
                            legend_dirty.set(false);
                            let change = if legend_change_required(&value, accepted_legend.as_deref()) {
                                KeycapEditChange::Legend(Some(value))
                            } else {
                                KeycapEditChange::Legend(accepted_legend.clone())
                            };
                            request(change, &mut legend_sequence, legend_change, &legend_actions);
                        }
                    },
                }
            }
            div { class: "m1-keycaps-settings-actions",
                button { onclick: move |_| { legend_draft.set(String::new()); request(KeycapEditChange::Legend(None), &mut inherit_sequence, inherit_change, &inherit_actions) }, "Use binding legend" }
                button { onclick: move |_| { legend_draft.set(String::new()); request(KeycapEditChange::Legend(Some(String::new())), &mut blank_sequence, blank_change, &blank_actions) }, "Blank keycap" }
            }
            label { "Keycap color",
                input {
                    aria_label: "Keycap color for {key.reference}",
                    r#type: "color",
                    value: "{accepted_color.as_deref().unwrap_or(&selected.board_color)}",
                    oninput: move |event| request(KeycapEditChange::Color(Some(event.value())), &mut color_sequence, color_change, &color_actions),
                }
            }
            if accepted_color.is_some() {
                button { onclick: move |_| request(KeycapEditChange::Color(None), &mut reset_color_sequence, reset_color_change, &reset_color_actions), "Use board color" }
            }
            details {
                summary { "Keycap overrides" }
                label { "Profile",
                    select {
                        aria_label: "Profile override for {key.reference}",
                        value: "{profile}",
                        onchange: move |event| request(KeycapEditChange::Profile(parse_profile(&event.value())), &mut profile_sequence, profile_change, &profile_actions),
                        option { value: "", "From matrix" }
                        option { value: "cherry", "Cherry" }
                        option { value: "oem", "OEM" }
                        option { value: "dcs", "DCS" }
                        option { value: "dsa", "DSA" }
                        option { value: "sa", "SA" }
                        option { value: "hi-pro", "Hi-Pro" }
                        option { value: "g20", "G20" }
                        option { value: "choc", "Choc" }
                    }
                }
                label { "Socket",
                    select {
                        aria_label: "Socket override for {key.reference}",
                        value: "{mount}",
                        onchange: move |event| request(KeycapEditChange::Mount(parse_mount(&event.value())), &mut mount_sequence, mount_change, &mount_actions),
                        option { value: "", "From matrix / switch" }
                        option { value: "mx", "MX cross" }
                        option { value: "choc-v1", "Choc v1" }
                        option { value: "choc-v2", "Choc v2 cross" }
                        option { value: "alps", "Alps" }
                    }
                }
                label { "Profile row",
                    input {
                        aria_label: "Profile row for {key.reference}",
                        r#type: "number",
                        min: "1",
                        max: "5",
                        value: "{accepted_row.map_or_else(String::new, |row| row.to_string())}",
                        placeholder: "From matrix",
                        oninput: move |event| {
                            let value = event.value();
                            if value.is_empty() {
                                request(KeycapEditChange::Row(None), &mut row_sequence, row_change, &row_actions);
                            } else if let Ok(row) = value.parse::<u8>() {
                                request(KeycapEditChange::Row(Some(row)), &mut row_sequence, row_change, &row_actions);
                            }
                        },
                    }
                }
                label { "Width (u)",
                    input {
                        aria_label: "Keycap width for {key.reference}",
                        r#type: "number",
                        min: "0.75",
                        max: "7",
                        step: "0.25",
                        value: "{width}",
                        placeholder: "From envelope",
                        oninput: move |event| {
                            let value = event.value();
                            if value.is_empty() {
                            request(KeycapEditChange::ClearUnits(KeycapEditField::UnitsWidth), &mut width_sequence, width_change, &width_actions);
                            } else if let Ok(width) = value.parse::<f64>() {
                                request(KeycapEditChange::UnitsWidth(width), &mut width_sequence, width_change, &width_actions);
                            }
                        },
                    }
                }
                label { "Depth (u)",
                    input {
                        aria_label: "Keycap depth for {key.reference}",
                        r#type: "number",
                        min: "0.75",
                        max: "7",
                        step: "0.25",
                        value: "{depth}",
                        placeholder: "From envelope",
                        oninput: move |event| {
                            let value = event.value();
                            if value.is_empty() {
                            request(KeycapEditChange::ClearUnits(KeycapEditField::UnitsDepth), &mut depth_sequence, depth_change, &depth_actions);
                            } else if let Ok(depth) = value.parse::<f64>() {
                                request(KeycapEditChange::UnitsDepth(depth), &mut depth_sequence, depth_change, &depth_actions);
                            }
                        },
                    }
                }
            }
            if feedback.is_some() {
                p { role: "status", "Saving keycap override…" }
            }
            if let Some(message) = legend_failure() {
                p { role: "alert", "{message}" }
            }
            for draft in retry_drafts.iter() {
                div { class: "m1-keycaps-retry-draft",
                    p { role: "status", "Unaccepted {field_label(&draft.field)} change: {change_label(&draft.change)}. {draft.message}" }
                    button { onclick: { let identity = retry_identity(draft); move |_| on_retry.call(identity.clone()) }, "Retry" }
                    button { onclick: { let identity = retry_identity(draft); move |_| on_discard.call(identity.clone()) }, "Discard" }
                }
            }
        }
    }
}

fn profile_value(profile: KeycapProfile) -> &'static str {
    match profile {
        KeycapProfile::Cherry => "cherry",
        KeycapProfile::Oem => "oem",
        KeycapProfile::Dcs => "dcs",
        KeycapProfile::Dsa => "dsa",
        KeycapProfile::Sa => "sa",
        KeycapProfile::HiPro => "hi-pro",
        KeycapProfile::G20 => "g20",
        KeycapProfile::Choc => "choc",
    }
}

fn parse_profile(value: &str) -> Option<KeycapProfile> {
    match value {
        "cherry" => Some(KeycapProfile::Cherry),
        "oem" => Some(KeycapProfile::Oem),
        "dcs" => Some(KeycapProfile::Dcs),
        "dsa" => Some(KeycapProfile::Dsa),
        "sa" => Some(KeycapProfile::Sa),
        "hi-pro" => Some(KeycapProfile::HiPro),
        "g20" => Some(KeycapProfile::G20),
        "choc" => Some(KeycapProfile::Choc),
        _ => None,
    }
}

fn mount_value(mount: KeycapMount) -> &'static str {
    match mount {
        KeycapMount::Mx => "mx",
        KeycapMount::ChocV1 => "choc-v1",
        KeycapMount::ChocV2 => "choc-v2",
        KeycapMount::Alps => "alps",
    }
}

fn parse_mount(value: &str) -> Option<KeycapMount> {
    match value {
        "mx" => Some(KeycapMount::Mx),
        "choc-v1" => Some(KeycapMount::ChocV1),
        "choc-v2" => Some(KeycapMount::ChocV2),
        "alps" => Some(KeycapMount::Alps),
        _ => None,
    }
}

fn legend_change_required(draft: &str, accepted: Option<&str>) -> bool {
    draft != accepted.unwrap_or("")
}

fn legend_blur_requires_reconciliation(
    draft: &str,
    accepted: Option<&str>,
    retry_pending: bool,
) -> bool {
    legend_change_required(draft, accepted) || retry_pending
}

fn field_label(field: &KeycapEditField) -> &'static str {
    match field {
        KeycapEditField::BoardColor => "board keycap color",
        KeycapEditField::BoardLegendColor => "board legend color",
        KeycapEditField::BoardClearance => "minimum clearance",
        KeycapEditField::MatrixProfile => "matrix profile",
        KeycapEditField::MatrixMount => "matrix socket",
        KeycapEditField::MatrixFirstRow => "first profile row",
        KeycapEditField::MatrixWallThickness => "wall thickness",
        KeycapEditField::Legend => "legend",
        KeycapEditField::Color => "color",
        KeycapEditField::Profile => "profile",
        KeycapEditField::Mount => "socket",
        KeycapEditField::Row => "profile row",
        KeycapEditField::UnitsWidth => "width",
        KeycapEditField::UnitsDepth => "depth",
    }
}

fn change_label(change: &KeycapEditChange) -> String {
    match change {
        KeycapEditChange::BoardColor(value) => format!("board keycap color {value}"),
        KeycapEditChange::BoardLegendColor(value) => format!("board legend color {value}"),
        KeycapEditChange::BoardClearance(value) => format!("minimum clearance {value} mm"),
        KeycapEditChange::MatrixProfile(value) => value.map_or_else(
            || "no generated keycap".into(),
            |v| format!("profile {}", profile_value(v)),
        ),
        KeycapEditChange::MatrixMount(value) => value.map_or_else(
            || "switch-profile socket".into(),
            |v| format!("socket {}", mount_value(v)),
        ),
        KeycapEditChange::MatrixFirstRow(value) => format!("first profile row {value}"),
        KeycapEditChange::MatrixWallThickness(value) => format!("wall thickness {value} mm"),
        KeycapEditChange::Legend(Some(value)) => {
            if value.is_empty() {
                "blank legend".into()
            } else {
                format!("legend ‘{value}’")
            }
        }
        KeycapEditChange::Legend(None) => "binding legend".into(),
        KeycapEditChange::Color(Some(value)) => format!("color {value}"),
        KeycapEditChange::Color(None) => "board color".into(),
        KeycapEditChange::Profile(Some(value)) => format!("profile {}", profile_value(*value)),
        KeycapEditChange::Profile(None) => "matrix profile".into(),
        KeycapEditChange::Mount(Some(value)) => format!("socket {}", mount_value(*value)),
        KeycapEditChange::Mount(None) => "inherited socket".into(),
        KeycapEditChange::Row(Some(value)) => format!("row {value}"),
        KeycapEditChange::Row(None) => "inherited row".into(),
        KeycapEditChange::UnitsWidth(value) => format!("width {value}u"),
        KeycapEditChange::UnitsDepth(value) => format!("depth {value}u"),
        KeycapEditChange::ClearUnits(_) => "inherited cap size".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn test_scope() -> Scope {
        Scope {
            session_epoch: boardstudio_application::SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
        }
    }

    #[wasm_bindgen_test]
    fn width_and_depth_intents_merge_with_the_latest_accepted_sibling() {
        let scope = test_scope();
        let document = ProjectDoc {
            keycaps: Some(boardstudio_core::model::KeycapConfiguration {
                keys: [(
                    "sw1".into(),
                    KeycapKeySettings {
                        units: Some(Vec2 { x: 2.0, y: 1.5 }),
                        ..KeycapKeySettings::default()
                    },
                )]
                .into(),
                ..boardstudio_core::model::KeycapConfiguration::default()
            }),
            ..ProjectDoc::empty("keycaps-settings-test", "Keycaps settings test")
        };
        assert_eq!(
            edit_operation(
                &document,
                &scope,
                &KeycapsEditTarget::Key("sw1".into()),
                &KeycapEditChange::UnitsWidth(3.0)
            ),
            Some(EditOperation::SetKeycapKey {
                key_id: "sw1".into(),
                change: KeycapKeyChange::Units {
                    value: Some(Vec2 { x: 3.0, y: 1.5 })
                }
            })
        );
        assert_eq!(
            edit_operation(
                &document,
                &scope,
                &KeycapsEditTarget::Key("sw1".into()),
                &KeycapEditChange::UnitsDepth(2.25)
            ),
            Some(EditOperation::SetKeycapKey {
                key_id: "sw1".into(),
                change: KeycapKeyChange::Units {
                    value: Some(Vec2 { x: 2.0, y: 2.25 })
                }
            })
        );
        assert_eq!(
            edit_operation(
                &document,
                &scope,
                &KeycapsEditTarget::Key("sw1".into()),
                &KeycapEditChange::ClearUnits(KeycapEditField::UnitsDepth)
            ),
            Some(EditOperation::SetKeycapKey {
                key_id: "sw1".into(),
                change: KeycapKeyChange::Units { value: None }
            })
        );
    }

    #[wasm_bindgen_test]
    fn setting_noop_preserves_null_and_blank_legend() {
        let scope = test_scope();
        let document = ProjectDoc {
            keycaps: Some(boardstudio_core::model::KeycapConfiguration {
                keys: [(
                    "sw1".into(),
                    KeycapKeySettings {
                        legend: Some(String::new()),
                        ..KeycapKeySettings::default()
                    },
                )]
                .into(),
                ..boardstudio_core::model::KeycapConfiguration::default()
            }),
            ..ProjectDoc::empty("keycaps-settings-test", "Keycaps settings test")
        };
        assert!(change_is_noop(
            &document,
            &scope,
            &KeycapsEditTarget::Key("sw1".into()),
            &KeycapEditChange::Legend(Some(String::new()))
        ));
        assert!(!change_is_noop(
            &document,
            &scope,
            &KeycapsEditTarget::Key("sw1".into()),
            &KeycapEditChange::Legend(None)
        ));
    }

    #[wasm_bindgen_test]
    fn selection_owner_generation_changes_when_a_key_is_reselected() {
        let scope = Scope {
            session_epoch: boardstudio_application::SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let mut tracker = OwnerTracker::default();
        let first = tracker.observe(Some(&scope), 0, "Keycaps", Some("sw1"));
        let other = tracker.observe(Some(&scope), 0, "Keycaps", Some("sw2"));
        let returned = tracker.observe(Some(&scope), 0, "Keycaps", Some("sw1"));
        assert!(first < other && other < returned);
    }

    #[wasm_bindgen_test]
    fn latest_same_owner_retry_intent_survives_older_operation_failure() {
        let scope = Scope {
            session_epoch: boardstudio_application::SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let mut drafts = BTreeMap::new();
        let request = |request_id, width| KeycapsEditRequest {
            scope: scope.clone(),
            scope_generation: 3,
            selection_generation: 8,
            editor_instance_id: 11,
            request_id,
            admission_token: SnapshotToken(9),
            admission_revision: 4,
            target: KeycapsEditTarget::Key("sw1".into()),
            baseline: KeycapsEditBaseline::Key(KeycapKeySettings::default()),
            change: KeycapEditChange::UnitsWidth(width),
        };

        let first = request(1, 2.0);
        let latest = request(2, 3.0);
        store_retry_draft(&mut drafts, &latest, "another edit is pending".into());
        store_retry_draft(&mut drafts, &first, "older edit failed".into());

        let target = KeycapsEditTarget::Key("sw1".into());
        let retained = drafts
            .get(&(target.clone(), KeycapEditField::UnitsWidth))
            .unwrap();
        assert_eq!(retained.request_id, 2);
        assert_eq!(retained.change, KeycapEditChange::UnitsWidth(3.0));
        assert!(!draft_owner_mismatch(
            retained,
            11,
            &scope,
            3,
            8,
            Some("sw1"),
            "Keycaps"
        ));
        assert!(draft_owner_mismatch(
            retained,
            11,
            &scope,
            3,
            9,
            Some("sw1"),
            "Keycaps"
        ));
        let identity = retry_identity(retained);
        assert!(retry_identity_matches(retained, &identity));
        let mut stale_identity = identity.clone();
        stale_identity.request_id = first.request_id;
        assert!(!retry_identity_matches(retained, &stale_identity));
        remove_retry_draft_through_map(
            &mut drafts,
            &target,
            &KeycapEditField::UnitsWidth,
            first.request_id,
        );
        assert_eq!(
            drafts
                .get(&(target.clone(), KeycapEditField::UnitsWidth))
                .map(|draft| draft.request_id),
            Some(latest.request_id)
        );
        remove_retry_draft_through_map(
            &mut drafts,
            &target,
            &KeycapEditField::UnitsWidth,
            latest.request_id,
        );
        assert!(!drafts.contains_key(&(target, KeycapEditField::UnitsWidth)));
    }

    #[wasm_bindgen_test]
    fn untouched_inherited_legend_does_not_materialize_an_explicit_blank() {
        assert!(!legend_change_required("", None));
        assert!(!legend_change_required("", Some("")));
        assert!(legend_change_required("A", None));
        assert!(legend_blur_requires_reconciliation("", None, true));
        assert!(!legend_blur_requires_reconciliation("", None, false));
    }

    #[wasm_bindgen_test]
    fn board_and_matrix_settings_use_existing_core_operations() {
        let scope = test_scope();
        let document = ProjectDoc::empty("keycaps-settings-test", "Keycaps settings test");
        assert_eq!(
            edit_operation(
                &document,
                &scope,
                &KeycapsEditTarget::Board,
                &KeycapEditChange::BoardColor("#abcdef".into())
            ),
            Some(EditOperation::SetKeycapBoard {
                board_id: "board".into(),
                change: KeycapBoardChange::Color {
                    value: "#abcdef".into()
                }
            })
        );
        assert_eq!(
            edit_operation(
                &document,
                &scope,
                &KeycapsEditTarget::Matrix("matrix-a".into()),
                &KeycapEditChange::MatrixProfile(Some(KeycapProfile::Cherry))
            ),
            Some(EditOperation::SetMatrixKeycaps {
                matrix_id: "matrix-a".into(),
                change: KeycapMatrixChange::Profile {
                    value: Some(KeycapProfile::Cherry)
                }
            })
        );
        assert!(change_is_noop(
            &document,
            &scope,
            &KeycapsEditTarget::Board,
            &KeycapEditChange::BoardClearance(KeycapBoardSettings::default().clearance)
        ));
        assert!(change_is_noop(
            &document,
            &scope,
            &KeycapsEditTarget::Matrix("matrix-a".into()),
            &KeycapEditChange::MatrixFirstRow(KeycapMatrixSettings::default().first_row)
        ));
    }
}

#[cfg(test)]
mod queued_settings_tests {
    use super::*;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_application::Event;
    use wasm_bindgen_test::*;

    #[derive(Clone)]
    struct Probe {
        runtime: Rc<Runtime>,
        actions: Rc<RefCell<Option<KeycapsSettingsActions>>>,
        workspace: Rc<RefCell<Option<Signal<&'static str>>>>,
    }

    fn host() -> Element {
        let probe = use_context::<Probe>();
        let runtime = probe.runtime.clone();
        let version = use_signal(|| 0u64);
        use_context_provider(|| version);
        let observed = runtime.clone();
        use_hook(move || {
            observed.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }))
        });
        let _ = version();
        let workspace = use_signal(|| "Keycaps");
        *probe.workspace.borrow_mut() = Some(workspace);
        let generation = use_signal(|| 0u64);
        let accepted = runtime.model().accepted.unwrap();
        let source = runtime.scope().map(|scope| KeycapsEditSource {
            scope,
            token: accepted.token,
            revision: accepted.document.revision,
        });
        let scope = runtime.scope().unwrap();
        let view = keycaps_scene::project(&accepted, &scope, &scope.board_id).unwrap();
        let selected = project_selected_key(&accepted.document, &view, Some("key")).unwrap();
        // Outside the Keycaps workspace the hook offers no actions and nothing renders.
        let actions = use_keycaps_settings_actions(runtime, source, workspace, generation);
        *probe.actions.borrow_mut() = actions.clone();
        let Some(actions) = actions else {
            return rsx! {};
        };
        rsx! { KeycapsSettingsEditor { selected, actions } }
    }

    #[wasm_bindgen_test]
    async fn keycap_dimensions_queue_and_preserve_the_accepted_sibling() {
        let runtime = support::new_runtime();
        let mut doc = ProjectDoc::empty("keycaps-queue", "Keycaps");
        doc.boards.push(serde_json::from_value(serde_json::json!({"id":"board","name":"Board","outlineIds":[],"partIds":["key"],"netIds":[],"thickness":1.6,"traces":[],"vias":[]})).unwrap());
        doc.definitions.push(serde_json::from_value(serde_json::json!({"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]})).unwrap());
        doc.parts.push(serde_json::from_value(serde_json::json!({"id":"key","definitionId":"switch","reference":"SW1","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"})).unwrap());
        support::open_document(&runtime, doc).await;
        runtime.submit(Event::SelectParts {
            operation_id: runtime.operation(),
            part_ids: vec!["key".into()],
            range_part_ids: vec![],
            mode: boardstudio_application::SelectionMode::Replace,
        });
        let probe = Probe {
            runtime: runtime.clone(),
            actions: Rc::new(RefCell::new(None)),
            workspace: Rc::new(RefCell::new(None)),
        };
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(50).await;
        let submit = |change| {
            let borrowed = probe.actions.borrow();
            let actions = borrowed.as_ref().unwrap();
            let mut sequence = actions.request_sequence;
            let request_id = sequence() + 1;
            sequence.set(request_id);
            actions.on_change.call(KeycapsEditRequest {
                scope: actions.scope.clone(),
                scope_generation: actions.scope_generation,
                selection_generation: actions.selection_generation,
                editor_instance_id: actions.editor_instance_id,
                request_id,
                admission_token: actions.token,
                admission_revision: actions.revision,
                target: KeycapsEditTarget::Key("key".into()),
                baseline: KeycapsEditBaseline::Key(actions.accepted_settings.clone()),
                change,
            });
        };
        let (entered, release) = support::gate_next_core_reply(&runtime);
        use wasm_bindgen::JsCast;
        let width = root
            .query_selector("input[aria-label='Keycap width for SW1']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        width.set_value("2");
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        width
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        support::drive_pending(&runtime);
        entered.await.unwrap();
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert_eq!(width.value(), "2");
        submit(KeycapEditChange::UnitsDepth(1.5));
        release.send(()).unwrap();
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            settings_for_key(&runtime.model().accepted.unwrap().document, "key").units,
            Some(Vec2 { x: 2.0, y: 1.5 })
        );
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(
            settings_for_key(&runtime.model().accepted.unwrap().document, "key").units,
            Some(Vec2 { x: 2.0, y: 1.0 })
        );
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(
            settings_for_key(&runtime.model().accepted.unwrap().document, "key").units,
            None
        );
        gloo_timers::future::TimeoutFuture::new(50).await;
        // A return to the old accepted legend is still a newer committed intent.
        let input = root
            .query_selector("input[aria-label='Legend for SW1']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        let commit = |value: &str| {
            input.focus().unwrap();
            input.set_value(value);
            let event = web_sys::EventInit::new();
            event.set_bubbles(true);
            input
                .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
                .unwrap();
            input.blur().unwrap();
        };
        let (entered, release) = support::gate_next_core_reply(&runtime);
        commit("B");
        support::drive_pending(&runtime);
        entered.await.unwrap();
        gloo_timers::future::TimeoutFuture::new(20).await;
        commit("");
        release.send(()).unwrap();
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            settings_for_key(&runtime.model().accepted.unwrap().document, "key").legend,
            None
        );
        assert_eq!(input.value(), "");
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(
            settings_for_key(&runtime.model().accepted.unwrap().document, "key")
                .legend
                .as_deref(),
            Some("B")
        );
        gloo_timers::future::TimeoutFuture::new(50).await;
        let width = root
            .query_selector("input[aria-label='Keycap width for SW1']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        support::fail_next_core_reply(&runtime, "keycap executor failed");
        width.set_value("3");
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        width
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            settings_for_key(&runtime.model().accepted.unwrap().document, "key").units,
            None
        );
        assert_eq!(width.value(), "");
        assert!(
            root.text_content()
                .unwrap()
                .contains("keycap executor failed")
        );
        root.remove();
    }

    async fn mounted_legend(
        id: &str,
    ) -> (
        Rc<Runtime>,
        Probe,
        web_sys::Element,
        web_sys::HtmlInputElement,
    ) {
        use wasm_bindgen::JsCast;
        let runtime = support::new_runtime();
        let mut doc = ProjectDoc::empty(id, "Keycaps");
        doc.boards.push(serde_json::from_value(serde_json::json!({"id":"board","name":"Board","outlineIds":[],"partIds":["key"],"netIds":[],"thickness":1.6,"traces":[],"vias":[]})).unwrap());
        doc.definitions.push(serde_json::from_value(serde_json::json!({"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]})).unwrap());
        doc.parts.push(serde_json::from_value(serde_json::json!({"id":"key","definitionId":"switch","reference":"SW1","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"})).unwrap());
        support::open_document(&runtime, doc).await;
        runtime.submit(Event::SelectParts {
            operation_id: runtime.operation(),
            part_ids: vec!["key".into()],
            range_part_ids: vec![],
            mode: boardstudio_application::SelectionMode::Replace,
        });
        let probe = Probe {
            runtime: runtime.clone(),
            actions: Rc::new(RefCell::new(None)),
            workspace: Rc::new(RefCell::new(None)),
        };
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(50).await;
        let input = root
            .query_selector("input[aria-label='Legend for SW1']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        (runtime, probe, root, input)
    }

    fn type_legend(input: &web_sys::HtmlInputElement, value: &str) {
        input.focus().unwrap();
        input.set_value(value);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
    }

    #[wasm_bindgen_test]
    async fn a_newer_legend_draft_survives_an_older_failure_that_reports_inline() {
        let (runtime, _probe, root, input) = mounted_legend("keycaps-newer-draft").await;
        support::fail_next_core_reply(&runtime, "legend executor failed");
        type_legend(&input, "A");
        input.blur().unwrap();
        // The older edit has not settled when the user types the next draft.
        type_legend(&input, "AB");
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(input.value(), "AB", "a newer draft is never overwritten");
        assert!(
            root.text_content()
                .unwrap()
                .contains("legend executor failed"),
            "the older failure still reports inline"
        );
        assert_eq!(
            settings_for_key(&runtime.model().accepted.unwrap().document, "key").legend,
            None
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_failed_unchanged_legend_restores_the_accepted_text_with_an_inline_failure() {
        let (runtime, _probe, root, input) = mounted_legend("keycaps-unchanged-failure").await;
        support::fail_next_core_reply(&runtime, "legend executor failed");
        type_legend(&input, "A");
        input.blur().unwrap();
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            input.value(),
            "",
            "the unchanged draft restores the accepted text"
        );
        assert!(
            root.text_content()
                .unwrap()
                .contains("legend executor failed"),
            "the failure reports inline"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_second_legend_edit_replaces_the_observation_and_both_edits_undo() {
        let (runtime, _probe, root, input) = mounted_legend("keycaps-latest-per-key").await;
        let legend = |runtime: &Rc<Runtime>| {
            accepted_legend_text(&runtime.model().accepted.unwrap().document, "key")
        };
        let (entered, release) = support::gate_next_core_reply(&runtime);
        type_legend(&input, "A");
        input.blur().unwrap();
        support::drive_pending(&runtime);
        entered.await.unwrap();
        type_legend(&input, "B");
        input.blur().unwrap();
        support::drive_pending(&runtime);
        release.send(()).unwrap();
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(legend(&runtime), "B");
        assert_eq!(input.value(), "B");
        for expected in ["A", ""] {
            runtime.submit(Event::Undo {
                operation_id: runtime.operation(),
            });
            support::run_pending(&runtime).await;
            assert_eq!(legend(&runtime), expected, "each edit has an Undo step");
        }
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn leaving_the_keycaps_workspace_retires_a_pending_edit_silently() {
        let (runtime, probe, root, input) = mounted_legend("keycaps-owner-departs").await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        type_legend(&input, "Gone");
        input.blur().unwrap();
        support::drive_pending(&runtime);
        entered.await.unwrap();
        let mut workspace = probe
            .workspace
            .borrow()
            .expect("host exposes its workspace");
        workspace.set("Layout");
        gloo_timers::future::TimeoutFuture::new(30).await;
        release.send(()).unwrap();
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        workspace.set("Keycaps");
        gloo_timers::future::TimeoutFuture::new(30).await;
        let text = root.text_content().unwrap();
        assert!(
            !text.contains("Unaccepted") && !text.contains("Saving keycap"),
            "a departed owner leaves no status or retry draft: {text}"
        );
        root.remove();
    }
}
