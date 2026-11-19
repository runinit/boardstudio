//! Footprints integration suites that share one test binary.
//!
//! Keep the modules in place so fixture paths and focused test names stay easy to find.

mod common;

#[path = "choc_socket_qualification.rs"]
mod choc_socket_qualification;

#[path = "context.rs"]
mod context;

#[path = "diode_sod123_qualification.rs"]
mod diode_sod123_qualification;

#[path = "encoder_ec11e15244g1_qualification.rs"]
mod encoder_ec11e15244g1_qualification;

#[path = "generators.rs"]
mod generators;

#[path = "golden_forms.rs"]
mod golden_forms;

#[path = "licenses.rs"]
mod licenses;

#[path = "mx_socket_qualification.rs"]
mod mx_socket_qualification;

#[path = "numeric.rs"]
mod numeric;

#[path = "parameter_check.rs"]
mod parameter_check;

#[path = "reset_switch_smd_side_qualification.rs"]
mod reset_switch_smd_side_qualification;

#[path = "text_literal_escaping.rs"]
mod text_literal_escaping;
