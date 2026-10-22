//! Look copied from rust-ui: `app_crates/registry/src/ui/checkbox.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::checkbox` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - built-in border removed and the box filled with `bg-field`, cards style (theme contract section 8); focus ring kept;
//! - icons come from `crate::ui::icons` (ticket 004).

use dioxus::prelude::*;
use dioxus_primitives::checkbox::{self, CheckboxProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

use crate::ui::icons::Check;

#[component]
pub fn Checkbox(props: CheckboxProps) -> Element {
    let base = attributes!(button {
        class: "peer bg-field data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 inline-flex items-center justify-center size-4 shrink-0 rounded-[4px] transition-shadow outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50",
        "data-slot": "checkbox",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        checkbox::Checkbox {
            checked: props.checked,
            default_checked: props.default_checked,
            required: props.required,
            disabled: props.disabled,
            name: props.name,
            value: props.value,
            on_checked_change: props.on_checked_change,
            attributes: merged,
            checkbox::CheckboxIndicator { class: "flex justify-center items-center text-current transition-none", "data-slot": "checkbox-indicator",
                Check { class: "size-3.5" }
            }
        }
    }
}
