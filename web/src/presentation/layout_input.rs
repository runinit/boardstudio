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
        let typing = event_target(&key).is_some_and(|element| {
            element
                .closest(
                    "input, textarea, select, [contenteditable]:not([contenteditable='false'])",
                )
                .ok()
                .flatten()
                .is_some()
        });
        let code = CanvasKey::new(&key.key(), &key.code());
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

    pub fn pointerdown(mut self, event: PointerEvent) {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        self.interaction.sync_control(pointer.ctrl_key());
        let target = event_target(&pointer);
        if self.menu.peek().is_some()
            && !target.as_ref().is_some_and(|target| {
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
        if !target.as_ref().is_some_and(|target| {
            target
                .closest("button, a, input, textarea, select, [tabindex], [contenteditable]")
                .ok()
                .flatten()
                .is_some()
        }) {
            focus_workbench_if_body();
        }
    }
}

pub(super) fn focus_workbench_if_body() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
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
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use wasm_bindgen_test::*;

    #[component]
    fn SpaceHost() -> Element {
        let input = use_layout_input();
        rsx! {
            main { id: "m1-workbench", tabindex: "-1",
                onkeydown: move |event| input.keydown(&event, true, true),
                onkeyup: move |event| input.keyup(event),
                input { id: "typing-control" }
                button { id: "layout-control", "Layout" }
                button { id: "canvas-control", onkeydown: move |event| { event.stop_propagation(); input.keydown(&event, false, true); }, "Canvas" }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn space_pan_prevents_scroll_on_layout_controls_but_allows_typing() {
        let root = test_mount_helpers::mount(VirtualDom::new(SpaceHost), None);
        wait_until("workbench keyboard target", || {
            root.query_selector("#layout-control")
                .ok()
                .flatten()
                .is_some()
        })
        .await;
        let init = web_sys::KeyboardEventInit::new();
        init.set_key(" ");
        init.set_code("Space");
        init.set_bubbles(true);
        init.set_cancelable(true);
        let space =
            || web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
        for (target, prevents_scroll) in [
            ("#typing-control", false),
            ("#layout-control", true),
            ("#canvas-control", true),
        ] {
            let key = space();
            root.query_selector(target)
                .unwrap()
                .unwrap()
                .dispatch_event(&key)
                .unwrap();
            assert_eq!(key.default_prevented(), prevents_scroll);
        }
    }
}
