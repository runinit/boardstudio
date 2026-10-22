//! Ticket 015: the production strips with observable owner callbacks, without a CAD runtime.
#[cfg(target_arch = "wasm32")]
use boardstudio_web_layout::objects::*;
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
fn fixture() -> Element {
    let mut kind = use_signal(|| LayoutSelectionKind::Key);
    let mut tool = use_signal(|| None::<LayoutTransformTool>);
    let mut settings = use_signal(LayoutSnapSettings::default);
    let open_menu = use_signal(|| None::<LayoutCommandMenu>);
    let mut properties = use_signal(|| 0);
    let mut reference = use_signal(|| "0".to_owned());
    let mut reference_count = use_signal(|| 13);
    let mut has_selection = use_signal(|| true);
    let mut canvas_clicks = use_signal(|| 0);
    let mut snap_calls = use_signal(|| 0);
    let transform = LayoutTransformMenuMount {
        properties_available: has_selection(),
        column_available: has_selection(),
        row_available: has_selection(),
        pointer_tools_visible: true,
        pointer_tools_available: has_selection(),
        active_tool: tool(),
        on_selection_kind: EventHandler::new(move |next| kind.set(next)),
        on_show_properties: EventHandler::new(move |()| properties += 1),
        on_transform_tool: EventHandler::new(move |next| {
            tool.set(if tool() == Some(next) {
                None
            } else {
                Some(next)
            });
            if next != LayoutTransformTool::Offset || kind() != LayoutSelectionKind::Row {
                kind.set(LayoutSelectionKind::Column);
            }
        }),
    };
    rsx! {
        boardstudio_web_ui_shared::ui::toast::ToastProvider {
        section { class: "toolbar-fixture", role: "region", aria_label: "Layout canvas",
            svg { role: "img", "aria-label": "Layout drawing", style: "width:100%;height:620px;display:block", onclick: move |_| canvas_clicks += 1,
                rect { width: "100%", height: "100%", fill: "var(--canvas)" }
            }
            div { class: "m1-layout-toolbar-region",
            div { class: "m1-canvas-toolbar",
            LayoutCommandPill {
                command_label: "Layout commands", menu_owner_key: "fixture", open_menu,
                selection_kind: kind(), snap_settings: settings(), transform,
                align: LayoutAlignMount {
                    references: (0..reference_count()).map(|i| AlignReference { id: i.to_string(), label: format!("Reference {i}") }).collect(),
                    selected_reference: Some(reference()), action: None, enabled: false,
                    disabled_reason: Some("Select a part to align.".into()), busy: false, feedback: None,
                    on_reference: EventHandler::new(move |next| reference.set(next)), on_align: EventHandler::new(|_| {}),
                },
                show_relationships: false, has_selection_context: has_selection(),
                on_selection_kind: move |next| kind.set(next),
                on_snap_intent: move |intent| { snap_calls += 1; settings.with_mut(|settings| match intent {
                    LayoutSnapIntent::Fraction(value) => settings.snap_fraction = value,
                    LayoutSnapIntent::GeometrySnap(value) => settings.geometry_snap = value,
                    LayoutSnapIntent::GapSnap(value) => settings.gap_snap = value,
                    LayoutSnapIntent::GapOverride(value) => settings.gap_override = value,
                }); },
                on_show_relationships: move |_| {},
            }
            }
            }
            boardstudio_web_ui_shared::ui::button::Button { onclick: move |_| reference_count.set(3), "Few references" }
            boardstudio_web_ui_shared::ui::button::Button { onclick: move |_| has_selection.set(false), "Clear selection" }
            boardstudio_web_ui_shared::ui::button::Button { onclick: move |_| has_selection.set(true), "Restore selection" }
            output { role: "status", aria_label: "Canvas clicks", "{canvas_clicks}" }
            output { role: "status", aria_label: "Snap calls", "{snap_calls}" }
            output { role: "status", aria_label: "Reference state", "{reference}" }
            output { role: "status", aria_label: "Owner state", "{kind():?}:{tool():?}:{properties}:{settings().snap_fraction}:{settings().geometry_snap}:{settings().gap_override}" }
        }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus_web::launch::launch_virtual_dom(VirtualDom::new(fixture), dioxus_web::Config::new());
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    panic!("the toolbar fixture requires wasm32-unknown-unknown");
}
