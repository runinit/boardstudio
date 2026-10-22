//! PROTOTYPE (wayfinder ticket 013), throwaway. Question: do the Original and shadcn Themes, light and dark,
//! look right on copied rust-ui components inside a mini workbench? Variants (switch with the bottom bar,
//! `?variant=` or the arrow keys):
//!   A Original light, B Original dark, C shadcn light, D shadcn dark.
//! Round 2: cards style (no outlines; surfaces separated by background and elevation). Read-only.
#![allow(non_snake_case)]
mod hooks;
mod ui;

use dioxus::prelude::*;
use icons::{Keyboard, ChevronDown, Search, Undo2, Redo2, Sun, Plus, Component, MousePointer2, Magnet, ZoomOut, ZoomIn, Maximize, TriangleAlert, Info};
use ui::alert::*;
use ui::badge::*;
use ui::button::*;
use ui::card::*;
use ui::checkbox::Checkbox;
use ui::field::*;
use ui::input_group::*;
use ui::kbd::Kbd;
use ui::select::*;
use ui::separator::*;
use ui::sidenav::*;
use ui::toggle_group::*;

const VARIANTS: [(&str, &str, &str, &str); 4] = [
    ("A", "Original light", "original", "light"),
    ("B", "Original dark", "original", "dark"),
    ("C", "shadcn light", "shadcn", "light"),
    ("D", "shadcn dark", "shadcn", "dark"),
];

fn initial_variant() -> usize {
    let search = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
    let key = search.split(['?', '&']).find_map(|p| p.strip_prefix("variant=")).unwrap_or("A");
    VARIANTS.iter().position(|v| v.0 == key).unwrap_or(0)
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut variant = use_signal(initial_variant);
    use_effect(move || {
        let (key, _, theme, appearance) = VARIANTS[variant()];
        if let Some(win) = web_sys::window() {
            if let Some(root) = win.document().and_then(|d| d.document_element()) {
                let _ = root.set_attribute("data-theme", theme);
                let _ = root.set_attribute("data-appearance", appearance);
            }
            if let Ok(h) = win.history() {
                let _ = h.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&format!("?variant={key}")));
            }
        }
    });
    use_hook(|| {
        use wasm_bindgen::{JsCast, closure::Closure};
        let cb = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            let typing = e
                .target()
                .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
                .is_some_and(|t| matches!(t.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT"));
            if typing {
                return;
            }
            match e.key().as_str() {
                "ArrowRight" => variant.set((variant() + 1) % 4),
                "ArrowLeft" => variant.set((variant() + 3) % 4),
                _ => {}
            }
        });
        if let Some(d) = web_sys::window().and_then(|w| w.document()) {
            let _ = d.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        }
        cb.forget();
    });
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/fonts.css") }
        document::Link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
        Workbench {}
        Switcher { variant }
    }
}

#[component]
fn Switcher(mut variant: Signal<usize>) -> Element {
    let (key, name, _, _) = VARIANTS[variant()];
    let btn = "size-8 rounded-full text-lg leading-none hover:bg-white/20";
    rsx! {
        div {
            style: "position:fixed;left:50%;bottom:14px;transform:translateX(-50%);z-index:100;background:#111;color:#fff;border-radius:999px;padding:6px 10px;display:flex;gap:10px;align-items:center;box-shadow:0 4px 18px #0006;font:600 13px system-ui",
            button { class: btn, aria_label: "Previous variant", onclick: move |_| variant.set((variant() + 3) % 4), "←" }
            span { "{key} ({name})" }
            button { class: btn, aria_label: "Next variant", onclick: move |_| variant.set((variant() + 1) % 4), "→" }
        }
    }
}

#[component]
fn Workbench() -> Element {
    rsx! {
        // "desk" (canvas colour) with floating cards; no outlines anywhere, elevation and fill do the separating
        div { class: "flex h-screen flex-col gap-3 bg-canvas p-3 text-foreground font-sans",
            Header {}
            div { class: "flex min-h-0 flex-1 gap-3",
                ObjectsDock {}
                Center {}
                InspectorDock {}
            }
        }
    }
}

const CARD: &str = "overflow-hidden rounded-xl bg-card shadow-sm";

#[component]
fn Header() -> Element {
    let links = [("Layout", true), ("PCB", false), ("Keymap", false), ("Keycaps", false), ("Case", false), ("Parts", false)];
    rsx! {
        header { class: "flex h-12 shrink-0 items-center gap-2 rounded-xl bg-card px-3 shadow-sm",
            Keyboard { class: "size-6 text-primary" }
            Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, class: "gap-1 font-semibold",
                "Split 36 keys" ChevronDown { class: "size-4 text-muted-foreground" }
            }
            nav { class: "ml-2 hidden items-center gap-0.5 md:flex", aria_label: "Workspaces",
                for (name, active) in links {
                    Button {
                        key: "{name}",
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        class: if active { "bg-selected text-selected-foreground" } else { "text-muted-foreground" },
                        "{name}"
                    }
                }
            }
            div { class: "ml-auto flex items-center gap-2",
                Button { variant: ButtonVariant::Secondary, size: ButtonSize::Sm, class: "w-56 justify-between bg-field text-muted-foreground shadow-none",
                    span { class: "flex items-center gap-2", Search { class: "size-4" } "Search commands" }
                    Kbd { "Ctrl K" }
                }
                span { class: "hidden text-xs text-muted-foreground lg:inline", "Saved locally" }
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Icon, aria_label: "Undo", Undo2 { class: "size-4" } }
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Icon, aria_label: "Redo", Redo2 { class: "size-4" } }
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Icon, aria_label: "Theme", Sun { class: "size-4" } }
                Button { size: ButtonSize::Sm, "Export" }
            }
        }
    }
}

#[component]
fn ObjectsDock() -> Element {
    let rows = [("Board", 0, false), ("Left half", 1, false), ("Matrix 1 (3 x 6)", 2, true), ("Row 1", 3, false), ("Row 2", 3, false), ("Row 3", 3, false), ("Right half", 1, false), ("Controller", 1, false)];
    rsx! {
        Sidenav { data_collapsible: SidenavCollapsible::None, class: "w-64 shrink-0 {CARD}",
            SidenavHeader { class: "flex-row items-center justify-between p-3",
                span { class: "text-sm font-semibold", "Objects" }
                Button { variant: ButtonVariant::Secondary, size: ButtonSize::Sm, class: "gap-1 shadow-none", Plus { class: "size-4" } "Add" }
            }
            SidenavContent {
                SidenavGroup {
                    SidenavGroupLabel { "Layout" }
                    SidenavGroupContent {
                        SidenavMenu {
                            for (name, depth, selected) in rows {
                                SidenavMenuItem { key: "{name}",
                                    SidenavMenuButton {
                                        class: if selected { "bg-selected text-selected-foreground font-medium" } else { "" },
                                        span { style: "width:{depth * 12}px" }
                                        Component { class: "size-4 shrink-0" }
                                        span { class: "truncate", "{name}" }
                                        if selected { Badge { variant: BadgeVariant::Secondary, class: "ml-auto", "18" } }
                                    }
                                }
                            }
                        }
                    }
                }
                SidenavGroup {
                    SidenavGroupLabel { "Canvas visibility" }
                    SidenavGroupContent { class: "grid gap-2 px-2 pb-2",
                        for (name, on) in [("Keys", true), ("Outline", true), ("Mounting holes", false)] {
                            label { key: "{name}", class: "flex items-center gap-2 text-sm",
                                Checkbox { checked: on, aria_label: name }
                                "{name}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Center() -> Element {
    rsx! {
        main { class: "flex min-w-0 flex-1 flex-col {CARD}",
            // strip: tools sit in soft filled groups, not outlined buttons
            div { class: "flex items-center gap-2 p-3",
                ToggleGroup { class: "rounded-lg bg-toolbar p-1",
                    for (name, on) in [("Matrix", false), ("Row", false), ("Column", false), ("Key", true), ("Part", false)] {
                        ToggleGroupItem { key: "{name}", title: name, pressed: on, class: "gap-1.5 border-transparent px-3 data-[state=on]:bg-card data-[state=on]:shadow-sm",
                            MousePointer2 { class: "size-4" } "{name}"
                        }
                    }
                }
                div { class: "flex items-center gap-0.5 rounded-lg bg-toolbar p-1",
                    for name in ["Position", "Stagger", "Offset", "Splay"] {
                        Button { key: "{name}", variant: ButtonVariant::Ghost, size: ButtonSize::Sm, "{name}" }
                    }
                }
                Button { variant: ButtonVariant::Secondary, size: ButtonSize::Sm, class: "gap-1 shadow-none", "Align" ChevronDown { class: "size-4" } }
                Button { variant: ButtonVariant::Secondary, size: ButtonSize::Sm, class: "gap-1 shadow-none", "Transform" ChevronDown { class: "size-4" } }
                Button { size: ButtonSize::Sm, class: "ml-auto gap-1.5", Magnet { class: "size-4" } "Snap" }
            }
            Canvas {}
            div { class: "flex h-12 shrink-0 items-center gap-3 px-3 text-xs text-muted-foreground",
                span { "x 142.3 mm  y 61.0 mm" }
                Badge { variant: BadgeVariant::Warning, "2 layout findings" }
                Badge { variant: BadgeVariant::Success, "Outline valid" }
                div { class: "ml-auto flex items-center gap-1",
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::IconSm, aria_label: "Zoom out", ZoomOut { class: "size-4" } }
                    span { class: "w-10 text-center tabular-nums", "100%" }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::IconSm, aria_label: "Zoom in", ZoomIn { class: "size-4" } }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::IconSm, aria_label: "Fit", Maximize { class: "size-4" } }
                }
            }
        }
    }
}

#[component]
fn Canvas() -> Element {
    let keys: Vec<(f32, f32, bool)> = (0..3)
        .flat_map(|r| (0..6).map(move |c| (60.0 + c as f32 * 58.0 + r as f32 * 6.0, 80.0 + r as f32 * 58.0, r == 1 && c == 3)))
        .collect();
    rsx! {
        // the drawing surface is an inset rounded panel inside the card
        div { class: "relative mx-3 min-h-0 flex-1 overflow-hidden rounded-lg bg-canvas",
            svg { class: "absolute inset-0 h-full w-full", view_box: "0 0 640 360", preserve_aspect_ratio: "xMidYMid slice",
                defs {
                    pattern { id: "g", width: "29", height: "29", pattern_units: "userSpaceOnUse",
                        path { d: "M29 0H0V29", fill: "none", stroke: "var(--grid-small)", stroke_width: "1" }
                    }
                }
                rect { width: "640", height: "360", fill: "url(#g)" }
                rect { x: "40", y: "56", width: "390", height: "196", rx: "14", fill: "var(--geometry-fill)", stroke: "var(--geometry-board)", stroke_width: "2" }
                for (i, (x, y, sel)) in keys.into_iter().enumerate() {
                    g { key: "{i}",
                        rect {
                            x: "{x}", y: "{y}", width: "52", height: "52", rx: "6",
                            fill: if sel { "var(--selected)" } else { "var(--geometry-body)" },
                            stroke: if sel { "var(--primary)" } else { "var(--geometry-key)" },
                            stroke_width: if sel { "3" } else { "1.5" },
                        }
                        text { x: "{x + 26.0}", y: "{y + 30.0}", text_anchor: "middle", font_size: "13", fill: "var(--geometry-label)", "{i + 1}" }
                    }
                }
                rect { x: "470", y: "96", width: "120", height: "70", rx: "8", fill: "var(--geometry-body)", stroke: "var(--geometry-part)", stroke_width: "2" }
                text { x: "530", y: "136", text_anchor: "middle", font_size: "13", fill: "var(--geometry-label)", "Controller" }
            }
            div { class: "absolute left-3 top-3 rounded-md bg-card/90 px-2 py-1 text-xs text-muted-foreground shadow-sm", "Key 11 selected: Shift moves in 1 mm steps" }
        }
    }
}

#[component]
fn InspectorDock() -> Element {
    let soft = "rounded-lg border-transparent bg-muted shadow-none";
    rsx! {
        Sidenav { data_collapsible: SidenavCollapsible::None, data_side: SidenavSide::Right, class: "w-80 shrink-0 {CARD}",
            SidenavHeader { class: "p-3", span { class: "text-sm font-semibold", "Inspector" } }
            SidenavContent { class: "gap-3 px-3 pb-3",
                Card { size: CardSize::Sm, class: "{soft}",
                    CardHeader { CardTitle { "Key 11" } CardDescription { "Matrix 1, row 2, column 4" } }
                    CardContent { class: "grid gap-3",
                        Field {
                            FieldLabel { html_for: "x", "Position X" }
                            InputGroup { class: "border-transparent bg-card",
                                InputGroupInput { id: "x", r#type: ui::input::InputType::Number, value: "76.2" }
                                InputGroupAddon { align: InputGroupAddonAlign::InlineEnd, InputGroupText { "mm" } }
                            }
                        }
                        Field { invalid: true,
                            FieldLabel { html_for: "rot", "Rotation" }
                            InputGroup { class: "border-transparent bg-card",
                                InputGroupInput { id: "rot", r#type: ui::input::InputType::Number, value: "720" }
                                InputGroupAddon { align: InputGroupAddonAlign::InlineEnd, InputGroupText { "deg" } }
                            }
                            FieldError { "Enter an angle between -180 and 180." }
                        }
                        Field {
                            FieldLabel { "Switch family" }
                            Select { default_value: "choc",
                                SelectTrigger { class: "border-transparent bg-card shadow-none", SelectValue { placeholder: "Choose" } }
                                SelectContent { SelectGroup { SelectOption { value: "mx", "MX" } SelectOption { value: "choc", "Choc v1" } } }
                            }
                        }
                        label { class: "flex items-center gap-2 text-sm", Checkbox { checked: true, aria_label: "Hot-swap" } "Hot-swap sockets" }
                    }
                }
                Alert { variant: AlertVariant::Destructive, class: "rounded-lg border-transparent dark:border-transparent bg-destructive-light",
                    TriangleAlert { class: "size-4" }
                    AlertTitle { "Key overlaps its neighbour" }
                    AlertDescription { "Move key 11 right by 1.2 mm to clear it." }
                }
                Alert { class: "rounded-lg border-transparent dark:border-transparent bg-info-light text-info-dark",
                    Info { class: "size-4" }
                    AlertTitle { "Stagger applies to row 2" }
                    AlertDescription { "Undo restores the previous offsets." }
                }
                div { class: "flex flex-wrap gap-2",
                    Button { size: ButtonSize::Sm, "Apply" }
                    Button { variant: ButtonVariant::Secondary, size: ButtonSize::Sm, class: "shadow-none", "Reset" }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, "Details" }
                    Button { variant: ButtonVariant::Destructive, size: ButtonSize::Sm, "Delete matrix" }
                    Button { size: ButtonSize::Sm, disabled: true, "Disabled" }
                }
                div { class: "flex flex-wrap gap-2",
                    Badge { "Default" } Badge { variant: BadgeVariant::Secondary, "Secondary" } Badge { variant: BadgeVariant::Muted, "Muted" }
                    Badge { variant: BadgeVariant::Success, "Success" } Badge { variant: BadgeVariant::Warning, "Warning" }
                    Badge { variant: BadgeVariant::Info, "Info" } Badge { variant: BadgeVariant::Destructive, "Error" }
                }
            }
        }
    }
}
