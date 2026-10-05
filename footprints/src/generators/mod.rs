//! The bundled generators, one module per generator.
//!
//! Each module keeps the SPDX identifier and author of its source file.
//! `ceoloide/*` files are MIT or CC-BY-NC-SA-4.0 as marked in each module;
//! `infused-kim/*` files are CC-BY-NC-SA-4.0 (see the vendored LICENSE).
use std::sync::OnceLock;

use crate::registry::{GeneratorSpec, Registry};

pub mod ceoloide;
pub mod infused_kim;
pub(crate) mod util;

/// Every bundled generator.
pub static SPECS: &[&GeneratorSpec] = &[
    &ceoloide::switch_gateron_ks27_ks33::SPEC,
    &ceoloide::switch_choc_v1_v2::SPEC,
    &ceoloide::switch_mx::SPEC,
    &ceoloide::power_switch_smd_side::SPEC,
    &ceoloide::reset_switch_tht_top::SPEC,
    &ceoloide::reset_switch_smd_side::SPEC,
    &ceoloide::led_sk6812mini_e::SPEC,
    &ceoloide::diode_tht_sod123::SPEC,
    &ceoloide::mounting_hole_npth::SPEC,
    &ceoloide::mounting_hole_plated::SPEC,
    &ceoloide::utility_filled_zone::SPEC,
    &ceoloide::utility_keepout_zone::SPEC,
    &ceoloide::utility_logo::SPEC,
    &ceoloide::utility_point_debugger::SPEC,
    &ceoloide::utility_router::SPEC,
    &ceoloide::utility_text::SPEC,
    &infused_kim::icon_bat::SPEC,
    &infused_kim::mounting_hole::SPEC,
    &infused_kim::pads::SPEC,
    &infused_kim::point_debugger::SPEC,
    &infused_kim::text::SPEC,
];

/// The registry of bundled generators.
pub fn bundled() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| Registry::new(SPECS).expect("bundled generator declarations are valid"))
}
