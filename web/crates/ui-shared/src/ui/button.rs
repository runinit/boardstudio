//! Copied from rust-ui: `app_crates/registry/src/ui/button.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Edits to the upstream file:
//! - the router `Link` branch is a plain `a` with `href`;
//! - `data-slot="button"` on both elements: the native-control defaults in
//!   `workbench-base.css` skip slotted components, and `tailwind.css` gives them a minimal
//!   reset instead (the app has no Tailwind preflight).
//! - heights read the shared control sizes (`h-9` → `h-control`, `h-8` → `h-control-compact`,
//!   `h-10` → `h-control-touch`, `size-9`/`size-8` likewise; theme contract section 5).
//! - the `Outline` variant's border, including `dark:border-input`, is removed: cards style (theme
//!   contract section 8); focus rings kept.
//! - optional `aria-current` and `aria-pressed` forward caller-owned header/navigation state.

use dioxus::{core::AttributeValue, prelude::*};
use tw_merge::tw_merge;

#[derive(Default, Clone, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Default,
    Destructive,
    Warning,
    Success,
    Outline,
    Secondary,
    Ghost,
    Link,
}

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Eq)]
pub enum ButtonSize {
    #[default]
    Default,
    Sm,
    Lg,
    Icon,
    IconSm,
    IconXs,
}

impl ButtonVariant {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "bg-primary text-primary-foreground shadow-xs hover:bg-primary/90",
            Self::Destructive => {
                "bg-destructive text-white shadow-xs hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 dark:bg-destructive/60"
            }
            Self::Warning => "bg-warning text-warning-foreground hover:bg-warning/90",
            Self::Success => "bg-success text-success-foreground hover:bg-success/90",
            Self::Outline => {
                "bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:bg-input/30 dark:hover:bg-input/5"
            }
            Self::Secondary => {
                "bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80"
            }
            Self::Ghost => "hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
            Self::Link => "text-primary underline-offset-4 hover:underline",
        }
    }
}

impl ButtonSize {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "h-control px-4 py-2 has-[>svg]:px-3",
            Self::Sm => "h-control-compact rounded-md gap-1.5 px-3 has-[>svg]:px-2.5",
            Self::Lg => "h-control-touch rounded-md px-6 has-[>svg]:px-4",
            Self::Icon => "size-control",
            Self::IconSm => "size-control-compact rounded-md",
            Self::IconXs => "size-6 rounded-md",
        }
    }
}

// Compose platform listeners before their typed adapters, matching the public Dioxus event API.
fn button_attributes(
    mut attributes: Vec<Attribute>,
    owners: Vec<Attribute>,
    id: Option<&str>,
    description: Option<&str>,
) -> (Option<String>, Vec<Attribute>) {
    let id = id.map(str::to_owned).or_else(|| {
        attributes
            .iter()
            .rev()
            .find(|attribute| attribute.namespace.is_none() && attribute.name == "id")
            .and_then(|attribute| match &attribute.value {
                AttributeValue::Text(value) => Some(value.clone()),
                _ => None,
            })
    });
    let mut descriptions = Vec::new();
    for value in description
        .into_iter()
        .chain(attributes.iter().filter_map(|attribute| {
            if attribute.namespace.is_none()
                && attribute.name == "aria-describedby"
                && let AttributeValue::Text(value) = &attribute.value
            {
                Some(value.as_str())
            } else {
                None
            }
        }))
    {
        for token in value.split_ascii_whitespace() {
            if !descriptions.iter().any(|existing| existing == token) {
                descriptions.push(token.to_owned());
            }
        }
    }
    attributes.retain(|attribute| {
        if attribute.namespace == Some("style") {
            return false;
        }
        attribute.namespace.is_some()
            || !matches!(
                attribute.name,
                "class"
                    | "style"
                    | "data-slot"
                    | "type"
                    | "href"
                    | "disabled"
                    | "aria-label"
                    | "aria-describedby"
                    | "aria-current"
                    | "aria-pressed"
                    | "aria-expanded"
                    | "aria-controls"
                    | "aria-haspopup"
                    | "title"
                    | "id"
            )
    });
    if !descriptions.is_empty() {
        attributes.push(Attribute::new(
            "aria-describedby",
            descriptions.join(" "),
            None,
            false,
        ));
    }
    for mut owner in owners {
        if let Some(attribute) = attributes.iter_mut().find(|attribute| {
            attribute.name == owner.name && attribute.namespace == owner.namespace
        }) {
            if let (AttributeValue::Listener(trigger), AttributeValue::Listener(handler)) =
                (&attribute.value, &owner.value)
            {
                let trigger = trigger.clone();
                let handler = handler.clone();
                owner.value = AttributeValue::listener(
                    move |event: Event<dioxus::events::PlatformEventData>| {
                        trigger.call(event.clone().into_any());
                        handler.call(event.into_any());
                    },
                );
            }
            *attribute = owner;
        } else {
            attributes.push(owner);
        }
    }
    // Dioxus diffs each completed spread as a name-sorted list.
    attributes.sort_by_key(|attribute| attribute.name);
    (id, attributes)
}

#[cfg(test)]
mod attribute_tests {
    use super::*;

    #[test]
    fn removing_focus_preserves_the_native_blur_listener_mutation() {
        use std::{cell::RefCell, rc::Rc};
        fn fixture(state: Rc<RefCell<Option<Signal<bool>>>>) -> Element {
            let removed = use_signal(|| false);
            *state.borrow_mut() = Some(removed);
            rsx! {
                for href in [None, Some("#mutations")] {
                    Button { href: href.map(str::to_owned), class: "font-mono",
                        onfocus: (!removed()).then(|| EventHandler::new(|_| {})), onblur: |_| {},
                        attributes: {
                            let mut attributes = Vec::new();
                            attributes.push(Attribute::new("class", "collision", None, false));
                            if !removed() { attributes.push(Attribute::new("data-removed", "remove", None, false)); }
                            attributes.push(Attribute::new("data-retained", "kept", None, false));
                            if !removed() { attributes.push(dioxus::events::onfocus(|_| {})); }
                            attributes.push(dioxus::events::onblur(|_| {}));
                            attributes
                        }, "Native mutations"
                    }
                }
            }
        }
        let state = Rc::new(RefCell::new(None));
        let mut dom = VirtualDom::new_with_props(fixture, state.clone());
        dom.rebuild_to_vec();
        state.borrow_mut().as_mut().unwrap().set(false);
        dom.render_immediate_to_vec();
        state.borrow_mut().as_mut().unwrap().set(true);
        let edits = dom.render_immediate_to_vec().edits;
        assert_eq!(
            edits
                .iter()
                .filter(|edit| matches!(
                    edit,
                    dioxus::core::Mutation::RemoveEventListener { name, .. } if name == "focus"
                ))
                .count(),
            2,
            "{edits:?}"
        );
        assert!(
            !edits.iter().any(|edit| matches!(
                edit,
                dioxus::core::Mutation::RemoveEventListener { name, .. } if name == "blur"
            )),
            "{edits:?}"
        );
    }

    #[test]
    fn button_owns_scalars_and_styles_but_keeps_id_fallback_and_unknown_attributes() {
        let canonical = [
            "class",
            "style",
            "data-slot",
            "type",
            "href",
            "disabled",
            "aria-label",
            "aria-current",
            "aria-pressed",
            "aria-expanded",
            "aria-controls",
            "aria-haspopup",
            "title",
        ];
        let mut raw: Vec<_> = canonical
            .iter()
            .map(|name| Attribute::new(name, "collision", None, false))
            .collect();
        raw.extend([
            Attribute::new("color", "red", Some("style"), false),
            Attribute::new("id", "trigger-id", None, false),
            Attribute::new("aria-describedby", "tip owner tip", None, false),
            Attribute::new("data-extra", "kept", None, false),
            Attribute::new("tabindex", "0", None, false),
            Attribute::new("custom", "kept", Some("example"), false),
        ]);
        let (id, merged) = button_attributes(raw.clone(), vec![], None, Some("owner shared owner"));
        assert_eq!(id.as_deref(), Some("trigger-id"));
        assert!(
            merged
                .iter()
                .all(|attribute| !canonical.contains(&attribute.name)
                    && attribute.namespace != Some("style"))
        );
        for (name, value) in [
            ("aria-describedby", "owner shared tip"),
            ("data-extra", "kept"),
            ("tabindex", "0"),
            ("custom", "kept"),
        ] {
            assert_eq!(
                merged
                    .iter()
                    .find(|attribute| attribute.name == name)
                    .unwrap()
                    .value,
                AttributeValue::Text(value.into())
            );
        }
        assert!(!merged.iter().any(|attribute| attribute.name == "id"));
        let (id, attributes) = button_attributes(raw, vec![], Some("owner-id"), None);
        assert_eq!(id.as_deref(), Some("owner-id"));
        assert!(!attributes.iter().any(|attribute| attribute.name == "id"));
        let (id, attributes) = button_attributes(vec![], vec![], None, None);
        assert!(id.is_none() && attributes.is_empty());
    }
}

/// Explicit attributes support public trigger composition. Button owns its structural/style and
/// typed scalar attributes; id falls back to the vector, descriptions combine tokens, and raw
/// listeners run before typed owner listeners. Other attributes pass through on both branches.
#[component]
pub fn Button(
    #[props(into, optional)] class: Option<String>,
    #[props(into, optional)] id: Option<String>,
    #[props(default = ButtonVariant::default())] variant: ButtonVariant,
    #[props(default = ButtonSize::default())] size: ButtonSize,
    #[props(optional)] disabled: bool,
    #[props(into, optional)] href: Option<String>,
    // TODO: replace button_type with a proper ButtonType enum (e.g. ButtonType::Submit/Reset/Button)
    #[props(into, optional)] button_type: Option<String>,
    #[props(optional)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(optional)] onkeydown: Option<EventHandler<KeyboardEvent>>,
    #[props(optional)] onfocus: Option<EventHandler<FocusEvent>>,
    #[props(optional)] onblur: Option<EventHandler<FocusEvent>>,
    #[props(optional)] onpointerenter: Option<EventHandler<PointerEvent>>,
    #[props(optional)] onpointerdown: Option<EventHandler<PointerEvent>>,
    #[props(optional)] onpointerup: Option<EventHandler<PointerEvent>>,
    #[props(optional)] onpointerleave: Option<EventHandler<PointerEvent>>,
    #[props(optional)] onpointercancel: Option<EventHandler<PointerEvent>>,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(into, optional)] aria_describedby: Option<String>,
    #[props(into, optional)] aria_current: Option<String>,
    #[props(optional)] aria_pressed: Option<bool>,
    #[props(optional)] aria_expanded: Option<bool>,
    #[props(into, optional)] aria_controls: Option<String>,
    #[props(into, optional)] aria_haspopup: Option<String>,
    #[props(into, optional)] title: Option<String>,
    #[props(default)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let merged_class = tw_merge!(
        "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive w-fit hover:cursor-pointer active:scale-[0.98] active:opacity-100 touch-manipulation [-webkit-tap-highlight-color:transparent] select-none [-webkit-touch-callout:none]",
        variant.as_str(),
        size.as_str(),
        class.as_deref().unwrap_or("")
    );

    let owners = [
        onclick.map(|handler| dioxus::events::onclick(move |event| handler.call(event))),
        onkeydown.map(|handler| dioxus::events::onkeydown(move |event| handler.call(event))),
        onfocus.map(|handler| dioxus::events::onfocus(move |event| handler.call(event))),
        onblur.map(|handler| dioxus::events::onblur(move |event| handler.call(event))),
        onpointerenter
            .map(|handler| dioxus::events::onpointerenter(move |event| handler.call(event))),
        onpointerdown
            .map(|handler| dioxus::events::onpointerdown(move |event| handler.call(event))),
        onpointerup.map(|handler| dioxus::events::onpointerup(move |event| handler.call(event))),
        onpointerleave
            .map(|handler| dioxus::events::onpointerleave(move |event| handler.call(event))),
        onpointercancel
            .map(|handler| dioxus::events::onpointercancel(move |event| handler.call(event))),
    ]
    .into_iter()
    .flatten()
    .collect();
    let (id, attributes) = button_attributes(
        attributes,
        owners,
        id.as_deref(),
        aria_describedby.as_deref(),
    );

    if let Some(url) = href {
        rsx! {
            a {
                id: id,
                "data-slot": "button",
                class: "{merged_class}",
                href: url,
                "aria-label": aria_label.as_deref(),
                "aria-current": aria_current.as_deref(),
                "aria-pressed": aria_pressed,
                "aria-expanded": aria_expanded,
                "aria-controls": aria_controls.as_deref(),
                "aria-haspopup": aria_haspopup.as_deref(),
                title: title.as_deref(),
                ..attributes,
                {children}
            }
        }
    } else {
        rsx! {
            button {
                id: id.as_deref(),
                "data-slot": "button",
                class: "{merged_class}",
                r#type: button_type.as_deref().unwrap_or("button"),
                disabled,
                "aria-label": aria_label.as_deref(),
                "aria-current": aria_current.as_deref(),
                "aria-pressed": aria_pressed,
                "aria-expanded": aria_expanded,
                "aria-controls": aria_controls.as_deref(),
                "aria-haspopup": aria_haspopup.as_deref(),
                title: title.as_deref(),
                ..attributes,
                {children}
            }
        }
    }
}
