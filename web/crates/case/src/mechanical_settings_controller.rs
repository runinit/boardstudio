//! Private owner for the Case mechanical-settings field requests.
//!
//! Catalogue resources are loaded before dispatch, in commit order. Each prepared ticket
//! then submits one typed Core mechanical edit; Core owns defaults, validation, closure-mount
//! initialization, clearance projection, the precise changed ids and the no-op short-circuit,
//! and settlement never compares document content.
use super::mechanical_settings::{
    MechanicalSettingsFeedback, MechanicalSettingsFeedbackState, MechanicalSettingsIdentity,
    MechanicalSettingsPatch, MechanicalSettingsPatchExt, MechanicalSettingsRequest,
};
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_core::mechanical::change::{MechanicalEdit, MechanicalTarget};
use boardstudio_core::model::{
    EditOperation, MechanicalAssembly, MechanicalConfiguration, PartDefinition,
};
use boardstudio_web_runtime::edit_ticket::EditTicketPort;
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::{FieldView, PendingEditSignals};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;

type LocalFuture<T> = Pin<Box<dyn Future<Output = T> + 'static>>;

/// The page copies only accepted Arc handles and the small configuration projection displayed
/// by the controls. `editable` must be computed from the current Runtime, workspace,
/// physical-instance, readiness, preview, gesture, lifecycle and durability admission guards.
/// It excludes this controller's pending requests so fields remain editable. The
/// controller still re-reads it at every asynchronous boundary.
#[derive(Clone)]
pub struct MechanicalSettingsCurrent {
    pub identity: MechanicalSettingsIdentity,
    pub accepted: AcceptedSnapshot,
    pub configuration: Option<Rc<MechanicalConfiguration>>,
    pub editable: bool,
    pub lifecycle: Lifecycle,
    pub durability: Durability,
}

/// The page's existing effective-case policy is the source for both a fresh Core resolution and
/// generated boss height (notably the wireless battery default). The reflected/effective document
/// remains read-only; only this mechanical value is written back through the canonical ownership
/// mapping.
#[derive(Clone)]
pub struct MechanicalResolution {
    pub assembly: MechanicalAssembly,
    pub effective_configuration: MechanicalConfiguration,
}

/// Runtime supplies admission, catalogue loading and the edit-ticket port. Only the
/// normalized catalogue definition is prepared asynchronously; document planning runs
/// against the accepted snapshot when Session executes the intent.
#[derive(Clone)]
pub struct MechanicalSettingsPorts {
    pub current: Rc<dyn Fn() -> Option<MechanicalSettingsCurrent>>,
    pub load_mounting_hole: Rc<dyn Fn() -> LocalFuture<Result<Rc<PartDefinition>, String>>>,
    pub edit_port: Rc<dyn EditTicketPort>,
    pub publish: Rc<dyn Fn(MechanicalSettingsFeedback)>,
}

pub struct MechanicalSettingsController {
    ports: MechanicalSettingsPorts,
    requests: RefCell<Vec<SettingsEdit>>,
    /// The latest observed edit per field, for the owner it was admitted under; an older
    /// edit for the field still runs. A changed owner replaces the helper.
    edits: RefCell<(
        Option<MechanicalSettingsIdentity>,
        PendingEditSignals<String>,
    )>,
    last_request_id: Cell<u64>,
    fields: RefCell<Vec<MechanicalField>>,
    staged_field: RefCell<Option<PreparedField>>,
}

struct MechanicalField {
    key: String,
    view: FieldView,
    accepted: String,
    latest_request: Option<u64>,
}

/// Only catalogue preparation retains this commit-time text and binding identity.
/// Once a ticket starts, its helper owns both submitted text and binding epochs.
#[derive(Clone)]
struct PreparedField {
    key: String,
    view: FieldView,
    text: String,
}

fn same_field(left: FieldView, right: FieldView) -> bool {
    left.draft == right.draft && left.failure == right.failure
}

/// Connect actual child Signals to the existing controller without expanding panel props.
/// The registry contains only mounted logical controls; dropped views are never read.
pub(super) fn use_bound_mechanical_field(
    key: String,
    accepted: String,
    draft: Signal<String>,
    failure: Signal<Option<String>>,
) -> (Option<Rc<MechanicalSettingsController>>, bool) {
    let controller = try_consume_context::<Rc<MechanicalSettingsController>>();
    let bound = use_hook(|| Rc::new(RefCell::new(None::<String>)));
    let view = FieldView { draft, failure };
    if let Some(controller) = controller.as_ref() {
        let mut previous = bound.borrow_mut();
        if let Some(old) = previous.as_ref()
            && old != &key
        {
            controller.unbind_field(old, view);
        }
        controller.bind_field(key.clone(), view, accepted);
        *previous = Some(key.clone());
    }
    let pending = controller.as_ref().is_some_and(|controller| {
        let latest = controller
            .fields
            .borrow()
            .iter()
            .find(|field| field.key == key && same_field(field.view, view))
            .and_then(|field| field.latest_request);
        controller.requests.borrow().iter().any(|request| {
            request.key == key
                && Some(request.request.request_id) == latest
                && (!matches!(request.phase, SettingsPhase::Submitted)
                    || controller.edits.borrow().1.is_pending(&key))
        })
    });
    use_drop({
        let controller = controller.clone();
        move || {
            if let Some(controller) = controller
                && let Some(key) = bound.borrow().as_ref()
            {
                controller.unbind_field(key, view);
            }
        }
    });
    (controller, pending)
}

#[derive(Clone)]
struct SettingsEdit {
    request: MechanicalSettingsRequest,
    phase: SettingsPhase,
    key: String,
    prepared_field: Option<PreparedField>,
}

#[derive(Clone)]
enum SettingsPhase {
    Preparing,
    Prepared(EditResolver),
    Submitted,
}

impl MechanicalSettingsController {
    pub fn new(ports: MechanicalSettingsPorts) -> Rc<Self> {
        Rc::new(Self {
            ports,
            requests: RefCell::new(Vec::new()),
            edits: RefCell::new((None, PendingEditSignals::new())),
            last_request_id: Cell::new(0),
            fields: RefCell::new(Vec::new()),
            staged_field: RefCell::new(None),
        })
    }

    pub fn is_busy(&self) -> bool {
        !self.requests.borrow().is_empty()
    }

    fn bind_field(&self, key: String, view: FieldView, accepted: String) {
        self.follow_owner();
        self.edits
            .borrow()
            .1
            .bind_field(key.clone(), view.draft, view.failure);
        let mut fields = self.fields.borrow_mut();
        match fields.iter_mut().find(|field| field.key == key) {
            Some(field) => {
                if !same_field(field.view, view) {
                    field.latest_request = None;
                }
                field.view = view;
                field.accepted = accepted;
            }
            None => fields.push(MechanicalField {
                key,
                view,
                accepted,
                latest_request: None,
            }),
        }
    }

    fn unbind_field(&self, key: &str, view: FieldView) {
        let mut fields = self.fields.borrow_mut();
        if fields
            .iter()
            .any(|field| field.key == key && same_field(field.view, view))
        {
            self.edits.borrow().1.unbind_field(&key.to_owned());
            fields.retain(|field| field.key != key);
        }
    }

    /// Keep the original callback's admission path and transfer field metadata only
    /// during its synchronous submit. Rejected admission leaves no staged request.
    pub(super) fn request_field(
        &self,
        key: String,
        draft: Signal<String>,
        failure: Signal<Option<String>>,
        text: String,
        on_request: EventHandler<MechanicalSettingsRequest>,
        request: MechanicalSettingsRequest,
    ) {
        let previous = self.staged_field.replace(Some(PreparedField {
            key,
            view: FieldView { draft, failure },
            text,
        }));
        on_request.call(request);
        self.staged_field.replace(previous);
    }

    pub fn submit(self: &Rc<Self>, request: MechanicalSettingsRequest) -> bool {
        let prepared_field = self.staged_field.borrow_mut().take();
        if request.request_id <= self.last_request_id.get() {
            return false;
        }
        self.last_request_id.set(request.request_id);
        let Some(current) = (self.ports.current)() else {
            return false;
        };
        if !admitted(&current, &request.identity) || request.field_id != request.patch.field_id() {
            return false;
        }
        if is_one_shot(&request.patch)
            && self
                .requests
                .borrow()
                .iter()
                .any(|pending| pending.request.field_id == request.field_id)
        {
            return false;
        }
        // Preparation has no ticket yet. Remember only the latest admitted request
        // for the current mounted view, including after that preparation fails.
        if let Some(submitted) = prepared_field.as_ref()
            && let Some(field) =
                self.fields.borrow_mut().iter_mut().find(|field| {
                    field.key == submitted.key && same_field(field.view, submitted.view)
                })
        {
            field.latest_request = Some(request.request_id);
            // A newer admitted preparation owns this mounted field immediately.
            // Detach the old ticket's UI epoch while its Session write continues;
            // the new ticket will capture this binding only after its loader succeeds.
            let helper = self.edits.borrow().1.clone();
            helper.unbind_field(&field.key);
            helper.bind_field(field.key.clone(), field.view.draft, field.view.failure);
        }
        let opening_guard = opening_target_guard(&current, &request.patch, &self.requests.borrow());
        self.requests.borrow_mut().push(SettingsEdit {
            request: request.clone(),
            phase: SettingsPhase::Preparing,
            key: prepared_field.as_ref().map_or_else(
                || format!("request:{}", request.field_id),
                |field| field.key.clone(),
            ),
            prepared_field,
        });
        self.emit(&request, MechanicalSettingsFeedbackState::Pending, None);
        let owner = Rc::downgrade(self);
        let loader = self.ports.load_mounting_hole.clone();
        spawn_local(async move {
            let result = loader().await;
            let Some(owner) = owner.upgrade() else {
                return;
            };
            if !(owner.ports.current)()
                .is_some_and(|current| same_owner(&current.identity, &request.identity))
            {
                owner
                    .requests
                    .borrow_mut()
                    .retain(|pending| pending.request.request_id != request.request_id);
                owner.flush_prepared();
                return;
            }
            match result {
                Ok(template) => {
                    let intent = request.clone();
                    let resolver = EditResolver::new(
                        "mechanical-settings",
                        move |accepted: &AcceptedSnapshot| {
                            if let Some(guard) = &opening_guard
                                && let Err(reason) = guard.check(accepted, &intent.identity.scope)
                            {
                                return Resolution::Retire(reason);
                            }
                            resolve_mechanical_edit(&intent, &template)
                        },
                    );
                    if let Some(pending) = owner
                        .requests
                        .borrow_mut()
                        .iter_mut()
                        .find(|pending| pending.request.request_id == request.request_id)
                    {
                        pending.phase = SettingsPhase::Prepared(resolver);
                    }
                }
                Err(message) => {
                    let removed = {
                        let mut requests = owner.requests.borrow_mut();
                        requests
                            .iter()
                            .position(|pending| pending.request.request_id == request.request_id)
                            .map(|index| requests.remove(index))
                    };
                    // No ticket exists yet: this is catalogue preparation failure,
                    // projected only to the same still-mounted commit-time field.
                    if let Some(field) = removed.and_then(|entry| entry.prepared_field) {
                        owner.fail_preparation(&field, request.request_id, &message);
                    }
                    owner.emit(
                        &request,
                        MechanicalSettingsFeedbackState::Failed,
                        Some(message),
                    );
                }
            }
            owner.flush_prepared();
        });
        true
    }

    // Preserve commit order even if catalogue loads complete in a different order.
    fn flush_prepared(&self) {
        let prepared = {
            let requests = self.requests.borrow();
            requests
                .iter()
                .take_while(|pending| !matches!(pending.phase, SettingsPhase::Preparing))
                .filter_map(|pending| match &pending.phase {
                    SettingsPhase::Prepared(resolver) => Some((
                        pending.request.request_id,
                        pending.key.clone(),
                        pending.prepared_field.clone(),
                        resolver.clone(),
                    )),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        for (id, key, field, resolver) in prepared {
            self.follow_owner();
            let helper = self.edits.borrow().1.clone();
            let current_view = self
                .fields
                .borrow()
                .iter()
                .find(|entry| entry.key == key)
                .map(|entry| (entry.view, entry.latest_request));
            let detached = field.as_ref().is_some_and(|field| {
                current_view.is_none_or(|(view, latest)| {
                    !same_field(view, field.view) || latest != Some(id)
                })
            });
            if detached {
                // The ordered preparation prefix cannot have submitted a newer request
                // behind this one yet. Detaching here therefore cannot invalidate a
                // newer binding's in-flight ticket, and prevents the old preparation
                // from acquiring a replacement child's binding epoch.
                helper.unbind_field(&key);
            }
            helper.begin_field(
                self.ports.edit_port.as_ref(),
                key.clone(),
                "mechanical-settings",
                Some("mechanical settings".into()),
                resolver,
                field.as_ref().map_or("", |field| field.text.as_str()),
            );
            if detached && let Some((view, _)) = current_view {
                helper.bind_field(key.clone(), view.draft, view.failure);
            }
            let mut requests = self.requests.borrow_mut();
            // The newest observation replaces the older one for its field.
            requests.retain(|entry| {
                entry.request.request_id == id
                    || entry.key != key
                    || !matches!(entry.phase, SettingsPhase::Submitted)
            });
            if let Some(pending) = requests
                .iter_mut()
                .find(|pending| pending.request.request_id == id)
            {
                pending.phase = SettingsPhase::Submitted;
                pending.prepared_field = None;
            }
        }
    }

    pub fn settle(&self) {
        let current = (self.ports.current)();
        let live = |request: &MechanicalSettingsRequest| {
            current
                .as_ref()
                .is_some_and(|current| same_owner(&current.identity, &request.identity))
        };
        // A request whose owner has gone retires silently in any phase; a later result
        // for its field then finds no request to report.
        self.requests
            .borrow_mut()
            .retain(|pending| live(&pending.request));
        self.follow_owner();
        let helper = self.edits.borrow().1.clone();
        let results = helper.settle(current.is_some(), |key| {
            self.fields
                .borrow()
                .iter()
                .find(|field| &field.key == key)
                .map(|field| field.accepted.clone())
                .unwrap_or_default()
        });
        for result in results {
            let key = match &result {
                PendingEditResult::Landed { key, .. }
                | PendingEditResult::Failed { key, .. }
                | PendingEditResult::Retired { key } => key.clone(),
            };
            let settled = {
                let mut requests = self.requests.borrow_mut();
                requests
                    .iter()
                    .position(|entry| {
                        matches!(entry.phase, SettingsPhase::Submitted) && entry.key == key
                    })
                    .map(|index| requests.remove(index))
            };
            let Some(settled) = settled else {
                continue;
            };
            match result {
                PendingEditResult::Landed { .. } => self.emit(
                    &settled.request,
                    MechanicalSettingsFeedbackState::Landed,
                    None,
                ),
                PendingEditResult::Failed { message, .. } => self.emit(
                    &settled.request,
                    MechanicalSettingsFeedbackState::Failed,
                    Some(message),
                ),
                PendingEditResult::Retired { .. } => {}
            }
        }
        self.flush_prepared();
    }

    /// Presentation ownership governs UI observations, including leave/return to Case.
    /// Replacing the helper never cancels the already submitted Session-owned edits.
    fn follow_owner(&self) {
        let Some(current) = (self.ports.current)() else {
            return;
        };
        let mut edits = self.edits.borrow_mut();
        let changed = edits
            .0
            .as_ref()
            .is_none_or(|owner| !same_owner(&current.identity, owner));
        if changed {
            if edits.0.is_some() {
                edits.1 = PendingEditSignals::new();
                for field in self.fields.borrow_mut().iter_mut() {
                    field.latest_request = None;
                    edits
                        .1
                        .bind_field(field.key.clone(), field.view.draft, field.view.failure);
                }
            }
            edits.0 = Some(current.identity);
        }
    }

    fn fail_preparation(&self, submitted: &PreparedField, request_id: u64, message: &str) {
        let field = self
            .fields
            .borrow()
            .iter()
            .find(|field| {
                field.key == submitted.key
                    && same_field(field.view, submitted.view)
                    && field.latest_request == Some(request_id)
            })
            .map(|field| (field.view, field.accepted.clone()));
        if let Some((mut view, accepted)) = field {
            if *view.draft.peek() == submitted.text {
                view.draft.set(accepted);
            }
            view.failure.set(Some(message.to_owned()));
        }
    }

    fn emit(
        &self,
        request: &MechanicalSettingsRequest,
        state: MechanicalSettingsFeedbackState,
        message: Option<String>,
    ) {
        (self.ports.publish)(MechanicalSettingsFeedback {
            identity: request.identity.clone(),
            request_id: request.request_id,
            field_id: request.field_id.clone(),
            state,
            message,
        });
    }
}

// Openings and vertices currently have positional identities. Capture destructive
// predecessors from this owner so their later fields cannot retarget a shifted row.
// One-shot structural requests share a guard; a failed predecessor changes no count.
struct OpeningTargetGuard {
    opening_index: usize,
    point_index: Option<usize>,
    opening_count: usize,
    point_count: usize,
    removals: Vec<(usize, Option<usize>)>,
}

impl OpeningTargetGuard {
    fn check(&self, accepted: &AcceptedSnapshot, scope: &Scope) -> Result<(), String> {
        let effective = boardstudio_web_host::cad_jobs::captured_case_document(accepted, scope)
            .map_err(|_| "The selected access opening is no longer available.".to_owned())?;
        let openings = effective
            .mechanical
            .as_ref()
            .and_then(|configuration| configuration.openings.as_ref());
        let opening_count = openings.map_or(0, Vec::len);
        let point_count = openings
            .and_then(|openings| openings.get(self.opening_index))
            .map_or(0, |opening| opening.vertices.len());
        let shifted = self.removals.iter().any(|(opening, point)| match point {
            None => *opening <= self.opening_index && opening_count < self.opening_count,
            Some(removed) => {
                *opening == self.opening_index
                    && self.point_index.is_some_and(|target| *removed <= target)
                    && point_count < self.point_count
            }
        });
        if shifted {
            Err("The selected opening or vertex was removed before this edit could run.".into())
        } else {
            Ok(())
        }
    }
}

fn opening_target_guard(
    current: &MechanicalSettingsCurrent,
    patch: &MechanicalSettingsPatch,
    earlier: &[SettingsEdit],
) -> Option<OpeningTargetGuard> {
    let (opening_index, point_index) = match patch {
        MechanicalSettingsPatch::SetOpeningDimension {
            opening_index,
            point_index,
            ..
        } => (*opening_index, *point_index),
        MechanicalSettingsPatch::RemoveOpening { opening_index }
        | MechanicalSettingsPatch::AddOpeningVertex { opening_index } => (*opening_index, None),
        MechanicalSettingsPatch::RemoveOpeningVertex {
            opening_index,
            point_index,
        } => (*opening_index, Some(*point_index)),
        _ => return None,
    };
    let removals = earlier
        .iter()
        .filter_map(|entry| match &entry.request.patch {
            MechanicalSettingsPatch::RemoveOpening { opening_index } => {
                Some((*opening_index, None))
            }
            MechanicalSettingsPatch::RemoveOpeningVertex {
                opening_index,
                point_index,
            } => Some((*opening_index, Some(*point_index))),
            _ => None,
        })
        .collect::<Vec<_>>();
    if removals.is_empty() {
        return None;
    }
    let openings = current
        .configuration
        .as_ref()
        .and_then(|configuration| configuration.openings.as_ref());
    Some(OpeningTargetGuard {
        opening_index,
        point_index,
        opening_count: openings.map_or(0, Vec::len),
        point_count: openings
            .and_then(|openings| openings.get(opening_index))
            .map_or(0, |opening| opening.vertices.len()),
        removals,
    })
}

fn is_one_shot(patch: &MechanicalSettingsPatch) -> bool {
    matches!(
        patch,
        MechanicalSettingsPatch::AddOpening
            | MechanicalSettingsPatch::RemoveOpening { .. }
            | MechanicalSettingsPatch::AddOpeningVertex { .. }
            | MechanicalSettingsPatch::RemoveOpeningVertex { .. }
            | MechanicalSettingsPatch::AddMount { .. }
            | MechanicalSettingsPatch::AddSuggestedClosureMounts(_)
            | MechanicalSettingsPatch::SetMountCollection { .. }
            | MechanicalSettingsPatch::AdoptClosurePositions(_)
            | MechanicalSettingsPatch::InitializeClosures
            | MechanicalSettingsPatch::ResetGasketPlacement
            | MechanicalSettingsPatch::AddHardware { .. }
            | MechanicalSettingsPatch::RemoveHardware { .. }
            | MechanicalSettingsPatch::RemoveMount { .. }
            | MechanicalSettingsPatch::RemoveProfile { .. }
            | MechanicalSettingsPatch::AddProcessOverride { .. }
            | MechanicalSettingsPatch::RemoveProcessOverride { .. }
            | MechanicalSettingsPatch::AddCriticalFit { .. }
            | MechanicalSettingsPatch::RemoveCriticalFit { .. }
            | MechanicalSettingsPatch::AssignSwitchProfile { .. }
            | MechanicalSettingsPatch::AssignStabilizerProfile { .. }
            | MechanicalSettingsPatch::AssignImportedGeometryProfile { .. }
    )
}

/// Builds the one typed mechanical edit this request submits.
///
/// Core owns the whole settings vocabulary, defaults, closure-mount initialization,
/// clearance projection, precise changed ids and the no-op short-circuit. The adapter
/// therefore submits unconditionally: an equal value is a Core no-op that settles
/// without a revision step, and a malformed or ineligible target is a Core edit error.
/// The only local retirement is the opening-target guard in [`MechanicalSettingsController::submit`],
/// because a removed opening or vertex can no longer be addressed by the request's
/// positional identity. This is deliberately not a reimplementation of Core's rules.
fn resolve_mechanical_edit(
    request: &MechanicalSettingsRequest,
    template: &PartDefinition,
) -> Resolution {
    let target = match request.identity.scope.instance_id.as_deref() {
        Some(instance_id) => MechanicalTarget::Instance {
            instance_id: instance_id.to_owned(),
        },
        None => MechanicalTarget::Board {
            board_id: request.identity.configuration_board_id.clone(),
        },
    };
    let edit = MechanicalEdit {
        target,
        change: request.patch.clone(),
        mounting_hole: Some(Box::new(template.clone())),
    };
    // Session needs a changed-id hint before Core runs and Core reports the precise ids
    // in its reply. The configuration owner is the active board id, or the instance id
    // when the scope has one.
    let owner_id = request
        .identity
        .scope
        .instance_id
        .clone()
        .unwrap_or_else(|| request.identity.active_board_id.clone());
    Resolution::submit(
        vec![owner_id],
        EditOperation::ApplyMechanicalEdit {
            edit: Box::new(edit),
        },
    )
}

fn admitted(current: &MechanicalSettingsCurrent, identity: &MechanicalSettingsIdentity) -> bool {
    current.editable
        && same_owner(&current.identity, identity)
        && current.accepted.session_epoch == identity.scope.session_epoch
        && current.accepted.document.id == identity.scope.document_id
        && identity.scope.board_id == identity.active_board_id
}

fn same_owner(current: &MechanicalSettingsIdentity, request: &MechanicalSettingsIdentity) -> bool {
    current.editor_instance_id == request.editor_instance_id
        && current.scope_generation == request.scope_generation
        && current.presentation_generation == request.presentation_generation
        && current.scope == request.scope
        && current.active_board_id == request.active_board_id
        && current.configuration_board_id == request.configuration_board_id
}

#[cfg(test)]
mod adapter_tests {
    use super::*;
    use crate::runtime::{Runtime, project_name_test_support as support};
    use boardstudio_application::{
        AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, OperationId, Resolution,
        Scope, SnapshotToken,
    };
    use boardstudio_core::model::{EditOperation, ProjectDoc};
    use boardstudio_web_runtime::edit_ticket::EditTicketPort;
    use boardstudio_web_runtime::operation_outcomes::{LandingSlot, OutcomeSlot};
    use futures_channel::oneshot;
    use gloo_timers::future::TimeoutFuture;
    use std::collections::VecDeque;
    use wasm_bindgen_test::wasm_bindgen_test;

    /// Records every resolver the controller begins instead of running it, so a test
    /// can resolve the captured intent against any accepted snapshot it wants.
    #[derive(Clone, Default)]
    struct RecordingPort {
        resolutions: Rc<RefCell<Vec<EditResolver>>>,
        next_operation: Rc<Cell<u64>>,
    }

    impl EditTicketPort for RecordingPort {
        fn allocate_operation(&self) -> OperationId {
            let id = self.next_operation.get();
            self.next_operation.set(id + 1);
            OperationId(id)
        }

        fn scope_source(&self) -> Rc<dyn Fn() -> Option<Scope>> {
            Rc::new(|| None)
        }

        fn observe(&self, _operation: OperationId) -> (OutcomeSlot, LandingSlot) {
            (Rc::new(RefCell::new(None)), Rc::new(RefCell::new(None)))
        }

        fn submit(&self, event: Event) {
            if let Event::ResolveEdit { resolver, .. } = event {
                self.resolutions.borrow_mut().push(resolver);
            }
        }
    }

    /// A queue of catalogue replies; an empty queue resolves immediately.
    #[derive(Clone, Default)]
    struct TestCatalogue(Rc<RefCell<VecDeque<oneshot::Receiver<Result<(), String>>>>>);

    fn template() -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id": "hole", "name": "Hole", "kind": "utility", "courtyard": [], "pads": []
        }))
        .unwrap()
    }

    fn fixture_document(project_id: &str) -> ProjectDoc {
        let mut document = ProjectDoc::empty(project_id, "adapter test");
        document.boards.push(
            serde_json::from_value(serde_json::json!({
                "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
                "netIds": [], "thickness": 1.6, "traces": [], "vias": []
            }))
            .unwrap(),
        );
        document.mechanical = Some(
            serde_json::from_value(serde_json::json!({
                "boardId": "board", "method": "printed", "mount": "rigid",
                "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
                "bottomFoamThickness": 0.4, "batteryHeight": 4.0, "bottomThickness": 3.0,
                "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3, "profiles": [],
                "closureMounts": [], "openings": [{
                    "id": "opening-1",
                    "vertices": [
                        { "id": "vertex-1", "point": { "x": 0.0, "y": 0.0 } },
                        { "id": "vertex-2", "point": { "x": 5.0, "y": 0.0 } }
                    ],
                    "z": 0.0, "height": 10.0
                }]
            }))
            .unwrap(),
        );
        document
    }

    fn current(
        accepted: &AcceptedSnapshot,
        instance_id: Option<&str>,
    ) -> MechanicalSettingsCurrent {
        let scope = Scope {
            session_epoch: accepted.session_epoch,
            document_id: accepted.document.id.clone(),
            board_id: "board".into(),
            instance_id: instance_id.map(str::to_owned),
        };
        MechanicalSettingsCurrent {
            identity: MechanicalSettingsIdentity {
                editor_instance_id: 1,
                scope_generation: 1,
                presentation_generation: 1,
                snapshot_token: accepted.token,
                revision: accepted.document.revision,
                active_board_id: "board".into(),
                configuration_board_id: "board".into(),
                scope,
            },
            configuration: accepted.document.mechanical.clone().map(Rc::new),
            accepted: accepted.clone(),
            editable: true,
            lifecycle: Lifecycle::Ready,
            durability: Durability::Saved {
                revision: accepted.document.revision,
            },
        }
    }

    fn controller(
        current: MechanicalSettingsCurrent,
        catalogue: TestCatalogue,
        port: RecordingPort,
        feedback: Rc<RefCell<Vec<MechanicalSettingsFeedback>>>,
    ) -> Rc<MechanicalSettingsController> {
        MechanicalSettingsController::new(MechanicalSettingsPorts {
            current: Rc::new(move || Some(current.clone())),
            load_mounting_hole: Rc::new(move || {
                let reply = catalogue.0.borrow_mut().pop_front();
                Box::pin(async move {
                    if let Some(reply) = reply {
                        reply.await.unwrap()?;
                    }
                    Ok(Rc::new(template()))
                })
            }),
            edit_port: Rc::new(port),
            publish: Rc::new(move |entry| feedback.borrow_mut().push(entry)),
        })
    }

    fn request(
        identity: MechanicalSettingsIdentity,
        request_id: u64,
        patch: MechanicalSettingsPatch,
    ) -> MechanicalSettingsRequest {
        MechanicalSettingsRequest {
            identity,
            request_id,
            field_id: patch.field_id(),
            patch,
        }
    }

    fn submitted_edit(resolver: &EditResolver, accepted: &AcceptedSnapshot) -> MechanicalEdit {
        match resolver.resolve(accepted) {
            Resolution::Submit(command) => match command.operation {
                EditOperation::ApplyMechanicalEdit { edit } => *edit,
                other => panic!("expected ApplyMechanicalEdit, got {other:?}"),
            },
            other => panic!("expected a submission, got {other:?}"),
        }
    }

    async fn loaded() {
        TimeoutFuture::new(20).await;
    }

    async fn accepted_fixture(project_id: &str) -> (Rc<Runtime>, AcceptedSnapshot) {
        let runtime = support::new_runtime();
        support::open_document(&runtime, fixture_document(project_id)).await;
        let accepted = runtime.model().accepted.expect("accepted fixture");
        (runtime, accepted)
    }

    #[wasm_bindgen_test]
    async fn a_prepared_request_submits_one_apply_mechanical_edit_with_its_template() {
        let (_runtime, accepted) = accepted_fixture("adapter-one-edit").await;
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, None);
        let identity = current.identity.clone();
        let controller = controller(current, TestCatalogue::default(), port.clone(), feedback);

        assert!(controller.submit(request(
            identity,
            1,
            MechanicalSettingsPatch::SetDimension {
                field: boardstudio_core::mechanical::change::MechanicalDimension::WallThickness,
                value: 3.0,
            },
        )));
        loaded().await;

        let resolutions = port.resolutions.borrow();
        assert_eq!(resolutions.len(), 1, "exactly one Core intent is submitted");
        let edit = submitted_edit(&resolutions[0], &accepted);
        assert_eq!(
            edit.target,
            MechanicalTarget::Board {
                board_id: "board".into()
            }
        );
        assert_eq!(
            edit.change,
            MechanicalSettingsPatch::SetDimension {
                field: boardstudio_core::mechanical::change::MechanicalDimension::WallThickness,
                value: 3.0,
            }
        );
        assert_eq!(edit.mounting_hole.as_deref(), Some(&template()));
    }

    #[wasm_bindgen_test]
    async fn a_scope_without_an_instance_targets_the_board() {
        let (_runtime, accepted) = accepted_fixture("adapter-board-target").await;
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, None);
        let identity = current.identity.clone();
        let controller = controller(current, TestCatalogue::default(), port.clone(), feedback);

        assert!(controller.submit(request(identity, 1, MechanicalSettingsPatch::Enable)));
        loaded().await;

        let resolutions = port.resolutions.borrow();
        let resolution = resolutions[0].resolve(&accepted);
        let Resolution::Submit(command) = resolution else {
            panic!("expected a submission");
        };
        assert_eq!(command.target_ids, vec!["board".to_owned()]);
        let EditOperation::ApplyMechanicalEdit { edit } = command.operation else {
            panic!("expected ApplyMechanicalEdit");
        };
        assert_eq!(
            edit.target,
            MechanicalTarget::Board {
                board_id: "board".into()
            }
        );
    }

    #[wasm_bindgen_test]
    async fn an_instance_scope_targets_the_instance_and_reports_its_owner() {
        let (_runtime, accepted) = accepted_fixture("adapter-instance-target").await;
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, Some("left"));
        let identity = current.identity.clone();
        let controller = controller(current, TestCatalogue::default(), port.clone(), feedback);

        assert!(controller.submit(request(identity, 1, MechanicalSettingsPatch::Enable)));
        loaded().await;

        let resolutions = port.resolutions.borrow();
        let resolution = resolutions[0].resolve(&accepted);
        let Resolution::Submit(command) = resolution else {
            panic!("expected a submission");
        };
        assert_eq!(command.target_ids, vec!["left".to_owned()]);
        let EditOperation::ApplyMechanicalEdit { edit } = command.operation else {
            panic!("expected ApplyMechanicalEdit");
        };
        assert_eq!(
            edit.target,
            MechanicalTarget::Instance {
                instance_id: "left".into()
            }
        );
    }

    #[wasm_bindgen_test]
    async fn flush_prepared_keeps_commit_order_when_catalogue_loads_finish_out_of_order() {
        let (_runtime, accepted) = accepted_fixture("adapter-ordering").await;
        let (release_a, reply_a) = oneshot::channel();
        let (release_b, reply_b) = oneshot::channel();
        let catalogue = TestCatalogue::default();
        catalogue.0.borrow_mut().extend([reply_a, reply_b]);
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, None);
        let identity = current.identity.clone();
        let controller = controller(current, catalogue, port.clone(), feedback);

        assert!(controller.submit(request(
            identity.clone(),
            1,
            MechanicalSettingsPatch::SetDimension {
                field: boardstudio_core::mechanical::change::MechanicalDimension::WallThickness,
                value: 3.0,
            },
        )));
        assert!(controller.submit(request(
            identity,
            2,
            MechanicalSettingsPatch::SetDimension {
                field: boardstudio_core::mechanical::change::MechanicalDimension::WallThickness,
                value: 4.0,
            },
        )));
        // The newer load completes first; the older request still commits first.
        release_b.send(Ok(())).unwrap();
        loaded().await;
        assert!(
            port.resolutions.borrow().is_empty(),
            "a prepared request behind a loading one must not flush early"
        );
        release_a.send(Ok(())).unwrap();
        loaded().await;

        let resolutions = port.resolutions.borrow();
        assert_eq!(resolutions.len(), 2);
        let first = submitted_edit(&resolutions[0], &accepted);
        let second = submitted_edit(&resolutions[1], &accepted);
        assert_eq!(
            first.change,
            MechanicalSettingsPatch::SetDimension {
                field: boardstudio_core::mechanical::change::MechanicalDimension::WallThickness,
                value: 3.0,
            }
        );
        assert_eq!(
            second.change,
            MechanicalSettingsPatch::SetDimension {
                field: boardstudio_core::mechanical::change::MechanicalDimension::WallThickness,
                value: 4.0,
            }
        );
    }

    #[wasm_bindgen_test]
    async fn a_retired_request_reports_without_submitting() {
        let (_runtime, accepted) = accepted_fixture("adapter-retire").await;
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, None);
        let identity = current.identity.clone();
        let controller = controller(current, TestCatalogue::default(), port.clone(), feedback);

        // A destructive predecessor is queued first.
        assert!(controller.submit(request(
            identity.clone(),
            1,
            MechanicalSettingsPatch::RemoveOpening { opening_index: 0 },
        )));
        loaded().await;
        assert!(controller.submit(request(
            identity,
            2,
            MechanicalSettingsPatch::SetOpeningDimension {
                opening_index: 0,
                point_index: Some(0),
                field: boardstudio_core::mechanical::change::MechanicalDimension::OpeningPointX,
                value: 1.0,
            },
        )));
        loaded().await;

        let resolutions = port.resolutions.borrow();
        assert_eq!(resolutions.len(), 2);
        // The accepted document now has no openings, as if the queued removal applied.
        let mut removed = (*accepted.document).clone();
        removed.mechanical.as_mut().unwrap().openings = Some(vec![]);
        let shifted = AcceptedSnapshot {
            document: std::sync::Arc::new(removed),
            ..accepted.clone()
        };
        let resolution = resolutions[1].resolve(&shifted);
        let Resolution::Retire(reason) = resolution else {
            panic!("a shifted opening target must retire, got {resolution:?}");
        };
        assert!(reason.contains("removed before this edit could run"));
    }

    #[wasm_bindgen_test]
    async fn a_catalogue_failure_reports_without_submitting() {
        let (_runtime, accepted) = accepted_fixture("adapter-catalogue-failure").await;
        let (release, reply) = oneshot::channel();
        let catalogue = TestCatalogue::default();
        catalogue.0.borrow_mut().push_back(reply);
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, None);
        let identity = current.identity.clone();
        let controller = controller(current, catalogue, port.clone(), feedback.clone());

        assert!(controller.submit(request(
            identity,
            1,
            MechanicalSettingsPatch::SetDimension {
                field: boardstudio_core::mechanical::change::MechanicalDimension::WallThickness,
                value: 3.0,
            },
        )));
        release.send(Err("catalogue unavailable".into())).unwrap();
        loaded().await;

        assert!(
            port.resolutions.borrow().is_empty(),
            "a failed catalogue load submits nothing"
        );
        assert!(feedback.borrow().iter().any(|entry| {
            entry.state == MechanicalSettingsFeedbackState::Failed
                && entry.message.as_deref() == Some("catalogue unavailable")
        }));
    }
}
