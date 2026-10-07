//! KiCad output from rendered forms: model paths become exported project paths,
//! legacy `module` forms become `footprint`, and board-level objects (tracks,
//! vias, zones, graphics) separate from footprints.
//!
//! Legacy arcs are not rewritten here. Core already owns that upgrade
//! (`upgrade_legacy_arcs` in `core/src/artifact/source.rs`) and applies it to
//! this text.
use std::collections::BTreeMap;

use crate::error::{GeneratorError, Result};
use crate::models::model_asset_id;
use crate::sexpr::{Expr, serialize};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExportedForms {
    pub footprints: Vec<String>,
    pub objects: Vec<String>,
}

const OBJECT_FORMS: [&str; 9] = [
    "segment",
    "via",
    "zone",
    "gr_text",
    "gr_line",
    "gr_arc",
    "gr_circle",
    "gr_poly",
    "gr_rect",
];

fn unsafe_path(path: &str) -> bool {
    let scheme = path.split_once(':').is_some_and(|(scheme, _)| {
        !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_alphabetic())
    });
    path.is_empty() || path.starts_with('/') || path.contains("..") || path.contains('\\') || scheme
}

fn rewrite_model_paths(form: &mut [Expr], paths: &BTreeMap<String, String>) -> Result<()> {
    for item in form.iter_mut() {
        let Expr::List(model) = item else { continue };
        if !matches!(model.first(), Some(Expr::Atom(head)) if head == "model") {
            continue;
        }
        let original = model
            .get(1)
            .ok_or_else(crate::sexpr::scalar_error)?
            .value()?
            .to_owned();
        let exported = model_asset_id(&original)
            .and_then(|id| paths.get(&id))
            .filter(|path| !path.is_empty())
            .or_else(|| paths.get(&original))
            .filter(|path| !unsafe_path(path));
        let Some(path) = exported else {
            return Err(GeneratorError::Model(format!(
                "Ergogen model has no safe exported path: {original}"
            )));
        };
        model[1] = Expr::Str(format!("${{KIPRJMOD}}/{path}"));
    }
    Ok(())
}

/// Convert rendered forms to KiCad text. `paths` maps asset IDs (or raw model
/// paths) to exported project-relative files.
pub fn export_forms(forms: Vec<Expr>, paths: &BTreeMap<String, String>) -> Result<ExportedForms> {
    let mut exported = ExportedForms::default();
    for mut form in forms {
        let Expr::List(items) = &mut form else {
            continue;
        };
        let kind = items
            .first()
            .ok_or_else(crate::sexpr::scalar_error)?
            .value()?
            .to_owned();
        match kind.as_str() {
            "footprint" | "module" => {
                rewrite_model_paths(items, paths)?;
                items[0] = Expr::atom("footprint");
                exported.footprints.push(serialize(&form));
            }
            kind if OBJECT_FORMS.contains(&kind) => exported.objects.push(serialize(&form)),
            kind => {
                return Err(GeneratorError::Export(format!(
                    "Unsupported Ergogen output form: {kind}"
                )));
            }
        }
    }
    Ok(exported)
}
