//! Temporary workbench chrome. Domain content and its edit owners remain caller-owned.
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::icons::ChevronDown;
use boardstudio_web_ui_shared::ui::toolbar::{Toolbar, ToolbarButton};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::HtmlElement;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolPanel {
    Objects,
    Inspector,
}

impl ToolPanel {
    fn label(self) -> &'static str {
        match self {
            Self::Objects => "Objects",
            Self::Inspector => "Inspector",
        }
    }

    fn utility_label(self) -> &'static str {
        match self {
            Self::Objects => "Objects",
            Self::Inspector => "Inspect",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Objects => "workbench-objects",
            Self::Inspector => "workbench-inspector",
        }
    }

    fn icon_path(self) -> &'static str {
        match self {
            Self::Objects => "M3 3h5v5H3zM3 12h5v5H3zM12 3h5v5h-5zM12 12h5v5h-5z",
            Self::Inspector => "M3 3h14v14H3zM11 3v14M14 7v6",
        }
    }
}

fn focus_utility(panel: ToolPanel) {
    if let Some(target) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(&format!("{}-trigger", panel.id())))
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
        && target.is_connected()
        && target.closest("[hidden], [inert]").ok().flatten().is_none()
    {
        let _ = target.focus();
    }
}

fn close(mut panel: Signal<Option<ToolPanel>>, restore_focus: bool) {
    let previous = *panel.peek();
    panel.set(None);
    if restore_focus && let Some(previous) = previous {
        focus_utility(previous);
    }
}

struct PanelEvents {
    window: web_sys::Window,
    key: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    pointer: Closure<dyn FnMut(web_sys::PointerEvent)>,
}

impl Drop for PanelEvents {
    fn drop(&mut self) {
        let _ = self.window.remove_event_listener_with_callback("keydown", self.key.as_ref().unchecked_ref());
        let _ = self.window.remove_event_listener_with_callback("pointerdown", self.pointer.as_ref().unchecked_ref());
    }
}

/// Route each workspace to its sole panel authority while the caller retains
/// the real canvas and content owners. Legacy settings are read only here for
/// the still-unmigrated workspaces.
#[component]
pub(super) fn PanelComposition(
    workspace: &'static str,
    panel: Signal<Option<ToolPanel>>,
    objects: Option<Element>,
    objects_header: Option<Element>,
    objects_footer: Element,
    inspector: Option<Element>,
    legacy_objects_settings: Signal<super::PanelSettings>,
    legacy_inspector_settings: Signal<super::PanelSettings>,
    children: Element,
) -> Element {
    use boardstudio_web_ui_shared::panels::{DockLayout, InspectorPanel, ObjectsPanel};
    let legacy = use_context::<super::CompactPanelState>();
    if workspace == "Layout" {
        match (objects, inspector) {
            (Some(objects), Some(inspector)) => rsx! {
                PanelHost { panel,
                    objects: rsx! {
                        if let Some(header) = objects_header { {header} }
                        div { class: "workbench-panel-scroll", {objects} }
                        {objects_footer}
                    },
                    inspector,
                    {children}
                }
            },
            _ => rsx! { {children} },
        }
    } else {
        rsx! {
            DockLayout {
                objects: objects.map(|content| rsx! {
                    ObjectsPanel { compact_open: legacy.objects_open, settings: legacy_objects_settings,
                        header_action: objects_header,
                        div { class: "min-h-0 min-w-0 flex-[1_1_auto] overflow-x-hidden overflow-y-auto", {content} }
                        {objects_footer}
                    }
                }),
                inspector: inspector.map(|content| rsx! {
                    InspectorPanel { compact_open: legacy.inspector_open, settings: legacy_inspector_settings, {content} }
                }),
                {children}
            }
        }
    }
}

/// Kept at a stable child position, independent of which compact panel is visible.
#[component]
pub(super) fn PanelHost(
    panel: Signal<Option<ToolPanel>>,
    objects: Element,
    inspector: Element,
    children: Element,
) -> Element {
    let mut root = use_signal(|| None::<web_sys::Element>);
    let listeners = use_hook(|| Rc::new(RefCell::new(None::<PanelEvents>)));
    use_effect({
        let listeners = listeners.clone();
        move || {
            let Some(root) = root() else { return; };
            let Some(window) = web_sys::window() else { return; };
            let key_root = root.clone();
            // Window bubble runs after nested controls, canvas picks and document-owned
            // relationship returns; a handled Escape must not dismiss two surfaces.
            let key = Closure::new(move |event: web_sys::KeyboardEvent| {
                if key_root.is_connected() && event.key() == "Escape"
                    && !event.default_prevented() && panel.peek().is_some()
                {
                    event.prevent_default();
                    close(panel, true);
                }
            });
            let pointer = Closure::new(move |event: web_sys::PointerEvent| {
                let Some(kind) = *panel.peek() else { return; };
                if !root.is_connected() { return; }
                let Some(target) = event.target().and_then(|target| target.dyn_into::<web_sys::Element>().ok()) else { return; };
                let selector = format!("#{}, .workbench-utility, [role=menu], [role=listbox], [data-slot=popover-content]", kind.id());
                if target.closest(&selector).ok().flatten().is_none() {
                    close(panel, false);
                }
            });
            let _ = window.add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref());
            let _ = window.add_event_listener_with_callback("pointerdown", pointer.as_ref().unchecked_ref());
            *listeners.borrow_mut() = Some(PanelEvents { window, key, pointer });
        }
    });
    use_drop(move || { listeners.borrow_mut().take(); });
    use_effect(move || {
        let (Some(kind), Some(root)) = (panel(), root()) else { return; };
        let selector = match kind {
            ToolPanel::Objects => "#workbench-objects input[type=search]",
            ToolPanel::Inspector => "#workbench-inspector h2",
        };
        if let Some(target) = root.query_selector(selector).ok().flatten()
            .and_then(|element| element.dyn_into::<HtmlElement>().ok())
        {
            let _ = target.focus();
        }
    });
    rsx! {
        div { style: "display: contents",
            onmounted: move |event: MountedEvent| root.set(event.data().try_as_web_event()),
            {children}
            for (kind, content) in [(ToolPanel::Objects, objects), (ToolPanel::Inspector, inspector)] {
                section {
                    key: "{kind.id()}", id: kind.id(), aria_label: kind.label(),
                    class: "workbench-popout", hidden: panel() != Some(kind),
                    inert: (panel() != Some(kind)).then_some(""),
                    header { class: "workbench-popout-heading",
                        h2 { tabindex: "-1", "{kind.label()}" }
                        Button { variant: ButtonVariant::Ghost,
                            aria_label: format!("Close {}", kind.label()),
                            onclick: move |_| close(panel, true),
                            "Close"
                        }
                    }
                    div { class: "workbench-popout-content", {content} }
                }
            }
        }
    }
}

struct ToolRowObservers {
    resize: web_sys::ResizeObserver,
    mutation: web_sys::MutationObserver,
    _resize_callback: Closure<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>,
    _mutation_callback: Closure<dyn FnMut()>,
}

impl Drop for ToolRowObservers {
    fn drop(&mut self) {
        self.resize.disconnect();
        self.mutation.disconnect();
    }
}

trait RemoveAttribute {
    fn remove_attr(&self, name: &str);
}

impl RemoveAttribute for web_sys::Element {
    fn remove_attr(&self, name: &str) {
        let _ = web_sys::Element::remove_attribute(self, name);
    }
}

fn query_all(root: &web_sys::Element, selector: &str) -> Vec<web_sys::Element> {
    let Ok(nodes) = root.query_selector_all(selector) else {
        return Vec::new();
    };
    (0..nodes.length())
        .filter_map(|index| nodes.item(index)?.dyn_into::<web_sys::Element>().ok())
        .collect()
}

fn copy_attribute(source: &web_sys::Element, target: &web_sys::Element, name: &str) {
    match source.get_attribute(name) {
        Some(value) => {
            let _ = target.set_attribute(name, &value);
        }
        None => {
            target.remove_attr(name);
        }
    }
}

fn track_fits(track: &web_sys::Element) -> bool {
    track.client_width() > 0 && track.scroll_width() <= track.client_width() + 1
}

fn set_tool_distance(group: &web_sys::Element, track: &web_sys::Element) {
    if let Some(group) = group.dyn_ref::<HtmlElement>() {
        let _ = group
            .style()
            .set_property("--workbench-tool-distance", &format!("{}px", track.scroll_width()));
    }
}

fn fit_open_labels(root: &web_sys::Element, group: &web_sys::Element) {
    let Some(track) = group
        .query_selector(".workbench-tool-track")
        .ok()
        .flatten()
    else {
        return;
    };
    let _ = root.set_attribute("data-fit-headers", "none");
    root.remove_attr("data-utility-labels");
    let _ = group.set_attribute("data-action-labels", "full");
    if !track_fits(&track) {
        let _ = group.set_attribute("data-action-labels", "adaptive");
        let mut actions: Vec<_> = query_all(&track, "[data-workbench-action]")
            .into_iter()
            .enumerate()
            .collect();
        for (_, action) in &actions {
            action.remove_attr("data-show-label");
        }
        actions.sort_by_key(|(index, action)| {
            let priority = if action.get_attribute("aria-pressed").as_deref() == Some("true")
                || action.get_attribute("aria-checked").as_deref() == Some("true")
            {
                0
            } else {
                1
            };
            (priority, *index)
        });
        for (_, action) in actions {
            let _ = action.set_attribute("data-show-label", "");
            if !track_fits(&track) {
                action.remove_attr("data-show-label");
            }
        }
        let _ = root.set_attribute("data-utility-labels", "full");
        if !track_fits(&track) {
            root.remove_attr("data-utility-labels");
        }
        return;
    }
    let _ = root.set_attribute("data-utility-labels", "full");
    if !track_fits(&track) {
        root.remove_attr("data-utility-labels");
    }
    let _ = root.set_attribute("data-fit-headers", "active");
    if !track_fits(&track) {
        let _ = root.set_attribute("data-fit-headers", "none");
    }
    let previous = root.get_attribute("data-fit-headers");
    let _ = root.set_attribute("data-fit-headers", "all");
    if !track_fits(&track) {
        match previous {
            Some(value) => {
                let _ = root.set_attribute("data-fit-headers", &value);
            }
            None => {
                root.remove_attr("data-fit-headers");
            }
        }
    }
}

fn fit_closed_labels(root: &web_sys::Element) {
    root.remove_attr("data-fit-headers");
    let _ = root.set_attribute("data-utility-labels", "full");
    let triggers = query_all(root, "[data-workbench-section-trigger]");
    for trigger in &triggers {
        trigger.remove_attr("data-closed-hide");
    }
    for trigger in triggers.iter().rev() {
        if root.scroll_width() <= root.client_width() + 1 {
            break;
        }
        let _ = trigger.set_attribute("data-closed-hide", "");
    }
    if root.scroll_width() > root.client_width() + 1 {
        root.remove_attr("data-utility-labels");
    }
}

fn fit_tool_row(live: &web_sys::Element) {
    if !live.is_connected() {
        return;
    }
    if let Some(document) = live.owner_document()
        && let Some(active) = document.active_element()
        && active
            .closest("[data-workbench-section][data-open=false] .workbench-tool-reveal")
            .ok()
            .flatten()
            .is_some()
        && let Some(group) = active
            .closest("[data-workbench-section]")
            .ok()
            .flatten()
        && let Some(trigger) = group
            .query_selector("[data-workbench-section-trigger]")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlElement>().ok())
    {
        let _ = trigger.focus();
    }
    let Ok(probe_node) = live.clone_node_with_deep(true) else {
        return;
    };
    let Ok(probe) = probe_node.dyn_into::<web_sys::Element>() else {
        return;
    };
    probe.remove_attr("id");
    probe.remove_attr("data-dioxus-id");
    for descendant in query_all(&probe, "[id], [data-dioxus-id]") {
        descendant.remove_attr("id");
        descendant.remove_attr("data-dioxus-id");
    }
    let probe_class = format!("{} workbench-tool-row-measuring", probe.get_attribute("class").unwrap_or_default());
    let _ = probe.set_attribute("class", &probe_class);
    let _ = probe.set_attribute("inert", "");
    let _ = probe.set_attribute("aria-hidden", "true");
    if let Some(probe_html) = probe.dyn_ref::<HtmlElement>() {
        probe_html.style().set_css_text(&format!(
            "position:fixed;left:-10000px;top:0;width:{}px;visibility:hidden;pointer-events:none",
            live.get_bounding_client_rect().width()
        ));
    }
    let parent = live.parent_element().map(|element| element.into()).or_else(|| {
        live.owner_document()
            .and_then(|document| document.body())
            .map(Into::into)
    });
    let Some(parent) = parent else { return };
    let parent: web_sys::Node = parent;
    if parent.append_child(probe.as_ref()).is_err() {
        return;
    }
    let measured_groups = query_all(&probe, "[data-workbench-section]");
    let actual_groups = query_all(live, "[data-workbench-section]");
    if let Some(open_group) = measured_groups
        .iter()
        .find(|group| group.get_attribute("data-open").as_deref() == Some("true"))
    {
        fit_open_labels(&probe, open_group);
        copy_attribute(&probe, live, "data-fit-headers");
        copy_attribute(&probe, live, "data-utility-labels");
        for trigger in query_all(live, "[data-workbench-section-trigger]") {
            trigger.remove_attr("data-closed-hide");
        }
        if let Some(actual_group) = actual_groups
            .iter()
            .find(|group| group.get_attribute("data-open").as_deref() == Some("true"))
        {
            copy_attribute(open_group, actual_group, "data-action-labels");
            let measured_actions = query_all(open_group, "[data-workbench-action]");
            let actual_actions = query_all(actual_group, "[data-workbench-action]");
            for (measured, actual) in measured_actions.iter().zip(actual_actions.iter()) {
                if measured.has_attribute("data-show-label") {
                    let _ = actual.set_attribute("data-show-label", "");
                } else {
                    actual.remove_attr("data-show-label");
                }
            }
            if let Some(track) = open_group
                .query_selector(".workbench-tool-track")
                .ok()
                .flatten()
            {
                set_tool_distance(actual_group, &track);
            }
        }
    } else {
        fit_closed_labels(&probe);
        copy_attribute(&probe, live, "data-utility-labels");
        live.remove_attr("data-fit-headers");
        let measured_triggers = query_all(&probe, "[data-workbench-section-trigger]");
        let actual_triggers = query_all(live, "[data-workbench-section-trigger]");
        for (measured, actual) in measured_triggers.iter().zip(actual_triggers.iter()) {
            if measured.has_attribute("data-closed-hide") {
                let _ = actual.set_attribute("data-closed-hide", "");
            } else {
                actual.remove_attr("data-closed-hide");
            }
        }
        for (measured, actual) in measured_groups.iter().zip(actual_groups.iter()) {
            if let Some(track) = measured
                .query_selector(".workbench-tool-track")
                .ok()
                .flatten()
            {
                set_tool_distance(actual, &track);
            }
        }
    }
    probe.remove();
}

fn fonts_ready(document: Option<web_sys::Document>) -> Option<js_sys::Promise> {
    let document = document?;
    let document_value: &wasm_bindgen::JsValue = document.as_ref();
    let fonts = js_sys::Reflect::get(document_value, &"fonts".into()).ok()?;
    let ready = js_sys::Reflect::get(&fonts, &"ready".into()).ok()?;
    ready.dyn_into::<js_sys::Promise>().ok()
}

/// One bounded tool row. Fitting is applied to this final DOM by the host;
/// section and domain state remain with their caller.
#[component]
pub(super) fn ToolRow(label: String, fit_key: String, children: Element) -> Element {
    let mut root = use_signal(|| None::<web_sys::Element>);
    let observers = use_hook(|| Rc::new(RefCell::new(None::<ToolRowObservers>)));
    use_effect(use_reactive((&fit_key,), {
        let observers = observers.clone();
        move |_| {
            let Some(root_element) = root() else { return };
            fit_tool_row(&root_element);
            if observers.borrow().is_some() {
                return;
            }
            let resize_root = root_element.clone();
            let resize_callback =
                Closure::<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>::new(move |_, _| {
                    if resize_root.is_connected() {
                        fit_tool_row(&resize_root);
                    }
                });
            let Ok(resize) =
                web_sys::ResizeObserver::new(resize_callback.as_ref().unchecked_ref())
            else {
                return;
            };
            resize.observe(&root_element);
            let mutation_root = root_element.clone();
            let mutation_callback = Closure::<dyn FnMut()>::new(move || {
                if mutation_root.is_connected() {
                    fit_tool_row(&mutation_root);
                }
            });
            let Ok(mutation) =
                web_sys::MutationObserver::new(mutation_callback.as_ref().unchecked_ref())
            else {
                return;
            };
            let options = web_sys::MutationObserverInit::new();
            options.set_attributes(true);
            let attribute_filter = js_sys::Array::new();
            for attribute in ["data-open", "aria-pressed", "aria-checked", "disabled"] {
                attribute_filter.push(&attribute.into());
            }
            options.set_attribute_filter(&attribute_filter);
            options.set_child_list(true);
            options.set_subtree(true);
            if mutation.observe_with_options(&root_element, &options).is_err() {
                return;
            }
            if let Some(promise) = fonts_ready(root_element.owner_document()) {
                let font_root = root_element.clone();
                spawn(async move {
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                    if font_root.is_connected() {
                        fit_tool_row(&font_root);
                    }
                });
            }
            *observers.borrow_mut() = Some(ToolRowObservers {
                resize,
                mutation,
                _resize_callback: resize_callback,
                _mutation_callback: mutation_callback,
            });
        }
    }));
    use_drop(move || {
        observers.borrow_mut().take();
    });
    rsx! {
        Toolbar { aria_label: label, class: "workbench-tool-row",
            attributes: vec![dioxus::events::onmounted(move |event: MountedEvent| root.set(event.data().try_as_web_event()))],
            {children}
        }
    }
}

/// A section trigger and its retained, inert action lane.
#[component]
pub(super) fn ToolSection(
    id: &'static str,
    index: usize,
    label: String,
    caption: Option<String>,
    expanded: bool,
    on_toggle: EventHandler<()>,
    icon: Element,
    children: Element,
) -> Element {
    rsx! {
        div { class: "workbench-tool-group", "data-workbench-section": id, "data-open": expanded,
            ToolbarButton { index, class: "workbench-tool-trigger",
                "data-workbench-section-trigger": id,
                aria_label: label.clone(), aria_expanded: expanded,
                aria_controls: format!("workbench-section-{id}"),
                on_click: move |_| on_toggle.call(()),
                span { class: "workbench-tool-icon", "aria-hidden": "true", {icon} }
                span { class: "workbench-tool-heading",
                    span { class: "workbench-tool-name", "{label}" }
                    if let Some(caption) = caption {
                        span { class: "workbench-tool-caption", "{caption}" }
                    }
                }
                ChevronDown { class: "workbench-tool-chevron" }
            }
            div { class: "workbench-tool-reveal", id: format!("workbench-section-{id}"),
                inert: (!expanded).then_some(""), "aria-hidden": (!expanded).then_some("true"),
                div { class: "workbench-tool-reveal-inner",
                    div { class: "workbench-tool-track", {children} }
                }
            }
        }
    }
}

/// Utilities join the workspace's own tool row; they do not own domain task state.
#[component]
pub(super) fn PanelUtilities(
    panel: Signal<Option<ToolPanel>>,
    on_hide_transient: EventHandler<()>,
    #[props(default = 100)] first_index: usize,
) -> Element {
    rsx! {
        for (offset, kind) in [ToolPanel::Objects, ToolPanel::Inspector].into_iter().enumerate() {
            ToolbarButton { index: first_index + offset,
                id: format!("{}-trigger", kind.id()),
                class: "workbench-utility",
                aria_label: kind.utility_label(), aria_expanded: panel() == Some(kind),
                aria_controls: kind.id(),
                on_click: move |_| {
                    if *panel.peek() == Some(kind) {
                        close(panel, true);
                    } else {
                        on_hide_transient.call(());
                        let mut panel = panel;
                        panel.set(Some(kind));
                    }
                },
                svg { view_box: "0 0 20 20", "aria-hidden": "true",
                    path { d: kind.icon_path() }
                }
                span { class: "workbench-utility-label", "{kind.utility_label()}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_web_ui_shared::test_role::by_role_name;
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn fixture() -> Element {
        let panel = use_signal(|| None::<ToolPanel>);
        rsx! {
            PanelHost { panel,
                objects: rsx! { input { aria_label: "Find an object", r#type: "search" } },
                inspector: rsx! { input { aria_label: "Retained value", value: "unresolved A" } },
                Toolbar { aria_label: "Layout tools",
                    PanelUtilities { panel, on_hide_transient: move |_| {}, first_index: 0usize }
                }
                button { "Canvas target" }
            }
        }
    }

    fn mount_component(component: fn() -> Element) -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(component),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        root
    }

    fn mount() -> web_sys::Element {
        mount_component(fixture)
    }

    fn tool_row_fixture() -> Element {
        let panel = use_signal(|| None::<ToolPanel>);
        let mut section = use_signal(|| Some("alpha"));
        let actions = [
            (1usize, "First very long action name", true),
            (2usize, "Second very long action name", false),
            (3usize, "Third very long action name", false),
            (4usize, "Fourth very long action name", false),
            (5usize, "Fifth very long action name", false),
            (6usize, "Sixth very long action name", false),
        ];
        rsx! {
            style { {include_str!("../../assets/workbench-base.css")} }
            style { {include_str!("../../assets/tailwind.css")} }
            style { {include_str!("../../assets/shell.css")} }
            div { id: "tool-row-fixture", style: "width:520px",
                ToolRow { label: "Layout tools".to_owned(), fit_key: section().unwrap_or("closed").to_owned(),
                    ToolSection { id: "alpha", index: 0usize, label: "Alpha".to_owned(), caption: Some("Current".to_owned()), expanded: section() == Some("alpha"),
                        on_toggle: move |_| section.set((section() != Some("alpha")).then_some("alpha")),
                        icon: rsx! { span { "A" } },
                        for (index, label, pressed) in actions {
                            ToolbarButton { index, class: "workbench-action", "data-workbench-action": "", aria_label: label, aria_pressed: pressed,
                                span { class: "workbench-action-label", "{label}" }
                            }
                        }
                    }
                    ToolSection { id: "beta", index: 7usize, label: "Beta".to_owned(), caption: None, expanded: section() == Some("beta"),
                        on_toggle: move |_| section.set((section() != Some("beta")).then_some("beta")),
                        icon: rsx! { span { "B" } },
                        ToolbarButton { index: 8usize, class: "workbench-action", "data-workbench-action": "", aria_label: "Hidden beta action",
                            span { class: "workbench-action-label", "Hidden beta action" }
                        }
                    }
                    PanelUtilities { panel, on_hide_transient: move |_| {} }
                }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn utilities_exclude_each_other_and_same_utility_closes_without_unmounting() {
        let root = mount();
        wait_until("workbench utilities mount", || by_role_name(&root, "button", "Objects").is_some()).await;
        let objects = by_role_name(&root, "button", "Objects").unwrap();
        objects.click();
        wait_until("Objects opens", || objects.get_attribute("aria-expanded").as_deref() == Some("true")).await;
        let retained = root.query_selector("input[aria-label='Retained value']").unwrap().unwrap();
        let inspect = by_role_name(&root, "button", "Inspect").unwrap();
        inspect.click();
        wait_until("Inspect opens", || inspect.get_attribute("aria-expanded").as_deref() == Some("true")).await;
        assert_eq!(objects.get_attribute("aria-expanded").as_deref(), Some("false"));
        assert!(root.query_selector("#workbench-objects[inert][hidden]").unwrap().is_some(), "closed Objects is hidden and inert");
        inspect.click();
        wait_until("Inspect closes", || inspect.get_attribute("aria-expanded").as_deref() == Some("false")).await;
        assert!(root.query_selector("#workbench-inspector[inert][hidden]").unwrap().is_some(), "closed Inspector is hidden and inert");
        assert!(retained.is_same_node(root.query_selector("input[aria-label='Retained value']").unwrap().as_ref().map(|element| element.as_ref())), "visibility never replaces the editor node");
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn dismissal_returns_focus_except_outside_pointer_keeps_its_target() {
        let root = mount();
        wait_until("workbench utilities mount", || by_role_name(&root, "button", "Objects").is_some()).await;
        let objects = by_role_name(&root, "button", "Objects").unwrap();
        let search = root.query_selector("input[type=search]").unwrap().unwrap().dyn_into::<HtmlElement>().unwrap();
        let canvas = by_role_name(&root, "button", "Canvas target").unwrap();
        for dismissal in ["Close", "Escape", "outside pointer"] {
            objects.click();
            wait_until("Objects focuses search", || search.owner_document().unwrap().active_element().as_ref() == Some(search.as_ref())).await;
            match dismissal {
                "Close" => by_role_name(&root, "button", "Close Objects").unwrap().click(),
                "Escape" => {
                    let init = web_sys::KeyboardEventInit::new();
                    init.set_key("Escape");
                    init.set_bubbles(true);
                    init.set_cancelable(true);
                    search.dispatch_event(&web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap()).unwrap();
                }
                _ => {
                    canvas.focus().unwrap();
                    let init = web_sys::PointerEventInit::new();
                    init.set_bubbles(true);
                    canvas.dispatch_event(&web_sys::PointerEvent::new_with_event_init_dict("pointerdown", &init).unwrap()).unwrap();
                }
            }
            wait_until("Objects dismissed", || objects.get_attribute("aria-expanded").as_deref() == Some("false")).await;
            let expected = if dismissal == "outside pointer" { &canvas } else { &objects };
            assert_eq!(root.owner_document().unwrap().active_element().as_ref(), Some(expected.as_ref()), "{dismissal} focus target");
            assert!(root.query_selector("#workbench-objects[hidden][inert]").unwrap().is_some(), "{dismissal} hides and inerts the content");
        }
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn tool_row_fits_final_labels_before_roving_past_closed_actions() {
        let root = mount_component(tool_row_fixture);
        wait_until("the fitted tool row allocates its labels", || {
            root.query_selector("[data-workbench-section='alpha']")
                .unwrap()
                .is_some_and(|group| group.get_attribute("data-action-labels").as_deref() == Some("adaptive"))
        })
        .await;
        let row = root.query_selector(".workbench-tool-row").unwrap().unwrap();
        let rect = row.get_bounding_client_rect();
        assert!((rect.height() - 42.0).abs() < 0.5, "row stays 42px high: {}", rect.height());
        assert!(row.scroll_width() <= row.client_width() + 1, "the row itself does not overflow");
        let track = root.query_selector("[data-workbench-section='alpha'] .workbench-tool-track").unwrap().unwrap();
        let window = web_sys::window().unwrap();
        let label_display = |selector: &str| {
            let label = root.query_selector(selector).unwrap().unwrap();
            window.get_computed_style(&label).unwrap().unwrap().get_property_value("display").unwrap()
        };
        assert_ne!(label_display("[aria-label='First very long action name'] .workbench-action-label"), "none", "the pressed action keeps its name");
        assert_eq!(label_display("[aria-label='Third very long action name'] .workbench-action-label"), "none", "a lower-priority action folds before captions consume the lane");
        root.query_selector("#tool-row-fixture").unwrap().unwrap().dyn_into::<HtmlElement>().unwrap().style().set_property("width", "320px").unwrap();
        wait_until("the narrow action lane retains native overflow", || {
            track.scroll_width() > track.client_width() && row.scroll_width() <= row.client_width() + 1
        })
        .await;
        assert!(root.query_selector(".workbench-tool-row-measuring").unwrap().is_none(), "the measurement probe is removed");
        let alpha = by_role_name(&root, "button", "Alpha").unwrap();
        alpha.focus().unwrap();
        let init = web_sys::KeyboardEventInit::new();
        init.set_key("End");
        init.set_bubbles(true);
        init.set_cancelable(true);
        alpha.dispatch_event(&web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap()).unwrap();
        wait_until("End reaches Inspect without entering the closed section", || {
            by_role_name(&root, "button", "Inspect").is_some_and(|inspect| inspect.matches(":focus").unwrap())
        })
        .await;
        root.remove();
    }
}
