//! Workbench keyboard and focus routing for the mounted Layout SVG.

use super::*;

#[derive(Clone, Copy)]
pub(super) struct LayoutInput {
    pub keyboard: Signal<Option<EventHandler<KeyboardEvent>>>,
    pub cancel_placement: Signal<Option<EventHandler<CanvasMode>>>,
    pub relation_escape: Signal<Option<EventHandler<()>>>,
    pub menu: Signal<Option<objects::LayoutCommandMenu>>,
    interaction: LayoutInteraction,
}

pub(super) fn use_layout_input() -> LayoutInput {
    let interaction = boardstudio_web_layout::canvas_interaction::use_layout_interaction();
    use_context_provider(|| interaction);
    let input = LayoutInput {
        keyboard: use_signal(|| None),
        cancel_placement: use_signal(|| None),
        relation_escape: use_signal(|| None),
        menu: use_signal(|| None),
        interaction,
    };
    use_context_provider(|| input)
}

impl LayoutInput {
    pub fn dispatch(self, event: CanvasEvent) -> Vec<InteractionEffect> {
        let effects = self.interaction.handle(event);
        for effect in &effects {
            if let InteractionEffect::Cancel(
                mode @ (CanvasMode::PlacingPart
                | CanvasMode::PlacingMatrix
                | CanvasMode::PlacingMirroredPair),
            ) = effect
                && let Some(cancel) = *self.cancel_placement.peek()
            {
                cancel.call(*mode);
            }
        }
        effects
    }

    pub fn keydown(self, event: &KeyboardEvent, layout: bool, canvas: bool) {
        let Some(key) = event.data().try_as_web_event() else {
            return;
        };
        let target = event_target(&key);
        let typing = target.as_ref().is_some_and(|element| {
            element
                .closest(
                    "input, textarea, select, [contenteditable]:not([contenteditable='false'])",
                )
                .ok()
                .flatten()
                .is_some()
        });
        let code = CanvasKey::new(&key.key(), &key.code());
        let canvas = layout
            || (canvas
                && (code == CanvasKey::Escape
                    || target.as_ref().is_some_and(|element| {
                        element
                            .closest("svg[role='group']")
                            .ok()
                            .flatten()
                            .is_some()
                    })));
        let prevented = key.default_prevented() || !event.default_action_enabled();
        if self
            .dispatch(CanvasEvent::KeyDown {
                key: code,
                layout,
                canvas,
                typing,
                prevented,
            })
            .contains(&InteractionEffect::PreventDefault)
        {
            event.prevent_default();
        } else if layout && !typing && !prevented {
            if code == CanvasKey::Escape
                && let Some(escape) = *self.relation_escape.peek()
            {
                escape.call(());
                event.prevent_default();
            } else if let Some(keyboard) = *self.keyboard.peek() {
                keyboard.call(event.clone());
            }
        }
    }

    pub fn keyup(self, event: KeyboardEvent) {
        if let Some(key) = event.data().try_as_web_event() {
            self.dispatch(CanvasEvent::KeyUp(CanvasKey::new(&key.key(), &key.code())));
        }
    }

    pub fn focusout(self, event: FocusEvent) {
        let Some(focus) = event.data().try_as_web_event() else {
            return;
        };
        let remains_inside = event_target(&focus)
            .and_then(|target| target.closest("#m1-workbench").ok().flatten())
            .zip(
                focus
                    .related_target()
                    .and_then(|target| target.dyn_into::<web_sys::Node>().ok()),
            )
            .is_some_and(|(root, related)| root.contains(Some(&related)));
        if !remains_inside {
            self.dispatch(CanvasEvent::Blur);
        }
    }

    /// Pointer motion and release anywhere in the workbench keep `control_down`
    /// honest when the keyup was missed (focus on `<body>` or the pointer outside
    /// the SVG). `sync_control` peeks, so normal motion neither subscribes nor
    /// re-renders; only an actual modifier change publishes.
    pub fn pointermove(self, event: PointerEvent) {
        if let Some(pointer) = event.data().try_as_web_event() {
            self.interaction.sync_control(pointer.ctrl_key());
        }
    }

    pub fn pointerup(self, event: PointerEvent) {
        if let Some(pointer) = event.data().try_as_web_event() {
            self.interaction.sync_control(pointer.ctrl_key());
        }
    }

    pub fn pointerdown(mut self, event: PointerEvent) {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        self.interaction.sync_control(pointer.ctrl_key());
        if pointer.button() != 0 {
            return;
        }
        if self.menu.peek().is_some()
            && !event_target(&pointer).is_some_and(|target| {
                target
                    .closest(
                        "#m1-layout-command-pill, [data-slot='collapsible']:has(#m1-layout-snap-trigger)",
                    )
                    .ok()
                    .flatten()
                    .is_some()
            })
        {
            self.menu.set(None);
        }
    }
}

pub(super) fn focus_workbench_if_body() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    // An open modal owns focus even when the click inside it left `<body>` active;
    // recovering the workbench then would strand keyboard input behind the dialog.
    if document
        .query_selector(
            "[data-state='open'] [aria-modal='true'], [aria-modal='true'][data-state='open'], dialog[open]",
        )
        .ok()
        .flatten()
        .is_some()
    {
        return;
    }
    if document
        .active_element()
        .is_some_and(|element| element.tag_name() == "BODY")
        && let Some(main) = document
            .get_element_by_id("m1-workbench")
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _ = main.focus_with_options(&options);
    }
}

fn event_target(event: &web_sys::Event) -> Option<web_sys::Element> {
    event.target()?.dyn_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_web_ui_shared::{
        test_wait::wait_until,
        ui::dialog::{Dialog, DialogTitle},
    };
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    async fn space_pan_prevents_scroll_on_layout_controls_but_allows_typing() {
        let root = test_mount_helpers::mount(VirtualDom::new(CanvasHost), None);
        expect_state(&root, "None|false|false").await;
        for (target, prevents_scroll) in [
            ("#typing-control", false),
            ("#arm-pan", true),
            ("#canvas-panel-key", false),
            ("#canvas-key", true),
        ] {
            assert_eq!(keydown(&root, target, " ", "Space"), prevents_scroll);
        }
    }

    #[component]
    fn CanvasHost() -> Element {
        let input = use_layout_input();
        let interaction = use_context::<super::super::LayoutInteraction>();
        let mut refresh = use_signal(|| 0u64);
        let mut modal_open = use_signal(|| false);
        let _ = (refresh(), interaction.version());
        let state = format!(
            "{:?}|{}|{}",
            interaction.current(),
            interaction.space_down(),
            interaction.control_down()
        );
        let mut arm = move |mode: CanvasMode| {
            interaction.handle(CanvasEvent::PointerDown {
                mode,
                id: 7,
                client: (0.0, 0.0),
                control: false,
            });
            refresh += 1;
        };
        rsx! {
            main { id: "m1-workbench", tabindex: "-1",
                onkeydown: move |event| input.keydown(&event, true, true),
                onkeyup: move |event| input.keyup(event),
                onpointerdown: move |event| input.pointerdown(event),
                input { id: "typing-control" }
                div { onkeydown: move |event| { event.stop_propagation(); input.keydown(&event, false, true); },
                    svg { id: "canvas-key", role: "group", tabindex: "0" }
                    button { id: "canvas-panel-key", "Panel" }
                }
                button { id: "plain-key", onkeydown: move |event| { event.stop_propagation(); input.keydown(&event, false, false); }, "Plain" }
                button { id: "arm-pan", onclick: move |_| arm(CanvasMode::Panning), "Arm pan" }
                button { id: "arm-drag", onclick: move |_| arm(CanvasMode::DraggingParts), "Arm drag" }
                button { id: "set-control", onclick: move |_| { interaction.sync_control(true); refresh += 1; }, "Set control" }
                button { id: "open-modal", onclick: move |_| modal_open.set(true), "Modal" }
                button { id: "refresh", onclick: move |_| refresh += 1, "Refresh" }
                div { id: "pointer-sync", "Sync target" }
                output { id: "interaction-state", "{state}" }
                Dialog { open: Some(modal_open()), on_open_change: move |next| modal_open.set(next), DialogTitle { "Modal" } }
            }
        }
    }

    fn canvas_state(root: &test_mount_helpers::MountedRoot) -> Option<String> {
        root.query_selector("#interaction-state")
            .ok()
            .flatten()
            .and_then(|state| state.text_content())
    }

    async fn expect_state(root: &test_mount_helpers::MountedRoot, expected: &'static str) {
        wait_until("canvas interaction state", || {
            canvas_state(root).as_deref() == Some(expected)
        })
        .await;
        assert_eq!(canvas_state(root).as_deref(), Some(expected));
    }

    fn click(root: &test_mount_helpers::MountedRoot, selector: &str) {
        root.query_selector(selector)
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
    }

    fn keydown(
        root: &test_mount_helpers::MountedRoot,
        selector: &str,
        key: &str,
        code: &str,
    ) -> bool {
        let init = web_sys::KeyboardEventInit::new();
        init.set_key(key);
        init.set_code(code);
        init.set_bubbles(true);
        init.set_cancelable(true);
        let event =
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
        root.query_selector(selector)
            .unwrap()
            .unwrap()
            .dispatch_event(&event)
            .unwrap();
        event.default_prevented()
    }

    #[wasm_bindgen_test]
    async fn non_layout_canvas_admits_space_pan_and_escape_without_control() {
        let root = test_mount_helpers::mount(VirtualDom::new(CanvasHost), None);
        expect_state(&root, "None|false|false").await;
        assert!(!keydown(&root, "#canvas-panel-key", " ", "Space"));
        assert!(keydown(&root, "#canvas-key", " ", "Space"));
        click(&root, "#refresh");
        expect_state(&root, "None|true|false").await;
        click(&root, "#arm-pan");
        expect_state(&root, "Some(Panning)|true|false").await;
        assert!(!keydown(&root, "#canvas-key", "Control", "ControlLeft"));
        click(&root, "#refresh");
        expect_state(&root, "Some(Panning)|true|false").await;
        assert!(keydown(&root, "#canvas-panel-key", "Escape", "Escape"));
        expect_state(&root, "None|true|false").await;
        click(&root, "#arm-drag");
        expect_state(&root, "Some(DraggingParts)|true|false").await;
        assert!(keydown(&root, "#canvas-key", "Escape", "Escape"));
        expect_state(&root, "None|true|false").await;
    }

    #[wasm_bindgen_test]
    async fn non_canvas_ignores_space_and_pointerdown_clears_stale_control() {
        let root = test_mount_helpers::mount(VirtualDom::new(CanvasHost), None);
        expect_state(&root, "None|false|false").await;
        assert!(!keydown(&root, "#plain-key", " ", "Space"));
        click(&root, "#arm-pan");
        expect_state(&root, "None|false|false").await;
        click(&root, "#set-control");
        expect_state(&root, "None|false|true").await;
        let pointer = js_sys::eval(
            "new PointerEvent('pointerdown', {bubbles: true, button: 0, ctrlKey: false})",
        )
        .unwrap()
        .dyn_into::<web_sys::Event>()
        .unwrap();
        root.query_selector("#pointer-sync")
            .unwrap()
            .unwrap()
            .dispatch_event(&pointer)
            .unwrap();
        expect_state(&root, "None|false|false").await;
        let document = web_sys::window().unwrap().document().unwrap();
        let blur = || {
            if let Some(el) = document
                .active_element()
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = el.blur();
            }
            assert_eq!(document.active_element().unwrap().tag_name(), "BODY");
        };
        blur();
        focus_workbench_if_body();
        assert_eq!(document.active_element().unwrap().id(), "m1-workbench");
        click(&root, "#open-modal");
        wait_until("open dialog", || {
            document
                .query_selector("[data-state='open'] [aria-modal='true']")
                .ok()
                .flatten()
                .is_some()
        })
        .await;
        blur();
        focus_workbench_if_body();
        assert_eq!(document.active_element().unwrap().tag_name(), "BODY");
    }
}
