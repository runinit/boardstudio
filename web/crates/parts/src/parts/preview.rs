//! Read-only, source-backed 2D preview for the selected Parts definition.
use super::{GeneratorPreviewDraft, GeneratorPreviewStatus};
use crate::footprint_graphics::Shape;
use crate::presentation::footprint_graphics::{self, GraphicElement};
use crate::preview_logic::*;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{Pad, PadShape, PartDefinition, Side, Vec2};
use boardstudio_web_ui_shared::canvas_layers::{LayerPanel, LayerToggleButton};
use boardstudio_web_ui_shared::svg_points::polygon_points;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    callout::{Callout, CalloutVariant},
    spinner::Spinner,
    toggle_group::{ToggleGroup, ToggleItem},
};
use boardstudio_web_ui_shared::viewport_3d::ViewportSource;
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};

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

/// The Parts slot supplies accepted immutable source identity; this component
/// owns only its transient generator request and visibility controls.
#[component]
pub fn PartsPreviewPanel(
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
            section { class: "relative order-2 flex flex-1 flex-col min-w-0 min-h-0 overflow-hidden rounded-xl bg-card shadow-sm", "aria-label": "Parts footprint preview",
                div { role: "status", Callout { "Select a component from the Parts catalogue to preview its footprint." } }
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

    let view_controls = rsx! {
        ToggleGroup {
            aria_label: "Part preview view", horizontal: true,
            pressed: Some([usize::from(show_3d())].into_iter().collect()),
            on_pressed_change: move |next: std::collections::HashSet<usize>| {
                if next.contains(&0) {
                    show_3d.set(false);
                    lease_slot.invalidate();
                } else if next.contains(&1) { show_3d.set(true); }
            },
            ToggleItem { index: 0usize, aria_label: "2D footprint", class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [@media(pointer:coarse)]:min-h-control-touch [@media(pointer:coarse)]:min-w-control-touch",
                svg { class: "fill-none stroke-current stroke-[1.5]", view_box: "0 0 20 20", "aria-hidden": "true",
                    rect { x: "4", y: "4", width: "12", height: "12", rx: "2" }
                    path { d: "M1 7h3M1 13h3M16 7h3M16 13h3M7 1v3M13 1v3M7 16v3M13 16v3" }
                }
                "2D"
            }
            ToggleItem { index: 1usize, aria_label: "3D model", class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [@media(pointer:coarse)]:min-h-control-touch [@media(pointer:coarse)]:min-w-control-touch",
                svg { class: "fill-none stroke-current stroke-[1.5]", view_box: "0 0 20 20", "aria-hidden": "true",
                    path { d: "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8" }
                }
                "3D"
            }
        }
    };
    rsx! {
        section { class: "relative order-2 flex flex-1 flex-col min-w-0 min-h-0 overflow-hidden rounded-xl bg-card shadow-sm [section[aria-roledescription=carousel]_&]:order-none [section[aria-roledescription=carousel]_&]:min-h-[260px] [section[aria-roledescription=carousel]_&]:h-[320px] [section[aria-roledescription=carousel]_&]:p-3 [section[aria-roledescription=carousel]_&]:bg-canvas", "aria-label": "Parts footprint preview",
            h2 { class: "flex-none min-h-[72px] m-0 pt-5 pr-[120px] pb-5 pl-4 text-lg [section[aria-roledescription=carousel]_&]:min-h-0 [section[aria-roledescription=carousel]_&]:p-0 [section[aria-roledescription=carousel]_&]:text-base", "{preview_title.as_deref().map(str::to_owned).unwrap_or_else(|| super::placement_label(&definition).to_owned())}" }
            if !show_3d() || recipe_pending || matching_sample.as_ref().is_none_or(|result| result.is_err()) {
                div { class: "absolute right-3 top-3 z-10 [section[aria-roledescription=carousel]_&]:static [section[aria-roledescription=carousel]_&]:my-2 [section[aria-roledescription=carousel]_&]:self-start", {view_controls.clone()} }
            }
            if let Some(draft) = generator_draft.as_ref() {
                match &draft.status {
                    GeneratorPreviewStatus::Pending => rsx! { div { role: "status", Spinner {} "Generating the current generator preview…" } },
                    GeneratorPreviewStatus::Ready => rsx! { div { role: "status", Callout { "Unapplied generator preview" } } },
                    GeneratorPreviewStatus::Failed(error) => rsx! { Alert { variant: AlertVariant::Destructive, "Generator preview failed; showing the accepted footprint: {error}" } },
                }
            }
            if show_3d() {
                if recipe_pending {
                    div { role: "status", Spinner {} "Preparing assembly footprint preview…" }
                } else { match matching_sample {
                    None => rsx! { div { role: "status", Spinner {} "Preparing isolated 3D sample…" } },
                    Some(Err(error)) => rsx! { Alert { variant: AlertVariant::Destructive, "3D Parts preview failed: {error}" } },
                    Some(Ok(preview)) => rsx! { PartsSampleViewer { preview, view_controls: view_controls.clone() } },
                }}
            } else {
                if recipe_pending {
                    div { role: "status", Spinner {} "Preparing assembly footprint preview…" }
                } else { match matching {
                    None => rsx! {
                        div { role: "status", Spinner {} "Preparing {definition.name} footprint preview…" }
                    },
                    Some(Err(PreviewFailure::Unsupported)) => rsx! {
                        div { role: "status", Callout { "A source-backed Parts preview is not available for this definition yet." } }
                    },
                    Some(Err(PreviewFailure::Failed(error))) => rsx! {
                        Alert { variant: AlertVariant::Destructive, "Footprint preview failed: {error}" }
                    },
                    Some(Ok(content)) => rsx! {
                    if let Some(notice) = definition.envelope_notice.as_deref().filter(|notice| !notice.is_empty()) {
                        div { role: "status", Callout { variant: CalloutVariant::Warning, "{notice}" } }
                    }
                    svg {
                        class: "block w-full h-full min-h-[250px] bg-canvas touch-none max-[760px]:h-[54svh] max-[760px]:min-h-[270px] [section[aria-roledescription=carousel]_&]:static [section[aria-roledescription=carousel]_&]:flex-1 [section[aria-roledescription=carousel]_&]:min-h-0 [section[aria-roledescription=carousel]_&]:h-[180px]",
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
                                                        g { class: "pointer-events-none [&_rect]:fill-geometry-body [&_rect]:[fill-opacity:0.75] [&_rect]:stroke-geometry-outline [&_rect]:stroke-[0.22]", "data-layer": "Keycap",
                                                            rect { x: "{x}", y: "{y}", width: "{width}", height: "{height}", rx: "0.9" }
                                                        }
                                                    }
                                                } else {
                                                    polygon { class: "fill-geometry-fill stroke-geometry-board stroke-[0.3]", "data-layer": "{outline.label}", points: polygon_points(&outline.points) }
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
fn PartsSampleViewer(
    preview: Rc<crate::parts_preview::PartsPreviewSnapshot>,
    #[props(default = rsx! {})] view_controls: Element,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let theme = use_context::<super::super::ResolvedAppearance>().0;
    let mut display = use_signal(boardstudio_web_ui_shared::case_display::CaseDisplay::default);
    let is_current: Rc<dyn Fn(&boardstudio_web_ui_shared::viewport_3d::ViewerIdentity) -> bool> = {
        let runtime = runtime.clone();
        let preview = preview.clone();
        Rc::new(move |identity| {
            runtime.parts_preview_snapshot_is_current(&preview)
                && boardstudio_web_ui_model::owner::owner_is_current(
                    &preview.owner.scope,
                    preview.owner.snapshot_token,
                    &identity.scope,
                    identity.snapshot_token,
                )
        })
    };
    let on_signal = {
        let runtime = runtime.clone();
        let is_current = is_current.clone();
        move |event: boardstudio_web_ui_shared::viewport_3d::ScopedViewerSignal| {
            if !event.is_current() || !is_current(&event.identity) {
                return;
            }
            if let boardstudio_web_ui_shared::viewport_3d::ViewerSignalKind::Failed(message) =
                event.kind
            {
                runtime.report(message);
            }
        }
    };
    let on_display_change =
        move |event: boardstudio_web_ui_shared::viewport_3d::ScopedDisplayChange| {
            if event.is_current() && is_current(&event.identity) {
                display.set(event.display);
            }
        };
    rsx! {
        section { class: "flex flex-1 flex-col min-w-0 min-h-0 [&>p]:my-2 [&>p]:mx-3 [&>div:has(canvas)]:relative [&>div:has(canvas)]:flex-1 [&>div:has(canvas)]:min-h-0 [&>div:has(canvas)]:overflow-hidden [&_div:has(>canvas)]:relative [&_div:has(>canvas)]:flex [&_div:has(>canvas)]:flex-1 [&_div:has(>canvas)]:min-w-0 [&_div:has(>canvas)]:min-h-[160px] [&_canvas]:flex-1 [&_canvas]:w-full [&_canvas]:min-w-0 [&_canvas]:min-h-0 [&_canvas]:h-full", "aria-label": "3D footprint model preview",
            if preview.preview.models.is_empty() {
                div { role: "status", Callout { "No 3D model is linked to this definition." } }
            }
            if let Some(rows) = preview.model_rows.as_ref() {
                for failure in &rows.failures {
                    Alert { variant: AlertVariant::Destructive, "{failure.reference}: {failure.reason}" }
                }
            }
            boardstudio_web_ui_shared::viewport_3d::Viewport3D {
                source: ViewportSource::Parts(preview.clone(), Some(view_controls)),
                display: display(),
                resolved_theme: theme().to_owned(),
                on_signal,
                on_display_change,
            }
        }
    }
}

/// The carousel uses the same compiled source geometry as the full Parts preview.
#[component]
pub(super) fn ControllerFootprintThumbnail(definition: Rc<PartDefinition>) -> Element {
    let source = use_resource(use_reactive((&definition,), |(definition,)| async move {
        let preview = load_single_preview(&definition, &Side::Front).await;
        (definition, preview)
    }));
    let matching = source
        .read()
        .clone()
        .filter(|(owner, _)| owner == &definition);
    rsx! {
        div { class: "grid place-items-center w-[168px] h-[132px] p-2 rounded-lg bg-canvas text-muted-foreground text-xs [&_svg]:block [&_svg]:w-full [&_svg]:h-full",
            match matching {
                None => rsx! { span { "Loading footprint…" } },
                Some((_, Err(_))) => rsx! { span { "Footprint preview unavailable" } },
                Some((_, Ok(content))) => rsx! {
                    svg { view_box: "{content.view_box}", preserve_aspect_ratio: "xMidYMid meet",
                        role: "img", "aria-label": "{definition.name} footprint",
                        g { transform: "scale(1,-1)",
                            for member in &content.members {
                                if let Some(outline) = &member.outline {
                                    polygon { class: "fill-geometry-fill stroke-geometry-board stroke-[0.3]", points: polygon_points(&outline.points) }
                                }
                                for pad in &member.definition.pads { {render_pad(pad, "F.Cu", false, true)} }
                                for graphic in member.drawings.iter().filter(|graphic| !matches!(graphic.shape, Shape::Text(..))) {
                                    GraphicElement { graphic: graphic.clone(), hidden: false }
                                }
                            }
                        }
                    }
                },
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
    let defaults = (
        defaults.keycap_width,
        defaults.keycap_height,
        defaults.include_keycap,
    );
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

fn render_pad(pad: &Pad, copper_layer: &str, hide_drill: bool, hide_number: bool) -> Element {
    let rx = match &pad.shape {
        PadShape::Circle | PadShape::Oval => pad.size.x.min(pad.size.y) / 2.0,
        PadShape::Roundrect => pad.size.x.min(pad.size.y) / 4.0,
        PadShape::Rect => 0.0,
    };
    let rotation = pad.rotation.unwrap_or(0.0);
    rsx! {
        g { transform: "translate({pad.at.x} {pad.at.y}) rotate({rotation})",
            rect { class: "fill-geometry-part stroke-none", "data-layer": "{copper_layer}", x: "{-pad.size.x / 2.0}", y: "{-pad.size.y / 2.0}", width: "{pad.size.x}", height: "{pad.size.y}", rx: "{rx}" }
            if let Some(drill) = pad.drill.filter(|drill| *drill > 0.0)
                && !hide_drill {
                circle { class: "{drill_class(pad)}", "data-layer": "Drills", r: "{drill / 2.0}" }
            }
            if !hide_number {
                text { class: "font-mono text-[1.8px] fill-geometry-label pointer-events-none", "data-layer": "Pad numbers", transform: "scale(1,-1)", text_anchor: "middle", x: "0", y: "{-pad.size.y / 2.0 - 0.7}", "{pad.number}" }
            }
        }
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

#[component]
fn PartsPreviewLayers(
    layers: Vec<PreviewLayer>,
    hidden: BTreeSet<String>,
    on_toggle: EventHandler<String>,
) -> Element {
    let mut groups = Vec::<(&'static str, Vec<PreviewLayer>)>::new();
    for layer in layers {
        if let Some((_, entries)) = groups.iter_mut().find(|(group, _)| *group == layer.group) {
            entries.push(layer);
        } else {
            groups.push((layer.group, vec![layer]));
        }
    }
    let content = rsx! {
        LayerPanel {
            trigger_label: "Preview layers",
            trigger_id: "m1-parts-preview-layers-trigger",
            list_id: "m1-parts-preview-layers-list",
            for (group_name, group_layers) in groups {
                div { key: "{group_name}", class: "grid gap-0.5 [&:not(:first-child)]:mt-1 [&:not(:first-child)]:pt-1 [&:not(:first-child)]:border-t [&:not(:first-child)]:border-border", role: "group", "aria-label": "{group_name}",
                    div { class: "pt-[9px] px-1.5 pb-1 text-muted-foreground font-semibold", "{group_name}" }
                    for layer in group_layers {
                        { let id = layer.id.clone();
                          let visible = !hidden.contains(&id);
                          let action = if visible { "Hide" } else { "Show" };
                          rsx! {
                              LayerToggleButton {
                                  key: "{layer.id}",
                                  label: layer.label.clone(),
                                  swatch: layer.label.clone(),
                                  aria_label: format!("{action} {}", layer.label),
                                  visible,
                                  on_toggle: move |_| on_toggle.call(id.clone()),
                              }
                          }
                        }
                    }
                }
            }
        }
    };
    boardstudio_web_ui_shared::layer_dock::use_docked_layers(content)
}
