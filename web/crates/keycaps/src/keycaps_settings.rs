//! Editor-owned keycap settings edits and exact-operation feedback.
use super::keycaps_scene::{self, KeycapsKey, KeycapsView};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_model::model::{
    EditOperation, KeycapBoardChange, KeycapBoardSettings, KeycapKeyChange, KeycapKeySettings,
    KeycapMatrixChange, KeycapMatrixSettings, KeycapMount, KeycapProfile, ProjectDoc, Vec2,
};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use boardstudio_web_ui_shared::ui::field::{Field, FieldError, FieldLabel};
use boardstudio_web_ui_shared::ui::input::InputType;
use boardstudio_web_ui_shared::ui::input_group::{InputGroup, InputGroupInput};
use boardstudio_web_ui_shared::ui::select::{Select, SelectOption};
use boardstudio_web_ui_shared::ui::spinner::Spinner;
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
}

impl Default for KeycapsEdits {
    fn default() -> Self {
        Self {
            shared: KeycapsHelper::new(),
            shared_owner: None,
            key: KeycapsHelper::new(),
            key_owner: 0,
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

    /// Replace each helper whose owner lifetime ended.
    fn follow_owner(
        &mut self,
        scope: Option<&Scope>,
        scope_generation: u64,
        workspace: &'static str,
        owner_generation: u64,
    ) {
        let shared_owner = (scope.cloned(), scope_generation, workspace);
        if self.shared_owner.as_ref() != Some(&shared_owner) {
            if self.shared_owner.is_some() {
                self.shared = KeycapsHelper::new();
            }
            self.shared_owner = Some(shared_owner);
        }
        if self.key_owner != owner_generation {
            if self.key_owner != 0 {
                self.key = KeycapsHelper::new();
            }
            self.key_owner = owner_generation;
        }
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
        self.shared.has_terminal() || self.key.has_terminal()
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
        .helper(target)
        .edits()
        .pending_keys()
        .filter_map(|entry| {
            let request = entry.request.as_ref()?;
            (request.scope == actions.scope
                && request.scope_generation == actions.scope_generation
                && request.selection_generation == actions.selection_generation
                && request.target == *target)
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
                guard.bind_legend(live_key_id.as_deref(), legend_draft, legend_failure);
            }
            if !edits.peek().has_terminal() {
                return;
            }
            let document = runtime.model().accepted.map(|snapshot| snapshot.document);
            let results = {
                let guard = edits.write();
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
        let shared = guard.shared.edits();
        let key = guard.key.edits();
        shared
            .pending_keys()
            .chain(key.pending_keys())
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

#[path = "keycaps_settings/retry_drafts.rs"]
mod retry_drafts;
use retry_drafts::{
    draft_owner_mismatch, keep_retry_draft, remove_retry_draft_through, retry_identity_is_current,
    retry_identity_matches, submit_keycap_edit,
};
#[cfg(all(test, not(target_arch = "wasm32")))]
use retry_drafts::{remove_retry_draft_through_map, retry_identity, store_retry_draft};

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
            || boardstudio_model::model::KeycapBoardSettings::default().color,
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

const PROFILES: &[(&str, &str)] = &[
    ("cherry", "Cherry"),
    ("oem", "OEM"),
    ("dcs", "DCS"),
    ("dsa", "DSA"),
    ("sa", "SA"),
    ("hi-pro", "Hi-Pro"),
    ("g20", "G20"),
    ("choc", "Choc"),
];
const MOUNTS: &[(&str, &str)] = &[
    ("mx", "MX cross"),
    ("choc-v1", "Choc v1"),
    ("choc-v2", "Choc v2 cross"),
    ("alps", "Alps"),
];

#[path = "keycaps_settings/editors.rs"]
mod editors;
pub use editors::{
    KeycapsBoardSettingsEditor, KeycapsBoardSettingsEditorProps, KeycapsMatrixSettingsEditor,
    KeycapsMatrixSettingsEditorProps, KeycapsSettingsEditor, KeycapsSettingsEditorProps,
};

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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod queued_settings_tests;
