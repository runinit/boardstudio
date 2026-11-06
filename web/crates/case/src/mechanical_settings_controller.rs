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
use boardstudio_core::mechanical::{MechanicalEditDisposition, evaluate_edit};
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
        let opening_guard = opening_target_guard(&request.patch, &self.requests.borrow());
        let reviewed_pair_guard = reviewed_pair_guard(
            &request.patch,
            request.reviewed_fingerprint.clone(),
            &current.accepted,
        );
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
                            if let Some(guard) = &reviewed_pair_guard
                                && let Err(reason) = guard.check(accepted)
                            {
                                return Resolution::Retire(reason);
                            }
                            resolve_mechanical_edit(&intent, &template, accepted)
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

// Authored openings and vertices carry durable ids, so a queued edit names the exact
// target it was authored against and a reorder can never redirect it. The one local
// retirement this guard owns is a target that no longer exists: an earlier queued
// removal of that same opening or vertex, or a target already gone from the accepted
// configuration. Both retire silently instead of surfacing a Core error.
struct OpeningTargetGuard {
    opening_id: String,
    vertex_id: Option<String>,
    removals: Vec<(String, Option<String>)>,
}

impl OpeningTargetGuard {
    fn check(&self, accepted: &AcceptedSnapshot, scope: &Scope) -> Result<(), String> {
        let removed_earlier = self.removals.iter().any(|(opening_id, vertex_id)| {
            *opening_id == self.opening_id
                && vertex_id
                    .as_deref()
                    .is_none_or(|removed| self.vertex_id.as_deref() == Some(removed))
        });
        let vanished = match boardstudio_web_host::cad_jobs::captured_case_document(accepted, scope)
        {
            Err(_) => true,
            Ok(document) => {
                let openings = document
                    .mechanical
                    .as_ref()
                    .and_then(|configuration| configuration.openings.as_ref());
                match openings.and_then(|openings| {
                    openings
                        .iter()
                        .find(|opening| opening.id == self.opening_id)
                }) {
                    None => true,
                    Some(opening) => self.vertex_id.as_deref().is_some_and(|vertex_id| {
                        !opening.vertices.iter().any(|vertex| vertex.id == vertex_id)
                    }),
                }
            }
        };
        if removed_earlier || vanished {
            Err("The selected opening or vertex was removed before this edit could run.".into())
        } else {
            Ok(())
        }
    }
}

fn opening_target_guard(
    patch: &MechanicalSettingsPatch,
    earlier: &[SettingsEdit],
) -> Option<OpeningTargetGuard> {
    let (opening_id, vertex_id) = match patch {
        MechanicalSettingsPatch::SetOpeningDimension {
            opening_id,
            vertex_id,
            ..
        } => (opening_id.clone(), vertex_id.clone()),
        MechanicalSettingsPatch::RemoveOpening { opening_id }
        | MechanicalSettingsPatch::AddOpeningVertex { opening_id } => (opening_id.clone(), None),
        MechanicalSettingsPatch::RemoveOpeningVertex {
            opening_id,
            vertex_id,
        } => (opening_id.clone(), Some(vertex_id.clone())),
        _ => return None,
    };
    let removals = earlier
        .iter()
        .filter_map(|entry| match &entry.request.patch {
            MechanicalSettingsPatch::RemoveOpening { opening_id } => {
                Some((opening_id.clone(), None))
            }
            MechanicalSettingsPatch::RemoveOpeningVertex {
                opening_id,
                vertex_id,
            } => Some((opening_id.clone(), Some(vertex_id.clone()))),
            _ => None,
        })
        .collect::<Vec<_>>();
    if removals.is_empty() {
        return None;
    }
    Some(OpeningTargetGuard {
        opening_id,
        vertex_id,
        removals,
    })
}

/// The reviewed inputs a mechanical pair action was submitted with.
///
/// Link and rejoin review a proposed result before the user commits it. The
/// fingerprint of the inputs that result depends on is captured here, at review
/// time, and recomputed from the accepted document when Session resolves the
/// submission. A different fingerprint means a relevant input changed since the
/// review, so the queued action retires and the user reviews the refreshed result;
/// unrelated edits and revision changes leave it valid. A target that already
/// vanished has no fingerprint, which also retires.
struct ReviewedPairGuard {
    change: MechanicalSettingsPatch,
    reviewed: Result<String, String>,
}

impl ReviewedPairGuard {
    fn check(&self, accepted: &AcceptedSnapshot) -> Result<(), String> {
        let current = crate::mechanical_pair::reviewed_fingerprint(
            &accepted.document,
            &accepted.scene.board_contours,
            &self.change,
        );
        match (&self.reviewed, current) {
            (Ok(reviewed), Ok(current)) if *reviewed == current => Ok(()),
            (Ok(_), Ok(_)) => Err(
                "The reviewed inputs changed since this pair action was submitted; review it again."
                    .into(),
            ),
            (Err(reason), _) => Err(reason.clone()),
            (Ok(_), Err(reason)) => Err(reason),
        }
    }
}

/// The review guard for a pair action that previews its result, or `None`.
///
/// The reviewed basis is the fingerprint the *review* was produced with, carried by the
/// submission. It is never recomputed from the accepted document at confirmation time:
/// that would silently apply newer state the user has not seen. A request without a
/// carried fingerprint (an action that produced no review) falls back to the accepted
/// document so it still retires when its inputs are already ineligible.
fn reviewed_pair_guard(
    patch: &MechanicalSettingsPatch,
    reviewed_fingerprint: Option<String>,
    accepted: &AcceptedSnapshot,
) -> Option<ReviewedPairGuard> {
    if !matches!(
        patch,
        MechanicalSettingsPatch::RejoinFeatureMember { .. }
            | MechanicalSettingsPatch::LinkMirrorPair { .. }
    ) {
        return None;
    }
    Some(ReviewedPairGuard {
        change: patch.clone(),
        reviewed: match reviewed_fingerprint {
            Some(fingerprint) => Ok(fingerprint),
            None => crate::mechanical_pair::reviewed_fingerprint(
                &accepted.document,
                &accepted.scene.board_contours,
                patch,
            ),
        },
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
            | MechanicalSettingsPatch::RemovePhysicalInstance
            | MechanicalSettingsPatch::MaterializeLegacyConstruction { .. }
    )
}

/// Builds the one typed mechanical edit this request submits.
///
/// Core owns the whole settings vocabulary, defaults, closure-mount initialization,
/// clearance projection, precise changed ids and the no-op short-circuit. The adapter
/// asks Core's read-only evaluator for the disposition first: `Unchanged` settles
/// without a request or revision step, and a target that is already gone retires
/// instead of being redirected at whatever now occupies its place. A malformed value
/// still reaches Core through the submission, so the user sees the Core error rather
/// than a silent retirement. The only local retirement besides that is the
/// opening-target guard in [`MechanicalSettingsController::submit`], because a removed
/// opening or vertex can no longer be addressed by the request's positional identity.
/// This is deliberately not a reimplementation of Core's rules.
fn resolve_mechanical_edit(
    request: &MechanicalSettingsRequest,
    template: &PartDefinition,
    accepted: &AcceptedSnapshot,
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
    match evaluate_edit(&accepted.document, &accepted.scene.board_contours, &edit) {
        Ok(MechanicalEditDisposition::Unchanged) => return Resolution::Unchanged,
        Ok(MechanicalEditDisposition::Retire) => {
            return Resolution::Retire("The mechanical target is no longer available.".into());
        }
        // A malformed value stays a Core edit error: submitting lets Core report it.
        Ok(MechanicalEditDisposition::Changed) | Err(_) => {}
    }
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
        Scope,
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
    type CatalogueReply = oneshot::Receiver<Result<(), String>>;

    #[derive(Clone, Default)]
    struct TestCatalogue(Rc<RefCell<VecDeque<CatalogueReply>>>);

    /// The normalized mounting-hole definition Core validates on every supplied
    /// template: the `ceoloide/mounting_hole_npth` source with a drilled pad.
    fn template() -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id": "generator:ceoloide/mounting_hole_npth", "name": "mounting hole npth",
            "kind": "custom",
            "pads": [{
                "id": "pad-0", "number": "", "at": { "x": 0, "y": 0 },
                "size": { "x": 2.2, "y": 2.2 }, "shape": "circle", "drill": 2.2,
                "plated": false, "rotation": 0
            }],
            "courtyard": [
                { "x": -1.1, "y": -1.1 }, { "x": 1.1, "y": -1.1 },
                { "x": 1.1, "y": 1.1 }, { "x": -1.1, "y": 1.1 }
            ],
            "envelopeSource": { "courtyard": "generated" },
            "envelopeNotice": "No closed courtyard is available; the outline uses physical graphics and pad extents.",
            "generator": { "source": "ceoloide/mounting_hole_npth", "version": "bundled-1", "parameters": {} }
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
            reviewed_fingerprint: None,
        }
    }

    /// A request whose reviewed result the user has already seen, carrying the
    /// fingerprint of the accepted inputs that produced it.
    fn reviewed_request(
        identity: MechanicalSettingsIdentity,
        request_id: u64,
        patch: MechanicalSettingsPatch,
        reviewed_fingerprint: String,
    ) -> MechanicalSettingsRequest {
        MechanicalSettingsRequest {
            identity,
            request_id,
            field_id: patch.field_id(),
            patch,
            reviewed_fingerprint: Some(reviewed_fingerprint),
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

    /// The same fixture with a second, differently placed opening so an edit can
    /// prove it keeps its own durable id when the first one is removed or reordered.
    async fn accepted_fixture_with_two_openings(
        project_id: &str,
    ) -> (Rc<Runtime>, AcceptedSnapshot) {
        let mut document = fixture_document(project_id);
        document
            .mechanical
            .as_mut()
            .unwrap()
            .openings
            .as_mut()
            .unwrap()
            .push(
                serde_json::from_value(serde_json::json!({
                    "id": "opening-2",
                    "vertices": [
                        { "id": "vertex-1", "point": { "x": 10.0, "y": 0.0 } },
                        { "id": "vertex-2", "point": { "x": 15.0, "y": 0.0 } },
                        { "id": "vertex-3", "point": { "x": 15.0, "y": 5.0 } }
                    ],
                    "z": 1.0,
                    "height": 5.0
                }))
                .unwrap(),
            );
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
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

        // A value that really changes so the evaluator admits the submission; the
        // target and owner reporting are what this test covers.
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
        let resolution = resolutions[0].resolve(&accepted);
        let Resolution::Submit(command) = resolution else {
            panic!("expected a submission, got {resolution:?}");
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
        let document = pair_fixture_document("adapter-instance-target");
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let accepted = runtime.model().accepted.expect("accepted fixture");
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, Some("left"));
        let identity = current.identity.clone();
        let controller = controller(current, TestCatalogue::default(), port.clone(), feedback);

        // A value that really changes on the instance so the evaluator admits it.
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
        let resolution = resolutions[0].resolve(&accepted);
        let Resolution::Submit(command) = resolution else {
            panic!("expected a submission, got {resolution:?}");
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
    async fn a_per_feature_source_choice_is_reviewed_payload_not_accepted_state() {
        let mut document = pair_fixture_document("adapter-feature-source");
        document.hardware.as_mut().unwrap().pairs.clear();
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let accepted = runtime.model().accepted.expect("accepted fixture");
        let source = |instance: Option<&str>| MechanicalSettingsPatch::LinkMirrorPair {
            pair_id: "pair-new".into(),
            members: ["left".into(), "right".into()],
            mapping: boardstudio_core::model::Pose2 {
                at: boardstudio_core::model::Vec2::default(),
                rotation: 0.0,
            },
            construction_source: "left".into(),
            correspondences: vec![
                boardstudio_core::mechanical::change::MechanicalFeatureCorrespondence {
                    kind: boardstudio_core::model::MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1-left".into(),
                    second_feature_id: "mount-1-right".into(),
                    source_instance_id: instance.map(str::to_owned),
                },
            ],
        };
        for choice in [Some("right"), Some("left"), None] {
            let patch = source(choice);
            let fingerprint = crate::mechanical_pair::reviewed_fingerprint(
                &accepted.document,
                &accepted.scene.board_contours,
                &patch,
            )
            .expect("the reviewed inputs fingerprint");
            let port = RecordingPort::default();
            let feedback = Rc::new(RefCell::new(Vec::new()));
            let current = current(&accepted, Some("left"));
            let identity = current.identity.clone();
            let controller = controller(current, TestCatalogue::default(), port.clone(), feedback);
            assert!(controller.submit(reviewed_request(identity, 1, patch, fingerprint)));
            loaded().await;
            let resolutions = port.resolutions.borrow();
            assert_eq!(resolutions.len(), 1);
            let resolution = resolutions[0].resolve(&accepted);
            assert!(
                matches!(resolution, Resolution::Submit(_)),
                "a per-feature source choice is payload, so it stays valid, got {resolution:?}"
            );
        }
    }

    #[wasm_bindgen_test]
    async fn a_confirmed_action_uses_the_reviewed_basis_not_the_state_at_confirmation() {
        let document = pair_fixture_document("adapter-reviewed-basis");
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let accepted = runtime.model().accepted.expect("accepted fixture");

        let rejoin = MechanicalSettingsPatch::RejoinFeatureMember {
            feature_id: "mount-1-left".into(),
            instance_id: "left".into(),
            source_instance_id: "right".into(),
        };
        // The review the user saw was produced from the accepted document as it was;
        // a relevant change (the pair's map) has already landed before confirmation.
        let reviewed_fingerprint = crate::mechanical_pair::reviewed_fingerprint(
            &accepted.document,
            &accepted.scene.board_contours,
            &rejoin,
        )
        .expect("the reviewed inputs fingerprint");
        let mut changed_document = (*accepted.document).clone();
        changed_document.hardware.as_mut().unwrap().pairs[0]
            .mapping
            .rotation = 90.0;
        let changed = AcceptedSnapshot {
            document: std::sync::Arc::new(changed_document),
            ..accepted.clone()
        };

        let reviewed_port = RecordingPort::default();
        let reviewed_feedback = Rc::new(RefCell::new(Vec::new()));
        let reviewed_current = current(&changed, Some("left"));
        let reviewed_identity = reviewed_current.identity.clone();
        let reviewed_controller = controller(
            reviewed_current,
            TestCatalogue::default(),
            reviewed_port.clone(),
            reviewed_feedback,
        );
        assert!(reviewed_controller.submit(reviewed_request(
            reviewed_identity,
            1,
            rejoin.clone(),
            reviewed_fingerprint,
        )));
        loaded().await;

        {
            let resolutions = reviewed_port.resolutions.borrow();
            assert_eq!(resolutions.len(), 1);
            let resolution = resolutions[0].resolve(&changed);
            let Resolution::Retire(reason) = resolution else {
                panic!(
                    "a review produced before a relevant change must retire rather than apply \
                     state the user never reviewed, got {resolution:?}"
                );
            };
            assert!(reason.contains("reviewed inputs changed"), "{reason}");
        }

        // Edits outside the reviewed inputs leave the reviewed result valid.
        let current_fingerprint = crate::mechanical_pair::reviewed_fingerprint(
            &changed.document,
            &changed.scene.board_contours,
            &rejoin,
        )
        .expect("the reviewed inputs fingerprint");
        let mut unrelated_document = (*changed.document).clone();
        unrelated_document.name = "Renamed".into();
        unrelated_document.revision += 4;
        unrelated_document
            .parameters
            .insert("unrelated".into(), serde_json::json!("keep"));
        // A genuinely unrelated edit: a matrix on another board, which cannot change the
        // reviewed half's contour, source geometry, materials or proposed fit findings.
        unrelated_document.boards.push(
            serde_json::from_value(serde_json::json!({
                "id": "unrelated-board", "name": "Unrelated", "outlineIds": [], "partIds": [],
                "netIds": [], "thickness": 1.6, "traces": [], "vias": []
            }))
            .expect("an unrelated board parses"),
        );
        unrelated_document.matrices.push(
            serde_json::from_value(serde_json::json!({
                "id": "unrelated-matrix", "rows": 1, "columns": 1,
                "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": 0.0, "y": 0.0 },
                "definitionId": "switch-definition", "partIds": [], "boardId": "unrelated-board",
                "diodeDirection": "row2col", "rowLayout": [], "columnLayout": [],
            "cells": []
            }))
            .expect("a matrix edit outside the reviewed inputs parses"),
        );
        let unrelated = AcceptedSnapshot {
            document: std::sync::Arc::new(unrelated_document),
            ..changed.clone()
        };
        let unrelated_port = RecordingPort::default();
        let unrelated_feedback = Rc::new(RefCell::new(Vec::new()));
        let unrelated_current = current(&changed, Some("left"));
        let unrelated_identity = unrelated_current.identity.clone();
        let unrelated_controller = controller(
            unrelated_current,
            TestCatalogue::default(),
            unrelated_port.clone(),
            unrelated_feedback,
        );
        assert!(unrelated_controller.submit(reviewed_request(
            unrelated_identity,
            1,
            rejoin.clone(),
            current_fingerprint,
        )));
        loaded().await;
        let resolutions = unrelated_port.resolutions.borrow();
        assert_eq!(resolutions.len(), 1);
        let resolution = resolutions[0].resolve(&unrelated);
        assert!(
            matches!(resolution, Resolution::Submit(_)),
            "a project/matrix edit outside the reviewed inputs keeps the review valid, got {resolution:?}"
        );
    }

    #[wasm_bindgen_test]
    async fn a_reviewed_link_retires_when_the_source_geometry_it_reviewed_changed() {
        let mut document = pair_fixture_document("adapter-reviewed-link");
        // The halves are independent: no pair owns them yet.
        document.hardware.as_mut().unwrap().pairs.clear();
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let accepted = runtime.model().accepted.expect("accepted fixture");
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, Some("left"));
        let identity = current.identity.clone();
        let controller = controller(current, TestCatalogue::default(), port.clone(), feedback);

        let link = MechanicalSettingsPatch::LinkMirrorPair {
            pair_id: "pair-new".into(),
            members: ["left".into(), "right".into()],
            mapping: boardstudio_core::model::Pose2 {
                at: boardstudio_core::model::Vec2::default(),
                rotation: 0.0,
            },
            construction_source: "left".into(),
            correspondences: vec![
                boardstudio_core::mechanical::change::MechanicalFeatureCorrespondence {
                    kind: boardstudio_core::model::MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1-left".into(),
                    second_feature_id: "mount-1-right".into(),
                    source_instance_id: None,
                },
            ],
        };
        assert!(controller.submit(request(identity, 1, link.clone())));
        loaded().await;
        let resolutions = port.resolutions.borrow();
        assert_eq!(resolutions.len(), 1);
        let resolver = resolutions[0].clone();
        let edit = submitted_edit(&resolver, &accepted);
        assert_eq!(edit.change, link);

        // Unrelated edits and a revision change keep the reviewed link valid.
        let mut unrelated = (*accepted.document).clone();
        unrelated.name = "Renamed".into();
        unrelated.revision += 2;
        let unrelated = AcceptedSnapshot {
            document: std::sync::Arc::new(unrelated),
            ..accepted.clone()
        };
        assert!(
            matches!(resolver.resolve(&unrelated), Resolution::Submit(_)),
            "unrelated edits must not retire a reviewed link"
        );

        // Moving the source geometry the review chose retires it for a fresh review.
        let mut moved = (*accepted.document).clone();
        moved.hardware.as_mut().unwrap().instances[0].mechanical = Some({
            let mut configuration = accepted.document.mechanical.clone().unwrap();
            configuration.board_id = "board".into();
            configuration.mounts = vec![boardstudio_core::model::Mount {
                id: "mount-1-left".into(),
                at: boardstudio_core::model::Vec2 { x: 9.0, y: 1.0 },
                kind: boardstudio_core::model::MountKind::Hole,
                hole_diameter: 2.2,
                boss_diameter: None,
                height: None,
            }];
            configuration
        });
        let moved = AcceptedSnapshot {
            document: std::sync::Arc::new(moved),
            ..accepted.clone()
        };
        let resolution = resolver.resolve(&moved);
        let Resolution::Retire(reason) = resolution else {
            panic!("a moved source feature must retire the reviewed link, got {resolution:?}");
        };
        assert!(reason.contains("reviewed inputs changed"), "{reason}");
    }

    #[wasm_bindgen_test]
    async fn an_unchanged_value_settles_without_submitting_again() {
        let (_runtime, accepted) = accepted_fixture("adapter-unchanged").await;
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, None);
        let identity = current.identity.clone();
        let controller = controller(current, TestCatalogue::default(), port.clone(), feedback);
        let wall_thickness = accepted
            .document
            .mechanical
            .as_ref()
            .unwrap()
            .wall_thickness;

        assert!(controller.submit(request(
            identity,
            1,
            MechanicalSettingsPatch::SetDimension {
                field: boardstudio_core::mechanical::change::MechanicalDimension::WallThickness,
                value: wall_thickness,
            },
        )));
        loaded().await;

        let resolutions = port.resolutions.borrow();
        assert_eq!(resolutions.len(), 1);
        assert!(
            matches!(resolutions[0].resolve(&accepted), Resolution::Unchanged),
            "Core's evaluator reports an equal value as Unchanged, so no revision step is needed"
        );
    }

    #[wasm_bindgen_test]
    async fn a_vanished_mechanical_target_retires_instead_of_redirecting() {
        let document = pair_fixture_document("adapter-vanished-target");
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let accepted = runtime.model().accepted.expect("accepted fixture");
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, Some("left"));
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
        assert_eq!(resolutions.len(), 1);
        let resolution = resolutions[0].resolve(&accepted);
        assert!(
            matches!(resolution, Resolution::Submit(_)),
            "a present instance still submits to Core, got {resolution:?}"
        );

        // The queued intent was authored against an instance that is now gone.
        let mut gone = (*accepted.document).clone();
        gone.hardware.as_mut().unwrap().instances.clear();
        let gone = AcceptedSnapshot {
            document: std::sync::Arc::new(gone),
            ..accepted.clone()
        };
        let resolution = resolutions[0].resolve(&gone);
        let Resolution::Retire(reason) = resolution else {
            panic!("a vanished target must retire rather than redirect, got {resolution:?}");
        };
        assert!(reason.contains("no longer available"), "{reason}");
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
    async fn a_vertex_edit_retires_after_its_opening_is_removed() {
        let (_runtime, accepted) = accepted_fixture("adapter-retire-opening").await;
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, None);
        let identity = current.identity.clone();
        let controller = controller(current, TestCatalogue::default(), port.clone(), feedback);

        // A destructive predecessor is queued first, addressed to the whole opening.
        assert!(controller.submit(request(
            identity.clone(),
            1,
            MechanicalSettingsPatch::RemoveOpening {
                opening_id: "opening-1".into(),
            },
        )));
        loaded().await;
        assert!(controller.submit(request(
            identity,
            2,
            MechanicalSettingsPatch::SetOpeningDimension {
                opening_id: "opening-1".into(),
                vertex_id: Some("vertex-1".into()),
                field: boardstudio_core::mechanical::change::MechanicalDimension::OpeningPointX,
                value: 1.0,
            },
        )));
        loaded().await;

        let resolutions = port.resolutions.borrow();
        assert_eq!(resolutions.len(), 2);
        let resolution = resolutions[1].resolve(&accepted);
        let Resolution::Retire(reason) = resolution else {
            panic!("a queued edit to a removed opening must retire, got {resolution:?}");
        };
        assert!(reason.contains("removed before this edit could run"));
    }

    #[wasm_bindgen_test]
    async fn a_vertex_edit_retires_after_its_vertex_is_removed() {
        let (_runtime, accepted) = accepted_fixture("adapter-retire-vertex").await;
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, None);
        let identity = current.identity.clone();
        let controller = controller(current, TestCatalogue::default(), port.clone(), feedback);

        assert!(controller.submit(request(
            identity.clone(),
            1,
            MechanicalSettingsPatch::RemoveOpeningVertex {
                opening_id: "opening-1".into(),
                vertex_id: "vertex-1".into(),
            },
        )));
        loaded().await;
        assert!(controller.submit(request(
            identity,
            2,
            MechanicalSettingsPatch::SetOpeningDimension {
                opening_id: "opening-1".into(),
                vertex_id: Some("vertex-1".into()),
                field: boardstudio_core::mechanical::change::MechanicalDimension::OpeningPointY,
                value: 1.0,
            },
        )));
        loaded().await;

        let resolutions = port.resolutions.borrow();
        let resolution = resolutions[1].resolve(&accepted);
        let Resolution::Retire(reason) = resolution else {
            panic!("a queued edit to a removed vertex must retire, got {resolution:?}");
        };
        assert!(reason.contains("removed before this edit could run"));
    }

    #[wasm_bindgen_test]
    async fn removing_or_reordering_an_opening_does_not_redirect_an_earlier_edit() {
        let (_runtime, accepted) = accepted_fixture_with_two_openings("adapter-reorder").await;
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, None);
        let identity = current.identity.clone();
        let controller = controller(current, TestCatalogue::default(), port.clone(), feedback);

        // The destructive predecessor removes the *other* opening, so the queued
        // edit must keep addressing its own durable vertex even after the shift.
        assert!(controller.submit(request(
            identity.clone(),
            1,
            MechanicalSettingsPatch::RemoveOpening {
                opening_id: "opening-1".into(),
            },
        )));
        loaded().await;
        assert!(controller.submit(request(
            identity,
            2,
            MechanicalSettingsPatch::SetOpeningDimension {
                opening_id: "opening-2".into(),
                vertex_id: Some("vertex-3".into()),
                field: boardstudio_core::mechanical::change::MechanicalDimension::OpeningPointX,
                value: 2.0,
            },
        )));
        loaded().await;

        let resolutions = port.resolutions.borrow();
        assert_eq!(resolutions.len(), 2);
        // The accepted document now has opening-2 moved to index 0.
        let mut shifted_document = (*accepted.document).clone();
        let openings = shifted_document
            .mechanical
            .as_mut()
            .unwrap()
            .openings
            .as_mut()
            .unwrap();
        openings.retain(|opening| opening.id != "opening-1");
        let shifted = AcceptedSnapshot {
            document: std::sync::Arc::new(shifted_document),
            ..accepted.clone()
        };
        let edit = submitted_edit(&resolutions[1], &shifted);
        assert_eq!(
            edit.change,
            MechanicalSettingsPatch::SetOpeningDimension {
                opening_id: "opening-2".into(),
                vertex_id: Some("vertex-3".into()),
                field: boardstudio_core::mechanical::change::MechanicalDimension::OpeningPointX,
                value: 2.0,
            },
            "the queued edit must keep its own opening id, not the shifted index"
        );
    }

    /// A fixture half pair whose linked mount is authored on the left half.
    fn pair_fixture_document(project_id: &str) -> ProjectDoc {
        use boardstudio_core::mechanical::features::shared_construction;
        use boardstudio_core::model::{
            HardwareConfiguration, HardwareTopology, HardwareTransport,
            MechanicalConstructionOwnership, MechanicalFeature, MechanicalFeatureKind,
            MechanicalFeatureMember, MechanicalFeatureState, MechanicalFeatureValue,
            MechanicalMirrorPair, Mount, MountKind, PhysicalBoardInstance, Pose2, Vec2,
        };
        let mut document = fixture_document(project_id);
        let mut configuration = document
            .mechanical
            .clone()
            .expect("the adapter fixture has a canonical configuration");
        let local_mount = |id: &str, x: f64| Mount {
            id: id.into(),
            at: Vec2 { x, y: 3.0 },
            kind: MountKind::Hole,
            hole_diameter: 2.2,
            boss_diameter: None,
            height: None,
        };
        configuration.mounts = vec![
            local_mount("mount-1-left", 2.0),
            local_mount("mount-1-right", -2.0),
        ];
        let configuration = configuration;
        let instance = |id: &str, half: &str, flipped: bool| PhysicalBoardInstance {
            id: id.into(),
            name: id.into(),
            board_id: "board".into(),
            half: half.into(),
            role: "custom-role".into(),
            flipped,
            controller_part_id: None,
            // A member's own configuration is the base the pair derives from, so a
            // resolved effective read needs it before linking.
            mechanical: Some(configuration.clone()),
            construction_ownership: MechanicalConstructionOwnership::Local,
            mechanical_identity_reservation: None,
        };
        let mount = |id: &str, x: f64| Mount {
            id: id.into(),
            at: Vec2 { x, y: 3.0 },
            kind: MountKind::Hole,
            hole_diameter: 2.2,
            boss_diameter: None,
            height: None,
        };
        let pair = MechanicalMirrorPair {
            id: "pair-1".into(),
            members: ["left".into(), "right".into()],
            mapping: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            construction: shared_construction(&configuration),
            construction_source: None,
            halves: None,
            features: vec![MechanicalFeature {
                id: "mount-1".into(),
                kind: MechanicalFeatureKind::Mount,
                author: "left".into(),
                value: MechanicalFeatureValue::Mount {
                    mount: mount("mount-1-left", 2.0),
                },
                members: vec![
                    MechanicalFeatureMember {
                        instance_id: "left".into(),
                        feature_id: "mount-1-left".into(),
                        state: MechanicalFeatureState::Local,
                    },
                    MechanicalFeatureMember {
                        instance_id: "right".into(),
                        feature_id: "mount-1-right".into(),
                        state: MechanicalFeatureState::Linked,
                    },
                ],
            }],
        };
        document.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Split,
            transport: HardwareTransport::None,
            instances: vec![
                instance("left", "left", false),
                instance("right", "right", true),
            ],
            boards: vec![],
            shared_construction: None,
            pairs: vec![pair],
        });
        document
    }

    #[wasm_bindgen_test]
    async fn a_reviewed_rejoin_tolerates_unrelated_edits_and_retires_on_relevant_ones() {
        let document = pair_fixture_document("adapter-reviewed-rejoin");
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let accepted = runtime.model().accepted.expect("accepted fixture");
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, Some("left"));
        let identity = current.identity.clone();
        let controller = controller(current, TestCatalogue::default(), port.clone(), feedback);

        let rejoin = MechanicalSettingsPatch::RejoinFeatureMember {
            feature_id: "mount-1-left".into(),
            instance_id: "left".into(),
            source_instance_id: "right".into(),
        };
        assert!(controller.submit(request(identity, 1, rejoin.clone())));
        loaded().await;
        let resolutions = port.resolutions.borrow();
        assert_eq!(resolutions.len(), 1, "the reviewed rejoin is submitted");
        let resolver = resolutions[0].clone();

        // Unrelated edits and a revision change keep the reviewed inputs valid.
        let mut unrelated = (*accepted.document).clone();
        unrelated.name = "Renamed".into();
        unrelated.revision += 5;
        unrelated
            .parameters
            .insert("unrelated".into(), serde_json::json!("keep"));
        let unrelated = AcceptedSnapshot {
            document: std::sync::Arc::new(unrelated),
            ..accepted.clone()
        };
        let resolution = resolver.resolve(&unrelated);
        assert!(
            matches!(resolution, Resolution::Submit(_)),
            "unrelated edits must not retire a reviewed rejoin, got {resolution:?}"
        );

        // A changed mapping is a relevant input: the review must be refreshed.
        let mut remapped = (*accepted.document).clone();
        remapped.hardware.as_mut().unwrap().pairs[0]
            .mapping
            .rotation = 90.0;
        let remapped = AcceptedSnapshot {
            document: std::sync::Arc::new(remapped),
            ..accepted.clone()
        };
        let resolution = resolver.resolve(&remapped);
        let Resolution::Retire(reason) = resolution else {
            panic!("a changed mapping must retire the review, got {resolution:?}");
        };
        assert!(reason.contains("reviewed inputs changed"), "{reason}");

        // A pair that is gone retires: the queued action can no longer be reviewed.
        let mut removed = (*accepted.document).clone();
        removed.hardware.as_mut().unwrap().pairs.clear();
        let removed = AcceptedSnapshot {
            document: std::sync::Arc::new(removed),
            ..accepted.clone()
        };
        let resolution = resolver.resolve(&removed);
        assert!(
            matches!(resolution, Resolution::Retire(_)),
            "a vanished pair retires the queued rejoin, got {resolution:?}"
        );

        // The reviewed submission still targets the instance and carries the change.
        let edit = submitted_edit(&resolver, &accepted);
        assert_eq!(
            edit.target,
            MechanicalTarget::Instance {
                instance_id: "left".into()
            }
        );
        assert_eq!(edit.change, rejoin);
    }

    #[wasm_bindgen_test]
    async fn a_stale_critical_fit_setter_cannot_mutate_the_monotonic_replacement() {
        use boardstudio_core::mechanical::change::{
            MechanicalChange, MechanicalCriticalFitTextField, MechanicalEdit,
        };
        use boardstudio_core::model::{EditCommand, EditPhase, MechanicalCriticalFit};

        fn command(base_revision: u64, transaction: &str, change: MechanicalChange) -> EditCommand {
            EditCommand {
                base_revision,
                transaction_id: transaction.into(),
                phase: EditPhase::Commit,
                target_ids: vec!["board".into()],
                operation: EditOperation::ApplyMechanicalEdit {
                    edit: Box::new(MechanicalEdit {
                        target: MechanicalTarget::Board {
                            board_id: "board".into(),
                        },
                        change,
                        mounting_hole: Some(Box::new(template())),
                    }),
                },
            }
        }

        let mut document = fixture_document("adapter-stale-fit");
        document.mechanical.as_mut().unwrap().critical_fits = Some(vec![MechanicalCriticalFit {
            id: "case-critical-fit-1".into(),
            part_id: "case-closure/part".into(),
            label: "Critical dimension".into(),
            from: boardstudio_core::model::Vec2 { x: 0.0, y: 0.0 },
            to: boardstudio_core::model::Vec2 { x: 10.0, y: 0.0 },
            tolerance: "±0.2 mm".into(),
        }]);
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let accepted = runtime.model().accepted.expect("accepted fixture");
        let port = RecordingPort::default();
        let feedback = Rc::new(RefCell::new(Vec::new()));
        let current = current(&accepted, None);
        let identity = current.identity.clone();
        let controller = controller(
            current,
            TestCatalogue::default(),
            port.clone(),
            feedback.clone(),
        );
        assert!(controller.submit(request(
            identity,
            1,
            MechanicalSettingsPatch::SetCriticalFitText {
                fit_id: "case-critical-fit-1".into(),
                field: MechanicalCriticalFitTextField::Label,
                value: "stale value".into(),
            },
        )));
        loaded().await;
        let resolver = port.resolutions.borrow()[0].clone();

        let revision = accepted.document.revision;
        support::submit_fixed_command(
            &runtime,
            command(
                revision,
                "remove-fit",
                MechanicalChange::RemoveCriticalFit {
                    fit_id: "case-critical-fit-1".into(),
                },
            ),
        );
        support::run_pending(&runtime).await;
        let revision = runtime
            .model()
            .accepted
            .expect("removal accepted")
            .document
            .revision;
        support::submit_fixed_command(
            &runtime,
            command(
                revision,
                "add-fit",
                MechanicalChange::AddCriticalFit {
                    part_id: "case-closure/part".into(),
                },
            ),
        );
        support::run_pending(&runtime).await;
        let accepted = runtime.model().accepted.expect("addition accepted");

        match resolver.resolve(&accepted) {
            Resolution::Retire(_) => {}
            Resolution::Submit(edit) => {
                support::submit_fixed_command(&runtime, edit);
                support::run_pending(&runtime).await;
            }
            other => panic!("unexpected stale resolution: {other:?}"),
        }

        let accepted = runtime.model().accepted.expect("accepted document");
        let fits = accepted
            .document
            .mechanical
            .as_ref()
            .unwrap()
            .critical_fits
            .as_ref()
            .unwrap();
        assert_eq!(fits.len(), 1);
        assert_eq!(fits[0].id, "case-critical-fit-2");
        assert_eq!(
            fits[0].label, "Critical dimension",
            "the stale setter must never change the replacement's value"
        );
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
