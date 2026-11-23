use crate::ui::toggle_group::{ToggleGroup, ToggleItem};
use boardstudio_web_ui_model::Workspace;
use dioxus::prelude::*;

/// Reusable canvas header and view switch for the Keymap and Keycaps consumers.
/// The page owner supplies guarded actions and the accepted shared view state.
#[derive(Props, Clone, PartialEq)]
pub struct DesignViewToolbarProps {
    #[props(default)]
    pub contextual_tools: Option<Element>,
    pub workspace: Workspace,
    pub detail: String,
    pub assembly_3d: bool,
    pub footprints_visible: bool,
    pub on_view_mode: EventHandler<bool>,
    pub on_toggle_footprints: EventHandler<()>,
}

#[component]
pub fn DesignViewToolbar(props: DesignViewToolbarProps) -> Element {
    let label = props.workspace.as_str();
    let detail = props.detail;
    let assembly_3d = props.assembly_3d;
    let footprints_visible = props.footprints_visible;
    rsx! {
        div {
            class: "absolute top-4 left-1/2 z-[1] flex min-h-10 w-fit max-w-[calc(100%-32px)] -translate-x-1/2 items-center gap-2 rounded-2xl border border-border-strong bg-card px-2.5 py-1.5",
            role: "group",
            aria_label: "{label} commands",
            if let Some(tools) = props.contextual_tools { {tools} } else {
            div { class: "flex items-center gap-3 p-0 [&_strong]:text-xs [&_strong]:text-foreground [&_span]:p-0 [&_span]:text-xs [&_span]:text-muted-foreground",
                svg {
                    class: "size-[17px] flex-none max-[760px]:w-3.5",
                    view_box: "0 0 20 20",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    "aria-hidden": "true",
                    if props.workspace == Workspace::Keymap { path { d: "M3 5h14v10H3zM6 8h2m2 0h2m2 0h1M6 11h2m2 0h2M6 14h8" } }
                    else { path { d: "m3 15 2-10h10l2 10zM5 5l2 4h6l2-4M7 9l-1 6m7-6 1 6" } }
                }
                strong { "{label}" }
                span { "{detail}" }
            }
            }
        }
        div { class: "absolute bottom-3 left-3 z-[11] flex gap-0.5 rounded-lg border border-border bg-card p-[3px] max-[760px]:left-2",
            ToggleGroup {
                aria_label: "View mode",
                pressed: Some(if assembly_3d { [1].into_iter().collect() } else { [0].into_iter().collect() }),
                on_pressed_change: move |next: std::collections::HashSet<usize>| {
                    if next.contains(&1) {
                        props.on_view_mode.call(true);
                    } else if next.contains(&0) {
                        props.on_view_mode.call(false);
                    }
                },
                ToggleItem { index: 0usize, class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11", "2D" }
                ToggleItem { index: 1usize, class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11", "3D assembly" }
            }
            if !assembly_3d {
                ToggleGroup {
                    aria_label: "Footprints",
                    pressed: Some(if footprints_visible { [0].into_iter().collect() } else { Default::default() }),
                    on_pressed_change: move |next: std::collections::HashSet<usize>| {
                        if next.contains(&0) != footprints_visible {
                            props.on_toggle_footprints.call(());
                        }
                    },
                    ToggleItem { index: 0usize, class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11", "Footprints" }
                }
            }
        }
    }
}
