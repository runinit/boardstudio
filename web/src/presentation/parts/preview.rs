//! Read-only, source-backed 2D preview for the selected Parts definition.
use super::{GeneratorPreviewDraft, GeneratorPreviewStatus};
use crate::footprint_forms::{Graphic, Shape};
use crate::presentation::footprint_graphics::{self, Drawings, GraphicElement};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{EnvelopeOrigin, Pad, PadShape, PartDefinition, Side, Vec2};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

#[cfg(test)]
thread_local! {
    static NEXT_PARTS_TEST_PREVIEW: RefCell<Option<boardstudio_core::model::PcbPreview>> = const { RefCell::new(None) };
}

#[cfg(test)]
struct PartsTestPreviewGuard;

#[cfg(test)]
impl Drop for PartsTestPreviewGuard {
    fn drop(&mut self) {
        NEXT_PARTS_TEST_PREVIEW.with(|preview| preview.borrow_mut().take());
    }
}

#[cfg(test)]
fn seed_parts_preview_for_test(
    preview: boardstudio_core::model::PcbPreview,
) -> PartsTestPreviewGuard {
    NEXT_PARTS_TEST_PREVIEW.with(|next| {
        assert!(next.borrow_mut().replace(preview).is_none());
    });
    PartsTestPreviewGuard
}

#[cfg(test)]
fn take_parts_preview_for_test() -> Option<boardstudio_core::model::PcbPreview> {
    NEXT_PARTS_TEST_PREVIEW.with(|preview| preview.borrow_mut().take())
}

#[derive(Clone, Debug, PartialEq)]
struct PreviewInput {
    scope: Option<Scope>,
    snapshot_token: SnapshotToken,
    definition_id: String,
    definition_json: String,
    recipe_identity: String,
    selection_generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct PreviewOwner {
    input: PreviewInput,
    generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct PreviewLayer {
    id: String,
    label: String,
    group: &'static str,
}

#[derive(Clone, Debug)]
struct PreviewOutline {
    label: &'static str,
    points: Vec<Vec2>,
}

#[derive(Clone, Debug)]
enum PreviewFailure {
    Unsupported,
    Failed(String),
}

#[derive(Clone, Debug)]
struct PreviewContent {
    members: Vec<PreviewGeometry>,
    layers: Vec<PreviewLayer>,
    view_box: String,
}

#[derive(Clone, Debug)]
struct PreviewGeometry {
    definition: PartDefinition,
    drawings: Drawings,
    outline: Option<PreviewOutline>,
    at: Vec2,
    rotation: f64,
    side: Side,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Visibility {
    definition_id: String,
    hidden: BTreeSet<String>,
}

/// The Parts slot supplies accepted immutable source identity; this component
/// owns only its transient generator request and visibility controls.
#[component]
pub(in crate::presentation) fn PartsPreviewPanel(
    definition: Option<Rc<PartDefinition>>,
    recipe: Vec<crate::parts_preview::PartsPreviewRecipeMember>,
    recipe_error: Option<String>,
    recipe_pending: bool,
    recipe_identity: String,
    preview_title: Option<String>,
    scope: Option<Scope>,
    snapshot_token: SnapshotToken,
    generator_draft: Option<GeneratorPreviewDraft>,
    #[props(default = false)] start_in_3d: bool,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let selection_generation = use_context::<super::PartsSelectionGeneration>().0;
    let preview_activation = use_context::<super::PartsPreviewActivation>().0;
    let recipe = if recipe.is_empty() && recipe_error.is_none() && !recipe_pending {
        definition
            .as_ref()
            .map(
                |definition| crate::parts_preview::PartsPreviewRecipeMember {
                    id: "switch".into(),
                    definition: (**definition).clone(),
                    assets: Vec::new(),
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                    side: Side::Front,
                    generator_parameters: Default::default(),
                },
            )
            .into_iter()
            .collect::<Vec<_>>()
    } else {
        recipe
    };
    let recipe_identity = format!(
        "{}:{}",
        recipe_identity,
        serde_json::to_string(&recipe).unwrap_or_default()
    );
    let input = PreviewInput {
        scope: scope.clone(),
        snapshot_token,
        definition_id: definition
            .as_ref()
            .map_or_else(String::new, |definition| definition.id.clone()),
        definition_json: definition
            .as_ref()
            .and_then(|definition| serde_json::to_string(definition.as_ref()).ok())
            .unwrap_or_default(),
        recipe_identity,
        selection_generation: selection_generation(),
    };
    let last_input = use_hook(|| Rc::new(RefCell::new(None::<PreviewInput>)));
    let generation_counter = use_hook(|| Rc::new(Cell::new(0_u64)));
    let current_owner = next_preview_owner(&last_input, &generation_counter, input.clone());
    let generation = current_owner.generation;
    let lease_slot = use_hook(|| Rc::new(crate::parts_preview::PartsPreviewLeaseSlot::default()));
    lease_slot.select_generation(generation);
    use_drop({
        let lease_slot = lease_slot.clone();
        move || lease_slot.invalidate()
    });
    let mut show_3d = use_signal(|| start_in_3d);
    let previous_activation = use_hook(|| Rc::new(RefCell::new(None::<(String, u64, u64)>)));
    use_effect(use_reactive(
        (&input.definition_id, &preview_activation(), &generation),
        {
            let previous_activation = previous_activation.clone();
            let lease_slot = lease_slot.clone();
            move |(definition_id, activation_generation, generation)| {
                let activation = (definition_id.clone(), activation_generation, generation);
                let mut previous = previous_activation.borrow_mut();
                if let Some((previous_definition, previous_activation, previous_generation)) =
                    previous.replace(activation)
                {
                    if previous_definition != definition_id
                        || previous_activation != activation_generation
                    {
                        show_3d.set(false);
                    }
                    if previous_generation != generation {
                        lease_slot.invalidate_generation(previous_generation);
                    }
                }
            }
        },
    ));
    let source = use_resource(use_reactive(
        (
            &definition,
            &input,
            &generation,
            &recipe,
            &recipe_error,
            &recipe_pending,
        ),
        |(definition, input, generation, recipe, recipe_error, recipe_pending)| async move {
            let result = if recipe_pending {
                Err(PreviewFailure::Unsupported)
            } else if let Some(error) = recipe_error {
                Err(PreviewFailure::Failed(error))
            } else {
                match definition {
                    Some(_) if !recipe.is_empty() => load_recipe_preview(recipe).await,
                    None => Err(PreviewFailure::Unsupported),
                    _ => Err(PreviewFailure::Unsupported),
                }
            };
            (PreviewOwner { input, generation }, result)
        },
    ));
    let sample_source = use_resource(use_reactive(
        (
            &definition,
            &input,
            &generation,
            &scope,
            &show_3d(),
            &recipe,
            &recipe_error,
            &recipe_pending,
        ),
        {
            let runtime = runtime.clone();
            let lease_slot = lease_slot.clone();
            move |(
                definition,
                input,
                generation,
                scope,
                show_3d,
                recipe,
                recipe_error,
                recipe_pending,
            )| {
                let runtime = runtime.clone();
                let lease_slot = lease_slot.clone();
                let recipe_error = recipe_error.clone();
                async move {
                    if !show_3d {
                        return (PreviewOwner { input, generation }, None);
                    }
                    if recipe_pending {
                        return (PreviewOwner { input, generation }, None);
                    }
                    if let Some(error) = recipe_error {
                        return (PreviewOwner { input, generation }, Some(Err(error)));
                    }
                    let result = async {
                        let scope = scope.ok_or_else(|| {
                            "Open a project before preparing the Parts sample.".to_owned()
                        })?;
                        let _definition = definition.ok_or_else(|| {
                            "Select a component before preparing the Parts sample.".to_owned()
                        })?;
                        let accepted = runtime
                            .model()
                            .accepted
                            .ok_or_else(|| "No accepted project is available.".to_owned())?;
                        if accepted.token != input.snapshot_token
                            || runtime.scope().as_ref() != Some(&scope)
                            || accepted.document.id != scope.document_id
                        {
                            return Err(
                                "The accepted Parts selection changed before preview.".into()
                            );
                        }
                        let request_token = format!(
                            "parts-sample-{}-{}-{}",
                            accepted.token.0, accepted.document.revision, generation
                        );
                        let capture = crate::parts_preview::PartsPreviewCapture::capture(
                            &accepted,
                            &scope,
                            generation,
                            request_token,
                            &recipe,
                        )?;
                        lease_slot.replace(capture.lease.clone());
                        #[cfg(test)]
                        if let Some(preview) = take_parts_preview_for_test() {
                            return capture.accept_preview(preview, None).map(Rc::new);
                        }
                        runtime
                            .prepare_parts_library_preview(capture)
                            .await
                            .map(Rc::new)
                    }
                    .await;
                    (PreviewOwner { input, generation }, Some(result))
                }
            }
        },
    ));
    let mut visibility = use_signal(Visibility::default);

    let Some(definition) = definition else {
        lease_slot.invalidate();
        return rsx! {
            section { class: "m1-workspace-content m1-parts-preview", "aria-label": "Parts footprint preview",
                p { class: "m1-parts-empty", role: "status", "Select a component from the Parts catalogue to preview its footprint." }
            }
        };
    };
    let definition_id = definition.id.clone();
    let stored_visibility = visibility.read().clone();
    let hidden = visible_hidden(&definition_id, &stored_visibility);
    let source_state = source.read().clone();
    let matching = source_state
        .filter(|(owner, _)| owner_is_current(owner, &current_owner))
        .map(|(_, result)| result);
    let sample_state = sample_source.read().clone();
    let matching_sample = sample_state
        .filter(|(owner, _)| owner_is_current(owner, &current_owner))
        .and_then(|(_, result)| result);

    let toggle_layer = move |layer_id: String| {
        let mut next = visibility();
        if next.definition_id != definition_id {
            next.definition_id = definition_id.clone();
            next.hidden.clear();
        }
        if !next.hidden.insert(layer_id.clone()) {
            next.hidden.remove(&layer_id);
        }
        visibility.set(next);
    };

    rsx! {
        section { class: "m1-workspace-content m1-parts-preview", "aria-label": "Parts footprint preview",
            h2 { class: "m1-library-workspace-title", "{preview_title.as_deref().map(str::to_owned).unwrap_or_else(|| super::placement_label(&definition).to_owned())}" }
            div { class: "m1-design-view-group", role: "group", aria_label: "Part preview view",
                button {
                    r#type: "button",
                    aria_pressed: "{!show_3d()}",
                    onclick: move |_| {
                        show_3d.set(false);
                        lease_slot.invalidate();
                    },
                    "2D footprint"
                }
                button {
                    r#type: "button",
                    aria_pressed: "{show_3d()}",
                    onclick: move |_| show_3d.set(true),
                    "3D model"
                }
            }
            if let Some(draft) = generator_draft.as_ref() {
                match &draft.status {
                    GeneratorPreviewStatus::Pending => rsx! { p { class: "m1-parts-preview-status", role: "status", "Generating the current generator preview…" } },
                    GeneratorPreviewStatus::Ready => rsx! { p { class: "m1-parts-preview-status", role: "status", "Unapplied generator preview" } },
                    GeneratorPreviewStatus::Failed(error) => rsx! { p { class: "m1-parts-preview-error", role: "alert", "Generator preview failed; showing the accepted footprint: {error}" } },
                }
            }
            if show_3d() {
                if recipe_pending {
                    p { class: "m1-parts-loading", role: "status", "Preparing assembly footprint preview…" }
                } else { match matching_sample {
                    None => rsx! { p { class: "m1-parts-loading", role: "status", "Preparing isolated 3D sample…" } },
                    Some(Err(error)) => rsx! { p { class: "m1-parts-load-error", role: "alert", "3D Parts preview failed: {error}" } },
                    Some(Ok(preview)) => rsx! { PartsSampleViewer { preview } },
                }}
            } else {
                if recipe_pending {
                    p { class: "m1-parts-loading", role: "status", "Preparing assembly footprint preview…" }
                } else { match matching {
                    None => rsx! {
                        p { class: "m1-parts-loading", role: "status", "Preparing {definition.name} footprint preview…" }
                    },
                    Some(Err(PreviewFailure::Unsupported)) => rsx! {
                        p { class: "m1-parts-empty", role: "status", "A source-backed Parts preview is not available for this definition yet." }
                    },
                    Some(Err(PreviewFailure::Failed(error))) => rsx! {
                        p { class: "m1-parts-load-error", role: "alert", "Footprint preview failed: {error}" }
                    },
                    Some(Ok(content)) => rsx! {
                    if let Some(notice) = definition.envelope_notice.as_deref().filter(|notice| !notice.is_empty()) {
                        p { class: "m1-parts-preview-note", role: "status", "{notice}" }
                    }
                    svg {
                        class: "m1-canvas",
                        view_box: "{content.view_box}",
                        preserve_aspect_ratio: "xMidYMid meet",
                        role: "img",
                        "aria-label": "{definition.name} footprint preview",
                        g { transform: "scale(1,-1)",
                            for (part_index, member) in content.members.iter().enumerate() {
                                { let part_hidden = hidden.contains(&format!("part:{part_index}"));
                                  let transform = format!("translate({} {}) rotate({})", member.at.x, member.at.y, member.rotation);
                                  rsx! {
                                    g { transform: "{transform}",
                                        if !part_hidden {
                                            if let Some(outline) = &member.outline
                                                && !hidden.contains(&format!("outline:{}", outline.label)) {
                                                if outline.label == "Keycap" {
                                                    if let Some((x, y, width, height)) = outline_bounds(&outline.points) {
                                                        g { class: "m1-keycap-overlay", "data-layer": "Keycap",
                                                            rect { x: "{x}", y: "{y}", width: "{width}", height: "{height}", rx: "0.9" }
                                                        }
                                                    }
                                                } else {
                                                    polygon { class: "m1-outline", "data-layer": "{outline.label}", points: polygon_points(&outline.points) }
                                                }
                                            }
                                            for pad in &member.definition.pads {
                                                { let pad_copper = copper_id(pad, &member.definition, &member.side);
                                                  let pad_visible = !hidden.contains(&pad_copper);
                                                  rsx! {
                                                    if pad_visible {
                                                        { render_pad(
                                                            pad,
                                                            pad_copper.trim_start_matches("copper:"),
                                                            hidden.contains("drills"),
                                                            hidden.contains("pad-labels"),
                                                        ) }
                                                    }
                                                  }
                                                }
                                            }
                                            for (index, graphic) in member.drawings.iter().enumerate() {
                                                { let graphic_hidden = hidden.contains(&format!("graphics:{}", graphic.layer));
                                                  rsx! { GraphicElement { key: "{part_index}-{index}", graphic: graphic.clone(), hidden: graphic_hidden } }
                                                }
                                            }
                                        }
                                    }
                                  }
                                }
                            }
                        }
                    }
                    PartsPreviewLayers { layers: content.layers, hidden, on_toggle: toggle_layer }
                    },
                }}
            }
        }
    }
}

#[component]
fn PartsSampleViewer(preview: Rc<crate::parts_preview::PartsPreviewSnapshot>) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let theme = use_context::<super::super::ResolvedTheme>().0;
    let mut display = use_signal(super::super::case_display::CaseDisplay::default);
    let on_signal = {
        let runtime = runtime.clone();
        let preview = preview.clone();
        move |event: super::super::shared_viewer::ScopedViewerSignal| {
            if !event.is_current()
                || !runtime.parts_preview_snapshot_is_current(&preview)
                || event.identity.scope != preview.owner.scope
                || event.identity.snapshot_token != preview.owner.snapshot_token
            {
                return;
            }
            if let super::super::shared_viewer::ViewerSignalKind::Failed(message) = event.kind {
                runtime.report(message);
            }
        }
    };
    let on_display_change = {
        let runtime = runtime.clone();
        let preview = preview.clone();
        move |event: super::super::shared_viewer::ScopedDisplayChange| {
            if event.is_current()
                && runtime.parts_preview_snapshot_is_current(&preview)
                && event.identity.scope == preview.owner.scope
                && event.identity.snapshot_token == preview.owner.snapshot_token
            {
                display.set(event.display);
            }
        }
    };
    rsx! {
        section { class: "m1-parts-sample-viewer", "aria-label": "3D footprint model preview",
            if preview.preview.models.is_empty() {
                p { class: "m1-parts-preview-status", role: "status", "No 3D model is linked to this definition." }
            }
            if let Some(rows) = preview.model_rows.as_ref() {
                for failure in &rows.failures {
                    p { class: "m1-parts-load-error", role: "alert", "{failure.reference}: {failure.reason}" }
                }
            }
            super::super::shared_viewer::CaseSharedViewer {
                scene: None,
                preview: None,
                layout_preview: None,
                keycaps_preview: None,
                parts_preview: Some(preview.clone()),
                model_rows: None,
                selected_layer: "pcb".to_owned(),
                display: display(),
                resolved_theme: theme().to_owned(),
                on_signal,
                on_display_change,
                mechanical_settings: None,
            }
        }
    }
}

async fn load_recipe_preview(
    recipe: Vec<crate::parts_preview::PartsPreviewRecipeMember>,
) -> Result<PreviewContent, PreviewFailure> {
    let mut members = Vec::with_capacity(recipe.len());
    let mut layers = Vec::new();
    for (index, source) in recipe.into_iter().enumerate() {
        let content = load_single_preview(&source.definition, &source.side).await?;
        let mut member = content.members.into_iter().next().ok_or_else(|| {
            PreviewFailure::Failed("The footprint preview returned no member.".into())
        })?;
        member.at = source.at;
        member.rotation = source.rotation;
        member.side = source.side;
        for mut layer in content.layers {
            if layer.id == "part:0" {
                layer.id = format!("part:{index}");
                layer.label = super::catalogue::preferred_label(&source.definition).to_owned();
            }
            if !layers
                .iter()
                .any(|existing: &PreviewLayer| existing.id == layer.id)
            {
                layers.push(layer);
            }
        }
        members.push(member);
    }
    let view_box = recipe_view_box(&members).unwrap_or_else(|| "0 0 0 0".into());
    Ok(PreviewContent {
        members,
        layers,
        view_box,
    })
}

async fn load_single_preview(
    definition: &PartDefinition,
    part_side: &Side,
) -> Result<PreviewContent, PreviewFailure> {
    if definition.kicad_source.is_some() {
        return source_backed_preview(definition, part_side);
    }

    let generator = definition
        .generator
        .as_ref()
        .ok_or(PreviewFailure::Unsupported)?;
    let defaults = footprint_graphics::generator_preview_defaults(&generator.source)
        .await
        .map_err(PreviewFailure::Failed)?
        .ok_or(PreviewFailure::Unsupported)?;
    let keycap = keycap_size(definition, defaults);
    let include_keycap = include_keycap(definition, defaults);
    // React draws the library envelope separately from generator graphics.
    let drawings =
        footprint_graphics::generator_drawings((*definition).clone(), None, keycap.map(|_| false))
            .await
            .map_err(PreviewFailure::Failed)?
            .ok_or(PreviewFailure::Unsupported)?;
    let outline = if let Some(size) = keycap.filter(|_| include_keycap) {
        Some(PreviewOutline {
            label: "Keycap",
            points: rectangle_points(size),
        })
    } else if keycap.is_none() && !definition.courtyard.is_empty() {
        Some(PreviewOutline {
            label: "Courtyard",
            points: definition.courtyard.clone(),
        })
    } else {
        None
    };
    let layers = preview_layers(definition, &drawings, outline.as_ref(), part_side);
    let member = PreviewGeometry {
        definition: definition.clone(),
        drawings,
        outline: outline.clone(),
        at: Vec2 { x: 0.0, y: 0.0 },
        rotation: 0.0,
        side: Side::Front,
    };
    let view_box =
        recipe_view_box(std::slice::from_ref(&member)).unwrap_or_else(|| "0 0 0 0".into());
    Ok(PreviewContent {
        members: vec![member],
        layers,
        view_box,
    })
}

fn source_backed_preview(
    definition: &PartDefinition,
    part_side: &Side,
) -> Result<PreviewContent, PreviewFailure> {
    let drawings = Rc::new(Vec::new());
    let outline = (!definition.courtyard.is_empty()).then(|| PreviewOutline {
        label: "Courtyard",
        points: definition.courtyard.clone(),
    });
    let layers = preview_layers(definition, &drawings, outline.as_ref(), part_side);
    let member = PreviewGeometry {
        definition: definition.clone(),
        drawings,
        outline: outline.clone(),
        at: Vec2 { x: 0.0, y: 0.0 },
        rotation: 0.0,
        side: Side::Front,
    };
    let view_box =
        recipe_view_box(std::slice::from_ref(&member)).unwrap_or_else(|| "0 0 0 0".into());
    Ok(PreviewContent {
        members: vec![member],
        layers,
        view_box,
    })
}

fn next_preview_owner(
    last_input: &Rc<RefCell<Option<PreviewInput>>>,
    generation_counter: &Rc<Cell<u64>>,
    input: PreviewInput,
) -> PreviewOwner {
    let mut previous = last_input.borrow_mut();
    if previous.as_ref() != Some(&input) {
        generation_counter.set(generation_counter.get().wrapping_add(1));
        *previous = Some(input.clone());
    }
    PreviewOwner {
        input,
        generation: generation_counter.get(),
    }
}

fn valid_size(size: Vec2) -> Option<Vec2> {
    (size.x.is_finite() && size.y.is_finite() && size.x > 0.0 && size.y > 0.0).then_some(size)
}

fn keycap_size(
    definition: &PartDefinition,
    defaults: footprint_graphics::GeneratorPreviewDefaults,
) -> Option<Vec2> {
    let saved = definition.keycap.and_then(valid_size);
    let authored = definition
        .envelope_source
        .as_ref()
        .is_none_or(|source| source.keycap == Some(EnvelopeOrigin::Authored));
    if authored && let Some(saved) = saved {
        return Some(saved);
    }

    let parameters = definition
        .generator
        .as_ref()
        .map(|generator| &generator.parameters);
    let width = parameters
        .and_then(|parameters| parameters.get("keycap_width"))
        .and_then(serde_json::Value::as_f64)
        .or(defaults.keycap_width);
    let height = parameters
        .and_then(|parameters| parameters.get("keycap_height"))
        .and_then(serde_json::Value::as_f64)
        .or(defaults.keycap_height);
    let generated = width
        .zip(height)
        .map(|(x, y)| Vec2 { x, y })
        .and_then(valid_size);
    generated.or(saved)
}

fn include_keycap(
    definition: &PartDefinition,
    defaults: footprint_graphics::GeneratorPreviewDefaults,
) -> bool {
    definition
        .generator
        .as_ref()
        .and_then(|generator| generator.parameters.get("include_keycap"))
        .and_then(serde_json::Value::as_bool)
        .or(defaults.include_keycap)
        .unwrap_or(true)
}

fn rectangle_points(size: Vec2) -> Vec<Vec2> {
    vec![
        Vec2 {
            x: -size.x / 2.0,
            y: -size.y / 2.0,
        },
        Vec2 {
            x: size.x / 2.0,
            y: -size.y / 2.0,
        },
        Vec2 {
            x: size.x / 2.0,
            y: size.y / 2.0,
        },
        Vec2 {
            x: -size.x / 2.0,
            y: size.y / 2.0,
        },
    ]
}

fn preview_layers(
    definition: &PartDefinition,
    drawings: &[Graphic],
    outline: Option<&PreviewOutline>,
    part_side: &Side,
) -> Vec<PreviewLayer> {
    let mut copper = BTreeSet::new();
    for pad in &definition.pads {
        copper.insert(copper_id(pad, definition, part_side));
    }
    let mut layers = Vec::new();
    for id in ["copper:F.Cu", "copper:B.Cu"] {
        if copper.contains(id) {
            layers.push(PreviewLayer {
                id: id.into(),
                label: id.trim_start_matches("copper:").into(),
                group: "Footprint",
            });
        }
    }
    let graphics = drawings
        .iter()
        .map(|graphic| graphic.layer.clone())
        .collect::<BTreeSet<_>>();
    for layer in graphics {
        layers.push(PreviewLayer {
            id: format!("graphics:{layer}"),
            label: layer,
            group: "Footprint",
        });
    }
    if let Some(outline) = outline {
        layers.push(PreviewLayer {
            id: format!("outline:{}", outline.label),
            label: outline.label.into(),
            group: "Footprint",
        });
    }
    if definition
        .pads
        .iter()
        .any(|pad| pad.drill.is_some_and(|drill| drill > 0.0))
    {
        layers.push(PreviewLayer {
            id: "drills".into(),
            label: "Drills".into(),
            group: "Footprint",
        });
    }
    if !definition.pads.is_empty() {
        layers.push(PreviewLayer {
            id: "pad-labels".into(),
            label: "Pad numbers".into(),
            group: "Footprint",
        });
    }
    layers.push(PreviewLayer {
        id: "part:0".into(),
        label: super::catalogue::preferred_label(definition).into(),
        group: "Parts",
    });
    layers
}

fn copper_id(pad: &Pad, definition: &PartDefinition, part_side: &Side) -> String {
    let side = pad.side.as_ref().map_or_else(
        || {
            let configured_back = definition
                .generator
                .as_ref()
                .and_then(|generator| generator.parameters.get("side"))
                .and_then(serde_json::Value::as_str)
                == Some("B");
            if configured_back {
                "copper:B.Cu"
            } else {
                match part_side {
                    Side::Back => "copper:B.Cu",
                    Side::Front => "copper:F.Cu",
                }
            }
        },
        |side| match side {
            Side::Back => "copper:B.Cu",
            Side::Front => "copper:F.Cu",
        },
    );
    side.into()
}

fn owner_is_current(captured: &PreviewOwner, current: &PreviewOwner) -> bool {
    captured == current
}

fn visible_hidden(definition_id: &str, visibility: &Visibility) -> BTreeSet<String> {
    if visibility.definition_id == definition_id {
        visibility.hidden.clone()
    } else {
        BTreeSet::new()
    }
}

fn recipe_view_box(members: &[PreviewGeometry]) -> Option<String> {
    let mut complete = Bounds::default();
    for member in members {
        let mut local = Bounds::default();
        if member
            .outline
            .as_ref()
            .is_some_and(|outline| outline.label == "Courtyard")
        {
            for point in &member.definition.courtyard {
                local.include(point.x, point.y);
            }
        }
        if let Some(outline) = member.outline.as_ref() {
            for point in &outline.points {
                local.include(point.x, point.y);
            }
        }
        for pad in &member.definition.pads {
            let angle = pad.rotation.unwrap_or(0.0).to_radians();
            let (sin, cos) = angle.sin_cos();
            for x in [-pad.size.x / 2.0, pad.size.x / 2.0] {
                for y in [-pad.size.y / 2.0, pad.size.y / 2.0] {
                    local.include(pad.at.x + x * cos - y * sin, pad.at.y + x * sin + y * cos);
                }
            }
        }
        for graphic in member.drawings.iter() {
            include_graphic_bounds(&mut local, graphic);
        }
        let Some((min_x, min_y, max_x, max_y)) = local.finish() else {
            continue;
        };
        let rotation = member.rotation.to_radians();
        let (sin, cos) = rotation.sin_cos();
        for x in [min_x, max_x] {
            for y in [min_y, max_y] {
                complete.include(
                    member.at.x + x * cos - y * sin,
                    member.at.y + x * sin + y * cos,
                );
            }
        }
    }
    // Match the reference's empty source-backed library surface. A zero-size
    // viewBox represents no geometry and deliberately adds no synthetic point.
    let Some(_) = complete.finish() else {
        return Some("0 0 0 0".into());
    };
    complete.include(0.0, 0.0);
    let (min_x, min_y, max_x, max_y) = complete.finish()?;
    let margin = 3.0;
    Some(format!(
        "{:.3} {:.3} {:.3} {:.3}",
        min_x - margin,
        -(max_y + margin),
        max_x - min_x + margin * 2.0,
        max_y - min_y + margin * 2.0
    ))
}

#[derive(Default)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    has_point: bool,
}

impl Bounds {
    fn include(&mut self, x: f64, y: f64) {
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        if !self.has_point {
            self.min_x = x;
            self.min_y = y;
            self.max_x = x;
            self.max_y = y;
            self.has_point = true;
            return;
        }
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn finish(&self) -> Option<(f64, f64, f64, f64)> {
        self.has_point
            .then_some((self.min_x, self.min_y, self.max_x, self.max_y))
    }
}

fn include_graphic_bounds(bounds: &mut Bounds, graphic: &Graphic) {
    let include = |bounds: &mut Bounds, point: &crate::footprint_forms::Point| {
        // `footprint_forms::point` already converts native Y into the preview
        // frame. GraphicElement consumes that value under the one outer flip;
        // use the same point here rather than applying a second conversion.
        bounds.include(point.0, point.1);
    };
    match &graphic.shape {
        Shape::Line(start, end) | Shape::Rect(start, end) => {
            include(bounds, start);
            include(bounds, end);
        }
        Shape::Arc(start, middle, end) => {
            include(bounds, start);
            include(bounds, middle);
            include(bounds, end);
        }
        Shape::Circle(center, radius) => {
            bounds.include(center.0 - radius, center.1 - radius);
            bounds.include(center.0 + radius, center.1 + radius);
        }
        Shape::Polygon(points, _) => {
            for point in points {
                include(bounds, point);
            }
        }
        Shape::Text(at, _) => include(bounds, at),
    }
}

fn render_pad(pad: &Pad, copper_layer: &str, hide_drill: bool, hide_number: bool) -> Element {
    let rx = match &pad.shape {
        PadShape::Circle | PadShape::Oval => pad.size.x.min(pad.size.y) / 2.0,
        PadShape::Roundrect => pad.size.x.min(pad.size.y) / 4.0,
        PadShape::Rect => 0.0,
    };
    let rotation = pad.rotation.unwrap_or(0.0);
    rsx! {
        g { transform: "translate({pad.at.x} {pad.at.y}) rotate({rotation})",
            rect { class: "m1-part-pad", "data-layer": "{copper_layer}", x: "{-pad.size.x / 2.0}", y: "{-pad.size.y / 2.0}", width: "{pad.size.x}", height: "{pad.size.y}", rx: "{rx}" }
            if let Some(drill) = pad.drill.filter(|drill| *drill > 0.0)
                && !hide_drill {
                circle { class: "{drill_class(pad)}", "data-layer": "Drills", r: "{drill / 2.0}" }
            }
            if !hide_number {
                text { class: "m1-part-label", "data-layer": "Pad numbers", transform: "scale(1,-1)", text_anchor: "middle", x: "0", y: "{-pad.size.y / 2.0 - 0.7}", "{pad.number}" }
            }
        }
    }
}

fn drill_class(pad: &Pad) -> &'static str {
    if pad.plated == Some(false) {
        "m1-part-drill is-mechanical"
    } else {
        "m1-part-drill"
    }
}

fn outline_bounds(points: &[Vec2]) -> Option<(f64, f64, f64, f64)> {
    let first = points.first()?;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
    for point in &points[1..] {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Some((min_x, min_y, max_x - min_x, max_y - min_y))
}

fn polygon_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ")
}

#[component]
fn PartsPreviewLayers(
    layers: Vec<PreviewLayer>,
    hidden: BTreeSet<String>,
    on_toggle: EventHandler<String>,
) -> Element {
    let mut open = use_signal(|| false);
    let mut close_and_restore_focus = move || {
        open.set(false);
        if let Some(trigger) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id("m1-parts-preview-layers-trigger"))
            .and_then(|element| element.dyn_into::<HtmlElement>().ok())
        {
            let _ = trigger.focus();
        }
    };
    let keydown = move |event: KeyboardEvent| {
        if event.data().key().to_string() == "Escape" && open() {
            event.prevent_default();
            close_and_restore_focus();
        }
    };
    let mut groups = Vec::<(&'static str, Vec<PreviewLayer>)>::new();
    for layer in layers {
        if let Some((_, entries)) = groups.iter_mut().find(|(group, _)| *group == layer.group) {
            entries.push(layer);
        } else {
            groups.push((layer.group, vec![layer]));
        }
    }
    rsx! {
        section { class: "m1-layers", "data-open": "{open()}", "aria-label": "Footprint preview layers", onkeydown: keydown,
            button { id: "m1-parts-preview-layers-trigger", class: "m1-layers-trigger", type: "button", "aria-expanded": "{open()}", "aria-controls": "m1-parts-preview-layers-list", onclick: move |_| open.set(!open()),
                "Layers"
                svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: if open() { "m5 12 5-5 5 5" } else { "m5 8 5 5 5-5" } } }
            }
            if open() {
                button { class: "m1-layers-close", type: "button", onclick: move |_| close_and_restore_focus(), "Close" }
            }
            div { id: "m1-parts-preview-layers-list", class: "m1-layer-list", hidden: !open(),
                for (group_name, group_layers) in groups {
                    div { key: "{group_name}", class: "m1-layer-group", role: "group", "aria-label": "{group_name}",
                        p { class: "m1-layer-label", "{group_name}" }
                        for layer in group_layers {
                            { let id = layer.id.clone();
                              let visible = !hidden.contains(&id);
                              let action = if visible { "Hide" } else { "Show" };
                              let accessible = format!("{action} {}", layer.label);
                              rsx! {
                                  button { key: "{layer.id}", type: "button", "aria-pressed": "{visible}", "aria-label": "{accessible}", onclick: move |_| on_toggle.call(id.clone()),
                                      span { class: "m1-layer-swatch", "data-layer": "{layer.label}" }
                                      span { class: "m1-layer-label", "{layer.label}" }
                                      svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M2 10q8-12 16 0-8 12-16 0Z" }, circle { cx: "10", cy: "10", r: "2.5" }, if !visible { path { d: "m3 17 14-14" } } }
                                  }
                              }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{AcceptedSnapshot, Scope, SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{Board, ProjectDoc, SceneDelta};
    use std::sync::Arc;
    use wasm_bindgen_test::wasm_bindgen_test;
    use web_sys::{Element as DomElement, HtmlElement};

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
    use boardstudio_core::model::{EnvelopeSource, KicadSource, PartGenerator, PartKind};

    fn definition() -> PartDefinition {
        PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: "ergogen:ceoloide/switch_mx".into(),
            name: "switch mx".into(),
            kind: PartKind::Switch,
            keycap: Some(Vec2 { x: 18.0, y: 18.0 }),
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![
                Vec2 { x: -8.0, y: -8.0 },
                Vec2 { x: 8.0, y: -8.0 },
                Vec2 { x: 8.0, y: 8.0 },
                Vec2 { x: -8.0, y: 8.0 },
            ],
            pads: vec![pad("1", Side::Front, Some(1.5)), pad("2", Side::Back, None)],
            models: None,
            generator: Some(PartGenerator {
                source: "ceoloide/switch_mx".into(),
                version: "bundled-1".into(),
                parameters: Default::default(),
            }),
            mechanical_profile: None,
        }
    }

    fn pad(number: &str, side: Side, drill: Option<f64>) -> Pad {
        Pad {
            id: format!("pad-{number}"),
            number: number.into(),
            at: Vec2::default(),
            size: Vec2 { x: 2.0, y: 2.0 },
            shape: PadShape::Rect,
            drill,
            plated: Some(true),
            side: Some(side),
            rotation: None,
            net_id: None,
        }
    }

    #[derive(Clone)]
    struct PartsMountedFixture {
        snapshot: AcceptedSnapshot,
        scope: Scope,
        definition: PartDefinition,
    }

    #[component]
    fn parts_renderer_failure_host() -> Element {
        let fixture = use_context::<PartsMountedFixture>();
        let mut selection = use_signal(|| None::<(Option<Scope>, String)>);
        let selected_context = use_signal(|| None);
        let anchor_scope = use_signal(|| None::<Scope>);
        let generation = use_signal(|| 1_u64);
        let adapter = use_hook(|| {
            super::super::super::selection::SelectionAdapter::new(
                selected_context,
                anchor_scope,
                generation,
            )
        });
        use_context_provider(|| adapter.clone());
        use_context_provider(|| super::super::PartsSelectionGeneration(generation));
        let activation = use_signal(|| 0_u64);
        use_context_provider(|| super::super::PartsPreviewActivation(activation));
        let workspace = use_signal(|| "Parts");
        use_context_provider(|| super::super::super::WorkspaceState(workspace));
        let theme = use_memo(|| "light");
        use_context_provider(|| super::super::super::ResolvedTheme(theme));
        let _: Signal<u64> = use_context_provider(|| Signal::new(0_u64));

        rsx! {
            super::super::mechanical_profile_ui::PartsMechanicalProfileWorkspace {
                snapshot: fixture.snapshot.clone(),
                scope: Some(fixture.scope.clone()),
                selection,
                definition: fixture.definition.clone(),
                preview_definition: Some(fixture.definition.clone()),
                generator_draft: None,
                recipe: Vec::new(),
                recipe_error: None,
                recipe_pending: false,
                recipe_identity: "parts-renderer-init-test".to_owned(),
                preview_title: None,
                source: crate::parts_mechanical_profile::ProfileDefinitionSource::Ergogen,
            }
        }
    }

    fn accepted_parts_fixture() -> (AcceptedSnapshot, Scope) {
        let mut document = ProjectDoc::empty("parts-renderer-init-project", "Parts test");
        document.revision = 7;
        let board_id = "parts-renderer-init-board";
        document.boards.push(Board {
            id: board_id.into(),
            name: "Parts test board".into(),
            outline_ids: Vec::new(),
            part_ids: Vec::new(),
            net_ids: Vec::new(),
            thickness: 1.6,
            traces: Vec::new(),
            vias: Vec::new(),
        });
        let session_epoch = SessionEpoch(3);
        let scope = Scope {
            session_epoch,
            document_id: document.id.clone(),
            board_id: board_id.into(),
            instance_id: None,
        };
        let scene: SceneDelta = serde_json::from_value(serde_json::json!({
            "revision": 7,
            "transactionId": "parts-renderer-init-fixture",
            "changedIds": [],
            "transforms": [],
            "matrixScenes": [],
            "contours": [],
            "boardContours": [],
            "boardReadiness": [],
            "findings": [],
            "readiness": {
                "layout": false,
                "outline": false,
                "pcb": false,
                "case": false
            }
        }))
        .unwrap();
        (
            AcceptedSnapshot {
                token: SnapshotToken(11),
                session_epoch,
                document: Arc::new(document),
                scene: Arc::new(scene),
            },
            scope,
        )
    }

    fn click_button(root: &DomElement, label: &str) {
        let buttons = root.query_selector_all("button").unwrap();
        for index in 0..buttons.length() {
            let Some(button) = buttons.item(index) else {
                continue;
            };
            if button.text_content().unwrap_or_default().trim() == label {
                button.dyn_into::<HtmlElement>().unwrap().click();
                return;
            }
        }
        panic!("missing mounted Parts button: {label}");
    }

    fn has_button(root: &DomElement, label: &str) -> bool {
        let buttons = root.query_selector_all("button").unwrap();
        (0..buttons.length()).any(|index| {
            buttons
                .item(index)
                .and_then(|button| button.text_content())
                .is_some_and(|text| text.trim() == label)
        })
    }

    async fn wait_for(root: &DomElement, selector: &str) -> DomElement {
        for _ in 0..500 {
            if let Some(element) = root.query_selector(selector).unwrap() {
                return element;
            }
            gloo_timers::future::TimeoutFuture::new(20).await;
        }
        let text = root.text_content().unwrap_or_default();
        let summary = text.chars().take(800).collect::<String>();
        panic!("timed out waiting for Parts preview element: {selector}; rendered text: {summary}");
    }

    async fn wait_for_button(root: &DomElement, label: &str) {
        for _ in 0..100 {
            if has_button(root, label) {
                return;
            }
            gloo_timers::future::TimeoutFuture::new(20).await;
        }
        panic!("timed out waiting for mounted Parts button: {label}");
    }

    #[wasm_bindgen_test]
    async fn parts_renderer_initialization_failure_keeps_form_and_returns_to_2d() {
        let runtime = crate::runtime::Runtime::new().expect("browser runtime fixture initializes");
        let (snapshot, scope) = accepted_parts_fixture();
        runtime.set_definition_name_test_state(snapshot.clone(), Some(scope.clone()));
        let mut definition = definition();
        definition.generator = None;
        definition.kicad_source = Some(KicadSource {
            format_version: 1,
            source:
                "(footprint \"Parts test switch\" (version 20240108) (generator \"BoardStudio\"))"
                    .into(),
        });
        let fixture = PartsMountedFixture {
            snapshot: snapshot.clone(),
            scope,
            definition,
        };
        let before = runtime
            .model()
            .accepted
            .expect("fixture project is current");

        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("parts-renderer-init-failure-test");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = dioxus::prelude::VirtualDom::new(parts_renderer_failure_host);
        dom.provide_root_context(runtime.clone());
        dom.provide_root_context(fixture);
        let _failure = super::super::super::shared_viewer::fail_next_renderer_mount_for_test(
            "injected renderer initialization failure",
        );
        let _preview = super::seed_parts_preview_for_test(boardstudio_core::model::PcbPreview {
            revision: snapshot.document.revision,
            thickness: 1.6,
            contours: Vec::new(),
            surfaces: Vec::new(),
            holes: Vec::new(),
            models: Vec::new(),
            diagnostics: Vec::new(),
        });
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );

        wait_for_button(&root, "3D model").await;
        click_button(&root, "3D model");
        let alert = wait_for(&root, ".m1-case-view-status[role=alert]").await;
        assert!(
            alert
                .text_content()
                .unwrap_or_default()
                .contains("injected renderer initialization failure"),
            "renderer mount failure must travel through SharedViewer's actual error status"
        );
        assert!(
            root.text_content()
                .unwrap_or_default()
                .contains("Mechanical fit")
        );
        assert!(
            root.text_content()
                .unwrap_or_default()
                .contains("Define profile")
        );
        assert!(has_button(&root, "2D footprint"));
        assert!(has_button(&root, "3D model"));

        click_button(&root, "2D footprint");
        wait_for(&root, "svg[aria-label='switch mx footprint preview']").await;
        assert!(root.query_selector("canvas").unwrap().is_none());
        assert!(
            root.text_content()
                .unwrap_or_default()
                .contains("Define profile")
        );
        let after = runtime
            .model()
            .accepted
            .expect("fixture project remains current");
        assert_eq!(after.token, before.token);
        assert_eq!(after.document.revision, before.document.revision);
        assert_eq!(after.document.id, before.document.id);
        assert!(Arc::ptr_eq(&after.document, &before.document));
        assert!(
            runtime.take_definition_name_test_event().is_none(),
            "the mounted renderer failure and 2D return must not submit a project edit"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    fn mx_preview_layer_options_follow_geometry_and_retained_source_layers() {
        let definition = definition();
        let drawings = vec![Graphic {
            layer: "B.SilkS".into(),
            shape: Shape::Line(
                crate::footprint_forms::Point(-2.0, -3.0),
                crate::footprint_forms::Point(2.0, -3.0),
            ),
        }];
        let outline = PreviewOutline {
            label: "Keycap",
            points: rectangle_points(Vec2 { x: 18.0, y: 18.0 }),
        };
        let layers = preview_layers(&definition, &drawings, Some(&outline), &Side::Front);
        assert_eq!(
            layers
                .iter()
                .map(|layer| (layer.id.as_str(), layer.label.as_str(), layer.group))
                .collect::<Vec<_>>(),
            [
                ("copper:F.Cu", "F.Cu", "Footprint"),
                ("copper:B.Cu", "B.Cu", "Footprint"),
                ("graphics:B.SilkS", "B.SilkS", "Footprint"),
                ("outline:Keycap", "Keycap", "Footprint"),
                ("drills", "Drills", "Footprint"),
                ("pad-labels", "Pad numbers", "Footprint"),
                ("part:0", "MX switch", "Parts"),
            ]
        );
    }

    #[wasm_bindgen_test]
    fn mx_view_box_contains_the_source_envelope_and_rendered_graphics() {
        let definition = definition();
        let drawings = vec![Graphic {
            layer: "B.SilkS".into(),
            shape: Shape::Line(
                crate::footprint_forms::Point(-11.0, -14.0),
                crate::footprint_forms::Point(11.0, -14.0),
            ),
        }];
        let outline = PreviewOutline {
            label: "Keycap",
            points: rectangle_points(Vec2 { x: 18.0, y: 18.0 }),
        };
        let member = PreviewGeometry {
            definition,
            drawings: Rc::new(drawings),
            outline: Some(outline),
            at: Vec2::default(),
            rotation: 0.0,
            side: Side::Front,
        };
        assert_eq!(
            recipe_view_box(&[member]).as_deref(),
            Some("-14.000 -12.000 28.000 29.000")
        );
    }

    #[wasm_bindgen_test]
    fn view_box_keeps_the_reference_origin_for_one_sided_graphics() {
        let mut definition = definition();
        definition.pads.clear();
        definition.courtyard.clear();
        let drawings = vec![Graphic {
            layer: "F.SilkS".into(),
            shape: Shape::Line(
                crate::footprint_forms::Point(5.0, 4.0),
                crate::footprint_forms::Point(8.0, 7.0),
            ),
        }];
        let member = PreviewGeometry {
            definition,
            drawings: Rc::new(drawings),
            outline: None,
            at: Vec2::default(),
            rotation: 0.0,
            side: Side::Front,
        };
        assert_eq!(
            recipe_view_box(&[member]).as_deref(),
            Some("-3.000 -10.000 14.000 13.000")
        );
    }

    #[wasm_bindgen_test]
    fn layer_options_omit_source_shapes_that_cannot_be_toggled() {
        let mut definition = definition();
        definition.pads = vec![pad("1", Side::Front, None)];
        definition.keycap = None;
        definition.courtyard.clear();
        let drawings = vec![Graphic {
            layer: "B.SilkS".into(),
            shape: Shape::Line(
                crate::footprint_forms::Point(0.0, 0.0),
                crate::footprint_forms::Point(1.0, -1.0),
            ),
        }];
        let layers = preview_layers(&definition, &drawings, None, &Side::Front);
        let ids = layers
            .iter()
            .map(|layer| layer.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            ["copper:F.Cu", "graphics:B.SilkS", "pad-labels", "part:0"]
        );
    }

    #[wasm_bindgen_test]
    fn stale_async_result_cannot_reclaim_same_definition_after_selection_round_trip() {
        let input_a = PreviewInput {
            selection_generation: 1,
            scope: Some(Scope {
                session_epoch: SessionEpoch(4),
                document_id: "doc-a".into(),
                board_id: "board-a".into(),
                instance_id: None,
            }),
            snapshot_token: SnapshotToken(7),
            definition_id: "ergogen:ceoloide/switch_mx".into(),
            definition_json: String::new(),
            recipe_identity: String::new(),
        };
        let last_input = Rc::new(RefCell::new(None));
        let generation_counter = Rc::new(Cell::new(0));
        let first_a = next_preview_owner(&last_input, &generation_counter, input_a.clone());
        assert_eq!(
            next_preview_owner(&last_input, &generation_counter, input_a.clone()),
            first_a,
            "an ordinary repaint of the same accepted resource keeps its generation"
        );
        let input_b = PreviewInput {
            definition_id: "ergogen:ceoloide/encoder".into(),
            ..input_a.clone()
        };
        let _b = next_preview_owner(&last_input, &generation_counter, input_b);
        let second_a = next_preview_owner(&last_input, &generation_counter, input_a.clone());
        assert_eq!(first_a.generation, 1);
        assert_eq!(second_a.generation, 3);
        assert!(!owner_is_current(&first_a, &second_a));

        let mut other_scope = input_a.clone();
        other_scope.scope.as_mut().unwrap().instance_id = Some("right".into());
        let scoped_owner = next_preview_owner(&last_input, &generation_counter, other_scope);
        let returned_a = next_preview_owner(&last_input, &generation_counter, input_a);
        assert_eq!(scoped_owner.generation, 4);
        assert_eq!(returned_a.generation, 5);
        assert!(!owner_is_current(&scoped_owner, &returned_a));
    }

    #[wasm_bindgen_test]
    fn generated_keycap_uses_retained_dimensions_unless_the_envelope_is_authored() {
        let defaults = footprint_graphics::GeneratorPreviewDefaults {
            keycap_width: Some(18.0),
            keycap_height: Some(18.0),
            include_keycap: Some(true),
        };
        let mut definition = definition();
        definition.envelope_source = Some(EnvelopeSource {
            courtyard: None,
            keycap: Some(EnvelopeOrigin::Generated),
        });
        definition
            .generator
            .as_mut()
            .unwrap()
            .parameters
            .insert("keycap_width".into(), serde_json::Value::from(19.0));
        definition
            .generator
            .as_mut()
            .unwrap()
            .parameters
            .insert("keycap_height".into(), serde_json::Value::from(20.0));
        assert_eq!(
            keycap_size(&definition, defaults),
            Some(Vec2 { x: 19.0, y: 20.0 })
        );

        definition.keycap = Some(Vec2 { x: 21.0, y: 22.0 });
        definition.envelope_source.as_mut().unwrap().keycap = Some(EnvelopeOrigin::Authored);
        assert_eq!(
            keycap_size(&definition, defaults),
            Some(Vec2 { x: 21.0, y: 22.0 })
        );
    }

    #[wasm_bindgen_test]
    fn keycap_visibility_prefers_generator_value_then_retained_default() {
        let defaults = footprint_graphics::GeneratorPreviewDefaults {
            include_keycap: Some(false),
            ..Default::default()
        };
        let mut definition = definition();
        assert!(!include_keycap(&definition, defaults));
        definition
            .generator
            .as_mut()
            .unwrap()
            .parameters
            .insert("include_keycap".into(), serde_json::Value::Bool(true));
        assert!(include_keycap(&definition, defaults));
    }

    #[wasm_bindgen_test]
    fn non_plated_drills_keep_the_mechanical_hole_class() {
        let mut mechanical = pad("NPTH", Side::Front, Some(1.0));
        mechanical.plated = Some(false);
        let plated = pad("PTH", Side::Front, Some(1.0));
        assert_eq!(drill_class(&mechanical), "m1-part-drill is-mechanical");
        assert_eq!(drill_class(&plated), "m1-part-drill");
    }

    #[wasm_bindgen_test]
    fn visibility_is_one_definition_pair_like_the_reference_workspace() {
        let stored_a = Visibility {
            definition_id: "a".into(),
            hidden: BTreeSet::from(["drills".into()]),
        };
        assert_eq!(
            visible_hidden("a", &stored_a),
            BTreeSet::from(["drills".into()])
        );
        assert!(visible_hidden("b", &stored_a).is_empty());
        let after_b_toggle = Visibility {
            definition_id: "b".into(),
            hidden: BTreeSet::from(["copper:F.Cu".into()]),
        };
        assert!(visible_hidden("a", &after_b_toggle).is_empty());
        assert_eq!(
            visible_hidden("b", &after_b_toggle),
            BTreeSet::from(["copper:F.Cu".into()])
        );
    }
}
