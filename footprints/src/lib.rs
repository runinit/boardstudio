//! Footprint generators and the provider framework around them.
//!
//! Step 2 of `docs/investigations/footprint-generators-rust.md`: the framework
//! only. Generator modules arrive in step 3, one per generator, each recording
//! the SPDX identifier and author of its source (MIT or CC-BY-NC-SA-4.0; the
//! crate as a whole is mixed-licence once they land).
#![forbid(unsafe_code)]

pub mod build;
pub mod context;
pub mod definition;
pub mod error;
pub mod export;
pub mod generators;
pub mod geometry;
pub mod models;
pub mod nets;
pub mod number;
pub mod params;
pub mod registry;
pub mod sexpr;
pub mod types;

pub use error::{GeneratorError, Result};
pub use generators::bundled;
pub use sexpr::Expr;

/// JavaScript whitespace, shared by [`number`] (`Number(text)`) and the [`sexpr`]
/// tokenizer: the two previous copies spelled it `is_js_space` and `is_space`.
pub(crate) fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}
