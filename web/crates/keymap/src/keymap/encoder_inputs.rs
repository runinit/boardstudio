//! Accepted-document and current F5-plan inputs for the bounded encoder editor.
use super::super::pcb_wiring::{PcbWiringResolution, WiringPlanIdentity};
use super::binding_editor::EncoderInputIdentity;
use super::layer_controller::LayerSource;
use crate::encoder::{
    EncoderInputProjection, InputCache, InputSource, inputs, plan_scope_matches, update_cache,
};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core_mechanical::electrical::ElectricalPlan;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub struct EncoderInputActions {
    pub projection: Memo<Option<EncoderInputProjection>>,
    pub current: Rc<dyn Fn() -> Option<EncoderInputProjection>>,
}

fn plan_identity_matches(
    runtime: &Runtime,
    scope: &Scope,
    snapshot: &AcceptedSnapshot,
    identity: &WiringPlanIdentity,
    plan: &ElectricalPlan,
) -> bool {
    let model = runtime.model();
    plan_scope_matches(
        scope,
        &identity.scope,
        &model.active_board_id,
        model.active_instance_id.as_deref(),
    ) && identity.token == snapshot.token
        && identity.revision == snapshot.document.revision
        && scope.session_epoch == snapshot.session_epoch
        && scope.document_id == snapshot.document.id
        && model.active_board_id == scope.board_id
        && model.active_instance_id == scope.instance_id
        && plan.instance_id.is_none()
        && plan.board_id.as_deref() == Some(scope.board_id.as_str())
        && plan.revision == snapshot.document.revision
}

fn current_plan(
    runtime: &Runtime,
    scope: &Scope,
    snapshot: &AcceptedSnapshot,
    resolution: &PcbWiringResolution,
) -> (Option<Rc<ElectricalPlan>>, Option<String>, bool) {
    match resolution {
        PcbWiringResolution::Current { identity, plan }
            if plan_identity_matches(runtime, scope, snapshot, identity, plan) =>
        {
            (Some(plan.clone()), Some(plan.fingerprint.clone()), true)
        }
        PcbWiringResolution::Failed { identity, .. }
            if failed_identity_matches(runtime, scope, snapshot, identity) =>
        {
            (None, None, true)
        }
        _ => (None, None, false),
    }
}

fn failed_identity_matches(
    runtime: &Runtime,
    scope: &Scope,
    snapshot: &AcceptedSnapshot,
    identity: &WiringPlanIdentity,
) -> bool {
    let model = runtime.model();
    plan_scope_matches(
        scope,
        &identity.scope,
        &model.active_board_id,
        model.active_instance_id.as_deref(),
    ) && identity.token == snapshot.token
        && identity.revision == snapshot.document.revision
        && scope.session_epoch == snapshot.session_epoch
        && scope.document_id == snapshot.document.id
}

fn current_projection(
    runtime: &Runtime,
    cache: &RefCell<InputCache>,
    resolution: &Signal<PcbWiringResolution>,
) -> Option<EncoderInputProjection> {
    let model = runtime.model();
    let mut cache = cache.borrow_mut();
    let Some(scope) = runtime.scope() else {
        cache.source = None;
        cache.lineage = None;
        cache.rows = Rc::from([]);
        return None;
    };
    let snapshot = model.accepted.as_ref()?;
    if scope.session_epoch != snapshot.session_epoch
        || scope.document_id != snapshot.document.id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
    {
        return None;
    }
    if cache
        .lineage
        .as_ref()
        .is_some_and(|lineage| lineage.scope != scope)
    {
        cache.source = None;
        cache.lineage = None;
        cache.rows = Rc::from([]);
    }
    let (plan, fingerprint, electrical_plan_settled) =
        current_plan(runtime, &scope, snapshot, &resolution.read());
    let source = InputSource {
        scope: scope.clone(),
        token: snapshot.token,
        revision: snapshot.document.revision,
        fingerprint: fingerprint.clone(),
        plan_settled: electrical_plan_settled,
    };
    let settled_lineage = if plan.is_some() || electrical_plan_settled || cache.lineage.is_none() {
        Some(inputs(
            &snapshot.document,
            scope.clone(),
            plan.as_deref(),
            fingerprint.clone(),
        ))
    } else {
        None
    };
    // The accepted edit temporarily invalidates F5's old plan. Hide physical
    // rows while pending, but preserve its last validated lineage for the
    // asynchronous Current/Failed transition.
    let pending_rows = Rc::from(inputs(&snapshot.document, scope.clone(), None, None).rows);
    update_cache(&mut cache, source, settled_lineage, pending_rows)?;
    let lineage_fingerprint = if electrical_plan_settled {
        fingerprint
    } else {
        cache
            .lineage
            .as_ref()
            .and_then(|lineage| lineage.fingerprint.clone())
    };
    Some(EncoderInputProjection {
        identity: EncoderInputIdentity {
            scope,
            token: snapshot.token,
            revision: snapshot.document.revision,
            projection_generation: cache.generation,
            electrical_fingerprint: lineage_fingerprint,
        },
        encoders: cache.rows.clone(),
        electrical_plan_settled,
    })
}

pub fn use_encoder_inputs(
    runtime: Rc<Runtime>,
    source: Option<LayerSource>,
    resolution: Signal<PcbWiringResolution>,
) -> EncoderInputActions {
    let cache = use_hook(|| Rc::new(RefCell::new(InputCache::default())));
    let current = use_hook(move || {
        Rc::new(move || current_projection(&runtime, &cache, &resolution))
            as Rc<dyn Fn() -> Option<EncoderInputProjection>>
    });
    let scope = source.as_ref().map(|source| source.scope.clone());
    let token = source.as_ref().map(|source| source.token);
    let revision = source.as_ref().map(|source| source.revision);
    let resolution_value = resolution.read().clone();
    let projection = use_memo(use_reactive(
        (&scope, &token, &revision, &resolution_value),
        {
            let current = current.clone();
            move |(scope, token, revision, _resolution)| {
                let projection = current()?;
                (scope.as_ref() == Some(&projection.identity.scope)
                    && token == Some(projection.identity.token)
                    && revision == Some(projection.identity.revision))
                .then_some(projection)
            }
        },
    ));
    EncoderInputActions {
        projection,
        current,
    }
}
