//! Private owner for the Case mechanical-settings field requests.
//!
//! Mounting-hole catalogue preparation precedes a synchronous typed edit.
use super::mechanical_settings::{
    MechanicalSettingsFeedback, MechanicalSettingsFeedbackState, MechanicalSettingsIdentity,
    MechanicalSettingsPatch, MechanicalSettingsPatchExt, MechanicalSettingsRequest,
};
use boardstudio_application::{AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution};
use boardstudio_core_mechanical::mechanical::change::{MechanicalEdit, MechanicalTarget};
use boardstudio_core_mechanical::mechanical::{MechanicalEditDisposition, evaluate_edit};
use boardstudio_model::model::{
    EditOperation, MechanicalAssembly, MechanicalConfiguration, PartDefinition,
};
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
    pub commit: Rc<dyn Fn(EditResolver) -> Result<AcceptedSnapshot, String>>,
    pub publish: Rc<dyn Fn(MechanicalSettingsFeedback)>,
}

pub struct MechanicalSettingsController {
    ports: MechanicalSettingsPorts,
    template: RefCell<Option<Rc<PartDefinition>>>,
    loading: Cell<bool>,
    last_request_id: Cell<u64>,
}

impl MechanicalSettingsController {
    pub fn new(ports: MechanicalSettingsPorts) -> Rc<Self> {
        Rc::new(Self {
            ports,
            template: RefCell::default(),
            loading: Cell::new(false),
            last_request_id: Cell::new(0),
        })
    }

    pub fn is_busy(&self) -> bool {
        self.loading.get()
    }

    pub fn submit(self: &Rc<Self>, request: MechanicalSettingsRequest) -> bool {
        if request.request_id <= self.last_request_id.get() || self.loading.get() {
            return false;
        }
        let Some(current) = (self.ports.current)() else {
            return false;
        };
        if !admitted(&current, &request.identity) || request.field_id != request.patch.field_id() {
            return false;
        }
        self.last_request_id.set(request.request_id);
        let review = reviewed_pair_guard(
            &request.patch,
            request.reviewed_fingerprint.clone(),
            &current.accepted,
        );
        if let Some(template) = self.template.borrow().clone() {
            self.commit(request, template, review);
        } else {
            self.loading.set(true);
            self.emit(&request, MechanicalSettingsFeedbackState::Pending, None);
            let owner = Rc::downgrade(self);
            let load = self.ports.load_mounting_hole.clone();
            spawn_local(async move {
                let result = load().await;
                let Some(owner) = owner.upgrade() else {
                    return;
                };
                owner.loading.set(false);
                if let Ok(template) = &result {
                    owner.template.replace(Some(template.clone()));
                }
                if !(owner.ports.current)()
                    .is_some_and(|current| admitted(&current, &request.identity))
                {
                    return;
                }
                match result {
                    Ok(template) => owner.commit(request, template, review),
                    Err(message) => owner.emit(
                        &request,
                        MechanicalSettingsFeedbackState::Failed,
                        Some(message),
                    ),
                }
            });
        }
        true
    }

    fn commit(
        &self,
        request: MechanicalSettingsRequest,
        template: Rc<PartDefinition>,
        review: Option<ReviewedPairGuard>,
    ) {
        let intent = request.clone();
        let resolver =
            EditResolver::new("mechanical-settings", move |accepted: &AcceptedSnapshot| {
                if let Some(review) = &review
                    && let Err(reason) = review.check(accepted)
                {
                    return Resolution::Retire(reason);
                }
                resolve_mechanical_edit(&intent, &template, accepted)
            });
        match (self.ports.commit)(resolver) {
            Ok(_) => self.emit(&request, MechanicalSettingsFeedbackState::Landed, None),
            Err(message) => self.emit(
                &request,
                MechanicalSettingsFeedbackState::Failed,
                Some(message),
            ),
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
