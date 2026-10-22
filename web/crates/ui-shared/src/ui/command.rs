//! Local command-palette wiring, not an upstream component copy.
//! Owners supply the same callbacks as their controls; no document or tool state lives here.
use dioxus::prelude::*;
use std::{collections::BTreeMap, rc::Rc};

#[derive(Clone, PartialEq)]
pub struct Command {
    pub group: &'static str,
    pub label: String,
    pub reason: Option<String>,
    pub hint: Option<&'static str>,
    pub on_select: EventHandler<()>,
}

#[derive(Clone, Copy)]
pub struct CommandRegistry(Signal<BTreeMap<ScopeId, (Rc<()>, Vec<Command>)>>);

pub fn use_command_registry() -> CommandRegistry {
    let registry = CommandRegistry(use_signal(BTreeMap::new));
    use_context_provider(|| registry);
    registry
}

impl CommandRegistry {
    /// Each returned callback admits only the command rendered by this live owner.
    pub fn commands(self) -> Vec<Command> {
        self.0
            .read()
            .iter()
            .flat_map(|(owner, (identity, commands))| {
                commands
                    .iter()
                    .enumerate()
                    .map(|(index, command)| {
                        let owner = *owner;
                        let identity = identity.clone();
                        let rendered = command.clone();
                        Command {
                            on_select: EventHandler::new(move |()| {
                                let callback =
                                    self.0.peek().get(&owner).and_then(|(current, commands)| {
                                        (Rc::ptr_eq(current, &identity)
                                            && commands.get(index) == Some(&rendered)
                                            && rendered.reason.is_none())
                                        .then_some(rendered.on_select)
                                    });
                                if let Some(callback) = callback {
                                    callback.call(());
                                }
                            }),
                            ..command.clone()
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

/// One registration per component scope. Isolated control fixtures need no palette provider.
/// A second hook in the same scope is rejected instead of overwriting sibling commands.
pub fn use_commands(commands: Vec<Command>) {
    let registry = try_use_context::<CommandRegistry>();
    let owner = dioxus::core::current_scope_id();
    let identity = use_hook(|| Rc::new(()));
    use_effect(use_reactive((&commands,), {
        let identity = identity.clone();
        move |(commands,)| {
            let Some(mut registry) = registry else {
                return;
            };
            if let Some((registered, previous)) = registry.0.peek().get(&owner) {
                assert!(
                    Rc::ptr_eq(registered, &identity),
                    "use_commands must be called once per component scope"
                );
                if previous == &commands {
                    return;
                }
            }
            registry
                .0
                .write()
                .insert(owner, (identity.clone(), commands));
        }
    }));
    use_drop(move || {
        if let Some(mut registry) = registry {
            registry.0.write().remove(&owner);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_role::by_role_name, test_wait::wait_until};
    use std::cell::RefCell;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[component]
    fn Owner(
        mut result: Signal<String>,
        disabled: Signal<bool>,
        reverse: Signal<bool>,
        version: Signal<u32>,
    ) -> Element {
        let generation = version();
        let mut calls = use_signal(|| 0u32);
        let mut commands: Vec<_> = ["Alpha", "Beta"]
            .into_iter()
            .map(|label| Command {
                group: "Edit",
                label: label.into(),
                reason: disabled().then(|| "Unavailable now".into()),
                hint: (generation > 0).then_some("refreshed"),
                on_select: EventHandler::new(move |()| {
                    calls += 1;
                    result.set(format!("{label}:{generation}:{}", calls()));
                }),
            })
            .collect();
        if reverse() {
            commands.reverse();
        }
        use_commands(commands);
        rsx! {}
    }

    #[component]
    fn Fixture() -> Element {
        let registry = use_command_registry();
        let mut mounted = use_signal(|| true);
        let mut disabled = use_signal(|| false);
        let mut reverse = use_signal(|| false);
        let mut version = use_signal(|| 0u32);
        let result = use_signal(|| "untouched".to_owned());
        let saved = use_hook(|| Rc::new(RefCell::new(None::<Command>)));
        let capture = saved.clone();
        let commands = registry.commands();
        let summary = commands
            .iter()
            .map(|c| format!("{}:{}", c.label, c.reason.as_deref().unwrap_or("enabled")))
            .collect::<Vec<_>>()
            .join(",");
        rsx! {
            if mounted() { Owner { result, disabled, reverse, version } }
            button { onclick: move |_| *capture.borrow_mut() = registry.commands().into_iter().find(|c| c.label == "Alpha"), "Capture Alpha" }
            button { onclick: move |_| { if let Some(command) = saved.borrow().as_ref() { command.on_select.call(()); } }, "Invoke captured" }
            button { onclick: move |_| disabled.set(!disabled()), "Toggle unavailable" }
            button { onclick: move |_| reverse.set(!reverse()), "Reverse commands" }
            button { onclick: move |_| version += 1, "Refresh callback" }
            output { role: "status", aria_label: "Callback", "{commands.first().and_then(|c| c.hint).unwrap_or_default()}" }
            button { onclick: move |_| mounted.set(!mounted()), "Toggle owner" }
            button { onclick: move |_| { if let Some(command) = registry.commands().into_iter().find(|c| c.label == "Alpha") { command.on_select.call(()); } }, "Run Alpha" }
            output { role: "status", aria_label: "Registry", "{summary}" }
            output { role: "status", aria_label: "Result", "{result}" }
        }
    }

    fn text(root: &web_sys::Element, name: &str) -> String {
        by_role_name(root, "status", name)
            .unwrap()
            .text_content()
            .unwrap()
    }
    fn click(root: &web_sys::Element, name: &str) {
        by_role_name(root, "button", name).unwrap().click();
    }

    #[wasm_bindgen_test]
    async fn registry_refreshes_callbacks_reasons_and_removes_unmounted_owners() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(Fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("registry fixture mounted", || {
            by_role_name(&root, "status", "Registry").is_some()
        })
        .await;
        wait_until("two live commands", || {
            text(&root, "Registry") == "Alpha:enabled,Beta:enabled"
        })
        .await;
        click(&root, "Run Alpha");
        wait_until("original callback", || text(&root, "Result") == "Alpha:0:1").await;
        click(&root, "Refresh callback");
        wait_until("owner refreshed", || text(&root, "Callback") == "refreshed").await;
        click(&root, "Run Alpha");
        wait_until("refreshed callback", || {
            text(&root, "Result") == "Alpha:1:2"
        })
        .await;
        click(&root, "Capture Alpha");
        click(&root, "Toggle unavailable");
        wait_until("reactive reason", || {
            text(&root, "Registry") == "Alpha:Unavailable now,Beta:Unavailable now"
        })
        .await;
        click(&root, "Invoke captured");
        click(&root, "Run Alpha");
        click(&root, "Toggle owner");
        wait_until("owner removed", || text(&root, "Registry").is_empty()).await;
        assert_eq!(text(&root, "Result"), "Alpha:1:2");
        click(&root, "Invoke captured");
        click(&root, "Toggle owner");
        wait_until("owner remounted without duplicates", || {
            text(&root, "Registry") == "Alpha:Unavailable now,Beta:Unavailable now"
        })
        .await;
        click(&root, "Toggle unavailable");
        wait_until("enabled again", || {
            text(&root, "Registry") == "Alpha:enabled,Beta:enabled"
        })
        .await;
        click(&root, "Invoke captured");
        assert_eq!(text(&root, "Result"), "Alpha:1:2");
        click(&root, "Capture Alpha");
        click(&root, "Reverse commands");
        wait_until("reordered commands", || {
            text(&root, "Registry") == "Beta:enabled,Alpha:enabled"
        })
        .await;
        click(&root, "Invoke captured");
        assert_eq!(text(&root, "Result"), "Alpha:1:2");
        click(&root, "Run Alpha");
        wait_until("current Alpha remains callable", || {
            text(&root, "Result") == "Alpha:1:1"
        })
        .await;
        root.remove();
    }

    #[component]
    fn DisclosureFixture() -> Element {
        let mut open = use_signal(|| false);
        let mut callbacks = use_signal(|| 0u32);
        rsx! {
            button { onclick: move |_| open.set(!open()), "External toggle" }
            crate::controls::BoardAccordionSection { class: "", label: "Controlled section", id: "controlled-section",
                open: Some(open()), on_open_change: move |next| { callbacks += 1; open.set(next); },
                heading: rsx! { "Controlled section" }, "Controlled content"
            }
            crate::controls::BoardAccordionSection { class: "", label: "Uncontrolled section", id: "uncontrolled-section",
                heading: rsx! { "Uncontrolled section" }, "Uncontrolled content"
            }
            output { role: "status", aria_label: "Callbacks", "{callbacks}" }
        }
    }

    #[component]
    fn DisclosureKeyboardFixture(calls: Rc<RefCell<Vec<String>>>) -> Element {
        let mut open = use_signal(|| false);
        let controlled_calls = calls.clone();
        let uncontrolled_calls = calls.clone();
        rsx! {
            div { onkeydown: move |event| calls.borrow_mut().push(format!("ancestor:{}", event.key())),
                crate::controls::BoardAccordionSection {
                    class: "", label: "Controlled keyboard section", id: "controlled-keyboard-section",
                    open: Some(open()), on_open_change: move |next| open.set(next),
                    onkeydown: move |event: KeyboardEvent| controlled_calls.borrow_mut().push(format!("controlled:{}", event.key())),
                    heading: rsx! { "Controlled keyboard section" }, "Controlled content"
                }
                crate::controls::BoardAccordionSection {
                    class: "", label: "Uncontrolled keyboard section", id: "uncontrolled-keyboard-section",
                    onkeydown: move |event: KeyboardEvent| uncontrolled_calls.borrow_mut().push(format!("uncontrolled:{}", event.key())),
                    heading: rsx! { "Uncontrolled keyboard section" }, "Uncontrolled content"
                }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn controlled_space_preserves_caller_once_without_consuming_native_activation() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let calls = Rc::new(RefCell::new(Vec::new()));
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(
                DisclosureKeyboardFixture,
                DisclosureKeyboardFixtureProps {
                    calls: calls.clone(),
                },
            ),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("keyboard disclosure fixture mounted", || {
            by_role_name(&root, "button", "Uncontrolled keyboard section").is_some()
        })
        .await;
        for (name, key, expected) in [
            ("Controlled keyboard section", " ", vec!["controlled: "]),
            (
                "Controlled keyboard section",
                "Enter",
                vec!["controlled:Enter", "ancestor:Enter"],
            ),
            (
                "Uncontrolled keyboard section",
                " ",
                vec!["uncontrolled: ", "ancestor: "],
            ),
            (
                "Uncontrolled keyboard section",
                "Enter",
                vec!["uncontrolled:Enter", "ancestor:Enter"],
            ),
        ] {
            calls.borrow_mut().clear();
            let init = web_sys::KeyboardEventInit::new();
            init.set_key(key);
            init.set_bubbles(true);
            init.set_cancelable(true);
            let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                .unwrap();
            by_role_name(&root, "button", name)
                .unwrap()
                .dispatch_event(&event)
                .unwrap();
            assert_eq!(
                *calls.borrow(),
                expected,
                "{name} {key:?}: caller once, controlled Space stops before ancestor"
            );
            if name == "Controlled keyboard section" {
                assert!(!event.default_prevented(), "preserve native key activation");
            }
        }
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn controlled_disclosure_and_existing_uncontrolled_accordion_keep_their_owners() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(DisclosureFixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("both disclosure triggers", || {
            by_role_name(&root, "button", "Uncontrolled section").is_some()
        })
        .await;
        let controlled = by_role_name(&root, "button", "Controlled section").unwrap();
        let uncontrolled = by_role_name(&root, "button", "Uncontrolled section").unwrap();
        assert_eq!(controlled.id(), "controlled-section");
        assert_eq!(
            controlled.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        click(&root, "External toggle");
        wait_until("external open", || {
            controlled.get_attribute("aria-expanded").as_deref() == Some("true")
        })
        .await;
        let content_id = controlled.get_attribute("aria-controls").unwrap();
        assert!(document.get_element_by_id(&content_id).is_some());
        assert_eq!(text(&root, "Callbacks"), "0");
        controlled.click();
        wait_until("native close callback once", || {
            text(&root, "Callbacks") == "1"
        })
        .await;
        assert_eq!(
            controlled.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        controlled.click();
        wait_until("native open callback once", || {
            text(&root, "Callbacks") == "2"
        })
        .await;
        click(&root, "External toggle");
        wait_until("external close", || {
            controlled.get_attribute("aria-expanded").as_deref() == Some("false")
        })
        .await;
        assert_eq!(text(&root, "Callbacks"), "2");
        uncontrolled.click();
        wait_until("uncontrolled still opens", || {
            uncontrolled.get_attribute("aria-expanded").as_deref() == Some("true")
        })
        .await;
        uncontrolled.click();
        wait_until("uncontrolled still closes", || {
            uncontrolled.get_attribute("aria-expanded").as_deref() == Some("false")
        })
        .await;
        root.remove();
    }
}
