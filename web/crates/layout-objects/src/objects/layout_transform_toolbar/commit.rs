//! Single-ticket transform commits without polling: begin intent, settle on the
//! production `version` signal, release observers on retirement.
//!
//! The rail drags, nudges and Anchor picks used temporary unit-keyed
//! `PendingEdits<()>` maps with a 16 ms settlement loop. Each commit below is a
//! single [`EditTicket`]: resolvers already compose queued edits against the
//! accepted snapshot at execution time (ADR-0005), so no keyed map or polling
//! loop is needed. Pending tickets live in a component `Signal<Vec<...>>` and
//! settle in a `use_effect` on the workbench `version` signal
//! (`use_workbench_panels` provides it; browser test hosts provide it the same
//! way via a runtime subscription). Terminal tickets are dropped, which
//! releases their outcome/landing observers without cancelling the Session
//! operation.
//!
//! Durable completion: a `Landed` ticket means the edit is in the accepted
//! document; `Failed` carries the exact save/refusal wording and is surfaced
//! with `Runtime::report`; `Retired` (scope moved on, owner gone, Session
//! superseded/cancelled/closed) is silent. Owner liveness for settlement is
//! scope + workspace + generation + selection only: token/revision and
//! lifecycle/preview/gesture are deliberately not checked, so a commit queued
//! behind an Applying/Saving Session still lands instead of retiring.

use super::*;
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};

/// The workbench `version` signal every `Runtime::changed` bumps
/// (`use_workbench_runtime` subscribes the runtime; `use_workbench_panels`
/// provides it). Overlays settle on it; there is deliberately no local
/// fallback — without a publisher settlement would silently never run, which is
/// worse than the panic that surfaces a missing provider.
pub(super) fn transform_version() -> Signal<u64> {
    use_context::<Signal<u64>>()
}

/// Begin one transform intent through the shared ticket route.
pub(super) fn begin_transform_ticket(
    runtime: &Rc<Runtime>,
    label: &'static str,
    resolver: EditResolver,
) -> EditTicket {
    EditTicket::begin(runtime, label, Some("transform".into()), resolver)
}

/// Whether a committed transform still owns its feedback: same scope,
/// workspace and generation, with the selection still on its context.
/// Token, revision, lifecycle, preview and gesture are not checked, so a
/// commit queued behind earlier edits still lands instead of retiring.
pub(super) fn transform_commit_is_live(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    owner: &LayoutOwnerIdentity,
    context: &TreeContext,
) -> bool {
    owner.workspace == "Layout"
        && workspace() == owner.workspace
        && scope_generation() == owner.generation
        && runtime.scope() == owner.scope
        && owner.scope.as_ref().is_some_and(|scope| {
            selected_context()
                .is_some_and(|selected| selected.scope == *scope && selected.context == *context)
        })
}

/// One queued transform commit with the owner/context its feedback needs.
/// The toast is raised only on `Landed`; failures report; retirement is silent.
#[derive(Clone)]
pub(super) struct PendingTransformCommit {
    pub(super) ticket: EditTicket,
    pub(super) owner: LayoutOwnerIdentity,
    pub(super) context: TreeContext,
    pub(super) toast: Option<PendingTransformToast>,
}

/// The durable toast for a placement drag, raised once its commit lands.
#[derive(Clone)]
pub(super) struct PendingTransformToast {
    pub(super) tool_name: &'static str,
    pub(super) value: String,
    pub(super) delta: FeedbackDelta,
}

/// Raise one durable commit toast and arm its 1.8 s expiry: the shared tail
/// of `settle_transform_commits`, the Anchor follow-up and the detached part
/// `landed` closure. The caller owns the `Commit` value/delta wording.
pub(super) fn raise_commit_toast(
    mut feedback: Signal<FeedbackState>,
    mut toast_gen: Signal<u64>,
    value: String,
    delta: FeedbackDelta,
) {
    feedback.set(advance_feedback(
        &feedback(),
        FeedbackEvent::Commit { value, delta },
    ));
    let seen = {
        toast_gen.set(toast_gen() + 1);
        toast_gen()
    };
    arm_commit_toast_expiry(feedback, toast_gen, seen);
}

/// Settle fire-and-forget commits (drags, nudges): drop terminal tickets to
/// release their observers, raise durable toasts on landing, report failures.
/// `peek` (not `read`) keeps the settlement effect subscribed to `version`
/// only; `pending` is written only when at least one ticket settles, so
/// version bumps while Core/save holds every commit (all `Pending`) neither
/// write nor re-trigger the effect.
pub(super) fn settle_transform_commits(
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    mut pending: Signal<Vec<PendingTransformCommit>>,
    feedback: Signal<FeedbackState>,
    toast_gen: Signal<u64>,
) {
    let queued = pending.peek().clone();
    if queued.is_empty() {
        return;
    }
    let mut remaining = Vec::with_capacity(queued.len());
    let mut settled_any = false;
    for commit in queued {
        let live = transform_commit_is_live(
            runtime,
            workspace,
            scope_generation,
            selected_context,
            &commit.owner,
            &commit.context,
        );
        match commit.ticket.settlement(live) {
            Settlement::Pending => remaining.push(commit),
            Settlement::Landed { .. } => {
                settled_any = true;
                if let Some(toast) = commit.toast {
                    raise_commit_toast(
                        feedback,
                        toast_gen,
                        format!("{} {}", toast.tool_name, toast.value),
                        toast.delta,
                    );
                }
            }
            Settlement::Failed { message } => {
                settled_any = true;
                runtime.report(message);
            }
            Settlement::Retired => {
                settled_any = true;
            }
        }
    }
    if settled_any {
        pending.set(remaining);
    }
}

/// Shared settlement effect for the `Vec` pending queues (matrix + part rail).
/// The component owns `pending`/`feedback`/`toast_gen`; settlement itself runs
/// on the production `version` signal.
pub(super) fn use_settled_transform_commits(
    runtime: Rc<Runtime>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    pending: Signal<Vec<PendingTransformCommit>>,
    feedback: Signal<FeedbackState>,
    toast_gen: Signal<u64>,
) {
    let version = transform_version();
    use_effect(use_reactive((&version(),), {
        move |_| {
            settle_transform_commits(
                &runtime,
                workspace,
                scope_generation,
                selected_context,
                pending,
                feedback,
                toast_gen,
            );
        }
    }));
}
