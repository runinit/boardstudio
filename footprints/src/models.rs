//! 3D model references in generated forms: asset IDs, bindings and the paths a
//! preview supplies for models that have no exported file.
use std::collections::BTreeMap;

use crate::error::{GeneratorError, Result};
use crate::number::{decode_uri_component, encode_uri_component, js_to_number};
use crate::sexpr::{Expr, child, children, scalar_error};
use crate::types::{ModelBinding, Vec3};

/// Prefix of bundled model asset IDs. Step 5 of the plan renames it to
/// `bundled-model:`; this constant is the single place that changes.
pub const BUNDLED_MODEL_PREFIX: &str = "ergogen:model:";
const UNRESOLVED_PREFIX: &str = "unresolved-model:";

fn plain(text: &str) -> bool {
    !text.is_empty() && !text.contains(['\n', '\r', '\u{2028}', '\u{2029}'])
}

/// The asset an attached or bundled model path refers to.
pub fn model_asset_id(path: &str) -> Option<String> {
    if let Some(id) = path.strip_prefix("boardstudio-asset:")
        && !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Some(id.to_owned());
    }
    if let Some(rest) = path.strip_prefix("${KIPRJMOD}/models/boardstudio/")
        && plain(rest)
    {
        return Some(format!("{BUNDLED_MODEL_PREFIX}{rest}"));
    }
    if let Some(rest) = path.strip_prefix("${EG_INFUSED_KIM_3D_MODELS}/")
        && plain(rest)
    {
        return Some(format!("{BUNDLED_MODEL_PREFIX}infused-kim/{rest}"));
    }
    None
}

pub fn model_asset_ids_for_paths(paths: &[&str]) -> Vec<Option<String>> {
    paths.iter().map(|path| model_asset_id(path)).collect()
}

fn footprints(forms: &[Expr]) -> impl Iterator<Item = &[Expr]> {
    forms.iter().filter_map(|form| {
        let items = form.as_list()?;
        matches!(items.first(), Some(Expr::Atom(head)) if head == "footprint" || head == "module")
            .then_some(items)
    })
}

fn model_path(model: &[Expr]) -> Result<&str> {
    model.get(1).ok_or_else(scalar_error)?.value()
}

/// The distinct asset IDs of every model, in order. A path that is neither
/// attached nor bundled is an error.
pub fn model_asset_ids(forms: &[Expr]) -> Result<Vec<String>> {
    let mut ids: Vec<String> = Vec::new();
    for footprint in footprints(forms) {
        for model in children(footprint, "model") {
            let path = model_path(model)?;
            let id = model_asset_id(path).ok_or_else(|| {
                GeneratorError::Model(format!(
                    "Ergogen model path is not attached or bundled: {path}"
                ))
            })?;
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    Ok(ids)
}

fn xyz(model: &[Expr], name: &str, fallback: Vec3) -> Result<Vec3> {
    let Some(entry) = child(child(model, name).unwrap_or(&[]), "xyz") else {
        return Ok(fallback);
    };
    let number = |index: usize| {
        let value = match entry.get(index) {
            Some(Expr::Atom(text)) => js_to_number(text),
            _ => f64::NAN,
        };
        if value.is_finite() {
            Ok(value)
        } else {
            Err(GeneratorError::Geometry(
                "Invalid numeric Ergogen geometry".into(),
            ))
        }
    };
    Ok(Vec3 {
        x: number(1)?,
        y: number(2)?,
        z: number(3)?,
    })
}

/// Every model with its placement in the document's axes. Paths that resolve to
/// no asset keep their text as `unresolved-model:<percent-encoded path>`.
pub fn model_bindings(forms: &[Expr]) -> Result<Vec<ModelBinding>> {
    let mut bindings = Vec::new();
    for footprint in footprints(forms) {
        for model in children(footprint, "model") {
            let path = model_path(model)?;
            let asset_id = model_asset_id(path)
                .unwrap_or_else(|| format!("{UNRESOLVED_PREFIX}{}", encode_uri_component(path)));
            let offset = xyz(
                model,
                "offset",
                Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            )?;
            let rotation = xyz(
                model,
                "rotate",
                Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            )?;
            bindings.push(ModelBinding {
                asset_id,
                offset: Vec3 {
                    y: -offset.y,
                    ..offset
                },
                rotation: Vec3 {
                    z: -rotation.z,
                    ..rotation
                },
                scale: xyz(
                    model,
                    "scale",
                    Vec3 {
                        x: 1.0,
                        y: 1.0,
                        z: 1.0,
                    },
                )?,
            });
        }
    }
    Ok(bindings)
}

/// Add a safe placeholder path for each unresolved model so a preview can render
/// models that have no exported file.
pub fn preview_model_paths(
    bindings: &[ModelBinding],
    supplied: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let mut paths = supplied.clone();
    for binding in bindings {
        if let Some(encoded) = binding.asset_id.strip_prefix(UNRESOLVED_PREFIX) {
            let original = decode_uri_component(encoded).ok_or_else(|| {
                GeneratorError::Model("Ergogen model path has invalid percent encoding".into())
            })?;
            let placeholder = format!("models/unresolved/{}", encode_uri_component(&original));
            paths.insert(original, placeholder);
        }
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_attached_and_bundled_paths_only() {
        assert_eq!(
            model_asset_id("boardstudio-asset:abc_1").as_deref(),
            Some("abc_1")
        );
        assert_eq!(model_asset_id("boardstudio-asset:bad id"), None);
        assert_eq!(
            model_asset_id("${KIPRJMOD}/models/boardstudio/kicad/a.step").as_deref(),
            Some("ergogen:model:kicad/a.step")
        );
        assert_eq!(
            model_asset_id("${EG_INFUSED_KIM_3D_MODELS}/b.wrl").as_deref(),
            Some("ergogen:model:infused-kim/b.wrl")
        );
        assert_eq!(model_asset_id("${KIPRJMOD}/other/c.step"), None);
        assert_eq!(model_asset_id("${KIPRJMOD}/models/boardstudio/"), None);
    }
}
