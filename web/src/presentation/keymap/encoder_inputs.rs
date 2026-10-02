//! Accepted-document inputs for the bounded encoder editor.
//!
//! Physical rows use Core's existing peripheral description. This is not the
//! complete F5 electrical-plan handoff; its fingerprint remains absent until
//! that integration supplies one. Attached module visibility follows React.
use super::binding_controller::{EncoderInputChoice, EncoderInputProjection};
use super::binding_editor::EncoderInputIdentity;
use super::layer_controller::LayerSource;
use crate::runtime::Runtime;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::{
    electrical_peripherals::{PeripheralRequirement, describe},
    model::{ProjectDoc, RotaryProfile},
};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(PartialEq)]
struct InputLineage {
    scope: Scope,
    physical: Vec<PeripheralRequirement>,
    modules: Vec<(String, String, Option<String>, RotaryProfile)>,
    rows: Vec<EncoderInputChoice>,
}

#[derive(Default)]
struct InputCache {
    source: Option<(Scope, SnapshotToken, u64)>,
    lineage: Option<InputLineage>,
    rows: Rc<[EncoderInputChoice]>,
    generation: u64,
}

pub(in crate::presentation) struct EncoderInputActions {
    pub(in crate::presentation) projection: Memo<Option<EncoderInputProjection>>,
    pub(in crate::presentation) current: Rc<dyn Fn() -> Option<EncoderInputProjection>>,
}

fn inputs(document: &ProjectDoc, scope: Scope) -> InputLineage {
    let physical: Vec<_> = describe(document, &scope.board_id)
        .into_iter()
        .filter(|requirement| requirement.kind == "encoder" && requirement.rotary.is_some())
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
        physical,
        modules,
        rows,
    }
}

fn current_projection(
    runtime: &Runtime,
    cache: &RefCell<InputCache>,
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
    let source = (scope.clone(), snapshot.token, snapshot.document.revision);
    if cache.source.as_ref() != Some(&source) {
        let lineage = inputs(&snapshot.document, scope.clone());
        if cache.lineage.as_ref() != Some(&lineage) {
            cache.generation = cache.generation.checked_add(1)?;
            cache.rows = Rc::from(lineage.rows.clone());
            cache.lineage = Some(lineage);
        }
        cache.source = Some(source);
    }
    Some(EncoderInputProjection {
        identity: EncoderInputIdentity {
            scope,
            token: snapshot.token,
            revision: snapshot.document.revision,
            projection_generation: cache.generation,
            // The real F5 integration must include its current fingerprint in
            // both this live query and cache source key, not a render snapshot.
            electrical_fingerprint: None,
        },
        encoders: cache.rows.clone(),
    })
}

pub(in crate::presentation) fn use_encoder_inputs(
    runtime: Rc<Runtime>,
    source: Option<LayerSource>,
) -> EncoderInputActions {
    let cache = use_hook(|| Rc::new(RefCell::new(InputCache::default())));
    let current = use_hook(move || {
        Rc::new(move || current_projection(&runtime, &cache))
            as Rc<dyn Fn() -> Option<EncoderInputProjection>>
    });
    let scope = source.as_ref().map(|source| source.scope.clone());
    let token = source.as_ref().map(|source| source.token);
    let revision = source.as_ref().map(|source| source.revision);
    let projection = use_memo(use_reactive((&scope, &token, &revision), {
        let current = current.clone();
        move |(scope, token, revision)| {
            let projection = current()?;
            (scope.as_ref() == Some(&projection.identity.scope)
                && token == Some(projection.identity.token)
                && revision == Some(projection.identity.revision))
            .then_some(projection)
        }
    }));
    EncoderInputActions {
        projection,
        current,
    }
}
