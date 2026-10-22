//! Look in the style of rust-ui (`app_crates/registry/src/ui/slider.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318`) (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::slider` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - upstream is a native range input whose look comes from a stylesheet outside the registry; the track,
//! -   range and thumb here use rust-ui tokens (`bg-muted`, `bg-primary`, the focus ring) on the primitive's parts;
//! - `RangeSlider` and upstream's `Flat` variant are dropped: the mapping needs single-value sliders.
//! - ticket 008 adds Home/End and ten-step Page keys through the public controlled-state hook,
//!   and applies native disabled semantics to the thumb; pointer and arrow behavior stays primitive-owned.

use dioxus::prelude::*;
use dioxus_primitives::slider::{self, SliderProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn Slider(props: SliderProps) -> Element {
    let (value, set_value) =
        dioxus_primitives::use_controlled(props.value, props.default_value, props.on_value_change);
    let controlled_value = use_memo(move || Some(value()));
    let base = attributes!(div {
        class: "relative flex w-full touch-none items-center select-none data-[disabled=true]:opacity-50 data-[orientation=vertical]:h-full data-[orientation=vertical]:min-h-44 data-[orientation=vertical]:w-auto data-[orientation=vertical]:flex-col",
        "data-slot": "slider",
        onkeydown: move |event: KeyboardEvent| {
            if (props.disabled)() { return; }
            let min = (props.min)();
            let max = (props.max)();
            let step = (props.step)();
            let next = match event.key() {
                Key::Home => min,
                Key::End => max,
                Key::PageUp => ((value() + step * 10.0) / step).round() * step,
                Key::PageDown => ((value() - step * 10.0) / step).round() * step,
                _ => return,
            };
            event.prevent_default();
            set_value.call(next.clamp(min, max));
        },
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        slider::Slider {
            value: controlled_value,
            default_value: props.default_value,
            min: props.min,
            max: props.max,
            step: props.step,
            disabled: props.disabled,
            horizontal: props.horizontal,
            inverted: props.inverted,
            on_value_change: set_value,
            label: props.label,
            attributes: merged,
            slider::SliderTrack {
                class: "relative grow overflow-hidden rounded-full bg-muted data-[orientation=horizontal]:h-1.5 data-[orientation=horizontal]:w-full data-[orientation=vertical]:h-full data-[orientation=vertical]:w-1.5",
                "data-slot": "slider-track",
                slider::SliderRange {
                    class: "absolute bg-primary data-[orientation=horizontal]:h-full data-[orientation=vertical]:w-full",
                    "data-slot": "slider-range",
                }
                slider::SliderThumb {
                    attributes: attributes!(button {
                        disabled: (props.disabled)(),
                        aria_disabled: (props.disabled)(),
                    }),
                    class: "block size-4 shrink-0 rounded-full bg-background shadow-sm ring-2 ring-primary transition-[color,box-shadow] hover:ring-4 focus-visible:ring-4 focus-visible:outline-hidden disabled:pointer-events-none",
                    "data-slot": "slider-thumb",
                }
            }
        }
    }
}
