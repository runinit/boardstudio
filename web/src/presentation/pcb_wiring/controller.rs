//! Editor-lifetime owner for automatic, board-scoped electrical plan resolution.
//! The selected part is intentionally absent from this controller's request identity.
use super::{PcbWiringResolution, WiringPlanIdentity};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Lifecycle, Scope};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
pub(in crate::presentation) struct PcbWiringMount {
    pub resolution: PcbWiringResolution,
    pub on_resolve: EventHandler<()>,
}

/// Keep the board-plan query alive at Editor lifetime, regardless of the selected component
/// or which workspace is currently visible. Call this hook unconditionally in the page parent.
pub(in crate::presentation) fn use_pcb_wiring_controller(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
) -> PcbWiringMount {
    let resolution = use_signal(|| PcbWiringResolution::Idle);
    let latest_request = use_hook(|| Rc::new(Cell::new(0_u64)));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });

    // Runtime is deliberately not a Dioxus signal. The page's existing version signal wakes
    // this projection after Runtime notifications; identical board identity is deduplicated.
    let _runtime_version = version();
    let identity = current_input(&runtime).map(|(identity, _, _)| identity);
    use_effect(use_reactive((&identity, &_runtime_version), {
        let runtime = runtime.clone();
        let mut resolution = resolution;
        let latest_request = latest_request.clone();
        let alive = alive.clone();
        move |(identity, _version)| {
            if identity.is_some() {
                start_resolution(
                    runtime.clone(),
                    resolution,
                    latest_request.clone(),
                    alive.clone(),
                    false,
                );
            } else {
                invalidate_pending(latest_request.as_ref());
                resolution.set(PcbWiringResolution::Idle);
            }
        }
    }));

    let on_resolve = use_callback({
        let runtime = runtime.clone();
        let latest_request = latest_request.clone();
        let alive = alive.clone();
        move |()| {
            start_resolution(
                runtime.clone(),
                resolution,
                latest_request.clone(),
                alive.clone(),
                true,
            );
        }
    });
    PcbWiringMount {
        resolution: resolution(),
        on_resolve,
    }
}

fn current_input(runtime: &Runtime) -> Option<(WiringPlanIdentity, AcceptedSnapshot, Scope)> {
    let model = runtime.model();
    if matches!(
        model.lifecycle,
        Lifecycle::Empty | Lifecycle::Opening | Lifecycle::Closing | Lifecycle::Closed
    ) {
        return None;
    }
    let accepted = model.accepted?;
    let mut scope = runtime.scope()?;
    // The electrical preview is a board plan with React's instanceId=null. Normalize away a
    // physical-instance selection so Case/Layout instance changes don't churn the board query.
    scope.instance_id = None;
    if scope.session_epoch != accepted.session_epoch
        || scope.document_id != accepted.document.id
        || model.active_board_id != scope.board_id
        || accepted.scene.revision != accepted.document.revision
        || !accepted
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
    {
        return None;
    }
    let identity = WiringPlanIdentity {
        scope: scope.clone(),
        token: accepted.token,
        revision: accepted.document.revision,
        executor_epoch: runtime.electrical_preview_executor_epoch(),
    };
    Some((identity, accepted, scope))
}

fn start_resolution(
    runtime: Rc<Runtime>,
    mut resolution: Signal<PcbWiringResolution>,
    latest_request: Rc<Cell<u64>>,
    alive: Rc<Cell<bool>>,
    force: bool,
) {
    if !alive.get() {
        return;
    }
    let Some((identity, accepted, scope)) = current_input(&runtime) else {
        invalidate_pending(&latest_request);
        resolution.set(PcbWiringResolution::Idle);
        return;
    };
    match &*resolution.read() {
        PcbWiringResolution::Pending { identity: pending } if pending == &identity => return,
        PcbWiringResolution::Current {
            identity: current, ..
        }
        | PcbWiringResolution::Failed {
            identity: current, ..
        } if !force && current == &identity => return,
        _ => {}
    }
    let request_generation = next_request(latest_request.as_ref());
    resolution.set(PcbWiringResolution::Pending {
        identity: identity.clone(),
    });
    spawn_local(async move {
        let mut resolution = resolution;
        let result = runtime.resolve_electrical_preview(accepted, scope).await;
        if !alive.get() || latest_request.get() != request_generation {
            return;
        }
        let Some((current, _, _)) = current_input(&runtime) else {
            resolution.set(PcbWiringResolution::Idle);
            return;
        };
        if current != identity {
            return;
        }
        resolution.set(match result {
            Ok(plan) => PcbWiringResolution::Current {
                identity,
                plan: Rc::new(plan),
            },
            Err(message) => PcbWiringResolution::Failed { identity, message },
        });
    });
}

fn next_request(sequence: &Cell<u64>) -> u64 {
    let request = sequence
        .get()
        .checked_add(1)
        .expect("PCB wiring request sequence exhausted");
    sequence.set(request);
    request
}

fn invalidate_pending(sequence: &Cell<u64>) {
    let _ = next_request(sequence);
}
