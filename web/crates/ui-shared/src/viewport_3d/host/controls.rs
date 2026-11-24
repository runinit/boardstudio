use super::{
    AssemblyView, CaseDisplay, RenderMode, TransientView, ViewerCanvasContext, ViewportHandle,
};
use crate::inspector_fields::{SelectField, ToggleField};
use crate::ui::{
    button::{Button as UiButton, ButtonSize as UiButtonSize, ButtonVariant as UiButtonVariant},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    field::Field,
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
    label::Label,
    slider::Slider,
    toggle_group::{ToggleGroup, ToggleItem},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use crate::viewport_3d::assembly_layers::{
    CaseAssemblyLayer, CaseAssemblyLayers, CaseComponentLayer,
};
use boardstudio_web_host::renderer_host::RendererPageHost;
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct ViewerOverlayConfig {
    pub(crate) assembly_layers: Vec<CaseAssemblyLayer>,
    pub(crate) component_layers: Vec<CaseComponentLayer>,
    pub(crate) selectable_layers: Vec<String>,
    pub(crate) layers: Vec<(String, String)>,
    pub(crate) selected_layer: String,
    pub(crate) selected_reference: Option<String>,
    pub(crate) display: CaseDisplay,
    pub(crate) toolbar_extra: Option<Element>,
    pub(crate) edit_labels: Vec<&'static str>,
    pub(crate) toolbar_leading: Option<Element>,
    pub(crate) handles: Vec<ViewportHandle>,
    pub(crate) gesture_message: Option<String>,
}

#[component]
pub(crate) fn ViewerChrome(
    overlay: ViewerOverlayConfig,
    context: ViewerCanvasContext,
    mut transient: Signal<TransientView>,
    mut edit_mode: Signal<Option<usize>>,
    status: Signal<String>,
    camera: EventHandler<CameraAction>,
    select_layer: EventHandler<String>,
    change_display: EventHandler<CaseDisplay>,
    children: Element,
) -> Element {
    let edit_groups = overlay
        .edit_labels
        .iter()
        .enumerate()
        .filter(|(group, _)| {
            overlay
                .handles
                .iter()
                .any(|handle| handle.edit_group == *group)
        })
        .map(|(group, _)| group)
        .collect::<Vec<_>>();
    let case_edit_controls = rsx! {
        if !edit_groups.is_empty() {
            ToggleGroup {
                aria_label: "Edit mode",
                pressed: Some(edit_mode().into_iter().collect()),
                on_pressed_change: move |next: std::collections::HashSet<usize>| {
                    let next = next.into_iter().min();
                    if next != edit_mode() {
                        edit_mode.set(next);
                        if next.is_some() { camera.call(CAMERA_VIEWS[0].2); }
                    }
                },
                for group in edit_groups { {icon_toggle(group, overlay.edit_labels[group])} }
            }
        }
    };
    let show_assembly_controls = overlay.layers.iter().any(|(id, _)| id != "pcb");
    let inline = context == ViewerCanvasContext::Case;

    let display_mode = view_choices(
        if inline {
            "Display mode".to_owned()
        } else {
            format!("{} display mode", context.name())
        },
        inline,
        transient().mode,
        [
            ("Shaded", RenderMode::Shaded),
            ("Wireframe", RenderMode::Wireframe),
            ("Hybrid", RenderMode::Hybrid),
        ],
        move |mode| transient.with_mut(|view| view.mode = mode),
    );
    let assembly_view = view_choices(
        if inline {
            "Mechanical assembly view".to_owned()
        } else if context == ViewerCanvasContext::Parts {
            "Parts sample view".to_owned()
        } else {
            format!("{} assembly view", context.name())
        },
        inline,
        transient().assembly,
        [
            ("Assembled", AssemblyView::Assembled),
            ("Exploded", AssemblyView::Exploded),
            ("Section", AssemblyView::Section),
        ],
        move |assembly| transient.with_mut(|view| view.assembly = assembly),
    );
    let inline_hidden_toggle = rsx! {
        ToggleGroup {
            aria_label: "Hidden lines",
            pressed: Some(if transient().show_hidden { [0].into_iter().collect() } else { Default::default() }),
            on_pressed_change: move |next: std::collections::HashSet<usize>| {
                if next.contains(&0) != transient().show_hidden {
                    transient.with_mut(|view| view.show_hidden = !view.show_hidden);
                }
            },
            Tooltip {
                TooltipTrigger { as: move |attributes: Vec<Attribute>| rsx! {
                        ToggleItem { index: 0usize, aria_label: "Show hidden lines",
                            class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [@media(pointer:coarse)]:min-h-11 [@media(pointer:coarse)]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]",
                            attributes,
                            svg { view_box: "0 0 24 24", "aria-hidden": "true",
                                path { d: "m12 2 9 5v10l-9 5-9-5V7ZM3 7l9 5 9-5M12 12v10" }
                                path { stroke_dasharray: "2 2", d: "M12 2v10M3 17l9-5 9 5" }
                            }
                        }
                    } }
                TooltipContent { "Show hidden lines" }
            }
        }
    };
    let show_hidden = rsx! {
        ToggleField {
            label: "Show hidden lines",
            checked: transient().show_hidden,
            on_commit: move |checked| transient.with_mut(|view| view.show_hidden = checked),
        }
    };
    let assembly_adjustments = assembly_adjustments(transient, inline);
    let docked_layers = crate::layer_dock::use_docked_layers(rsx! {
        CaseAssemblyLayers {
            assembly: overlay.assembly_layers,
            components: overlay.component_layers,
            selectable_layers: overlay.selectable_layers,
            selected_layer: overlay.selected_layer.clone(),
            on_select_layer: select_layer,
            display: overlay.display.clone(),
            on_display_change: change_display,
        }
        if !inline {
            {layer_controls(&overlay.layers, &overlay.selected_layer, &overlay.display, select_layer, change_display)}
        }
    });

    rsx! {
        div { class: "relative flex min-h-0 flex-1 flex-col gap-1.5 overflow-hidden",
            style: (!inline).then_some("container-type: size"),
            div { class: if inline { "relative z-[5] m-3 flex w-max max-w-[calc(100%-24px)] flex-none flex-wrap items-center self-start gap-1 overflow-visible rounded-2xl bg-toolbar p-[5px] shadow-[0_6px_18px_rgb(0_0_0/16%)] [&_[role=group]]:m-0 [&_[role=group]]:flex-none [&_[role=group]]:gap-0.5 [&_[role=group]]:p-0" } else { "relative flex flex-none flex-wrap items-center justify-center gap-1 rounded-md bg-toolbar p-1" },
                {overlay.toolbar_leading}
                if inline {
                    div { class: "static flex flex-wrap items-center gap-0.5 border-0 bg-transparent p-0", role: "group", "aria-label": "Assembly camera",
                        {camera_button("Fit", "Fit", "Fit model", camera, RendererPageHost::fit)}
                        for (name, label, action) in CAMERA_VIEWS {
                            {camera_button(name, name, label, camera, action)}
                        }
                    }
                    {display_mode.clone()}
                    {inline_hidden_toggle}
                    if show_assembly_controls { {assembly_view.clone()} }
                }
                {case_edit_controls}
                {overlay.toolbar_extra}
                if !inline {
                    {compact_view_controls(context, camera, show_hidden, display_mode, assembly_view, assembly_adjustments.clone())}
                }
            }
            if inline && show_assembly_controls {
                {assembly_adjustments}
            }
            div { class: "relative flex min-h-[160px] min-w-0 flex-1 [&>canvas]:h-full [&>canvas]:min-h-0 [&>canvas]:min-w-0 [&>canvas]:w-full [&>canvas]:flex-1 [&>canvas]:border-0",
                if let Some(reference) = overlay.selected_reference.as_deref() {
                    div { class: "pointer-events-none absolute bottom-14 left-3 z-[2] max-w-[min(70%,320px)] overflow-hidden truncate rounded-sm border border-border bg-card px-[9px] py-[7px] font-mono text-sm leading-[1.45] text-foreground", "aria-hidden": "true", "{reference}" }
                }
                {children}
                if let Some(message) = overlay.gesture_message.as_deref() {
                    p { class: "pointer-events-none absolute top-14 left-3 z-[3] m-0 max-w-[calc(100%-24px)] rounded-sm border border-border bg-card px-2 py-1.5 text-foreground", role: "status", "aria-label": "Case edit guidance", "{message}" }
                }
                if !status().is_empty() && !matches!(status().as_str(), "3D preview updated." | "3D preview ready." | "Starting the 3D renderer…" | "A stale 3D scene update was ignored.") {
                    p { class: "pointer-events-none absolute top-3 left-3 z-[3] m-0 max-w-[min(70%,420px)] rounded-md bg-card px-[7px] py-1 text-sm text-muted-foreground [&[role=alert]]:pointer-events-auto [&[role=alert]]:text-destructive", role: if status().to_ascii_lowercase().contains("unavailable") || status().contains("failed") { "alert" } else { "status" }, "aria-live": "polite", "{status()}" }
                }
            }
            {docked_layers}
        }
    }
}

fn assembly_adjustments(mut transient: Signal<TransientView>, inline: bool) -> Element {
    const SLIDER_CLASS: &str = "min-h-control max-[960px]:min-h-11 [&_[data-slot=slider-track]]:overflow-visible [&_[data-slot=slider-thumb]]:absolute [&_[data-slot=slider-thumb]]:top-1/2 [&_[data-slot=slider-thumb]]:-translate-x-1/2 [&_[data-slot=slider-thumb]]:-translate-y-1/2 [&_[data-slot=slider-thumb]]:scroll-m-1";
    if transient().assembly == AssemblyView::Exploded {
        return rsx! {
            Slider {
                label: "Explode amount", min: 0.0, max: 10.0, step: 0.1, class: SLIDER_CLASS,
                value: Some(f64::from(transient().explode_amount)),
                on_value_change: move |value: f64| transient.with_mut(|state| state.explode_amount = (value as f32).clamp(0.0, 10.0)),
            }
        };
    }
    if transient().assembly != AssemblyView::Section {
        return rsx! {};
    }
    let show_plane = if inline {
        "Show plane"
    } else {
        "Show section plane"
    };
    let section = rsx! {
        SelectField::<&'static str> {
            label: "Section plane",
            value: transient().section_plane,
            options: ["XY", "XZ", "YZ"].map(|plane| (plane, plane.to_owned())).into(),
            on_commit: move |plane| transient.with_mut(|state| state.section_plane = plane),
        }
        Slider {
            label: "Section position", min: -100.0, max: 100.0, step: 1.0, class: SLIDER_CLASS,
            value: Some(f64::from(transient().section_position)),
            on_value_change: move |value: f64| transient.with_mut(|state| state.section_position = (value as f32).clamp(-100.0, 100.0)),
        }
        ToggleField {
            label: show_plane,
            checked: transient().show_section_plane,
            on_commit: move |checked| transient.with_mut(|state| state.show_section_plane = checked),
        }
    };
    if inline {
        rsx! { div { class: "flex flex-wrap items-center gap-3 border-b border-border px-2.5 py-1.5", {section} } }
    } else {
        section
    }
}

fn layer_controls(
    layers: &[(String, String)],
    selected_layer: &str,
    display: &CaseDisplay,
    select_layer: EventHandler<String>,
    change_display: EventHandler<CaseDisplay>,
) -> Element {
    rsx! {
        Collapsible { keep_mounted: true,
            CollapsibleTrigger { "Case layers and colors" }
            CollapsibleContent { class: "data-[open=false]:hidden max-[960px]:[&_[data-slot=button]]:min-h-11 max-[960px]:[&_[data-slot=toggle-group-item]]:min-h-11",
                div { role: "group", "aria-label": "Case layers",
                    for (id, label) in layers {
                        {
                            let visible = crate::case_display::is_visible(display, id);
                            let color = display.color(id).unwrap_or("#b0b5bd").to_owned();
                            let has_color = display.has_color(id);
                            let toggle_next = display.clone();
                            let color_next = display.clone();
                            let select_id = id.clone();
                            let aliases = crate::case_display::preference_ids(id);
                            let color_id = id.clone();
                            let set_color = move |color: &str| {
                                let mut next = color_next.clone();
                                next.set_color(&color_id, color);
                                change_display.call(next);
                            };
                            let reset_color = set_color.clone();
                            rsx! {
                                div { class: "flex flex-wrap items-center gap-1.5 border-b border-border py-2 [&_label]:flex [&_label]:gap-1.5 [&_label]:text-muted-foreground [&_input[type=color]]:w-11 [&_input[type=color]]:p-[3px]", key: "{id}",
                                    UiButton { variant: UiButtonVariant::Ghost,
                                        aria_pressed: selected_layer == id.as_str(),
                                        onclick: move |_| select_layer.call(select_id.clone()),
                                        "Select {label}"
                                    }
                                    ToggleGroup {
                                        aria_label: format!("Visibility for {label}"), horizontal: true,
                                        pressed: Some(if visible { [0].into_iter().collect() } else { Default::default() }),
                                        on_pressed_change: move |_| {
                                            let mut next_display = toggle_next.clone();
                                            if !visible {
                                                next_display.hidden.retain(|entry| !aliases.contains(entry));
                                            } else {
                                                next_display.hidden.extend(aliases.iter().filter(|id| !toggle_next.hidden.contains(id)).cloned());
                                            }
                                            change_display.call(next_display);
                                        },
                                        ToggleItem { index: 0usize, aria_label: format!("Toggle visibility for {label}"),
                                            if visible { "Visible" } else { "Hidden" }
                                        }
                                    }
                                    Field {
                                        Label { html_for: format!("viewer-color-{}-{id}", dioxus::core::current_scope_id().0), "Color for {label}" }
                                        InputGroup {
                                            InputGroupInput {
                                                id: format!("viewer-color-{}-{id}", dioxus::core::current_scope_id().0),
                                                aria_label: format!("Color for {label}"),
                                                r#type: InputType::Color, value: "{color}",
                                                oninput: move |event: FormEvent| set_color(&event.value()),
                                            }
                                        }
                                    }
                                    UiButton { variant: UiButtonVariant::Secondary,
                                        disabled: !has_color,
                                        onclick: move |_| reset_color(""),
                                        "Reset color"
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

fn compact_view_controls(
    context: ViewerCanvasContext,
    camera: EventHandler<CameraAction>,
    show_hidden: Element,
    display_mode: Element,
    assembly_view: Element,
    adjustments: Element,
) -> Element {
    rsx! {
        {camera_button(context.fit_label(), "Fit", context.fit_label(), camera, RendererPageHost::fit)}
        Collapsible { keep_mounted: true,
            Tooltip {
                TooltipTrigger { as: move |attributes| rsx! {
                        CollapsibleTrigger { aria_label: "View controls", class: "size-control-compact justify-center max-[960px]:min-h-11 max-[960px]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]", attributes, {viewer_control_icon("View controls")} }
                    } }
                TooltipContent { "View controls" }
            }
            CollapsibleContent { class: "absolute left-0 top-full z-50 mt-2 max-h-[min(60vh,max(0px,calc(100cqh-5rem)))] w-[min(320px,calc(100vw-32px))] overflow-auto rounded-md bg-card p-3 shadow-md data-[open=false]:hidden max-[960px]:[&_[data-slot=button]]:min-h-11 max-[960px]:[&_[data-slot=toggle-group-item]]:min-h-11 max-[960px]:[&_[data-slot=select-trigger]]:min-h-11",
                div { role: "group", "aria-label": format!("{} camera", context.name()),
                    for (_, label, action) in CAMERA_VIEWS {
                        UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| camera.call(action), "{label}" }
                    }
                    for (label, action) in CAMERA_NAVIGATION {
                        UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| camera.call(action), "{label}" }
                    }
                }
                {display_mode}
                {assembly_view}
                {adjustments}
                {show_hidden}
            }
        }
    }
}

pub(crate) type CameraAction = fn(&RendererPageHost) -> Result<(), String>;

pub(crate) const CAMERA_VIEWS: [(&str, &str, CameraAction); 3] = [
    ("Top", "Top view", |host| host.view("top")),
    ("Bottom", "Bottom view", |host| host.view("bottom")),
    ("Isometric", "Isometric view", |host| host.view("isometric")),
];

const CAMERA_NAVIGATION: [(&str, CameraAction); 4] = [
    ("Rotate left", |host| host.orbit(-50.0, 0.0)),
    ("Rotate right", |host| host.orbit(50.0, 0.0)),
    ("Zoom in", |host| host.zoom(0.85)),
    ("Zoom out", |host| host.zoom(1.15)),
];

const ICON_CONTROL: &str = "max-[960px]:min-h-11 max-[960px]:min-w-11 [@media(pointer:coarse)]:min-h-11 [@media(pointer:coarse)]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]";

pub(crate) fn camera_button(
    name: &'static str,
    icon: &'static str,
    tooltip: &'static str,
    camera: EventHandler<CameraAction>,
    action: CameraAction,
) -> Element {
    rsx! {
        Tooltip {
            TooltipTrigger { as: move |attributes| rsx! {
                UiButton { attributes, variant: UiButtonVariant::Secondary, size: UiButtonSize::IconSm,
                    class: ICON_CONTROL, aria_label: name, onclick: move |_| camera.call(action),
                    {viewer_control_icon(icon)}
                }
            } }
            TooltipContent { "{tooltip}" }
        }
    }
}

fn view_choices<T: Copy + PartialEq + 'static>(
    label: String,
    inline: bool,
    selected: T,
    choices: [(&'static str, T); 3],
    on_change: impl FnMut(T) + 'static,
) -> Element {
    let mut on_change = on_change;
    let pressed = choices.iter().position(|(_, choice)| *choice == selected);
    rsx! {
        ToggleGroup {
            aria_label: label, horizontal: true,
            pressed: Some(pressed.into_iter().collect()),
            on_pressed_change: move |next: std::collections::HashSet<usize>| {
                if let Some((_, choice)) = next.iter().next().and_then(|index| choices.get(*index)) {
                    on_change(*choice);
                }
            },
            for (index, (name, _)) in choices.into_iter().enumerate() {
                if inline {
                    {icon_toggle(index, name)}
                } else {
                    ToggleItem { index, "{name.to_ascii_lowercase()}" }
                }
            }
        }
    }
}

pub(crate) fn icon_toggle(index: usize, name: &'static str) -> Element {
    rsx! {
        Tooltip {
            TooltipTrigger { as: move |attributes: Vec<Attribute>| rsx! {
                ToggleItem { index, aria_label: name, class: ICON_CONTROL, attributes,
                    {viewer_control_icon(name)}
                }
            } }
            TooltipContent { "{name}" }
        }
    }
}

fn viewer_control_icon(label: &str) -> Element {
    let path = match label {
        "Fit" => "M7 2H2v5M13 2h5v5M2 13v5h5M18 13v5h-5M6 6h8v8H6z",
        "Top" => "M3 5h14v11H3zM3 5l7-3 7 3M7 8h6",
        "Bottom" => "M3 4h14v11H3zM3 15l7 3 7-3M7 12h6",
        "Shaded" => "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8",
        "Wireframe" => "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8M10 2v8M3 14l7-4 7 4",
        "Hybrid" => "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8M13 8v8",
        "Assembled" => "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8M3 10l7 4 7-4",
        "Exploded" => "m3 5 7-3 7 3-7 3ZM3 10l7 3 7-3M3 15l7 3 7-3",
        "Section" => "M3 3h14v14H3zM10 1v18M12 5l3 3M12 10l3 3",
        "Edit gaskets" => "M5 3h10l3 4v6l-3 4H5l-3-4V7ZM6 7h8v6H6z",
        "Edit mounts" => {
            "M10 2v4M10 14v4M2 10h4M14 10h4M15 10a5 5 0 1 1-10 0 5 5 0 0 1 10 0ZM12 10a2 2 0 1 1-4 0 2 2 0 0 1 4 0Z"
        }
        "View controls" => "M2 5h16M2 10h16M2 15h16M6 3v4M14 8v4M8 13v4",
        _ => "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8",
    };
    rsx! {
        svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: path } }
        span { class: "sr-only", "{label}" }
    }
}
