use crate::MechanicalSettingsProps;
use crate::mechanical_settings::{self, MechanicalSettingsPatchExt};
use crate::shared_viewer::{
    AssemblyView, CaseDisplay, TransientView, ViewerHandle, ViewerHandleTarget,
    canvas::{CAMERA_VIEWS, CameraAction, camera_button, icon_toggle},
};
use boardstudio_web_host::renderer_host::RendererPageHost;
use boardstudio_web_ui_shared::ui::{
    button::{Button as UiButton, ButtonVariant as UiButtonVariant},
    checkbox::{Checkbox as UiCheckbox, CheckboxState},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    field::Field,
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
    label::Label,
    select::{Select, SelectOption},
    slider::Slider,
    toggle_group::{ToggleGroup, ToggleItem},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;

pub(crate) fn inline_view_controls(
    camera: EventHandler<CameraAction>,
    mut transient: Signal<TransientView>,
    display_mode: Element,
    assembly_view: Element,
    show_assembly_controls: bool,
) -> Element {
    rsx! {
        div { class: "static flex flex-wrap items-center gap-0.5 border-0 bg-transparent p-0", role: "group", "aria-label": "Assembly camera",
            {camera_button("Fit", "Fit", "Fit model", camera, RendererPageHost::fit)}
            for (name, label, action) in CAMERA_VIEWS {
                {camera_button(name, name, label, camera, action)}
            }
        }
        {display_mode}
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
        if show_assembly_controls { {assembly_view} }
    }
}

pub(crate) fn edit_controls(
    selected_layer: &str,
    mechanical_settings: Option<&MechanicalSettingsProps>,
    handles: &[ViewerHandle],
    mut edit_mode: Signal<Option<usize>>,
    camera: EventHandler<CameraAction>,
    accepts_edit: impl Fn(&boardstudio_application::Scope) -> bool + 'static,
) -> Element {
    let selected_support = mechanical_settings.and_then(|settings| {
        settings.gasket_supports.iter().find(|support| {
            !support.is_previous
                && (selected_layer == format!("gasket:{}:lower", support.id)
                    || selected_layer == format!("gasket:{}:upper", support.id))
        })
    });
    let can_edit_gaskets = handles
        .iter()
        .any(|handle| matches!(handle.target, ViewerHandleTarget::Gasket { .. }));
    let can_edit_mounts = handles
        .iter()
        .any(|handle| !matches!(handle.target, ViewerHandleTarget::Gasket { .. }));
    let unlink = selected_support
        .zip(mechanical_settings)
        .map(|(support, settings)| {
            let settings = settings.clone();
            let anchors = settings
                .gasket_supports
                .iter()
                .filter(|candidate| {
                    candidate.id == support.id
                        || candidate.pair_id.as_deref() == Some(support.id.as_str())
                })
                .map(mechanical_settings::MechanicalGasketSupportRow::saved_anchor)
                .collect();
            let patch = mechanical_settings::MechanicalSettingsPatch::SetGasketSupportUnlinked {
                support_id: support.id.clone(),
                pair_id: support.pair_id.clone(),
                anchors,
            };
            let field_id = patch.field_id();
            let feedback = settings
                .feedback
                .iter()
                .rev()
                .find(|feedback| feedback.field_id == field_id)
                .filter(|feedback| {
                    feedback.state == mechanical_settings::MechanicalSettingsFeedbackState::Failed
                })
                .map(|feedback| {
                    feedback
                        .message
                        .clone()
                        .unwrap_or_else(|| "The support could not be unlinked.".into())
                });
            let unlinked = support.unlinked;
            let disabled = !settings.editable || unlinked;
            let on_click = move |_| {
                if accepts_edit(&settings.identity.scope) {
                    super::submit_settings_patch(&settings, patch.clone());
                }
            };
            rsx! {
                UiButton { variant: UiButtonVariant::Secondary, button_type: "button",
                    class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11",
                    disabled, onclick: on_click, "Unlink selected support"
                }
                if let Some(message) = feedback { p { role: "alert", "{message}" } }
                if unlinked { p { "This gasket is unlinked from its pair." } }
            }
        });
    rsx! {
        if can_edit_gaskets || can_edit_mounts {
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
                if can_edit_gaskets { {icon_toggle(0, "Edit gaskets")} }
                if can_edit_mounts { {icon_toggle(1, "Edit mounts")} }
            }
        }
        if edit_mode() == Some(0) {
            if let Some(unlink) = unlink { {unlink} } else {
                p { "Select a gasket in Objects to unlink its mirrored support pair." }
            }
        }
    }
}

pub(crate) fn use_assembly_adjustments(
    mut transient: Signal<TransientView>,
    inline: bool,
) -> Element {
    const SLIDER_CLASS: &str = "min-h-control max-[960px]:min-h-11 [&_[data-slot=slider-track]]:overflow-visible [&_[data-slot=slider-thumb]]:absolute [&_[data-slot=slider-thumb]]:top-1/2 [&_[data-slot=slider-thumb]]:-translate-x-1/2 [&_[data-slot=slider-thumb]]:-translate-y-1/2 [&_[data-slot=slider-thumb]]:scroll-m-1";
    let section_plane = use_memo(move || Some(transient().section_plane.to_owned()));
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
        Field {
            Label { html_for: format!("viewer-section-plane-{}", dioxus::core::current_scope_id().0), "Section plane" }
            Select::<String> {
                id: format!("viewer-section-plane-{}", dioxus::core::current_scope_id().0), aria_label: "Section plane",
                value: Some(section_plane.into()),
                on_value_change: move |value: Option<String>| {
                    if let Some(value) = value {
                        transient.with_mut(|state| state.section_plane = match value.as_str() { "XY" => "XY", "XZ" => "XZ", _ => "YZ" });
                    }
                },
                for (index, plane) in ["XY", "XZ", "YZ"].into_iter().enumerate() {
                    SelectOption::<String> { index, value: plane.to_owned(), text_value: plane, "{plane}" }
                }
            }
        }
        Slider {
            label: "Section position", min: -100.0, max: 100.0, step: 1.0, class: SLIDER_CLASS,
            value: Some(f64::from(transient().section_position)),
            on_value_change: move |value: f64| transient.with_mut(|state| state.section_position = (value as f32).clamp(-100.0, 100.0)),
        }
        Label { html_for: format!("viewer-show-plane-{}", dioxus::core::current_scope_id().0), class: "max-[960px]:min-h-11",
            UiCheckbox { id: format!("viewer-show-plane-{}", dioxus::core::current_scope_id().0), aria_label: show_plane,
                checked: Some(if transient().show_section_plane { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                on_checked_change: move |value| transient.with_mut(|state| state.show_section_plane = bool::from(value)),
            }
            "{show_plane}"
        }
    };
    if inline {
        rsx! { div { class: "flex flex-wrap items-center gap-3 border-b border-border px-2.5 py-1.5", {section} } }
    } else {
        section
    }
}

pub(crate) fn layer_controls(
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
