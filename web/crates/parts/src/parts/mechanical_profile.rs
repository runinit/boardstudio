//! Center-workspace manual mechanical profile draft and accepted edit.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope};
use boardstudio_core_mechanical::mechanical::{
    default_plate_thickness, generator_switch_family, mounting_datum,
};
use boardstudio_model::model::{
    EditOperation, MechanicalBuiltinProfile, MechanicalPartProfile, MechanicalSwitchFamily,
    PartDefinition,
};

use crate::parts_custom_definition::replacement_commit;

pub(crate) const PROFILE_GONE: &str = "This part definition no longer exists.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileDefinitionSource {
    Generator,
    Imported,
    Project,
}

pub fn standard_profile_source_and_gap(
    family: MechanicalSwitchFamily,
) -> (MechanicalBuiltinProfile, f64) {
    match family {
        MechanicalSwitchFamily::Mx => (MechanicalBuiltinProfile::MxSwitch, 3.5),
        MechanicalSwitchFamily::ChocV1 => (MechanicalBuiltinProfile::ChocV1Switch, 2.2),
        MechanicalSwitchFamily::ChocV2 => (MechanicalBuiltinProfile::ChocV2Switch, 3.5),
    }
}

/// Keep the selector and the reusable built-in action's eligibility separate:
/// any switch can select a family, while any saved family can reload its fit.
pub fn standard_profile_controls(
    definition: &PartDefinition,
    profile: &MechanicalPartProfile,
) -> (bool, bool) {
    (
        definition.kind == boardstudio_model::model::PartKind::Switch,
        profile.switch_family.is_some(),
    )
}

fn profile_editor_is_current(
    workspace: &str,
    captured_generation: (u64, u64),
    current_generation: impl FnOnce() -> (u64, u64),
    owner_matches: impl FnOnce() -> bool,
) -> bool {
    workspace == "Parts" && captured_generation == current_generation() && owner_matches()
}

pub fn merge_standard_profile(
    draft: &mut MechanicalPartProfile,
    mut loaded: MechanicalPartProfile,
    family: MechanicalSwitchFamily,
    plate_to_pcb: f64,
) {
    // Core's optional fields are sparse: a missing value means retain the
    // user's current draft, including manually entered provenance and details.
    if loaded.source_geometry.is_some() {
        draft.source_geometry = loaded.source_geometry.take();
    }
    if loaded.pcb_holes.is_some() {
        draft.pcb_holes = loaded.pcb_holes.take();
    }
    if loaded.clearance_volumes.is_some() {
        draft.clearance_volumes = loaded.clearance_volumes.take();
    }
    if loaded.openings.is_some() {
        draft.openings = loaded.openings.take();
    }
    if loaded.clearances.is_some() {
        draft.clearances = loaded.clearances.take();
    }
    if loaded.supported_thickness.is_some() {
        draft.supported_thickness = loaded.supported_thickness.take();
    }
    draft.cutouts = loaded.cutouts;
    draft.source = loaded.source;
    draft.definition_id = loaded.definition_id;
    draft.switch_family = Some(family);
    draft.plate_to_pcb = plate_to_pcb;
}

/// Resolve one manual profile save against the accepted document at execution: a
/// project-owned definition must still exist, a catalogue definition materializes as a
/// project override (or updates one that appeared meanwhile), and the replacement is
/// cloned from the accepted document, so a save queued behind another edit never
/// reverts it.
pub fn mechanical_profile_resolver(
    source: ProfileDefinitionSource,
    definition_id: String,
    definition: PartDefinition,
    profile: MechanicalPartProfile,
) -> EditResolver {
    EditResolver::new(
        "parts-mechanical-profile",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            let mut replacement = document.as_ref().clone();
            match source {
                ProfileDefinitionSource::Project => {
                    let Some(target) = replacement
                        .definitions
                        .iter_mut()
                        .find(|item| item.id == definition_id)
                    else {
                        return Resolution::Retire(PROFILE_GONE.into());
                    };
                    target.mechanical_profile = Some(profile.clone());
                }
                ProfileDefinitionSource::Generator | ProfileDefinitionSource::Imported => {
                    // The captured catalogue definition is only the seed: if the project
                    // gained this definition while the editor was open, the accepted one
                    // wins and only the profile is set on it.
                    if let Some(target) = replacement
                        .definitions
                        .iter_mut()
                        .find(|item| item.id == definition_id)
                    {
                        target.mechanical_profile = Some(profile.clone());
                    } else {
                        let mut materialized = definition.clone();
                        materialized.mechanical_profile = Some(profile.clone());
                        replacement.definitions.push(materialized);
                    }
                }
            }
            // The clone is built from the accepted document, so equality means this
            // profile already holds: resolve Unchanged, not a landing heuristic
            // (ADR-0005).
            if replacement == **document {
                return Resolution::Unchanged;
            }
            replacement_commit(
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
                vec![definition_id.clone()],
            )
        },
    )
}

pub fn initial_profile(
    definition: &PartDefinition,
    existing: Option<&MechanicalPartProfile>,
) -> MechanicalPartProfile {
    if let Some(profile) = existing {
        return profile.clone();
    }
    let switch_family = generator_switch_family(definition, None);
    let plate_to_pcb = switch_family.map_or(0.0, |family| {
        mounting_datum(family) - default_plate_thickness(family)
    });
    MechanicalPartProfile {
        source_geometry: None,
        pcb_holes: None,
        clearance_volumes: None,
        openings: None,
        clearances: None,
        supported_thickness: None,
        switch_family,
        definition_id: definition.id.clone(),
        source: "Parts library".into(),
        cutouts: Vec::new(),
        plate_to_pcb,
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) mod ui {
    use super::*;
    use crate::parts::{GeneratorPreviewDraft, PartsPreviewPanel, PartsSelectionGeneration};
    use crate::{SelectionAdapter, WorkspaceState, runtime::Runtime};
    use boardstudio_model::model::{
        MechanicalGeometry, MechanicalPurpose, MechanicalPurposeMapping, Vec2,
    };
    use boardstudio_web_ui_shared::inspector_fields::{
        InspectorSection, NumericField, SelectField,
    };
    use boardstudio_web_ui_shared::ui::{
        alert::{Alert, AlertVariant},
        button::Button,
        spinner::Spinner,
    };
    use dioxus::prelude::*;
    use std::{collections::BTreeMap, rc::Rc};

    #[component]
    pub fn PartsMechanicalProfileWorkspace(
        snapshot: AcceptedSnapshot,
        scope: Option<Scope>,
        selection: Signal<Option<(Option<Scope>, String)>>,
        definition: PartDefinition,
        preview_definition: Option<PartDefinition>,
        generator_draft: Option<GeneratorPreviewDraft>,
        recipe: Vec<crate::parts_preview::PartsPreviewRecipeMember>,
        recipe_error: Option<String>,
        recipe_pending: bool,
        recipe_identity: String,
        preview_title: Option<String>,
        source: ProfileDefinitionSource,
    ) -> Element {
        let mut editing = use_signal(|| false);
        let profile_defined = definition.mechanical_profile.is_some();
        if editing() {
            return rsx! {
                ProfileEditor {
                    key: "{snapshot.session_epoch.0}:{definition.id}:{scope:?}",
                    snapshot, scope, selection, definition, source,
                    on_close: move |_| editing.set(false),
                }
            };
        }
        rsx! {
            section { "aria-label": "Mechanical fit profile",
                Button { onclick: {
                    let scope = scope.clone();
                    let id = definition.id.clone();
                    move |_| {
                        let mut selection = selection;
                        selection.set(Some((scope.clone(), id.clone())));
                        editing.set(true);
                    }
                }, if profile_defined { "Edit profile" } else { "Define profile" } }
            }
            PartsPreviewPanel {
                definition: Some(Rc::new(preview_definition.unwrap_or_else(|| definition.clone()))),
                recipe, recipe_error, recipe_pending, recipe_identity, preview_title,
                scope, snapshot_token: snapshot.token, generator_draft,
            }
        }
    }

    enum GeometryRequest {
        Standard(MechanicalSwitchFamily),
        KiCad { source: String, apply: bool },
    }

    #[component]
    fn ProfileEditor(
        snapshot: AcceptedSnapshot,
        scope: Option<Scope>,
        selection: Signal<Option<(Option<Scope>, String)>>,
        definition: PartDefinition,
        source: ProfileDefinitionSource,
        on_close: EventHandler<()>,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let workspace = use_context::<WorkspaceState>().0;
        let selection_generation = use_context::<PartsSelectionGeneration>().0;
        let scope_generation = use_context::<SelectionAdapter>().generation;
        let generation = (scope_generation(), selection_generation());
        let capture = crate::parts_custom_definition::DefinitionPanelCapture::new(
            &snapshot,
            scope,
            &definition,
        );
        let current = {
            let runtime = runtime.clone();
            move || {
                profile_editor_is_current(
                    workspace(),
                    generation,
                    || (scope_generation(), selection_generation()),
                    || {
                        runtime.model().accepted.as_ref().is_some_and(|snapshot| {
                            capture.owner_matches(snapshot, runtime.scope(), selection())
                        })
                    },
                )
            }
        };
        let mut draft =
            use_signal(|| initial_profile(&definition, definition.mechanical_profile.as_ref()));
        let mut busy = use_signal(|| false);
        let mut error = use_signal(|| None::<String>);
        let mut geometry = use_signal(|| None::<MechanicalGeometry>);
        let mut purposes = use_signal(|| {
            draft()
                .source_geometry
                .into_iter()
                .flat_map(|source| source.mappings)
                .filter_map(|mapping| Some((mapping.source_id?, mapping.purpose)))
                .collect::<BTreeMap<_, _>>()
        });

        let load = EventHandler::new({
            let runtime = runtime.clone();
            let current = current.clone();
            let definition_id = definition.id.clone();
            let name = definition.name.clone();
            move |request: GeometryRequest| {
                if busy() || !current() {
                    return;
                }
                busy.set(true);
                error.set(None);
                let runtime = runtime.clone();
                let current = current.clone();
                let definition_id = definition_id.clone();
                let name = name.clone();
                spawn(async move {
                    let id = runtime.operation();
                    let result = match request {
                        GeometryRequest::Standard(family) => {
                            let (_, gap) = standard_profile_source_and_gap(family);
                            runtime
                                .standard_switch_profile(id, definition_id, family, gap)
                                .await
                                .map(|loaded| {
                                    if current() {
                                        merge_standard_profile(
                                            &mut draft.write(),
                                            loaded,
                                            family,
                                            gap,
                                        );
                                    }
                                })
                        }
                        GeometryRequest::KiCad { source, apply } => {
                            let mappings = if apply {
                                purposes()
                                    .into_iter()
                                    .map(|(id, purpose)| MechanicalPurposeMapping {
                                        source_id: Some(id),
                                        kind: None,
                                        layer: None,
                                        purpose,
                                    })
                                    .collect()
                            } else {
                                Vec::new()
                            };
                            runtime
                                .extract_mechanical_profile(
                                    id.0.to_string(),
                                    source,
                                    mappings,
                                    0.005,
                                )
                                .await
                                .map(|extraction| {
                                    if !current() {
                                        return;
                                    }
                                    geometry.set(Some(extraction.geometry));
                                    if apply {
                                        draft.with_mut(|profile| {
                                            profile.source = format!("KiCad {name}");
                                            profile.source_geometry =
                                                Some(extraction.source_geometry);
                                            profile.pcb_holes = Some(extraction.pcb_holes);
                                            profile.cutouts = extraction.plate_cutouts;
                                            profile.clearances =
                                                Some(extraction.clearance_envelopes);
                                        });
                                    }
                                })
                        }
                    };
                    busy.set(false);
                    if current() {
                        error.set(result.err());
                    }
                });
            }
        });
        let profile = draft();
        let (show_family, show_standard) = standard_profile_controls(&definition, &profile);
        let read_only = busy().then(|| "Loading fit geometry".to_owned());
        let footprint = definition
            .kicad_source
            .as_ref()
            .map(|source| source.source.clone());
        let save_id = definition.id.clone();
        rsx! {
            InspectorSection { title: format!("{} fit", definition.name),
                Button { onclick: move |_| on_close.call(()), "Cancel" }
                if show_family {
                    SelectField::<Option<MechanicalSwitchFamily>> {
                        label: "Switch fit family", value: profile.switch_family, disabled_reason: read_only.clone(),
                        placeholder: "Select a standard family",
                        options: [(MechanicalSwitchFamily::Mx, "MX"), (MechanicalSwitchFamily::ChocV1, "Choc v1"),
                            (MechanicalSwitchFamily::ChocV2, "Choc v2")].map(|(family, label)| (Some(family), label.to_owned())).to_vec(),
                        on_commit: move |family| if let Some(family) = family { load.call(GeometryRequest::Standard(family)); },
                    }
                }
                if let Some(family) = profile.switch_family {
                    p { "Plate underside to PCB top: {profile.plate_to_pcb:.2} mm" }
                    if show_standard {
                        Button { disabled: busy(), onclick: move |_| load.call(GeometryRequest::Standard(family)), "Use standard cutout" }
                    }
                } else {
                    NumericField { label: "Plate underside to PCB top", unit: "mm", value: profile.plate_to_pcb,
                        min: Some(0.0), step: Some(0.01), disabled_reason: read_only.clone(),
                        on_commit: move |value| draft.with_mut(|profile| profile.plate_to_pcb = value),
                    }
                }
                if let Some(footprint) = footprint {
                    InspectorSection { title: "Footprint geometry",
                        Button { disabled: busy(),
                            onclick: { let footprint = footprint.clone(); move |_| load.call(GeometryRequest::KiCad { source: footprint.clone(), apply: false }) },
                            "Read KiCad layers"
                        }
                        if let Some(geometry) = geometry() {
                            for primitive in geometry.primitives {
                                SelectField::<Option<MechanicalPurpose>> { key: "{primitive.id}",
                                    label: format!("{} · {}", primitive.layer.as_deref().unwrap_or("Geometry"), primitive.id),
                                    value: purposes().get(&primitive.id).copied(), disabled_reason: read_only.clone(),
                                    options: [(None, "Unused"), (Some(MechanicalPurpose::PlateCutout), "Plate cutout"),
                                        (Some(MechanicalPurpose::ElectricalPcbMountingHole), "PCB mounting hole"),
                                        (Some(MechanicalPurpose::ClearanceEnvelope), "Clearance envelope"),
                                        (Some(MechanicalPurpose::DrawingGuide), "Drawing guide")]
                                        .map(|(purpose, label)| (purpose, label.to_owned())).to_vec(),
                                    on_commit: move |purpose| purposes.with_mut(|mappings| {
                                        if let Some(purpose) = purpose { mappings.insert(primitive.id.clone(), purpose); }
                                        else { mappings.remove(&primitive.id); }
                                    }),
                                }
                            }
                            Button { disabled: busy() || purposes().is_empty(),
                                onclick: move |_| load.call(GeometryRequest::KiCad { source: footprint.clone(), apply: true }),
                                "Apply selected geometry"
                            }
                        }
                    }
                }
                {contours(draft, false)}
                {contours(draft, true)}
                if busy() { div { role: "status", Spinner {} "Loading fit geometry…" } }
                if let Some(message) = error() { Alert { variant: AlertVariant::Destructive, "{message}" } }
                Button { disabled: busy(), onclick: move |_| {
                    if current() {
                        let resolver = mechanical_profile_resolver(source, save_id.clone(), definition.clone(), draft());
                        match runtime.commit("parts-mechanical-profile", resolver) {
                            Ok(_) => on_close.call(()),
                            Err(message) => error.set(Some(message)),
                        }
                    }
                }, "Save fit profile" }
            }
        }
    }

    fn profile_contours(
        profile: &mut MechanicalPartProfile,
        clearance: bool,
    ) -> &mut Vec<Vec<Vec2>> {
        if clearance {
            profile.clearances.get_or_insert_with(Vec::new)
        } else {
            &mut profile.cutouts
        }
    }

    fn contours(mut draft: Signal<MechanicalPartProfile>, clearance: bool) -> Element {
        let data = if clearance {
            draft().clearances.unwrap_or_default()
        } else {
            draft().cutouts
        };

        rsx! {
            InspectorSection { title: if clearance { "Component clearances" } else { "Plate cutouts" },
                Button { onclick: move |_| draft.with_mut(|profile| {
                    profile_contours(profile, clearance).push([(-2.5, -2.5), (2.5, -2.5), (2.5, 2.5), (-2.5, 2.5)].map(|(x, y)| Vec2 { x, y }).into());
                }), if clearance { "Add clearance" } else { "Add cutout" } }
                for (index, contour) in data.into_iter().enumerate() {
                    fieldset { key: "{index}",
                        legend { "Contour {index + 1}" }
                        for (point_index, point) in contour.into_iter().enumerate() {
                            div { class: "grid grid-cols-2 gap-2",
                                for (axis, value) in [("X", point.x), ("Y", point.y)] {
                                    NumericField { key: "{axis}", label: "Vertex {point_index + 1} {axis}", unit: "mm", value,
                                        on_commit: move |value| draft.with_mut(|profile| {
                                            if let Some(point) = profile_contours(profile, clearance).get_mut(index).and_then(|contour| contour.get_mut(point_index)) {
                                                if axis == "X" { point.x = value; } else { point.y = value; }
                                            }
                                        }),
                                    }
                                }
                            }
                        }
                        Button { onclick: move |_| draft.with_mut(|profile| {
                            if let Some(contour) = profile_contours(profile, clearance).get_mut(index)
                                && let Some(point) = contour.last().copied() { contour.push(point); }
                        }), "Add vertex" }
                        Button { onclick: move |_| draft.with_mut(|profile| {
                            let contours = profile_contours(profile, clearance);
                            if index < contours.len() { contours.remove(index); }
                        }), "Remove contour" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn switch() -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id": "switch", "name": "Switch", "kind": "switch", "courtyard": [], "pads": []
        }))
        .expect("switch fixture")
    }

    #[test]
    fn standard_fit_controls_distinguish_switches_from_saved_family() {
        let mut definition = switch();
        let mut draft = initial_profile(&definition, None);
        draft.switch_family = None;
        assert_eq!(
            standard_profile_controls(&definition, &draft),
            (true, false)
        );
        draft.switch_family = Some(MechanicalSwitchFamily::ChocV1);
        assert_eq!(standard_profile_controls(&definition, &draft), (true, true));
        definition.kind = boardstudio_model::model::PartKind::Passive;
        assert_eq!(
            standard_profile_controls(&definition, &draft),
            (false, true)
        );
    }

    #[test]
    fn delayed_geometry_requires_the_same_live_profile_editor() {
        let generation = (2, 3);
        assert!(profile_editor_is_current(
            "Parts",
            generation,
            || generation,
            || true
        ));
        assert!(!profile_editor_is_current(
            "Parts",
            generation,
            || generation,
            || false
        ));
        for (workspace, active) in [("Layout", generation), ("Parts", (3, 3)), ("Parts", (2, 4))] {
            assert!(!profile_editor_is_current(
                workspace,
                generation,
                || active,
                || { panic!("obsolete editor must be rejected before resolving the owner") }
            ));
        }
    }

    #[test]
    fn loading_a_standard_fit_updates_geometry_and_retains_optional_authored_data() {
        let definition = switch();
        let mut draft = initial_profile(&definition, None);
        draft.clearances = Some(vec![vec![boardstudio_model::model::Vec2 {
            x: 1.0,
            y: 2.0,
        }]]);
        let mut loaded = initial_profile(&definition, None);
        loaded.cutouts = vec![vec![boardstudio_model::model::Vec2 { x: -2.0, y: 3.0 }]];
        let cutouts = loaded.cutouts.clone();

        merge_standard_profile(&mut draft, loaded, MechanicalSwitchFamily::ChocV1, 2.2);

        assert_eq!(draft.switch_family, Some(MechanicalSwitchFamily::ChocV1));
        assert_eq!(draft.plate_to_pcb, 2.2);
        assert_eq!(draft.cutouts, cutouts);
        assert_eq!(draft.clearances.unwrap()[0][0].x, 1.0);
    }
}
