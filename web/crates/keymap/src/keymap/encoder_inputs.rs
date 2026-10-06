//! Accepted-document and current F5-plan inputs for the bounded encoder editor.
use super::super::pcb_wiring::{PcbWiringResolution, WiringPlanIdentity};
use super::binding_controller::{EncoderInputChoice, EncoderInputProjection};
use super::binding_editor::EncoderInputIdentity;
use super::layer_controller::LayerSource;
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::{
    electrical::ElectricalPlan,
    electrical_peripherals::PeripheralRequirement,
    model::{ProjectDoc, RotaryProfile},
};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, PartialEq)]
struct InputLineage {
    scope: Scope,
    fingerprint: Option<String>,
    physical: Vec<PeripheralRequirement>,
    modules: Vec<(String, String, Option<String>, RotaryProfile)>,
    rows: Vec<EncoderInputChoice>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InputSource {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    fingerprint: Option<String>,
    plan_settled: bool,
}

#[derive(Default)]
struct InputCache {
    source: Option<InputSource>,
    lineage: Option<InputLineage>,
    rows: Rc<[EncoderInputChoice]>,
    generation: u64,
}

fn update_cache(
    cache: &mut InputCache,
    source: InputSource,
    settled_lineage: Option<InputLineage>,
    pending_rows: Rc<[EncoderInputChoice]>,
) -> Option<()> {
    if cache.source.as_ref() == Some(&source) {
        return Some(());
    }
    if let Some(lineage) = settled_lineage {
        if cache.lineage.as_ref() != Some(&lineage) {
            cache.generation = cache.generation.checked_add(1)?;
            cache.lineage = Some(lineage.clone());
        }
        // A pending plan temporarily replaces visible rows with module rows.
        // Always restore the settled projection, even when its lineage is the
        // same one retained across that pending interval.
        cache.rows = Rc::from(lineage.rows);
    } else {
        cache.rows = pending_rows;
    }
    cache.source = Some(source);
    Some(())
}

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
        && identity.executor_epoch == runtime.electrical_preview_executor_epoch()
        && scope.session_epoch == snapshot.session_epoch
        && scope.document_id == snapshot.document.id
        && model.active_board_id == scope.board_id
        && model.active_instance_id == scope.instance_id
        && plan.instance_id.is_none()
        && plan.board_id.as_deref() == Some(scope.board_id.as_str())
        && plan.revision == snapshot.document.revision
}

fn plan_scope_matches(
    selection_scope: &Scope,
    plan_scope: &Scope,
    active_board_id: &str,
    active_instance_id: Option<&str>,
) -> bool {
    *plan_scope
        == (Scope {
            instance_id: None,
            ..selection_scope.clone()
        })
        && active_board_id == selection_scope.board_id
        && active_instance_id == selection_scope.instance_id.as_deref()
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
        && identity.executor_epoch == runtime.electrical_preview_executor_epoch()
        && scope.session_epoch == snapshot.session_epoch
        && scope.document_id == snapshot.document.id
}

fn inputs(
    document: &ProjectDoc,
    scope: Scope,
    plan: Option<&ElectricalPlan>,
    fingerprint: Option<String>,
) -> InputLineage {
    let physical: Vec<_> = plan
        .into_iter()
        .flat_map(|plan| plan.peripherals.iter())
        .filter(|requirement| requirement.kind == "encoder" && requirement.rotary.is_some())
        .cloned()
        .collect();
    let mut rows: Vec<_> = physical
        .iter()
        .map(|requirement| EncoderInputChoice {
            id: Rc::from(requirement.part_id.as_str()),
            label: Rc::from(
                document
                    .parts
                    .iter()
                    .find(|part| part.id == requirement.part_id)
                    .map_or(requirement.part_id.as_str(), |part| part.reference.as_str()),
            ),
            // The provider reports this ID; the frontend never invents it.
            push_key_id: requirement.press_key_id.as_deref().map(Rc::from),
        })
        .collect();
    let mut modules = Vec::new();
    for module in &document.modules {
        if module.host_board_id != scope.board_id || module.detached {
            continue;
        }
        let Some(definition) = document
            .module_definitions
            .iter()
            .find(|definition| definition.id == module.definition_id)
        else {
            continue;
        };
        if definition.catalogue_row.as_deref() != Some("ec11-evqwgd001") {
            continue;
        }
        let Some(profile) = definition.electrical.rotary_profile.as_ref() else {
            continue;
        };
        if rows.iter().any(|row| row.id.as_ref() == module.id) {
            continue;
        }
        modules.push((
            module.id.clone(),
            module.definition_id.clone(),
            module.host_instance_id.clone(),
            profile.clone(),
        ));
        rows.push(EncoderInputChoice {
            id: Rc::from(module.id.as_str()),
            label: Rc::from(definition.name.as_str()),
            push_key_id: None,
        });
    }
    InputLineage {
        scope,
        fingerprint,
        physical,
        modules,
        rows,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn scope(instance_id: Option<&str>) -> Scope {
        Scope {
            session_epoch: SessionEpoch(3),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: instance_id.map(str::to_owned),
        }
    }

    fn choice(id: &'static str) -> EncoderInputChoice {
        EncoderInputChoice {
            id: Rc::from(id),
            label: Rc::from(id),
            push_key_id: None,
        }
    }

    fn lineage(scope: &Scope, fingerprint: Option<&str>, ids: &[&'static str]) -> InputLineage {
        InputLineage {
            scope: scope.clone(),
            fingerprint: fingerprint.map(str::to_owned),
            physical: vec![],
            modules: vec![],
            rows: ids.iter().map(|id| choice(id)).collect(),
        }
    }

    fn source(
        scope: &Scope,
        revision: u64,
        fingerprint: Option<&str>,
        settled: bool,
    ) -> InputSource {
        InputSource {
            scope: scope.clone(),
            token: SnapshotToken(revision),
            revision,
            fingerprint: fingerprint.map(str::to_owned),
            plan_settled: settled,
        }
    }

    fn row_ids(rows: &[EncoderInputChoice]) -> Vec<&str> {
        rows.iter().map(|row| row.id.as_ref()).collect()
    }

    #[wasm_bindgen_test]
    fn board_plan_scope_is_admitted_for_the_selected_instance_only() {
        let selection_scope = scope(Some("instance-a"));
        let board_scope = scope(None);
        assert!(plan_scope_matches(
            &selection_scope,
            &board_scope,
            "board",
            Some("instance-a")
        ));
        assert!(!plan_scope_matches(
            &selection_scope,
            &scope(Some("instance-a")),
            "board",
            Some("instance-a")
        ));
        assert!(!plan_scope_matches(
            &selection_scope,
            &board_scope,
            "another-board",
            Some("instance-a")
        ));
    }

    #[wasm_bindgen_test]
    fn plan_arrival_without_revision_restores_physical_rows_and_module_fallback() {
        let scope = scope(None);
        let mut cache = InputCache::default();
        let modules: Rc<[EncoderInputChoice]> = Rc::from([choice("module-encoder")]);
        update_cache(
            &mut cache,
            source(&scope, 8, None, false),
            Some(lineage(&scope, None, &["module-encoder"])),
            modules.clone(),
        )
        .unwrap();
        assert_eq!(row_ids(&cache.rows), vec!["module-encoder"]);

        update_cache(
            &mut cache,
            source(&scope, 8, Some("plan-a"), true),
            Some(lineage(
                &scope,
                Some("plan-a"),
                &["physical-encoder", "module-encoder"],
            )),
            modules,
        )
        .unwrap();

        assert_eq!(
            row_ids(&cache.rows),
            vec!["physical-encoder", "module-encoder"]
        );
        assert_eq!(cache.generation, 2);
    }

    #[wasm_bindgen_test]
    fn own_edit_pending_then_same_plan_restores_rows_without_changing_lineage() {
        let scope = scope(None);
        let mut cache = InputCache::default();
        let module_rows: Rc<[EncoderInputChoice]> = Rc::from([choice("module-encoder")]);
        let current = lineage(
            &scope,
            Some("plan-a"),
            &["physical-encoder", "module-encoder"],
        );
        update_cache(
            &mut cache,
            source(&scope, 8, Some("plan-a"), true),
            Some(current.clone()),
            module_rows.clone(),
        )
        .unwrap();
        let generation = cache.generation;

        update_cache(
            &mut cache,
            source(&scope, 9, None, false),
            None,
            module_rows.clone(),
        )
        .unwrap();
        assert_eq!(row_ids(&cache.rows), vec!["module-encoder"]);

        update_cache(
            &mut cache,
            source(&scope, 9, Some("plan-a"), true),
            Some(lineage(
                &scope,
                Some("plan-a"),
                &["physical-encoder", "module-encoder"],
            )),
            module_rows,
        )
        .unwrap();
        assert_eq!(
            row_ids(&cache.rows),
            vec!["physical-encoder", "module-encoder"]
        );
        assert_eq!(cache.generation, generation);
    }

    #[wasm_bindgen_test]
    fn pending_to_failed_plan_clears_physical_rows_even_without_a_fingerprint() {
        let scope = scope(None);
        let mut cache = InputCache::default();
        let module_rows: Rc<[EncoderInputChoice]> = Rc::from([choice("module-encoder")]);
        update_cache(
            &mut cache,
            source(&scope, 8, Some("plan-a"), true),
            Some(lineage(
                &scope,
                Some("plan-a"),
                &["physical-encoder", "module-encoder"],
            )),
            module_rows.clone(),
        )
        .unwrap();
        update_cache(
            &mut cache,
            source(&scope, 9, None, false),
            None,
            module_rows.clone(),
        )
        .unwrap();
        update_cache(
            &mut cache,
            source(&scope, 9, None, true),
            Some(lineage(&scope, None, &["module-encoder"])),
            module_rows,
        )
        .unwrap();
        assert_eq!(row_ids(&cache.rows), vec!["module-encoder"]);
    }

    #[wasm_bindgen_test]
    fn replacement_fingerprint_replaces_same_revision_lineage() {
        let scope = scope(None);
        let mut cache = InputCache::default();
        let fallback: Rc<[EncoderInputChoice]> = Rc::from([]);
        update_cache(
            &mut cache,
            source(&scope, 8, Some("plan-a"), true),
            Some(lineage(&scope, Some("plan-a"), &["encoder-a"])),
            fallback.clone(),
        )
        .unwrap();
        let generation = cache.generation;
        update_cache(
            &mut cache,
            source(&scope, 8, Some("plan-b"), true),
            Some(lineage(&scope, Some("plan-b"), &["encoder-b"])),
            fallback,
        )
        .unwrap();
        assert_eq!(row_ids(&cache.rows), vec!["encoder-b"]);
        assert_eq!(cache.generation, generation + 1);
    }
}
