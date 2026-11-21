//! Host-compatible module catalogue projection for the Parts workspace.
//!
//! The production catalogue (wasm) delegates to this module so the same
//! merge/selection projection runs under `cargo test` without a browser.

use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{ModuleDefinition, PartDefinition, PartModel, Vec3};
use std::{collections::HashMap, rc::Rc};

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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ModuleCatalogueRequest {
    pub(crate) active: bool,
    pub(crate) token: SnapshotToken,
    pub(crate) project: Vec<ModuleDefinition>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ModuleCatalogueResult {
    pub(crate) request: ModuleCatalogueRequest,
    pub(crate) entries: Option<Rc<Vec<ModuleEntry>>>,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModuleCatalogueView {
    pub entries: Option<Rc<Vec<ModuleEntry>>>,
    pub error: Option<String>,
    pub pending: bool,
}

pub(crate) fn catalogue_view(
    request: &ModuleCatalogueRequest,
    result: Option<ModuleCatalogueResult>,
    bundled: Option<Rc<Vec<ModuleEntry>>>,
) -> ModuleCatalogueView {
    if !request.active {
        return ModuleCatalogueView::default();
    }
    match result.filter(|result| result.request == *request) {
        Some(result) => ModuleCatalogueView {
            entries: result.entries,
            error: result.error,
            pending: false,
        },
        None => match bundled {
            // The package catalogue is immutable. Re-project only the current accepted
            // document's overrides while the request-owned resource catches up, so an
            // accepted edit cannot unmount the editor that is settling that edit.
            Some(bundled) => ModuleCatalogueView {
                entries: Some(Rc::new(merge_project_overrides(&bundled, &request.project))),
                error: None,
                pending: false,
            },
            None => ModuleCatalogueView {
                pending: true,
                ..ModuleCatalogueView::default()
            },
        },
    }
}

pub(crate) fn merge_project_overrides(
    bundled: &[ModuleEntry],
    project: &[ModuleDefinition],
) -> Vec<ModuleEntry> {
    let mut entries = bundled.to_vec();
    let mut positions = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.definition.id.clone(), index))
        .collect::<HashMap<_, _>>();
    for definition in project {
        if let Some(index) = positions.get(&definition.id).copied() {
            entries[index].definition = Rc::new(definition.clone());
            entries[index].source = EntrySource::Project;
        } else {
            let row = definition
                .catalogue_row
                .clone()
                .unwrap_or_else(|| definition.id.clone());
            positions.insert(definition.id.clone(), entries.len());
            entries.push(ModuleEntry {
                row,
                definition: Rc::new(definition.clone()),
                source: EntrySource::Project,
            });
        }
    }
    entries
}

pub fn variants(entries: &[ModuleEntry], row: &str) -> Vec<ModuleEntry> {
    entries
        .iter()
        .filter(|entry| entry.row == row)
        .cloned()
        .collect()
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
    fn module_catalogue_refresh_keeps_current_projection_without_stale_project_entries() {
        let bundled = Rc::new(vec![ModuleEntry {
            row: "test".into(),
            definition: Rc::new(definition("bundled")),
            source: EntrySource::Bundled,
        }]);
        let request = ModuleCatalogueRequest {
            active: true,
            token: SnapshotToken(2),
            project: vec![definition("accepted")],
        };
        let stale_request = ModuleCatalogueRequest {
            active: true,
            token: SnapshotToken(1),
            project: vec![definition("previous")],
        };
        let stale_result = ModuleCatalogueResult {
            entries: Some(Rc::new(merge_project_overrides(
                &bundled,
                &stale_request.project,
            ))),
            request: stale_request,
            error: None,
        };
        let view = catalogue_view(&request, Some(stale_result), Some(bundled.clone()));
        assert!(
            !view.pending,
            "accepted token refresh must not unmount the selected module owner"
        );
        let entries = view.entries.unwrap();
        assert_eq!(entries[0].definition.name, "accepted");
        assert_eq!(entries[0].source, EntrySource::Project);

        let switched = ModuleCatalogueRequest {
            active: true,
            token: SnapshotToken(3),
            project: Vec::new(),
        };
        let previous = ModuleCatalogueResult {
            request: request.clone(),
            entries: Some(entries),
            error: None,
        };
        let view = catalogue_view(&switched, Some(previous), Some(bundled.clone()));
        assert_eq!(view.entries.unwrap()[0].definition.name, "bundled");
        assert!(catalogue_view(&switched, None, None).pending);
        let inactive = ModuleCatalogueRequest {
            active: false,
            ..switched
        };
        assert!(
            catalogue_view(&inactive, None, Some(bundled))
                .entries
                .is_none()
        );
    }

    #[test]
    fn catalogue_variants_and_selection_follow_only_the_current_scope() {
        let first = ModuleEntry {
            row: "test".into(),
            definition: Rc::new(definition("bundled")),
            source: EntrySource::Bundled,
        };
        let entries = vec![
            first.clone(),
            ModuleEntry {
                row: "other".into(),
                definition: Rc::new(definition("other")),
                source: EntrySource::Bundled,
            },
        ];
        assert_eq!(variants(&entries, "test"), vec![first]);
        assert!(variants(&entries, "missing").is_empty());

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
        assert_eq!(
            selected_id(
                Some((Some(scope.clone()), "module:test".into())),
                &Some(Scope {
                    board_id: "other".into(),
                    ..scope.clone()
                })
            ),
            None
        );
        assert_eq!(
            selected_id(Some((None, "module:test".into())), &Some(scope)),
            None
        );
    }
}
