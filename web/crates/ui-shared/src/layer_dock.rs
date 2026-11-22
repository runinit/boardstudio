//! Render viewer-owned layer controls in the dock without moving DOM or owning visibility.
use dioxus::prelude::*;

#[derive(Clone, Copy)]
struct LayerDock(Signal<Option<(ScopeId, Element)>>);

pub fn use_layer_dock_provider() {
    let content = use_signal(|| None);
    use_context_provider(|| LayerDock(content));
}

/// Standalone viewers retain their controls when there is no workbench outlet.
pub fn use_docked_layers(content: Element) -> Element {
    let dock = try_consume_context::<LayerDock>();
    let owner = dioxus::dioxus_core::current_scope_id();
    use_effect(use_reactive((&content,), move |(content,)| {
        if let Some(LayerDock(mut slot)) = dock {
            slot.set(Some((owner, content)));
        }
    }));
    use_drop(move || {
        if let Some(LayerDock(mut slot)) = dock {
            let owned = slot
                .peek()
                .as_ref()
                .is_some_and(|(scope, _)| *scope == owner);
            if owned {
                slot.set(None);
            }
        }
    });
    if dock.is_some() {
        rsx! {}
    } else {
        content
    }
}

#[component]
pub fn LayerDockOutlet() -> Element {
    let LayerDock(slot) = use_context();
    rsx! { div { class: "empty:hidden", {slot.read().as_ref().map(|(_, content)| content.clone())} } }
}
