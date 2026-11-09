//! Components copied from rust-ui, one file per component; each file records its upstream path and commit.
//!
//! Presentational components compile on every target. Interactive ones are built on `dioxus-primitives`,
//! which the crate only depends on for `wasm32`, so they are gated the same way.
pub mod alert;
pub mod badge;
pub mod button;
pub mod callout;
pub mod card;
pub mod empty;
pub mod field;
pub mod icons;
pub mod input;
pub mod input_group;
pub mod item;
pub mod kbd;
pub mod label;
pub mod separator;
pub mod sidenav;
pub mod skeleton;
pub mod spinner;
pub mod stepper;
pub mod table;

#[cfg(target_arch = "wasm32")]
pub mod accordion;
#[cfg(target_arch = "wasm32")]
pub mod alert_dialog;
#[cfg(target_arch = "wasm32")]
pub mod checkbox;
#[cfg(target_arch = "wasm32")]
pub mod collapsible;
#[cfg(target_arch = "wasm32")]
pub mod combobox;
#[cfg(target_arch = "wasm32")]
pub mod command;
#[cfg(target_arch = "wasm32")]
pub mod dialog;
#[cfg(target_arch = "wasm32")]
pub mod dropdown_menu;
#[cfg(target_arch = "wasm32")]
pub mod popover;
#[cfg(target_arch = "wasm32")]
pub mod select;
#[cfg(target_arch = "wasm32")]
pub mod sheet;
#[cfg(target_arch = "wasm32")]
pub mod slider;
#[cfg(target_arch = "wasm32")]
pub mod tabs;
#[cfg(target_arch = "wasm32")]
pub mod toast;
#[cfg(target_arch = "wasm32")]
pub mod toggle_group;
#[cfg(target_arch = "wasm32")]
pub mod toolbar;
#[cfg(target_arch = "wasm32")]
pub mod tooltip;

#[cfg(test)]
mod copy_rules_tests;
#[cfg(all(test, target_arch = "wasm32"))]
mod primitives_mount_tests;
