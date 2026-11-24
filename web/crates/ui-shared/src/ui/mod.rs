//! Components copied from rust-ui, one file per component; each file records its upstream path and commit.
//!
//! Components compile on every target; browser-specific behavior is gated locally.
pub mod alert;
pub mod badge;
pub mod button;
pub mod callout;
pub mod card;
pub(crate) mod class_merge;
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

pub mod accordion;
pub mod alert_dialog;
pub mod checkbox;
pub mod collapsible;
pub mod combobox;
pub mod command;
pub mod dialog;
pub mod dropdown_menu;
pub mod popover;
pub mod select;
pub mod sheet;
pub mod slider;
pub mod tabs;
pub mod toast;
pub mod toggle_group;
pub mod toolbar;
pub mod tooltip;

#[cfg(test)]
mod copy_rules_tests;
