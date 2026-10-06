use super::PartsStandardProfileLifetime;
use crate::parts_mechanical_profile::{
    ProfileEditOwner, StandardProfileRequestCapture, dispatch_standard_profile_family,
    displayed_mounting_gap, initial_profile, merge_standard_profile, standard_profile_controls,
    standard_profile_request_is_current, standard_profile_source_and_gap,
};
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::model::{
    MechanicalExtraction, MechanicalGeometry, MechanicalPartProfile, MechanicalPurpose,
    MechanicalPurposeMapping, MechanicalSwitchFamily, PartDefinition, Vec2,
};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

#[derive(Clone, Copy)]
enum ContourField {
    Cutouts,
    Clearances,
}

pub type StandardProfileFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<MechanicalPartProfile, String>>>>;
pub type StandardProfileRequester = Rc<
    dyn Fn(
        String,
        MechanicalSwitchFamily,
        f64,
    ) -> (boardstudio_application::OperationId, StandardProfileFuture),
>;
pub type MechanicalExtractionFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<MechanicalExtraction, String>>>>;
pub type MechanicalExtractionRequester = Rc<
    dyn Fn(
        String,
        Vec<MechanicalPurposeMapping>,
    ) -> (
        boardstudio_application::OperationId,
        MechanicalExtractionFuture,
    ),
>;
pub type DetachedProfileSpawner =
    Rc<dyn Fn(std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>)>;
pub type CurrentProfileScope = Rc<dyn Fn() -> Option<Scope>>;
pub type AcceptedProfileOwner = Rc<dyn Fn(&ProfileEditOwner) -> bool>;

#[derive(Clone)]
pub struct ManualProfileEditorPorts {
    pub request_standard_profile: StandardProfileRequester,
    pub request_mechanical_extraction: MechanicalExtractionRequester,
    pub spawn_detached: DetachedProfileSpawner,
    pub current_scope: CurrentProfileScope,
    pub accepted_owner_is_current: AcceptedProfileOwner,
}

impl PartialEq for ManualProfileEditorPorts {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(
            &self.request_standard_profile,
            &other.request_standard_profile,
        ) && Rc::ptr_eq(&self.spawn_detached, &other.spawn_detached)
            && Rc::ptr_eq(
                &self.request_mechanical_extraction,
                &other.request_mechanical_extraction,
            )
            && Rc::ptr_eq(&self.current_scope, &other.current_scope)
            && Rc::ptr_eq(
                &self.accepted_owner_is_current,
                &other.accepted_owner_is_current,
            )
    }
}

impl Eq for ManualProfileEditorPorts {}

#[component]
pub fn ManualProfileEditor(
    definition: PartDefinition,
    initial: Option<MechanicalPartProfile>,
    owner: ProfileEditOwner,
    snapshot: AcceptedSnapshot,
    selection: Signal<Option<(Option<Scope>, String)>>,
    selection_generation: Signal<u64>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    on_save: EventHandler<MechanicalPartProfile>,
    on_close: EventHandler<()>,
    ports: ManualProfileEditorPorts,
) -> Element {
    let mut draft = use_signal(|| initial_profile(&definition, initial.as_ref()));
    let mut pending_standard = use_signal(|| None::<StandardProfileRequestCapture>);
    let mut standard_error = use_signal(String::new);
    let mut extraction_pending = use_signal(|| None::<boardstudio_application::OperationId>);
    let mut extraction_error = use_signal(String::new);
    let extracted_geometry = use_signal(|| None::<MechanicalGeometry>);
    let mut purposes = use_signal(|| {
        initial
            .as_ref()
            .and_then(|profile| profile.source_geometry.as_ref())
            .map(|source| {
                source
                    .mappings
                    .iter()
                    .filter_map(|mapping| {
                        mapping
                            .source_id
                            .as_ref()
                            .map(|id| (id.clone(), mapping.purpose))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default()
    });
    let lifetime = use_hook(PartsStandardProfileLifetime::new);
    let current_view = use_hook(|| {
        Rc::new(RefCell::new((
            owner.clone(),
            definition.clone(),
            snapshot.clone(),
        )))
    });
    *current_view.borrow_mut() = (owner.clone(), definition.clone(), snapshot.clone());
    use_drop({
        let lifetime = lifetime.clone();
        move || lifetime.retire()
    });
    let profile = draft.read().clone();
    let family_details = profile.switch_family.map(|family| match family {
        boardstudio_core::model::MechanicalSwitchFamily::Mx
        | boardstudio_core::model::MechanicalSwitchFamily::ChocV2 => 1.5,
        boardstudio_core::model::MechanicalSwitchFamily::ChocV1 => 1.3,
    });
    let gap_label = displayed_mounting_gap(&profile).unwrap_or_default();
    let (show_family_selector, show_standard_action) =
        standard_profile_controls(&definition, &profile);
    let family_selector_label = String::from("Switch fit family");
    let standard_action_label = String::from("Use standard cutout");
    let save_profile_label = String::from("Save fit profile");
    let standard_current_view = current_view.clone();
    let standard_ports = ports.clone();
    let load_standard = Rc::new(std::cell::RefCell::new({
        let request_standard_profile = standard_ports.request_standard_profile.clone();
        let owner = owner.clone();
        let definition = definition.clone();
        let lifetime = lifetime.clone();
        move |family: MechanicalSwitchFamily| {
            if pending_standard.read().is_some() {
                return;
            }
            let (_, plate_to_pcb) = standard_profile_source_and_gap(family);
            let (operation_id, request_future) =
                request_standard_profile(definition.id.clone(), family, plate_to_pcb);
            let request = StandardProfileRequestCapture::new(
                operation_id,
                owner.clone(),
                definition.clone(),
                family,
                plate_to_pcb,
                scope_generation(),
                selection_generation(),
            );
            pending_standard.set(Some(request.clone()));
            standard_error.set(String::new());
            let request_future = request_future;
            let current_view = standard_current_view.clone();
            let selection = selection;
            let workspace = workspace;
            let scope_generation = scope_generation;
            let selection_generation = selection_generation;
            let current_scope = standard_ports.current_scope.clone();
            let accepted_owner_is_current = standard_ports.accepted_owner_is_current.clone();
            let lifetime = lifetime.clone();
            let mut pending_standard = pending_standard;
            let mut standard_error = standard_error;
            let mut draft = draft;
            (standard_ports.spawn_detached)(Box::pin(async move {
                let result = request_future.await;
                let Some(()) = lifetime.run_if_mounted(|| {
                    let (current_owner, current_definition, current_snapshot) =
                        current_view.borrow().clone();
                    let still_current = standard_profile_request_is_current(
                        &request,
                        pending_standard
                            .read()
                            .as_ref()
                            .map(|pending| pending.operation_id),
                        &current_owner,
                        current_scope().as_ref(),
                        &current_snapshot,
                        &selection(),
                        &current_definition,
                        scope_generation(),
                        selection_generation(),
                        workspace(),
                        accepted_owner_is_current(&request.owner),
                    );
                    if !still_current {
                        if pending_standard
                            .read()
                            .as_ref()
                            .is_some_and(|pending| pending.operation_id == request.operation_id)
                        {
                            pending_standard.set(None);
                        }
                        return;
                    }
                    pending_standard.set(None);
                    match result {
                        Ok(loaded) => merge_standard_profile(
                            &mut draft.write(),
                            loaded,
                            request.family,
                            request.plate_to_pcb,
                        ),
                        Err(message) => standard_error.set(message),
                    }
                }) else {
                    return;
                };
            }));
        }
    }));
    let selector_load_standard = load_standard.clone();
    let action_load_standard = load_standard.clone();
    let extraction_generations = Rc::new(Cell::new((scope_generation(), selection_generation())));
    let request_extraction = Rc::new(std::cell::RefCell::new({
        let request_mechanical_extraction = ports.request_mechanical_extraction.clone();
        let owner = owner.clone();
        let definition = definition.clone();
        let lifetime = lifetime.clone();
        let extraction_generations = extraction_generations.clone();
        move |mappings: Vec<MechanicalPurposeMapping>, apply: bool| {
            if extraction_pending.read().is_some() {
                return;
            }
            let Some(source) = definition
                .kicad_source
                .as_ref()
                .map(|source| source.source.clone())
            else {
                return;
            };
            let (operation_id, request_future) = request_mechanical_extraction(source, mappings);
            extraction_pending.set(Some(operation_id));
            extraction_error.set(String::new());
            let capture_owner = owner.clone();
            let capture_definition = definition.clone();
            let capture_snapshot = snapshot.clone();
            let (request_scope_generation, request_selection_generation) =
                extraction_generations.get();
            let expected_selection = (owner.scope.clone(), definition.id.clone());
            let request_future = request_future;
            let current_view = current_view.clone();
            let current_scope = ports.current_scope.clone();
            let accepted_owner_is_current = ports.accepted_owner_is_current.clone();
            let selection = selection;
            let selection_generation = selection_generation;
            let scope_generation = scope_generation;
            let workspace = workspace;
            let lifetime = lifetime.clone();
            let mut extraction_pending = extraction_pending;
            let mut extraction_error = extraction_error;
            let mut extracted_geometry = extracted_geometry;
            let mut purposes = purposes;
            let mut draft = draft;
            (ports.spawn_detached)(Box::pin(async move {
                let result = request_future.await;
                let _ = lifetime.run_if_mounted(|| {
                    let (current_owner, current_definition, current_snapshot) =
                        current_view.borrow().clone();
                    let still_current = extraction_pending.read().as_ref() == Some(&operation_id)
                        && current_owner == capture_owner
                        && current_scope().as_ref() == capture_owner.scope.as_ref()
                        && current_snapshot.session_epoch == capture_snapshot.session_epoch
                        && current_snapshot.document.id == capture_snapshot.document.id
                        && selection().as_ref() == Some(&expected_selection)
                        && current_definition == capture_definition
                        && scope_generation() == request_scope_generation
                        && selection_generation() == request_selection_generation
                        && workspace() == "Parts"
                        && accepted_owner_is_current(&capture_owner);
                    if !still_current {
                        if extraction_pending.read().as_ref() == Some(&operation_id) {
                            extraction_pending.set(None);
                        }
                        return;
                    }
                    extraction_pending.set(None);
                    match result {
                        Ok(extraction) => {
                            extracted_geometry.set(Some(extraction.geometry.clone()));
                            if apply {
                                draft.with_mut(|profile| {
                                    profile.source = format!("KiCad {}", capture_definition.name);
                                    profile.source_geometry = Some(extraction.source_geometry);
                                    profile.pcb_holes = Some(extraction.pcb_holes);
                                    profile.cutouts = extraction.plate_cutouts;
                                    profile.clearances = Some(extraction.clearance_envelopes);
                                });
                            } else if let Some(source) = draft().source_geometry {
                                purposes.set(
                                    source
                                        .mappings
                                        .into_iter()
                                        .filter_map(|mapping| {
                                            mapping.source_id.map(|id| (id, mapping.purpose))
                                        })
                                        .collect(),
                                );
                            }
                        }
                        Err(message) => extraction_error.set(message),
                    }
                });
            }));
        }
    }));
    // Refresh the captured generations immediately before each request.
    let request_extract = request_extraction.clone();
    let generation_capture = extraction_generations.clone();
    let read_geometry = move |_| {
        generation_capture.set((scope_generation(), selection_generation()));
        (request_extract.borrow_mut())(Vec::new(), false);
    };
    let request_apply = request_extraction.clone();
    let generation_capture = extraction_generations.clone();
    let apply_geometry = move |_| {
        generation_capture.set((scope_generation(), selection_generation()));
        let mappings = purposes()
            .into_iter()
            .map(|(source_id, purpose)| MechanicalPurposeMapping {
                source_id: Some(source_id),
                kind: None,
                layer: None,
                purpose,
            })
            .collect();
        (request_apply.borrow_mut())(mappings, true);
    };
    rsx! {
        section { class: "m1-parts-fit-editor", "aria-label": "Mechanical fit profile editor",
            header {
                div {
                    h2 { "{definition.name} fit" }
                    p { "Save the fit with this part. Every case using it inherits the profile." }
                }
                button { class: "m1-secondary", r#type: "button", onclick: move |_| on_close.call(()), "Cancel" }
            }
            if show_family_selector {
                label { class: "m1-parts-fit-field",
                    span { "Switch fit family" }
                    select {
                        "aria-label": "{family_selector_label}",
                        value: profile.switch_family.map(family_key).unwrap_or(""),
                        disabled: pending_standard.read().is_some() || extraction_pending.read().is_some(),
                        onchange: move |event| {
                            let load_standard = selector_load_standard.clone();
                            dispatch_standard_profile_family(&event.value(), move |family| {
                                (load_standard.borrow_mut())(family);
                            });
                        },
                        option { value: "", "Select a standard family" }
                        option { value: "mx", "MX" }
                        option { value: "choc-v1", "Choc v1" }
                        option { value: "choc-v2", "Choc v2" }
                    }
                }
            }
            if let Some(thickness) = family_details {
                p { "Mounting gap: {gap_label} mm with a {thickness:.2} mm plate. Case recalculates the gap for its plate thickness." }
            } else {
                label { class: "m1-parts-fit-field",
                    span { "Plate underside to PCB top (mm)" }
                    input {
                        r#type: "number",
                        min: "0",
                        step: "0.01",
                        value: "{profile.plate_to_pcb}",
                        oninput: move |event| {
                            if let Ok(value) = event.value().parse::<f64>()
                                && value.is_finite() && value >= 0.0 {
                                draft.with_mut(|profile| profile.plate_to_pcb = value);
                            }
                        },
                    }
                }
            }
            if show_standard_action {
                button {
                    class: "m1-secondary",
                    r#type: "button",
                    "aria-label": "{standard_action_label}",
                    disabled: pending_standard.read().is_some() || extraction_pending.read().is_some(),
                    onclick: move |_| {
                        if let Some(family) = draft().switch_family {
                            (action_load_standard.borrow_mut())(family);
                        }
                    },
                    if pending_standard.read().is_some() { "Loading standard fit…" } else { "Use standard cutout" }
                }
            }
            if let Some(request) = pending_standard.read().as_ref() {
                p { role: "status", "Loading the standard {request.family:?} cutout…" }
            }
            if !standard_error().is_empty() {
                p { role: "alert", "Standard fit could not be loaded: {standard_error()}" }
            }
            if definition.kicad_source.is_some() {
                section { class: "m1-parts-fit-section", "aria-label": "Footprint geometry",
                    header {
                        strong { "Footprint geometry" }
                        button {
                            class: "m1-secondary",
                            r#type: "button",
                            disabled: extraction_pending.read().is_some(),
                            onclick: read_geometry,
                            if extraction_pending.read().is_some() { "Reading KiCad layers…" } else { "Read KiCad layers" }
                        }
                    }
                    if let Some(geometry) = extracted_geometry.read().as_ref() {
                        for primitive in geometry.primitives.iter() {
                            {
                                let primitive_id = primitive.id.clone();
                                let purpose_label = format!(
                                    "{} · {}",
                                    primitive.layer.as_deref().unwrap_or(primitive_kind(primitive.kind)),
                                    primitive_kind(primitive.kind),
                                );
                                let selected_id = primitive_id.clone();
                                rsx! {
                                    label { class: "m1-parts-fit-field",
                                        span { "{purpose_label}" }
                                        select {
                                            "aria-label": "Purpose for {primitive_id}",
                                            value: purposes().get(&primitive_id).map(|purpose| purpose_key(*purpose)).unwrap_or(""),
                                            disabled: extraction_pending.read().is_some(),
                                            onchange: move |event| {
                                                purposes.with_mut(|selected| {
                                                    if let Some(purpose) = parse_purpose(&event.value()) {
                                                        selected.insert(selected_id.clone(), purpose);
                                                    } else {
                                                        selected.remove(&selected_id);
                                                    }
                                                });
                                            },
                                            option { value: "", "Unused" }
                                            option { value: "plate-cutout", "Plate cutout" }
                                            option { value: "electrical-pcb-mounting-hole", "PCB mounting hole" }
                                            option { value: "clearance-envelope", "Clearance envelope" }
                                            option { value: "drawing-guide", "Drawing guide" }
                                        }
                                    }
                                }
                            }
                        }
                        button {
                            class: "m1-secondary",
                            r#type: "button",
                            disabled: extraction_pending.read().is_some() || purposes.read().is_empty(),
                            onclick: apply_geometry,
                            if extraction_pending.read().is_some() { "Applying selected geometry…" } else { "Apply selected geometry" }
                        }
                    }
                    if !extraction_error().is_empty() {
                        p { role: "alert", "KiCad geometry could not be extracted: {extraction_error()}" }
                    }
                }
            }
            {contour_editor(draft, ContourField::Cutouts, "Plate cutouts")}
            {contour_editor(draft, ContourField::Clearances, "Component clearances")}
            button {
                class: "m1-primary",
                r#type: "button",
                "aria-label": "{save_profile_label}",
                disabled: pending_standard.read().is_some() || extraction_pending.read().is_some(),
                onclick: move |_| { on_save.call(draft()); on_close.call(()); },
                "Save fit profile"
            }
        }
    }
}

fn family_key(family: MechanicalSwitchFamily) -> &'static str {
    match family {
        MechanicalSwitchFamily::Mx => "mx",
        MechanicalSwitchFamily::ChocV1 => "choc-v1",
        MechanicalSwitchFamily::ChocV2 => "choc-v2",
    }
}

fn purpose_key(purpose: MechanicalPurpose) -> &'static str {
    match purpose {
        MechanicalPurpose::ElectricalPcbMountingHole => "electrical-pcb-mounting-hole",
        MechanicalPurpose::PlateCutout => "plate-cutout",
        MechanicalPurpose::ClearanceEnvelope => "clearance-envelope",
        MechanicalPurpose::DrawingGuide => "drawing-guide",
    }
}

fn parse_purpose(value: &str) -> Option<MechanicalPurpose> {
    match value {
        "electrical-pcb-mounting-hole" => Some(MechanicalPurpose::ElectricalPcbMountingHole),
        "plate-cutout" => Some(MechanicalPurpose::PlateCutout),
        "clearance-envelope" => Some(MechanicalPurpose::ClearanceEnvelope),
        "drawing-guide" => Some(MechanicalPurpose::DrawingGuide),
        _ => None,
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    use super::*;
    use crate::parts_mechanical_profile::ProfileDefinitionSource;
    use boardstudio_application::{OperationId, SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{
        MechanicalGeometryKind, MechanicalPrimitive, MechanicalProfileSource, MechanicalShape,
        ProjectDoc, SceneDelta,
    };
    use futures_channel::oneshot;
    use std::{cell::RefCell, collections::VecDeque};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    struct ExtractionRequest {
        complete: oneshot::Sender<Result<MechanicalExtraction, String>>,
    }

    struct Fixture {
        snapshot: AcceptedSnapshot,
        scope: Scope,
        owner: ProfileEditOwner,
        definition: PartDefinition,
        current_scope: Rc<RefCell<Option<Scope>>>,
        owner_current: Rc<Cell<bool>>,
        requests: Rc<RefCell<VecDeque<ExtractionRequest>>>,
        saved: Rc<RefCell<Option<MechanicalPartProfile>>>,
        ports: ManualProfileEditorPorts,
    }

    #[derive(Clone, Copy)]
    struct MountedSignals {
        selection: Signal<Option<(Option<Scope>, String)>>,
        selection_generation: Signal<u64>,
        scope_generation: Signal<u64>,
    }

    #[component]
    fn ManualProfileMountedHost() -> Element {
        let fixture = use_context::<Rc<Fixture>>();
        let handles = use_context::<Rc<RefCell<Option<MountedSignals>>>>();
        let selection =
            use_signal(|| Some((Some(fixture.scope.clone()), fixture.definition.id.clone())));
        let selection_generation = use_signal(|| 1_u64);
        let scope_generation = use_signal(|| 1_u64);
        let workspace = use_signal(|| "Parts");
        *handles.borrow_mut() = Some(MountedSignals {
            selection,
            selection_generation,
            scope_generation,
        });
        let saved = fixture.saved.clone();
        let on_save = EventHandler::new(move |profile| *saved.borrow_mut() = Some(profile));
        rsx! {
            ManualProfileEditor {
                definition: fixture.definition.clone(),
                initial: None,
                owner: fixture.owner.clone(),
                snapshot: fixture.snapshot.clone(),
                selection,
                selection_generation,
                scope_generation,
                workspace,
                on_save,
                on_close: EventHandler::default(),
                ports: fixture.ports.clone(),
            }
        }
    }

    fn extraction() -> MechanicalExtraction {
        MechanicalExtraction {
            geometry: MechanicalGeometry {
                source_name: Some("controlled delayed source".into()),
                primitives: vec![MechanicalPrimitive {
                    id: "stale-layer-primitive".into(),
                    source_group_id: "stale-group".into(),
                    kind: MechanicalGeometryKind::Polygon,
                    layer: Some("Dwgs.User".into()),
                    layers: vec!["Dwgs.User".into()],
                    purpose: None,
                    geometry: MechanicalShape::Polygon {
                        points: vec![
                            Vec2 { x: -1.0, y: -1.0 },
                            Vec2 { x: 1.0, y: -1.0 },
                            Vec2 { x: 1.0, y: 1.0 },
                        ],
                        width: None,
                    },
                }],
            },
            plate_cutouts: vec![vec![
                Vec2 { x: -1.0, y: -1.0 },
                Vec2 { x: 1.0, y: -1.0 },
                Vec2 { x: 1.0, y: 1.0 },
            ]],
            clearance_envelopes: Vec::new(),
            pcb_holes: Vec::new(),
            source_geometry: MechanicalProfileSource {
                text: "controlled delayed source".into(),
                sha256: "a".repeat(64),
                mappings: Vec::new(),
                source_ids: vec!["stale-layer-primitive".into()],
            },
        }
    }

    fn fixture() -> Rc<Fixture> {
        let definition: PartDefinition = serde_json::from_value(serde_json::json!({
            "id": "imported:thqwgd001c",
            "name": "THQWGD001C · 2-pin tactile · reversible",
            "kind": "switch",
            "kicadSource": {
                "formatVersion": 1,
                "source": "(footprint \"THQWGD001C\" (version 20240108) (generator \"BoardStudio\"))"
            },
            "courtyard": [
                {"x": -5.0, "y": -3.0}, {"x": 5.0, "y": -3.0}, {"x": 5.0, "y": 3.0}
            ],
            "pads": []
        })).unwrap();
        let document = ProjectDoc::empty("mechanical-extraction-owner", "Extraction owner");
        let scene: SceneDelta = serde_json::from_value(serde_json::json!({
            "revision": 2,
            "transactionId": "mechanical-extraction-owner",
            "changedIds": [], "transforms": [], "matrixScenes": [],
            "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
            "readiness": {"layout": false, "outline": false, "pcb": false, "case": false}
        }))
        .unwrap();
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(12),
            session_epoch: SessionEpoch(9),
            document: std::sync::Arc::new(document),
            scene: std::sync::Arc::new(scene),
        };
        let scope = Scope {
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            board_id: "left-pcb".into(),
            instance_id: None,
        };
        let owner = ProfileEditOwner::new(
            OperationId(44),
            &snapshot,
            Some(scope.clone()),
            ProfileDefinitionSource::Imported,
            &definition,
        );
        let requests = Rc::new(RefCell::new(VecDeque::new()));
        let request_queue = requests.clone();
        let request_mechanical_extraction: MechanicalExtractionRequester =
            Rc::new(move |_source, _mappings| {
                let (complete, result) = oneshot::channel();
                request_queue
                    .borrow_mut()
                    .push_back(ExtractionRequest { complete });
                (
                    OperationId(45),
                    Box::pin(async move {
                        result
                            .await
                            .unwrap_or_else(|_| Err("controlled extraction retired".into()))
                    }) as MechanicalExtractionFuture,
                )
            });
        let current_scope = Rc::new(RefCell::new(Some(scope.clone())));
        let current_scope_port = current_scope.clone();
        let owner_current = Rc::new(Cell::new(true));
        let owner_current_port = owner_current.clone();
        let saved = Rc::new(RefCell::new(None));
        Rc::new(Fixture {
            snapshot,
            scope,
            owner,
            definition,
            current_scope,
            owner_current,
            requests,
            saved,
            ports: ManualProfileEditorPorts {
                request_standard_profile: Rc::new(|_, _, _| {
                    (
                        OperationId(46),
                        Box::pin(async { Err("unused standard request".into()) }),
                    )
                }),
                request_mechanical_extraction,
                spawn_detached: Rc::new(|future| wasm_bindgen_futures::spawn_local(future)),
                current_scope: Rc::new(move || current_scope_port.borrow().clone()),
                accepted_owner_is_current: Rc::new(move |_| owner_current_port.get()),
            },
        })
    }

    fn click_button(root: &web_sys::Element, label: &str) {
        let buttons = root.query_selector_all("button").unwrap();
        for index in 0..buttons.length() {
            let Some(button) = buttons.item(index) else {
                continue;
            };
            if button.text_content().unwrap_or_default().trim() == label {
                button.dyn_into::<web_sys::HtmlElement>().unwrap().click();
                return;
            }
        }
        panic!("missing mounted profile button {label}");
    }

    async fn wait_for_text(root: &web_sys::Element, text: &str) {
        for _ in 0..100 {
            if root.text_content().unwrap_or_default().contains(text) {
                return;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("timed out waiting for {text}");
    }

    async fn wait_for_button_enabled(root: &web_sys::Element, label: &str) {
        for _ in 0..100 {
            let buttons = root.query_selector_all("button").unwrap();
            for index in 0..buttons.length() {
                let Some(button) = buttons
                    .item(index)
                    .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
                else {
                    continue;
                };
                if button.text_content().unwrap_or_default().trim() == label
                    && !button.has_attribute("disabled")
                {
                    return;
                }
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("timed out waiting for mounted {label} button to be enabled");
    }

    #[wasm_bindgen_test]
    async fn mounted_extraction_reply_cannot_mutate_profile_after_scope_and_selection_retire() {
        let fixture = fixture();
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("parts-mechanical-extraction-owner-test");
        document.body().unwrap().append_child(&root).unwrap();
        let handles: Rc<RefCell<Option<MountedSignals>>> = Rc::new(RefCell::new(None));
        let dom = VirtualDom::new(ManualProfileMountedHost);
        dom.provide_root_context(fixture.clone());
        dom.provide_root_context(handles.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_for_text(&root, "Read KiCad layers").await;
        click_button(&root, "Read KiCad layers");
        let request = {
            let mut request = None;
            for _ in 0..100 {
                request = fixture.requests.borrow_mut().pop_front();
                if request.is_some() {
                    break;
                }
                gloo_timers::future::TimeoutFuture::new(10).await;
            }
            request.expect("Read KiCad layers starts the controlled requester")
        };
        wait_for_text(&root, "Reading KiCad layers…").await;

        let mut mounted = handles.borrow().as_ref().copied().unwrap();
        let mut next_scope = fixture.scope.clone();
        next_scope.board_id = "right-pcb".into();
        *fixture.current_scope.borrow_mut() = Some(next_scope);
        mounted
            .selection
            .set(Some((None, "other-definition".into())));
        mounted
            .scope_generation
            .with_mut(|generation| *generation += 1);
        mounted
            .selection_generation
            .with_mut(|generation| *generation += 1);
        fixture.owner_current.set(false);
        request.complete.send(Ok(extraction())).unwrap();
        wait_for_button_enabled(&root, "Read KiCad layers").await;
        assert!(
            root.query_selector("select[aria-label^='Purpose for']")
                .unwrap()
                .is_none()
        );
        assert!(root.query_selector("p[role='alert']").unwrap().is_none());
        root.query_selector("button[aria-label='Save fit profile']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        let saved = fixture
            .saved
            .borrow()
            .clone()
            .expect("save captures current draft");
        assert!(saved.source_geometry.is_none());
        assert!(saved.cutouts.is_empty());
        assert!(saved.clearances.is_none());
        root.remove();
    }
}

fn primitive_kind(kind: boardstudio_core::model::MechanicalGeometryKind) -> &'static str {
    match kind {
        boardstudio_core::model::MechanicalGeometryKind::Line => "Line",
        boardstudio_core::model::MechanicalGeometryKind::Arc => "Arc",
        boardstudio_core::model::MechanicalGeometryKind::Circle => "Circle",
        boardstudio_core::model::MechanicalGeometryKind::Rectangle => "Rectangle",
        boardstudio_core::model::MechanicalGeometryKind::Polygon => "Polygon",
        boardstudio_core::model::MechanicalGeometryKind::Drill => "Drill",
    }
}

fn contour_editor(
    mut draft: Signal<MechanicalPartProfile>,
    field: ContourField,
    label: &'static str,
) -> Element {
    let profile = draft.read().clone();
    let contours = match field {
        ContourField::Cutouts => profile.cutouts.clone(),
        ContourField::Clearances => profile.clearances.clone().unwrap_or_default(),
    };
    let add_label = match field {
        ContourField::Cutouts => "cutout",
        ContourField::Clearances => "clearance",
    };
    rsx! {
        section { class: "m1-parts-fit-section", "aria-label": "{label}",
            header {
                strong { "{label}" }
                button {
                    class: "m1-secondary",
                    r#type: "button",
                    "aria-label": "Add {add_label}",
                    onclick: move |_| draft.with_mut(|profile| {
                        let mut next = empty_square();
                        match field {
                            ContourField::Cutouts => profile.cutouts.push(std::mem::take(&mut next)),
                            ContourField::Clearances => profile.clearances.get_or_insert_with(Vec::new).push(std::mem::take(&mut next)),
                        }
                    }),
                    "Add {add_label}"
                }
            }
            if contours.is_empty() { p { "No {label.to_lowercase()} defined." } }
            for (shape_index, contour) in contours.iter().enumerate() {
                div { class: "m1-parts-fit-contour",
                    header {
                        strong { "Contour {shape_index + 1}" }
                        button {
                            class: "m1-secondary",
                            r#type: "button",
                            onclick: move |_| draft.with_mut(|profile| match field {
                                ContourField::Cutouts => { if shape_index < profile.cutouts.len() { profile.cutouts.remove(shape_index); } },
                                ContourField::Clearances => if let Some(contours) = &mut profile.clearances
                                    && shape_index < contours.len() { contours.remove(shape_index); },
                            }),
                            "Remove contour {shape_index + 1}"
                        }
                    }
                    for (point_index, point) in contour.iter().enumerate() {
                        div { class: "m1-parts-fit-point",
                            span { "Vertex {point_index + 1}" }
                            for axis in ["X", "Y"] {
                                { let axis_value = if axis == "X" { point.x } else { point.y };
                                  rsx! {
                                    input {
                                        r#type: "number",
                                        step: "0.1",
                                        aria_label: "{label} contour {shape_index + 1} vertex {point_index + 1} {axis}",
                                        value: "{axis_value}",
                                        oninput: move |event| {
                                            let Ok(value) = event.value().parse::<f64>() else { return; };
                                            if !value.is_finite() { return; }
                                            draft.with_mut(|profile| {
                                                let contours = match field {
                                                    ContourField::Cutouts => &mut profile.cutouts,
                                                    ContourField::Clearances => profile.clearances.get_or_insert_with(Vec::new),
                                                };
                                                if let Some(vertex) = contours.get_mut(shape_index).and_then(|contour| contour.get_mut(point_index)) {
                                                    if axis == "X" { vertex.x = value; } else { vertex.y = value; }
                                                }
                                            });
                                        },
                                    }
                                  }
                                }
                            }
                        }
                    }
                    button {
                        class: "m1-secondary",
                        r#type: "button",
                        onclick: move |_| draft.with_mut(|profile| {
                            let contours = match field {
                                ContourField::Cutouts => &mut profile.cutouts,
                                ContourField::Clearances => profile.clearances.get_or_insert_with(Vec::new),
                            };
                            if let Some(last) = contours.get_mut(shape_index).and_then(|contour| contour.last().copied()) {
                                contours[shape_index].push(last);
                            }
                        }),
                        "Add vertex"
                    }
                }
            }
        }
    }
}

fn empty_square() -> Vec<Vec2> {
    vec![
        Vec2 { x: -2.5, y: -2.5 },
        Vec2 { x: 2.5, y: -2.5 },
        Vec2 { x: 2.5, y: 2.5 },
        Vec2 { x: -2.5, y: 2.5 },
    ]
}
