//! Host-compatible module catalogue projection for the Parts workspace.
//!
//! The production catalogue (wasm) delegates to this module so the same
//! merge/selection projection runs under `cargo test` without a browser.

use boardstudio_application::Scope;
use boardstudio_model::model::{ModuleDefinition, PartDefinition, PartModel, Vec3};
use serde::Deserialize;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleEntry {
    pub row: String,
    pub definition: Rc<ModuleDefinition>,
    pub(crate) source: EntrySource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntrySource {
    Bundled,
    Project,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModuleIndex {
    pub format_version: u32,
    pub host_connector_asset: String,
    pub modules: Vec<ModuleIndexEntry>,
}

#[derive(Deserialize)]
pub(crate) struct ModuleIndexEntry {
    pub id: String,
    pub row: String,
    pub asset: String,
    pub variant: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleVariantChoice {
    pub id: String,
    pub variant: String,
}

#[derive(Default)]
pub struct ModuleCatalogueView {
    pub selected: Option<ModuleEntry>,
    pub variants: Vec<ModuleVariantChoice>,
    pub error: Option<String>,
}

pub(crate) fn catalogue_view(
    id: &str,
    project: &[ModuleDefinition],
    index: &[ModuleIndexEntry],
    bundled: Option<ModuleEntry>,
    error: Option<String>,
) -> ModuleCatalogueView {
    let index_row = |id: &str| {
        index
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.row.as_str())
    };
    let project_row = |definition: &ModuleDefinition| {
        definition
            .catalogue_row
            .as_deref()
            .unwrap_or(&definition.id)
            .to_owned()
    };
    // Read the accepted project synchronously so an accepted edit cannot unmount its editor.
    let selected = project
        .iter()
        .rev()
        .find(|definition| definition.id == id)
        .map(|definition| ModuleEntry {
            row: index_row(id)
                .map(str::to_owned)
                .unwrap_or_else(|| project_row(definition)),
            definition: Rc::new(definition.clone()),
            source: EntrySource::Project,
        })
        .or(bundled);
    let variants = selected
        .as_ref()
        .map(|entry| {
            index
                .iter()
                .filter(|item| item.row == entry.row)
                .map(|item| ModuleVariantChoice {
                    id: item.id.clone(),
                    variant: project
                        .iter()
                        .rev()
                        .find(|definition| definition.id == item.id)
                        .map_or(&item.variant, |definition| &definition.variant)
                        .clone(),
                })
                .chain(
                    project
                        .iter()
                        .filter(|definition| {
                            index_row(&definition.id).is_none()
                                && project_row(definition) == entry.row
                        })
                        .map(|definition| ModuleVariantChoice {
                            id: definition.id.clone(),
                            variant: definition.variant.clone(),
                        }),
                )
                .collect()
        })
        .unwrap_or_default();
    ModuleCatalogueView {
        selected,
        variants,
        error,
    }
}

pub fn selected_id(
    selection: Option<(Option<Scope>, String)>,
    scope: &Option<Scope>,
) -> Option<String> {
    selection.and_then(|(selected_scope, id)| {
        (selected_scope == *scope)
            .then(|| id.strip_prefix("module:").map(str::to_owned))
            .flatten()
    })
}

pub(crate) fn normalize_host_connector_definition(
    mut definition: PartDefinition,
) -> Result<PartDefinition, String> {
    const MODEL_ASSET: &str =
        "bundled-model:vik/sadekbaroudi-vik/kicad/3dmodels/vik-connector-horizontal.stp";
    definition.id = "vik:source:horizontal-host-connector".into();
    definition.name = "VIK horizontal host connector".into();
    definition.models = Some(vec![PartModel {
        asset_id: MODEL_ASSET.into(),
        offset: Vec3 {
            x: -2.75,
            y: 2.3,
            z: 0.0,
        },
        rotation: Vec3::default(),
        scale: Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    }]);
    if let Some(source) = &mut definition.kicad_source {
        source.source = without_embedded_connector_model(&source.source)?;
    }
    Ok(definition)
}

pub(crate) fn without_embedded_connector_model(source: &str) -> Result<String, String> {
    const MARKER: &str = "(model \"../../kicad/3dmodels/vik-connector-horizontal.stp\"";
    let Some(start) = source.find(MARKER) else {
        return Ok(source.to_owned());
    };
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (offset, character) in source[start..].char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        match character {
            '"' => quoted = true,
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    "The source VIK host connector has a malformed embedded model form.".to_string()
                })?;
                if depth == 0 {
                    let end = start + offset + character.len_utf8();
                    return Ok(format!("{}{}", &source[..start], &source[end..]));
                }
            }
            _ => {}
        }
    }
    Err("The source VIK host connector has a malformed embedded model form.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition(name: &str) -> ModuleDefinition {
        serde_json::from_value(serde_json::json!({
            "id": "module/test", "name": name, "family": "test", "variant": "test",
            "source": {"repository": "test", "revision": "test", "path": "test", "license": "test"},
            "board": {"contours": []}, "electrical": {"protocol": "i2c"}
        }))
        .unwrap()
    }

    #[test]
    fn module_catalogue_refresh_projects_the_accepted_definition_over_cached_source() {
        let bundled = ModuleEntry {
            row: "test".into(),
            definition: Rc::new(definition("bundled")),
            source: EntrySource::Bundled,
        };
        let index = [
            ModuleIndexEntry {
                id: "module/test".into(),
                row: "test".into(),
                asset: "assets/modules/test.json".into(),
                variant: "bundled variant".into(),
            },
            ModuleIndexEntry {
                id: "module/sibling".into(),
                row: "test".into(),
                asset: "assets/modules/sibling.json".into(),
                variant: "sibling variant".into(),
            },
            ModuleIndexEntry {
                id: "module/other".into(),
                row: "other".into(),
                asset: "assets/modules/other.json".into(),
                variant: "other variant".into(),
            },
        ];
        let mut accepted = definition("accepted");
        accepted.variant = "edited variant".into();
        let view = catalogue_view(
            "module/test",
            &[accepted.clone()],
            &index,
            Some(bundled.clone()),
            None,
        );
        let entry = view
            .selected
            .expect("accepted refresh must keep the editor mounted");
        assert_eq!(entry.definition.name, "accepted");
        assert_eq!(entry.source, EntrySource::Project);
        assert_eq!(
            view.variants
                .iter()
                .map(|choice| (choice.id.as_str(), choice.variant.as_str()))
                .collect::<Vec<_>>(),
            [
                ("module/test", "edited variant"),
                ("module/sibling", "sibling variant")
            ]
        );

        let view = catalogue_view("module/test", &[accepted], &[], None, None);
        assert_eq!(view.selected.unwrap().source, EntrySource::Project);
        let view = catalogue_view("module/test", &[], &index, Some(bundled), None);
        assert_eq!(view.selected.unwrap().definition.name, "bundled");
        assert_eq!(view.variants[0].variant, "bundled variant");
        assert!(
            catalogue_view("module/sibling", &[], &index, None, None)
                .selected
                .is_none()
        );
    }

    #[test]
    fn catalogue_selection_follows_only_the_current_scope() {
        let scope = Scope {
            session_epoch: boardstudio_application::SessionEpoch(1),
            document_id: "project".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        assert_eq!(
            selected_id(
                Some((Some(scope.clone()), "module:test".into())),
                &Some(scope.clone())
            ),
            Some("test".into())
        );
        let other = Scope {
            board_id: "other".into(),
            ..scope.clone()
        };
        for owner in [Some(other), None] {
            assert!(
                selected_id(Some((owner, "module:test".into())), &Some(scope.clone())).is_none()
            );
        }
    }
}
