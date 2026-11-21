//! Pure encoder input projection: choices, editor rows, lineage and cache.
//!
//! This module compiles on every target so its synchronous tests run as
//! ordinary host `#[test]`. The wasm-only `keymap::encoder_inputs` keeps the
//! mounted hooks and current-plan wiring and reuses these names for them.
use crate::binding::{BindingLayerChoice, BindingMacroChoice, EncoderInputIdentity};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core_mechanical::{
    electrical::ElectricalPlan, electrical_peripherals::PeripheralRequirement,
};
use boardstudio_model::model::{ProjectDoc, RotaryProfile};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderInputChoice {
    pub id: Rc<str>,
    pub label: Rc<str>,
    pub push_key_id: Option<Rc<str>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderInputProjection {
    pub identity: EncoderInputIdentity,
    pub encoders: Rc<[EncoderInputChoice]>,
    /// True once the F5 board plan for this accepted snapshot has settled.
    pub electrical_plan_settled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderBindingRow {
    pub id: Rc<str>,
    pub label: Rc<str>,
    pub clockwise: boardstudio_model::model::KeyBinding,
    pub counterclockwise: boardstudio_model::model::KeyBinding,
    pub push_key_id: Option<Rc<str>>,
    pub push_binding: Option<boardstudio_model::model::KeyBinding>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderEditorProjection {
    pub input_identity: EncoderInputIdentity,
    pub effective_layer_id: String,
    pub rows: Rc<[EncoderBindingRow]>,
    pub layers: Rc<[BindingLayerChoice]>,
    pub macros: Rc<[BindingMacroChoice]>,
}

#[derive(Clone, PartialEq)]
pub(crate) struct InputLineage {
    pub(crate) scope: Scope,
    pub(crate) fingerprint: Option<String>,
    pub(crate) physical: Vec<PeripheralRequirement>,
    pub(crate) modules: Vec<(String, String, Option<String>, RotaryProfile)>,
    pub(crate) rows: Vec<EncoderInputChoice>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InputSource {
    pub(crate) scope: Scope,
    pub(crate) token: SnapshotToken,
    pub(crate) revision: u64,
    pub(crate) fingerprint: Option<String>,
    pub(crate) plan_settled: bool,
}

#[derive(Default)]
pub(crate) struct InputCache {
    pub(crate) source: Option<InputSource>,
    pub(crate) lineage: Option<InputLineage>,
    pub(crate) rows: Rc<[EncoderInputChoice]>,
    pub(crate) generation: u64,
}

pub(crate) fn update_cache(
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

pub(crate) fn plan_scope_matches(
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

pub(crate) fn same_input_lineage(
    captured: &EncoderInputIdentity,
    current: &EncoderInputIdentity,
) -> bool {
    captured.scope == current.scope
        && captured.projection_generation == current.projection_generation
        && captured.electrical_fingerprint == current.electrical_fingerprint
}

pub(crate) fn inputs(
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

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;

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

    #[test]
    fn encoder_lineage_survives_own_snapshot_advance_but_not_input_replacement() {
        let scope = Scope {
            session_epoch: SessionEpoch(4),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: Some("instance".into()),
        };
        let captured = EncoderInputIdentity {
            scope: scope.clone(),
            token: SnapshotToken(8),
            revision: 12,
            projection_generation: 21,
            electrical_fingerprint: Some("f5-layout".into()),
        };
        let after_own_edit = EncoderInputIdentity {
            token: SnapshotToken(9),
            revision: 13,
            ..captured.clone()
        };
        let replaced_inputs = EncoderInputIdentity {
            projection_generation: 22,
            ..after_own_edit.clone()
        };

        assert!(same_input_lineage(&captured, &after_own_edit));
        assert!(!same_input_lineage(&captured, &replaced_inputs));
    }

    #[test]
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

    #[test]
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

    #[test]
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

    #[test]
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

    #[test]
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
