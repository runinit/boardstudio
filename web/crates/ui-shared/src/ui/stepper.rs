//! Copied from rust-ui: `app_crates/registry/src/ui/stepper.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Edits to the upstream file:
//! - built-in borders removed, cards style (theme contract section 8); focus rings kept (step indicator)
//! - icons come from `crate::ui::icons` (ticket 004) instead of the `icons` crate
//! - `hooks/use_stepper.rs` is inlined at the end of this file
//! - every root element carries a `data-slot` (theme contract section 9)

use crate::ui::icons::Check;
use dioxus::prelude::*;
use tw_merge::tw_merge;

#[derive(Clone, Copy, PartialEq, Eq, Default, strum::IntoStaticStr)]
pub enum StepperOrientation {
    #[default]
    Horizontal,
    Vertical,
}

impl StepperOrientation {
    fn as_str(self) -> &'static str {
        self.into()
    }
}

#[derive(Clone, Copy)]
struct StepperItemCtx {
    step: usize,
    disabled: bool,
}

#[component]
pub fn StepperTitle(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge!(
        "text-sm font-medium text-foreground transition-colors group-data-[state=Pending]/stepper-item:text-muted-foreground group-data-[state=Disabled]/stepper-item:text-muted-foreground/50",
        class.as_deref().unwrap_or("")
    );
    rsx! { div { "data-slot": "stepper-title", "data-name": "StepperTitle", class: "{merged}", {children} } }
}

#[component]
pub fn StepperDescription(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge!(
        "text-sm text-muted-foreground transition-colors group-data-[state=Disabled]/stepper-item:text-muted-foreground/50",
        class.as_deref().unwrap_or("")
    );
    rsx! { div { "data-slot": "stepper-description", "data-name": "StepperDescription", class: "{merged}", {children} } }
}

#[component]
pub fn StepperSeparator(#[props(into, optional)] class: Option<String>) -> Element {
    let merged = tw_merge!(
        "absolute bg-border transition-colors",
        "group-data-[orientation=Horizontal]/stepper:top-4 group-data-[orientation=Horizontal]/stepper:left-[calc(50%+1.5rem)] group-data-[orientation=Horizontal]/stepper:w-[calc(100%-3rem)] group-data-[orientation=Horizontal]/stepper:h-0.5 group-data-[orientation=Horizontal]/stepper:-translate-y-1/2",
        "group-data-[orientation=Vertical]/stepper:top-8 group-data-[orientation=Vertical]/stepper:left-4 group-data-[orientation=Vertical]/stepper:h-full group-data-[orientation=Vertical]/stepper:w-0.5",
        "group-data-[state=Completed]/stepper-item:bg-primary",
        class.as_deref().unwrap_or("")
    );
    rsx! { div { "data-slot": "stepper-separator", "data-name": "StepperSeparator", class: "{merged}" } }
}

#[component]
pub fn Stepper(
    total_steps: usize,
    #[props(default = 0)] default_step: usize,
    #[props(default = StepperOrientation::Horizontal)] orientation: StepperOrientation,
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let ctx = use_stepper(total_steps, default_step);
    use_context_provider(|| ctx);

    let merged = tw_merge!(
        "group/stepper flex w-full",
        if orientation == StepperOrientation::Horizontal {
            "flex-row flex-wrap items-start"
        } else {
            "flex-col"
        },
        class.as_deref().unwrap_or("")
    );

    rsx! {
        div { "data-slot": "stepper", "data-name": "Stepper", "data-orientation": orientation.as_str(), class: "{merged}", {children} }
    }
}

#[component]
pub fn StepperItem(
    step: usize,
    #[props(default = false)] disabled: bool,
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let stepper_ctx = use_context::<StepperContext>();
    use_context_provider(|| StepperItemCtx { step, disabled });

    let state = if disabled {
        StepState::Disabled
    } else {
        stepper_ctx.step_state(step)
    };
    let merged = tw_merge!(
        "group/stepper-item relative flex flex-1 flex-col items-center gap-2",
        "group-data-[orientation=Vertical]/stepper:flex-row group-data-[orientation=Vertical]/stepper:items-start",
        class.as_deref().unwrap_or("")
    );

    rsx! {
        div { "data-slot": "stepper-item", "data-name": "StepperItem", "data-state": state.as_str(), class: "{merged}", {children} }
    }
}

#[component]
pub fn StepperTrigger(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let stepper_ctx = use_context::<StepperContext>();
    let item_ctx = use_context::<StepperItemCtx>();
    let step = item_ctx.step;

    let state = if item_ctx.disabled {
        StepState::Disabled
    } else {
        stepper_ctx.step_state(step)
    };
    let is_disabled = state == StepState::Disabled;
    let is_active = state == StepState::Active;

    let merged = tw_merge!(
        "group/stepper-trigger flex flex-1 flex-col items-center gap-2 rounded-md text-center outline-none cursor-pointer",
        "focus-visible:ring-[3px] focus-visible:ring-ring/50",
        "disabled:pointer-events-none disabled:opacity-50 disabled:cursor-not-allowed",
        "group-data-[orientation=Vertical]/stepper:flex-row group-data-[orientation=Vertical]/stepper:items-start group-data-[orientation=Vertical]/stepper:text-left group-data-[orientation=Vertical]/stepper:w-full",
        class.as_deref().unwrap_or("")
    );

    rsx! {
        button { "data-slot": "stepper-trigger",
            r#type: "button",
            "data-name": "StepperTrigger",
            disabled: is_disabled,
            "aria-current": if is_active { "step" } else { "" },
            class: "{merged}",
            onclick: move |_| stepper_ctx.go_to(step),
            {children}
        }
    }
}

#[component]
pub fn StepperIndicator(
    #[props(into, optional)] class: Option<String>,
    children: Option<Element>,
) -> Element {
    let stepper_ctx = use_context::<StepperContext>();
    let item_ctx = use_context::<StepperItemCtx>();
    let state = if item_ctx.disabled {
        StepState::Disabled
    } else {
        stepper_ctx.step_state(item_ctx.step)
    };

    let variant_class = match state {
        StepState::Pending => "bg-background text-muted-foreground",
        StepState::Active | StepState::Completed => "bg-primary text-primary-foreground",
        StepState::Disabled => "bg-muted text-muted-foreground/50",
    };
    let merged = tw_merge!(
        "relative z-10 flex size-8 shrink-0 items-center justify-center rounded-full text-sm font-medium transition-colors",
        variant_class,
        class.as_deref().unwrap_or("")
    );

    rsx! {
        span { "data-slot": "stepper-indicator", "data-name": "StepperIndicator", "aria-hidden": "true", class: "{merged}",
            {
                children.unwrap_or_else(|| {
                    if state == StepState::Completed {
                        rsx! { Check { class: "size-4" } }
                    } else {
                        rsx! { "{item_ctx.step + 1}" }
                    }
                })
            }
        }
    }
}

// ---- inlined from `hooks/use_stepper.rs` ----
/// Visual/interactive state of a single step, relative to the current index.
///
/// `Completed`/`Active`/`Pending` are derived automatically from index
/// comparison. `Disabled` is not produced by this hook — it's applied by
/// the caller (e.g. `StepperItem`) on top of the computed state, since it
/// depends on external conditions the hook has no visibility into.
#[derive(Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
pub enum StepState {
    Completed,
    Active,
    Pending,
    Disabled,
}

impl StepState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        self.into()
    }
}

/// Shared reactive state for a `Stepper` instance, provided via
/// `provide_context` and consumed by `StepperItem`/`StepperTrigger`.
///
/// All navigation methods (`go_next`, `go_prev`, `go_to`) clamp to
/// `[0, total_steps)`, so `current_index` can never be set out of range —
/// callers indexing a step list with it don't need to re-validate.
#[derive(Clone, Copy)]
pub struct StepperContext {
    pub current_index: Signal<usize>,
    pub total_steps: usize,
}

impl StepperContext {
    /// `true` when there is a previous state to undo to.
    #[must_use]
    pub fn can_go_prev(&self) -> bool {
        (self.current_index)() > 0
    }

    /// `true` when there is a future state to redo to.
    #[must_use]
    pub fn can_go_next(&self) -> bool {
        (self.current_index)() + 1 < self.total_steps
    }

    pub fn go_prev(&self) {
        let mut index = self.current_index;
        if index() > 0 {
            index.with_mut(|i| *i -= 1);
        }
    }

    pub fn go_next(&self) {
        let mut index = self.current_index;
        if index() + 1 < self.total_steps {
            index.with_mut(|i| *i += 1);
        }
    }

    /// Backs clickable step triggers — jumps straight to an arbitrary index
    /// rather than stepping by one, so out-of-range values need their own guard.
    pub fn go_to(&self, step: usize) {
        if step < self.total_steps {
            let mut index = self.current_index;
            index.set(step);
        }
    }

    /// Maps the issue's three-way rule (step < current -> completed, == -> active,
    /// > -> pending) onto Ordering so it reads as one exhaustive match.
    #[must_use]
    pub fn step_state(&self, step: usize) -> StepState {
        let current = (self.current_index)();
        match step.cmp(&current) {
            std::cmp::Ordering::Less => StepState::Completed,
            std::cmp::Ordering::Equal => StepState::Active,
            std::cmp::Ordering::Greater => StepState::Pending,
        }
    }
}

/// Builds the controlled navigation state for a stepper with `total_steps`
/// steps, starting at `default_index`.
#[must_use]
pub fn use_stepper(total_steps: usize, default_index: usize) -> StepperContext {
    // Clamp in case `default_index` is out of range (e.g. caller passes total_steps itself).
    let current_index = use_signal(|| default_index.min(total_steps.saturating_sub(1)));
    StepperContext {
        current_index,
        total_steps,
    }
}
