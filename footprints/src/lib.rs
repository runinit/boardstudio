//! Footprint generators and the provider framework around them.
//!
//! Step 2 of `docs/investigations/footprint-generators-rust.md`: the framework
//! only. Generator modules arrive in step 3, one per generator, each recording
//! the SPDX identifier and author of its source (MIT or CC-BY-NC-SA-4.0; the
//! crate as a whole is mixed-licence once they land).
#![forbid(unsafe_code)]

pub mod context;
pub mod definition;
pub mod error;
pub mod export;
pub mod geometry;
pub mod models;
pub mod nets;
pub mod number;
pub mod params;
pub mod registry;
pub mod sexpr;
pub mod types;

pub use error::{GeneratorError, Result};
pub use sexpr::Expr;
