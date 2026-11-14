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

/// Retained Show/hide commands toggle through the same dismissal as the utilities:
/// closing restores focus to the utility instead of stranding it in hidden content.
pub(super) fn toggle_tool_panel(
    panel: Signal<Option<ToolPanel>>,
    kind: ToolPanel,
    on_hide_transient: Callback<(), bool>,
) {
    if *panel.peek() == Some(kind) {
        close(panel, true);
    } else if on_hide_transient.call(()) {
        let mut panel = panel;
        panel.set(Some(kind));
    }
}

struct PanelEvents {
    window: web_sys::Window,
    key: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    pointer: Closure<dyn FnMut(web_sys::PointerEvent)>,
}

impl Drop for PanelEvents {
    fn drop(&mut self) {
        let _ = self
            .window
            .remove_event_listener_with_callback("keydown", self.key.as_ref().unchecked_ref());
        let _ = self.window.remove_event_listener_with_callback(
            "pointerdown",
            self.pointer.as_ref().unchecked_ref(),
        );
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
            let Some(root) = root() else {
                return;
            };
            let Some(window) = web_sys::window() else {
                return;
            };
            let key_root = root.clone();
            // Window bubble runs after nested controls, canvas picks and document-owned
            // relationship returns; a handled Escape must not dismiss two surfaces.
            let key = Closure::new(move |event: web_sys::KeyboardEvent| {
                if key_root.is_connected()
                    && event.key() == "Escape"
                    && !event.default_prevented()
                    && panel.peek().is_some()
                {
                    event.prevent_default();
                    close(panel, true);
                }
            });
            let pointer = Closure::new(move |event: web_sys::PointerEvent| {
                let Some(kind) = *panel.peek() else {
                    return;
                };
                if !root.is_connected() {
                    return;
                }
                let Some(target) = event
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                else {
                    return;
                };
                let selector = format!(
                    "#{}, .workbench-utility, [role=menu], [role=listbox], [data-slot=popover-content]",
                    kind.id()
                );
                if target.closest(&selector).ok().flatten().is_none() {
                    close(panel, false);
                }
            });
            let _ =
                window.add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref());
            let _ = window
                .add_event_listener_with_callback("pointerdown", pointer.as_ref().unchecked_ref());
            *listeners.borrow_mut() = Some(PanelEvents {
                window,
                key,
                pointer,
            });
        }
    });
    use_drop(move || {
        listeners.borrow_mut().take();
    });
    use_effect(move || {
        let (Some(kind), Some(root)) = (panel(), root()) else {
            return;
        };
        let selector = match kind {
            ToolPanel::Objects => "#workbench-objects input[type=search]",
            ToolPanel::Inspector => "#workbench-inspector h2",
        };
        if let Some(target) = root
            .query_selector(selector)
            .ok()
            .flatten()
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

fn query_all(root: &web_sys::Element, selector: &str) -> Vec<web_sys::Element> {
    let Ok(nodes) = root.query_selector_all(selector) else {
        return Vec::new();
    };
    (0..nodes.length())
        .filter_map(|index| nodes.item(index)?.dyn_into::<web_sys::Element>().ok())
        .collect()
}

fn set_tool_distance(group: &web_sys::Element, track: &web_sys::Element) {
    if let Some(group) = group.dyn_ref::<HtmlElement>() {
        let _ = group.style().set_property(
            "--workbench-tool-distance",
            &format!("{}px", track.scroll_width()),
        );
    }
}

fn fit_tool_row(live: &web_sys::Element) {
    // Slice 3 density pilot: container queries own density; keep only measured
    // animation distance from live tracks (no probe, no fitting attributes).
    if !live.is_connected() {
        return;
    }
    for group in query_all(live, "[data-workbench-section]") {
        if let Some(track) = group.query_selector(".workbench-tool-track").ok().flatten() {
            set_tool_distance(&group, &track);
        }
    }
}
fn fonts_ready(document: Option<web_sys::Document>) -> Option<js_sys::Promise> {
    let document = document?;
    let document_value: &wasm_bindgen::JsValue = document.as_ref();
    let fonts = js_sys::Reflect::get(document_value, &"fonts".into()).ok()?;
    let ready = js_sys::Reflect::get(&fonts, &"ready".into()).ok()?;
    ready.dyn_into::<js_sys::Promise>().ok()
}

/// One bounded tool row. Density is owned by container queries;
/// section and domain state remain with their caller.
#[component]
pub(super) fn ToolRow(label: String, children: Element) -> Element {
    let mut root = use_signal(|| None::<web_sys::Element>);
    let observers = use_hook(|| Rc::new(RefCell::new(None::<ToolRowObservers>)));
    use_effect({
        let observers = observers.clone();
        move || {
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
            let Ok(resize) = web_sys::ResizeObserver::new(resize_callback.as_ref().unchecked_ref())
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
            if mutation
                .observe_with_options(&root_element, &options)
                .is_err()
            {
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
    });
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
/// Opening a panel first asks the transient owner to hide; its refusal leaves both
/// the owner and the panels unchanged.
#[component]
pub(super) fn PanelUtilities(
    panel: Signal<Option<ToolPanel>>,
    on_hide_transient: Callback<(), bool>,
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
                    toggle_tool_panel(panel, kind, on_hide_transient);
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
                    PanelUtilities { panel, on_hide_transient: move |_| true, first_index: 0usize }
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
        // (index, label, pressed, checked, has_svg): pressed+checked shown when narrow,
        // unpressed SVG hidden when narrow, text-only (no SVG) always shown.
        let actions = [
            (1usize, "First very long action name", true, false, true),
            (2usize, "Second very long action name", false, true, true),
            (3usize, "Third very long action name", false, false, true),
            (4usize, "Fourth very long action name", false, false, true),
            (5usize, "Fifth very long action name", false, false, true),
            (6usize, "Sixth very long action name", false, false, false),
        ];
        rsx! {
            style { {include_str!("../../assets/workbench-base.css")} }
            style { {include_str!("../../assets/tailwind.css")} }
            style { {include_str!("../../assets/layout-toolbar.css")} }
            style { {include_str!("../../assets/shell.css")} }
            div { id: "tool-row-fixture", style: "width:520px",
                ToolRow { label: "Layout tools".to_owned(),
                    ToolSection { id: "alpha", index: 0usize, label: "Alpha".to_owned(), caption: Some("Current".to_owned()), expanded: section() == Some("alpha"),
                        on_toggle: move |_| section.set((section() != Some("alpha")).then_some("alpha")),
                        icon: rsx! { span { "A" } },
                        for (index, label, pressed, checked, has_svg) in actions {
                            ToolbarButton { index, class: "workbench-action", "data-workbench-action": "", aria_label: label, aria_pressed: pressed, "aria-checked": if checked { "true" } else { "false" },
                                if has_svg {
                                    svg { view_box: "0 0 20 20", "aria-hidden": "true",
                                        path { d: "M3 3h14v14H3z" }
                                    }
                                }
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
                    PanelUtilities { panel, on_hide_transient: move |_| true }
                }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn utilities_exclude_each_other_and_same_utility_closes_without_unmounting() {
        let root = mount();
        wait_until("workbench utilities mount", || {
            by_role_name(&root, "button", "Objects").is_some()
        })
        .await;
        let objects = by_role_name(&root, "button", "Objects").unwrap();
        objects.click();
        wait_until("Objects opens", || {
            objects.get_attribute("aria-expanded").as_deref() == Some("true")
        })
        .await;
        let retained = root
            .query_selector("input[aria-label='Retained value']")
            .unwrap()
            .unwrap();
        let inspect = by_role_name(&root, "button", "Inspect").unwrap();
        inspect.click();
        wait_until("Inspect opens", || {
            inspect.get_attribute("aria-expanded").as_deref() == Some("true")
        })
        .await;
        assert_eq!(
            objects.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        assert!(
            root.query_selector("#workbench-objects[inert][hidden]")
                .unwrap()
                .is_some(),
            "closed Objects is hidden and inert"
        );
        inspect.click();
        wait_until("Inspect closes", || {
            inspect.get_attribute("aria-expanded").as_deref() == Some("false")
        })
        .await;
        assert!(
            root.query_selector("#workbench-inspector[inert][hidden]")
                .unwrap()
                .is_some(),
            "closed Inspector is hidden and inert"
        );
        assert!(
            retained.is_same_node(
                root.query_selector("input[aria-label='Retained value']")
                    .unwrap()
                    .as_ref()
                    .map(|element| element.as_ref())
            ),
            "visibility never replaces the editor node"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn dismissal_returns_focus_except_outside_pointer_keeps_its_target() {
        let root = mount();
        wait_until("workbench utilities mount", || {
            by_role_name(&root, "button", "Objects").is_some()
        })
        .await;
        let objects = by_role_name(&root, "button", "Objects").unwrap();
        let search = root
            .query_selector("input[type=search]")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap();
        let canvas = by_role_name(&root, "button", "Canvas target").unwrap();
        for dismissal in ["Close", "Escape", "outside pointer"] {
            objects.click();
            wait_until("Objects focuses search", || {
                search.owner_document().unwrap().active_element().as_ref() == Some(search.as_ref())
            })
            .await;
            match dismissal {
                "Close" => by_role_name(&root, "button", "Close Objects")
                    .unwrap()
                    .click(),
                "Escape" => {
                    let init = web_sys::KeyboardEventInit::new();
                    init.set_key("Escape");
                    init.set_bubbles(true);
                    init.set_cancelable(true);
                    search
                        .dispatch_event(
                            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict(
                                "keydown", &init,
                            )
                            .unwrap(),
                        )
                        .unwrap();
                }
                _ => {
                    canvas.focus().unwrap();
                    let init = web_sys::PointerEventInit::new();
                    init.set_bubbles(true);
                    canvas
                        .dispatch_event(
                            &web_sys::PointerEvent::new_with_event_init_dict("pointerdown", &init)
                                .unwrap(),
                        )
                        .unwrap();
                }
            }
            wait_until("Objects dismissed", || {
                objects.get_attribute("aria-expanded").as_deref() == Some("false")
            })
            .await;
            let expected = if dismissal == "outside pointer" {
                &canvas
            } else {
                &objects
            };
            assert_eq!(
                root.owner_document().unwrap().active_element().as_ref(),
                Some(expected.as_ref()),
                "{dismissal} focus target"
            );
            assert!(
                root.query_selector("#workbench-objects[hidden][inert]")
                    .unwrap()
                    .is_some(),
                "{dismissal} hides and inerts the content"
            );
        }
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn tool_row_fits_final_labels_before_roving_past_closed_actions() {
        // Slice 3: container queries own density (no fitting attributes/probe).
        // Assertion map from prior fitting version:
        // - Wait adaptive / probe inert/hidden/focus/removed -> replaced by independent CQ checks below.
        // - 42px, row no-overflow, narrow lane overflow, End->Inspect roving -> retained.
        // - Pressed shown, unpressed hidden -> retained + extended (checked, text-only, both sides of 850).
        let root = mount_component(tool_row_fixture);
        wait_until("tool row mounts", || {
            root.query_selector(".workbench-tool-row")
                .unwrap()
                .is_some()
        })
        .await;
        let row = root.query_selector(".workbench-tool-row").unwrap().unwrap();
        let window = web_sys::window().unwrap();
        let display_of = |selector: &str| {
            let el = root.query_selector(selector).unwrap().unwrap();
            window
                .get_computed_style(&el)
                .unwrap()
                .unwrap()
                .get_property_value("display")
                .unwrap()
        };
        wait_until(
            "container density hides unpressed labels at narrow width",
            || {
                display_of("[aria-label='First very long action name'] .workbench-action-label")
                    != "none"
                    && display_of(
                        "[aria-label='Third very long action name'] .workbench-action-label",
                    ) == "none"
            },
        )
        .await;
        let rect = row.get_bounding_client_rect();
        assert!(
            (rect.height() - 42.0).abs() < 0.5,
            "row stays 42px high: {}",
            rect.height()
        );
        assert!(
            row.scroll_width() <= row.client_width() + 1,
            "the row itself does not overflow"
        );
        assert_ne!(
            display_of("[aria-label='First very long action name'] .workbench-action-label"),
            "none",
            "the pressed action keeps its name"
        );
        assert_ne!(
            display_of("[aria-label='Second very long action name'] .workbench-action-label"),
            "none",
            "the checked action keeps its name"
        );
        assert_eq!(
            display_of("[aria-label='Third very long action name'] .workbench-action-label"),
            "none",
            "an unpressed icon+label action folds at narrow width"
        );
        let text_only = root
            .query_selector("[aria-label='Sixth very long action name'] .workbench-action-label")
            .unwrap()
            .unwrap();
        assert_ne!(
            display_of("[aria-label='Sixth very long action name'] .workbench-action-label"),
            "none",
            "the unpressed text-only action never goes blank"
        );
        assert!(
            text_only
                .text_content()
                .is_some_and(|t| !t.trim().is_empty()),
            "text-only label has text"
        );
        let text_rect = text_only.get_bounding_client_rect();
        assert!(
            text_rect.width() > 0.0 && text_rect.height() > 0.0,
            "text-only label has bounds"
        );
        assert_eq!(
            display_of("[data-workbench-section='alpha'] .workbench-tool-heading"),
            "none",
            "open narrow hides headings"
        );
        assert_eq!(
            display_of(".workbench-utility .workbench-utility-label"),
            "none",
            "open narrow hides utility labels"
        );
        let beta_reveal = root
            .query_selector("[data-workbench-section='beta'] .workbench-tool-reveal")
            .unwrap()
            .unwrap();
        assert!(beta_reveal.has_attribute("inert"), "closed lane is inert");
        assert_eq!(
            beta_reveal.get_attribute("aria-hidden").as_deref(),
            Some("true"),
            "closed lane is aria-hidden"
        );
        // Fractional W just below 850 (parent 853.5px, row content-box 849.5px)
        // stays in the narrow band: half-open ranges leave no endpoint gap.
        root.query_selector("#tool-row-fixture")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap()
            .style()
            .set_property("width", "853.5px")
            .unwrap();
        assert_eq!(
            display_of("[aria-label='Third very long action name'] .workbench-action-label"),
            "none",
            "fractional width below 850 keeps unpressed labels folded"
        );
        assert_ne!(
            display_of("[aria-label='First very long action name'] .workbench-action-label"),
            "none",
            "fractional width below 850 keeps the pressed label"
        );
        root.query_selector("#tool-row-fixture")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap()
            .style()
            .set_property("width", "900px")
            .unwrap();
        wait_until("wide density shows unpressed labels", || {
            display_of("[aria-label='Third very long action name'] .workbench-action-label")
                != "none"
        })
        .await;
        assert_ne!(
            display_of("[aria-label='Third very long action name'] .workbench-action-label"),
            "none",
            "wide shows unpressed labels"
        );
        assert_ne!(
            display_of("[data-workbench-section='alpha'] .workbench-tool-heading"),
            "none",
            "wide open shows its heading"
        );
        assert_eq!(
            display_of("[data-workbench-section='beta'] .workbench-tool-heading"),
            "none",
            "wide open hides closed-section headings"
        );
        assert_ne!(
            display_of(".workbench-utility .workbench-utility-label"),
            "none",
            "wide open shows utility labels"
        );
        root.query_selector("#tool-row-fixture")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap()
            .style()
            .set_property("width", "320px")
            .unwrap();
        let track = root
            .query_selector("[data-workbench-section='alpha'] .workbench-tool-track")
            .unwrap()
            .unwrap();
        wait_until("the narrow action lane retains native overflow", || {
            track.scroll_width() > track.client_width()
                && row.scroll_width() <= row.client_width() + 1
        })
        .await;
        let alpha = by_role_name(&root, "button", "Alpha").unwrap();
        alpha.focus().unwrap();
        let init = web_sys::KeyboardEventInit::new();
        init.set_key("End");
        init.set_bubbles(true);
        init.set_cancelable(true);
        alpha
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap(),
            )
            .unwrap();
        wait_until(
            "End reaches Inspect without entering the closed section",
            || {
                by_role_name(&root, "button", "Inspect")
                    .is_some_and(|inspect| inspect.matches(":focus").unwrap())
            },
        )
        .await;
        root.remove();
    }

    fn clamp_fixture() -> Element {
        let panel = use_signal(|| None::<ToolPanel>);
        rsx! {
            style { {include_str!("../../assets/workbench-base.css")} }
            style { {include_str!("../../assets/tailwind.css")} }
            style { {include_str!("../../assets/shell.css")} }
            div { id: "clamp-fixture",
                PanelHost { panel,
                    objects: rsx! {
                        input { aria_label: "Find an object", r#type: "search" }
                        div {
                            for i in 0..60 {
                                div { aria_label: format!("Objects row {i}"), style: "height: 40px", "Objects row {i}" }
                            }
                        }
                    },
                    inspector: rsx! {
                        input { aria_label: "Retained value", value: "unresolved A" }
                        div {
                            for i in 0..60 {
                                div { aria_label: format!("Inspector row {i}"), style: "height: 40px", "Inspector row {i}" }
                            }
                        }
                    },
                    Toolbar { aria_label: "Layout tools",
                        PanelUtilities { panel, on_hide_transient: move |_| true, first_index: 0usize }
                    }
                    button { "Canvas target" }
                }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn panel_clamps_to_layout_viewport_and_synthetic_content_scroll_keeps_close_reachable() {
        let root = mount_component(clamp_fixture);
        wait_until("workbench utilities mount", || {
            by_role_name(&root, "button", "Objects").is_some()
        })
        .await;
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let viewport = || {
            let w = window.inner_width().unwrap().as_f64().unwrap();
            let h = window.inner_height().unwrap().as_f64().unwrap();
            (w, h)
        };
        let assert_clamped = |panel_id: &str, close_name: &str, expected_width: f64| {
            let (vw, vh) = viewport();
            let panel = root
                .query_selector(&format!("#{panel_id}"))
                .unwrap()
                .unwrap();
            let rect = panel.get_bounding_client_rect();
            assert!(
                rect.left() >= -0.5,
                "{panel_id} left within viewport: {}",
                rect.left()
            );
            assert!(
                rect.right() <= vw + 0.5,
                "{panel_id} right within viewport: {} <= {vw}",
                rect.right()
            );
            assert!(
                rect.top() >= -0.5,
                "{panel_id} top within viewport: {}",
                rect.top()
            );
            assert!(
                rect.bottom() <= vh + 0.5,
                "{panel_id} bottom within viewport: {} <= {vh}",
                rect.bottom()
            );
            assert!(
                (rect.width() - expected_width).abs() < 2.0,
                "{panel_id} width {expected_width}px: {}",
                rect.width()
            );
            assert!(
                rect.width() <= vw - 16.0 + 1.0,
                "{panel_id} width respects 16px safe inset"
            );
            let close = by_role_name(&root, "button", close_name).unwrap();
            let close_rect = close.get_bounding_client_rect();
            assert!(
                close_rect.left() >= -0.5 && close_rect.right() <= vw + 0.5,
                "{close_name} horizontally within viewport"
            );
            assert!(
                close_rect.top() >= -0.5 && close_rect.bottom() <= vh + 0.5,
                "{close_name} vertically within viewport"
            );
            let content = panel
                .query_selector(".workbench-popout-content")
                .unwrap()
                .unwrap();
            assert!(
                content.scroll_height() > content.client_height(),
                "{panel_id} tall content overflows its scroll container"
            );
            let overflow_y = window
                .get_computed_style(&content)
                .unwrap()
                .unwrap()
                .get_property_value("overflow-y")
                .unwrap();
            assert!(
                overflow_y == "auto" || overflow_y == "scroll",
                "{panel_id} content scrolls vertically, got {overflow_y}"
            );
            let doc_el = document.document_element().unwrap();
            assert!(
                doc_el.scroll_width() <= vw as i32 + 1,
                "no page horizontal overflow"
            );
            assert!(
                doc_el.scroll_height() <= vh as i32 + 1,
                "no page vertical overflow"
            );
            assert!(
                document
                    .query_selector("[data-backdrop]")
                    .unwrap()
                    .is_none(),
                "no backdrop"
            );
            assert!(
                panel.get_attribute("role").as_deref() != Some("dialog")
                    || panel.get_attribute("aria-modal").as_deref() != Some("true"),
                "no modal ARIA"
            );
            let row_kind = if panel_id == "workbench-objects" {
                "Objects"
            } else {
                "Inspector"
            };
            let last = panel
                .query_selector(&format!("[aria-label='{row_kind} row 59']"))
                .unwrap()
                .unwrap();
            content.set_scroll_top(content.scroll_height());
            let last_rect = last.get_bounding_client_rect();
            let panel_rect = panel.get_bounding_client_rect();
            assert!(
                last_rect.top() < panel_rect.bottom() && last_rect.bottom() > panel_rect.top(),
                "{panel_id} last row reachable after scroll"
            );
            content.set_scroll_top(0);
        };
        let objects = by_role_name(&root, "button", "Objects").unwrap();
        objects.click();
        wait_until("Objects opens", || {
            objects.get_attribute("aria-expanded").as_deref() == Some("true")
        })
        .await;
        let search = root.query_selector("input[type=search]").unwrap().unwrap();
        wait_until("Objects focuses search", || {
            document.active_element().as_ref() == Some(search.as_ref())
        })
        .await;
        assert_clamped("workbench-objects", "Close Objects", 320.0);
        root.query_selector("#clamp-fixture")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap()
            .style()
            .set_property("width", "320px")
            .unwrap();
        wait_until("panel stays clamped after lane resize", || {
            let panel = root.query_selector("#workbench-objects").unwrap().unwrap();
            let r = panel.get_bounding_client_rect();
            let (vw, vh) = viewport();
            r.left() >= -0.5 && r.right() <= vw + 0.5 && r.top() >= -0.5 && r.bottom() <= vh + 0.5
        })
        .await;
        assert_clamped("workbench-objects", "Close Objects", 320.0);
        let inspect = by_role_name(&root, "button", "Inspect").unwrap();
        inspect.click();
        wait_until("Inspect opens", || {
            inspect.get_attribute("aria-expanded").as_deref() == Some("true")
        })
        .await;
        let heading = root
            .query_selector("#workbench-inspector h2")
            .unwrap()
            .unwrap();
        wait_until("Inspect focuses heading", || {
            document.active_element().as_ref() == Some(heading.as_ref())
        })
        .await;
        assert_clamped("workbench-inspector", "Close Inspector", 400.0);
        root.remove();
    }

    fn transient_fixture() -> Element {
        let panel = use_signal(|| None::<ToolPanel>);
        let mut transient_open = use_signal(|| true);
        let mut allow_hide = use_signal(|| true);
        let mut picker_open = use_signal(|| false);
        let mut ping = use_signal(|| 0u32);
        let request_hide = use_callback(move |()| {
            if *allow_hide.peek() {
                transient_open.set(false);
                true
            } else {
                false
            }
        });
        rsx! {
            PanelHost { panel,
                objects: rsx! {
                    input { aria_label: "Find an object", r#type: "search" }
                    button {
                        aria_label: "Open nested picker",
                        onclick: move |_| picker_open.set(true),
                        "Open nested picker"
                    }
                    if picker_open() {
                        div {
                            role: "listbox", aria_label: "Nested picker", tabindex: "0",
                            onkeydown: move |event: KeyboardEvent| {
                                if event.key().to_string() == "Escape" {
                                    if let Some(raw) = event.data().try_as_web_event() {
                                        raw.prevent_default();
                                    }
                                    picker_open.set(false);
                                }
                            },
                            div { role: "option", aria_label: "Nested choice", "Nested choice" }
                        }
                    }
                },
                inspector: rsx! { input { aria_label: "Retained value", value: "unresolved A" } },
                Toolbar { aria_label: "Layout tools",
                    PanelUtilities { panel, on_hide_transient: request_hide, first_index: 0usize }
                }
                button { "Canvas target" }
                if transient_open() {
                    div { role: "menu", aria_label: "Transient task",
                        button {
                            aria_label: "Close transient task",
                            onclick: move |_| transient_open.set(false),
                            "Close transient task"
                        }
                    }
                }
                button {
                    aria_label: "Refuse transient hide",
                    aria_pressed: !allow_hide(),
                    onclick: move |_| allow_hide.set(false),
                    "Refuse transient hide"
                }
                button {
                    aria_label: "Allow transient hide",
                    onclick: move |_| allow_hide.set(true),
                    "Allow transient hide"
                }
                button {
                    aria_label: "Ping transient fixture",
                    onclick: move |_| ping.set(ping() + 1),
                    "Ping transient fixture"
                }
                output { aria_label: "Ping count", "{ping()}" }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn transient_refusal_blocks_utility_open_and_own_close_reopens_nothing() {
        let root = mount_component(transient_fixture);
        wait_until("workbench utilities mount", || {
            by_role_name(&root, "button", "Objects").is_some()
        })
        .await;
        let document = web_sys::window().unwrap().document().unwrap();
        let ping_count = || {
            root.query_selector("output[aria-label='Ping count']")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap_or_default()
        };
        assert!(
            by_role_name(&root, "menu", "Transient task").is_some(),
            "transient owner starts visible"
        );
        assert!(
            root.query_selector("#workbench-objects[hidden][inert]")
                .unwrap()
                .is_some()
        );
        assert!(
            root.query_selector("#workbench-inspector[hidden][inert]")
                .unwrap()
                .is_some()
        );
        by_role_name(&root, "button", "Refuse transient hide")
            .unwrap()
            .click();
        wait_until("refusal arms", || {
            by_role_name(&root, "button", "Refuse transient hide").is_some_and(|button| {
                button.get_attribute("aria-pressed").as_deref() == Some("true")
            })
        })
        .await;
        by_role_name(&root, "button", "Inspect").unwrap().click();
        let before = ping_count();
        by_role_name(&root, "button", "Ping transient fixture")
            .unwrap()
            .click();
        wait_until("fixture settles a render pass", || ping_count() != before).await;
        assert!(
            by_role_name(&root, "menu", "Transient task").is_some(),
            "refused hide keeps the owner visible"
        );
        assert!(
            root.query_selector("#workbench-inspector[hidden][inert]")
                .unwrap()
                .is_some(),
            "refused hide opens no panel"
        );
        assert_eq!(
            by_role_name(&root, "button", "Inspect")
                .unwrap()
                .get_attribute("aria-expanded")
                .as_deref(),
            Some("false")
        );
        by_role_name(&root, "button", "Allow transient hide")
            .unwrap()
            .click();
        by_role_name(&root, "button", "Objects").unwrap().click();
        wait_until("Objects opens after the owner hides", || {
            by_role_name(&root, "button", "Objects").is_some_and(|button| {
                button.get_attribute("aria-expanded").as_deref() == Some("true")
            })
        })
        .await;
        assert!(
            by_role_name(&root, "menu", "Transient task").is_none(),
            "allowed hide parks the owner"
        );
        by_role_name(&root, "button", "Inspect").unwrap().click();
        wait_until("Inspect replaces Objects", || {
            by_role_name(&root, "button", "Inspect").is_some_and(|button| {
                button.get_attribute("aria-expanded").as_deref() == Some("true")
            })
        })
        .await;
        by_role_name(&root, "button", "Close Inspector")
            .unwrap()
            .click();
        wait_until("Inspector closes", || {
            by_role_name(&root, "button", "Inspect").is_some_and(|button| {
                button.get_attribute("aria-expanded").as_deref() == Some("false")
            })
        })
        .await;
        assert!(
            root.query_selector("#workbench-objects[hidden][inert]")
                .unwrap()
                .is_some(),
            "own close reopens no previous panel"
        );
        let inspect = by_role_name(&root, "button", "Inspect").unwrap();
        assert_eq!(
            document.active_element().as_ref(),
            Some(inspect.as_ref()),
            "own close returns focus to its utility"
        );
        by_role_name(&root, "button", "Objects").unwrap().click();
        wait_until("Objects reopens", || {
            by_role_name(&root, "button", "Objects").is_some_and(|button| {
                button.get_attribute("aria-expanded").as_deref() == Some("true")
            })
        })
        .await;
        by_role_name(&root, "button", "Open nested picker")
            .unwrap()
            .click();
        wait_until("nested picker opens", || {
            by_role_name(&root, "listbox", "Nested picker").is_some()
        })
        .await;
        let picker = by_role_name(&root, "listbox", "Nested picker").unwrap();
        picker.focus().unwrap();
        let escape = web_sys::KeyboardEventInit::new();
        escape.set_key("Escape");
        escape.set_bubbles(true);
        escape.set_cancelable(true);
        picker
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape)
                    .unwrap(),
            )
            .unwrap();
        wait_until("picker consumes its Escape", || {
            by_role_name(&root, "listbox", "Nested picker").is_none()
        })
        .await;
        assert!(
            by_role_name(&root, "button", "Objects")
                .is_some_and(
                    |button| button.get_attribute("aria-expanded").as_deref() == Some("true")
                ),
            "the handled Escape preserves the parent panel"
        );
        let search = root
            .query_selector("#workbench-objects input[type=search]")
            .unwrap()
            .unwrap();
        search
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape)
                    .unwrap(),
            )
            .unwrap();
        wait_until("panel closes on its own Escape", || {
            by_role_name(&root, "button", "Objects").is_some_and(|button| {
                button.get_attribute("aria-expanded").as_deref() == Some("false")
            })
        })
        .await;
        root.remove();
    }

    fn case_canvas_fixture() -> Element {
        super::super::use_empty_test_instance_selection();
        let runtime = use_context::<Rc<super::super::Runtime>>();
        let model = runtime.model();
        let scope = runtime.scope();
        let panel = use_signal(|| None::<ToolPanel>);
        let version = use_signal(|| 0u64);
        use_context_provider(|| version);
        let state = super::super::case_workspace::use_case_workspace_state();
        let case_selection = state.selection();
        let selected_context = use_signal(|| None::<super::super::objects::ScopedTreeContext>);
        let expanded = state.tree_expanded();
        let instance_selection = use_context::<super::super::InstanceSelection>();
        let generation = use_signal(|| 1u64);
        let physical_setup = super::super::pcb_physical_setup::use_controller(
            runtime.clone(),
            version,
            generation,
            Rc::new(|| false),
            instance_selection,
            Rc::new(|_, _| true),
            Rc::new(|_, _| {
                Box::pin(async { panic!("canvas invariance test submits no physical edits") })
            }),
        );
        let objects =
            super::super::case_workspace::objects(super::super::case_workspace::ObjectsInput {
                model: &model,
                scope: scope.clone(),
                instance_scope_pending: false,
                scene: None,
                selected_context,
                selected_body_id: None,
                selected_layer_id: String::new(),
                case_selection,
                expanded,
                on_action: EventHandler::new(|_| {}),
                on_select: EventHandler::new(|_| {}),
                on_display: EventHandler::new(|_| {}),
            });
        let inspector =
            super::super::case_workspace::inspector(super::super::case_workspace::InspectorInput {
                physical_setup,
                mechanical_settings: super::super::MechanicalSettingsMount {
                    props: None,
                    generation_ready: false,
                },
                instance_scope_pending: false,
                scope,
                scene: None,
                case_selection,
                selected_layer_id: String::new(),
                selected_context: None,
                selected_part_summary: None,
                on_show_configured_board: EventHandler::new(|_| {}),
                on_display: EventHandler::new(|_| {}),
            });
        rsx! {
            style { {include_str!("../../assets/workbench-base.css")} }
            style { {include_str!("../../assets/tailwind.css")} }
            style { {include_str!("../../assets/shell.css")} }
            PanelHost { panel,
                objects: objects,
                inspector: inspector,
                Toolbar { aria_label: "Case tools",
                    PanelUtilities { panel, on_hide_transient: move |_| true, first_index: 0usize }
                }
                div {
                    id: "case-canvas-standin",
                    role: "img",
                    aria_label: "Case canvas stand-in",
                    "data-camera": "azimuth-0:elevation-0",
                    style: "width:640px;height:480px",
                }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn case_content_shapes_keep_canvas_rect_and_camera_across_open_switch_close() {
        use crate::runtime::{in_process_support, project_name_test_support as support};
        let runtime = support::new_runtime();
        support::open_document(&runtime, in_process_support::board_document()).await;
        let revision = runtime.model().accepted.as_ref().unwrap().document.revision;
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(case_canvas_fixture);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("the Case canvas and its panel utilities mount", || {
            root.query_selector("#case-canvas-standin")
                .unwrap()
                .is_some()
                && by_role_name(&root, "button", "Objects").is_some()
                && by_role_name(&root, "button", "Inspect").is_some()
        })
        .await;
        let canvas = root
            .query_selector("#case-canvas-standin")
            .unwrap()
            .unwrap();
        let rect = canvas.get_bounding_client_rect();
        let before = (rect.x(), rect.y(), rect.width(), rect.height());
        assert!(
            before.2 > 0.0 && before.3 > 0.0,
            "the stand-in canvas has bounds"
        );
        let camera = canvas.get_attribute("data-camera");
        let assert_canvas_unchanged = |root: &web_sys::Element| {
            let current = root
                .query_selector("#case-canvas-standin")
                .unwrap()
                .unwrap();
            let rect = current.get_bounding_client_rect();
            assert!(
                canvas.is_same_node(Some(current.as_ref())),
                "panel visibility must not remount the canvas"
            );
            assert_eq!((rect.x(), rect.y(), rect.width(), rect.height()), before);
            assert_eq!(current.get_attribute("data-camera"), camera);
            assert_eq!(
                runtime.model().accepted.as_ref().unwrap().document.revision,
                revision
            );
        };
        by_role_name(&root, "button", "Objects").unwrap().click();
        wait_until("actual Case Objects content opens", || {
            root.query_selector("#workbench-objects:not([hidden]) [aria-label='Current selection']")
                .unwrap()
                .is_some()
                && root
                    .query_selector("#workbench-objects:not([hidden]) button[aria-expanded]")
                    .unwrap()
                    .is_some()
        })
        .await;
        assert_canvas_unchanged(&root);
        by_role_name(&root, "button", "Inspect").unwrap().click();
        wait_until("Inspect replaces Objects", || {
            root.query_selector("#workbench-inspector:not([hidden]) [aria-label='Case Inspector']")
                .unwrap()
                .is_some()
                && root
                    .query_selector(
                        "#workbench-inspector:not([hidden]) [aria-label='Physical assembly']",
                    )
                    .unwrap()
                    .is_some()
                && root
                    .query_selector("#workbench-objects[hidden][inert]")
                    .unwrap()
                    .is_some()
        })
        .await;
        assert_canvas_unchanged(&root);
        by_role_name(&root, "button", "Inspect").unwrap().click();
        wait_until("both Case panels close", || {
            root.query_selector("#workbench-inspector[hidden][inert]")
                .unwrap()
                .is_some()
        })
        .await;
        assert_canvas_unchanged(&root);
        root.remove();
    }
}
