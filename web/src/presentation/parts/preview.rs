//! Read-only, source-backed 2D preview for the selected Parts definition.
use crate::footprint_forms::{Graphic, Shape};
use crate::footprint_graphics::{self, Drawings, GraphicElement};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{EnvelopeOrigin, Pad, PadShape, PartDefinition, Side, Vec2};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

#[derive(Clone, Debug, PartialEq)]
struct PreviewInput {
    scope: Option<Scope>,
    snapshot_token: SnapshotToken,
    definition_id: String,
}

#[derive(Clone, Debug, PartialEq)]
struct PreviewOwner {
    input: PreviewInput,
    generation: u64,
}

#[derive(Clone, Debug)]
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
    drawings: Drawings,
    outline: Option<PreviewOutline>,
    layers: Vec<PreviewLayer>,
    view_box: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Visibility {
    definition_id: String,
    hidden: BTreeSet<String>,
}

/// The Parts slot supplies accepted immutable source identity; this component
/// owns only its transient generator request and visibility controls.
#[component]
pub(super) fn PartsPreviewPanel(
    definition: Option<Rc<PartDefinition>>,
    scope: Option<Scope>,
    snapshot_token: SnapshotToken,
) -> Element {
    let input = PreviewInput {
        scope: scope.clone(),
        snapshot_token,
        definition_id: definition
            .as_ref()
            .map_or_else(String::new, |definition| definition.id.clone()),
    };
    let last_input = use_hook(|| Rc::new(RefCell::new(None::<PreviewInput>)));
    let generation_counter = use_hook(|| Rc::new(Cell::new(0_u64)));
    let current_owner = next_preview_owner(&last_input, &generation_counter, input.clone());
    let generation = current_owner.generation;
    let source = use_resource(use_reactive(
        (&definition, &input, &generation),
        |(definition, input, generation)| async move {
            let result = match definition {
                Some(definition) => load_preview(definition).await,
                None => Err(PreviewFailure::Unsupported),
            };
            (PreviewOwner { input, generation }, result)
        },
    ));
    let mut visibility = use_signal(Visibility::default);

    let Some(definition) = definition else {
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
            match matching {
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
                    h2 { class: "m1-library-workspace-title", "{super::catalogue::preferred_label(&definition)}" }
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
                            if !hidden.contains("part:0") {
                                if let Some(outline) = &content.outline
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
                                for pad in &definition.pads {
                                    let pad_copper = copper_id(pad, &definition);
                                    let pad_visible = !hidden.contains(&pad_copper);
                                    if pad_visible {
                                        { render_pad(
                                            pad,
                                            pad_copper.trim_start_matches("copper:"),
                                            hidden.contains("drills"),
                                            hidden.contains("pad-labels"),
                                        ) }
                                    }
                                }
                                for (index, graphic) in content.drawings.iter().enumerate() {
                                    { let graphic_hidden = hidden.contains(&format!("graphics:{}", graphic.layer));
                                      rsx! { GraphicElement { key: "{index}", graphic: graphic.clone(), hidden: graphic_hidden } }
                                    }
                                }
                            }
                        }
                    }
                    PartsPreviewLayers { layers: content.layers, hidden, on_toggle: toggle_layer }
                },
            }
        }
    }
}

async fn load_preview(definition: Rc<PartDefinition>) -> Result<PreviewContent, PreviewFailure> {
    let generator = definition
        .generator
        .as_ref()
        .ok_or(PreviewFailure::Unsupported)?;
    let defaults = footprint_graphics::generator_preview_defaults(&generator.source)
        .await
        .map_err(PreviewFailure::Failed)?
        .ok_or(PreviewFailure::Unsupported)?;
    let keycap = keycap_size(&definition, defaults);
    let include_keycap = include_keycap(&definition, defaults);
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
    let view_box = view_box(&definition, &drawings, outline.as_ref()).ok_or_else(|| {
        PreviewFailure::Failed("The selected definition contains no 2D footprint geometry.".into())
    })?;
    let layers = preview_layers(&definition, &drawings, outline.as_ref());
    Ok(PreviewContent {
        drawings,
        outline,
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
    if authored {
        if let Some(saved) = saved {
            return Some(saved);
        }
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
) -> Vec<PreviewLayer> {
    let mut copper = BTreeSet::new();
    for pad in &definition.pads {
        copper.insert(copper_id(pad, definition));
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

fn copper_id(pad: &Pad, definition: &PartDefinition) -> String {
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
                "copper:F.Cu"
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

fn view_box(
    definition: &PartDefinition,
    drawings: &[Graphic],
    outline: Option<&PreviewOutline>,
) -> Option<String> {
    let mut bounds = Bounds::default();
    for point in &definition.courtyard {
        if outline.is_some_and(|outline| outline.label == "Courtyard") {
            bounds.include(point.x, point.y);
        }
    }
    for point in outline.into_iter().flat_map(|outline| &outline.points) {
        bounds.include(point.x, point.y);
    }
    for pad in &definition.pads {
        let angle = pad.rotation.unwrap_or(0.0).to_radians();
        let (sin, cos) = angle.sin_cos();
        for x in [-pad.size.x / 2.0, pad.size.x / 2.0] {
            for y in [-pad.size.y / 2.0, pad.size.y / 2.0] {
                bounds.include(pad.at.x + x * cos - y * sin, pad.at.y + x * sin + y * cos);
            }
        }
    }
    for graphic in drawings {
        include_graphic_bounds(&mut bounds, graphic);
    }
    // The React library preview includes its origin even when every source
    // coordinate lies on one side of it. Keep empty geometry an error first.
    bounds.finish()?;
    bounds.include(0.0, 0.0);
    let (min_x, min_y, max_x, max_y) = bounds.finish()?;
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
    let close_and_restore_focus = move || {
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
        section { class: "m1-layers", "data-open": "{open()}", aria_label: "Footprint preview layers", onkeydown: keydown,
            button { id: "m1-parts-preview-layers-trigger", class: "m1-layers-trigger", type: "button", aria_expanded: "{open()}", aria_controls: "m1-parts-preview-layers-list", onclick: move |_| open.set(!open()),
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
                                  button { key: "{layer.id}", type: "button", aria_pressed: "{visible}", aria_label: "{accessible}", onclick: move |_| on_toggle.call(id.clone()),
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
    use boardstudio_application::SessionEpoch;
    use boardstudio_core::model::{EnvelopeSource, PartGenerator, PartKind};

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

    #[test]
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
        let layers = preview_layers(&definition, &drawings, Some(&outline));
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

    #[test]
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
        assert_eq!(
            view_box(&definition, &drawings, Some(&outline)).as_deref(),
            Some("-14.000 -12.000 28.000 29.000")
        );
    }

    #[test]
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
        assert_eq!(
            view_box(&definition, &drawings, None).as_deref(),
            Some("-3.000 -10.000 14.000 13.000")
        );
    }

    #[test]
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
        let layers = preview_layers(&definition, &drawings, None);
        let ids = layers
            .iter()
            .map(|layer| layer.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            ["copper:F.Cu", "graphics:B.SilkS", "pad-labels", "part:0"]
        );
    }

    #[test]
    fn stale_async_result_cannot_reclaim_same_definition_after_selection_round_trip() {
        let input_a = PreviewInput {
            scope: Some(Scope {
                session_epoch: SessionEpoch(4),
                document_id: "doc-a".into(),
                board_id: "board-a".into(),
                instance_id: None,
            }),
            snapshot_token: SnapshotToken(7),
            definition_id: "ergogen:ceoloide/switch_mx".into(),
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

    #[test]
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

    #[test]
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

    #[test]
    fn non_plated_drills_keep_the_mechanical_hole_class() {
        let mut mechanical = pad("NPTH", Side::Front, Some(1.0));
        mechanical.plated = Some(false);
        let plated = pad("PTH", Side::Front, Some(1.0));
        assert_eq!(drill_class(&mechanical), "m1-part-drill is-mechanical");
        assert_eq!(drill_class(&plated), "m1-part-drill");
    }

    #[test]
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
