use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_model::model::{
    KeyBinding, KeycapBoardSettings, KeycapKeySettings, KeymapLayer, KeymapMacro, Part, PartKind,
    Pose2, ProjectDoc, Vec2,
};
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub struct KeymapView {
    pub layers: Vec<KeymapLayerLabel>,
    pub keys: Vec<KeymapKey>,
}

#[derive(Debug, PartialEq)]
pub struct KeymapLayerLabel {
    pub id: Rc<str>,
    pub name: Rc<str>,
}

#[derive(Debug, PartialEq)]
pub struct KeymapKey {
    pub id: Rc<str>,
    pub reference: Rc<str>,
    pub pose: Pose2,
    pub binding_title: Rc<str>,
    pub color: Rc<str>,
    pub size: Vec2,
    pub search_index: String,
}

/// Build the single accepted-snapshot projection shared by the canvas and panel.
pub fn project(
    snapshot: &AcceptedSnapshot,
    scope: Option<&Scope>,
    board_id: &str,
    active_layer_id: &str,
) -> Option<Rc<KeymapView>> {
    let scope = scope?;
    if scope.session_epoch != snapshot.session_epoch
        || scope.document_id != snapshot.document.id
        || scope.board_id != board_id
    {
        return None;
    }

    let document = &snapshot.document;
    let board = document.boards.iter().find(|board| board.id == board_id)?;
    let saved_map = document
        .keymap
        .as_ref()
        .filter(|keymap| !keymap.layers.is_empty());
    let saved_layers = saved_map.map_or(&[][..], |map| map.layers.as_slice());
    let layer_index = saved_layers
        .iter()
        .position(|layer| layer.id == active_layer_id)
        .unwrap_or(0);
    let layers = if saved_layers.is_empty() {
        vec![KeymapLayerLabel {
            id: Rc::from("base"),
            name: Rc::from("Base"),
        }]
    } else {
        saved_layers
            .iter()
            .map(|layer| KeymapLayerLabel {
                id: Rc::from(layer.id.as_str()),
                name: Rc::from(layer.name.as_str()),
            })
            .collect()
    };
    let colors = document
        .keycaps
        .as_ref()
        .and_then(|keycaps| keycaps.boards.get(board_id));
    let default_colors = KeycapBoardSettings::default();
    let board_color = colors.map_or(default_colors.color.as_str(), |settings| {
        settings.color.as_str()
    });
    let macros = saved_map.map_or(&[][..], |map| map.macros.as_slice());
    let active_layer = saved_layers.get(layer_index);

    let keys = document
        .parts
        .iter()
        .filter(|part| board.part_ids.iter().any(|id| id == &part.id))
        .filter(|part| is_supported_key(document, part))
        .map(|part| {
            let settings = document
                .keycaps
                .as_ref()
                .and_then(|keycaps| keycaps.keys.get(&part.id));
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id);
            let size = keycap_size(part, definition, settings);
            let pose = snapshot
                .scene
                .transforms
                .iter()
                .find(|transform| transform.id == part.id)
                .map_or(part.pose, |transform| transform.pose);
            let title = binding_title_for(
                document,
                board_id,
                &part.id,
                active_layer,
                layer_index,
                saved_layers,
                macros,
            );
            let reference: Rc<str> = Rc::from(part.reference.as_str());
            let search_index = format!("{} {}", reference, title).to_lowercase();
            KeymapKey {
                id: Rc::from(part.id.as_str()),
                reference,
                pose,
                binding_title: Rc::from(title.as_str()),
                color: Rc::from(
                    settings
                        .and_then(|settings| settings.color.as_deref())
                        .unwrap_or(board_color),
                ),
                size,
                search_index,
            }
        })
        .collect();

    Some(Rc::new(KeymapView { layers, keys }))
}

fn is_supported_key(document: &ProjectDoc, part: &Part) -> bool {
    document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)
        .is_some_and(|definition| definition.kind == PartKind::Switch)
        || document.matrices.iter().any(|matrix| {
            matrix.part_ids.iter().any(|id| id == &part.id)
                && part
                    .id
                    .strip_prefix(&format!("matrix/{}/", matrix.id))
                    .is_some_and(|suffix| !suffix.contains('/'))
        })
}

fn keycap_size(
    part: &Part,
    definition: Option<&boardstudio_model::model::PartDefinition>,
    settings: Option<&KeycapKeySettings>,
) -> Vec2 {
    settings.and_then(|settings| settings.units).map_or_else(
        || {
            part.keycap
                .or_else(|| definition.and_then(|definition| definition.keycap))
                .unwrap_or(Vec2 { x: 18.2, y: 18.2 })
        },
        |units| Vec2 {
            x: units.x * 19.05 - 0.85,
            y: units.y * 19.05 - 0.85,
        },
    )
}

fn binding_title_for(
    document: &ProjectDoc,
    board_id: &str,
    key_id: &str,
    layer: Option<&KeymapLayer>,
    layer_index: usize,
    layers: &[KeymapLayer],
    macros: &[KeymapMacro],
) -> String {
    if let Some(binding) = layer.and_then(|layer| layer.bindings.get(key_id)) {
        return binding_title(binding, layers, macros);
    }
    if layer_index > 0 {
        return binding_title(&KeyBinding::Transparent, layers, macros);
    }
    let legacy = document
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|board| board.board_id == board_id)
        })
        .and_then(|board| board.key_bindings.get(key_id));
    match legacy.map(String::as_str) {
        Some(value) if value.starts_with("&kp ") => value[4..].to_owned(),
        Some("&trans") => "Transparent".to_owned(),
        _ => "Unassigned".to_owned(),
    }
}

fn binding_title(binding: &KeyBinding, layers: &[KeymapLayer], macros: &[KeymapMacro]) -> String {
    match binding {
        KeyBinding::KeyPress { keycode } | KeyBinding::StickyKey { keycode } => keycode.clone(),
        KeyBinding::ModTap { hold, tap } => format!("{tap} / {hold}"),
        KeyBinding::LayerTap { layer_id, tap } => format!(
            "{tap} / {}",
            layers
                .iter()
                .find(|layer| layer.id == *layer_id)
                .map_or("?", |layer| layer.name.as_str())
        ),
        KeyBinding::MomentaryLayer { layer_id }
        | KeyBinding::ToggleLayer { layer_id }
        | KeyBinding::ToLayer { layer_id }
        | KeyBinding::StickyLayer { layer_id } => layers
            .iter()
            .find(|layer| layer.id == *layer_id)
            .map_or_else(|| "?".to_owned(), |layer| layer.name.clone()),
        KeyBinding::Macro { macro_id } => macros
            .iter()
            .find(|item| item.id == *macro_id)
            .map_or_else(|| "Macro".to_owned(), |item| item.name.clone()),
        KeyBinding::Transparent => "Transparent".to_owned(),
        KeyBinding::None => "Unassigned".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;
    use boardstudio_model::model::{Board, KeymapChange, Matrix, MatrixCell, PartDefinition, Side};
    use std::sync::Arc;

    fn scope() -> Scope {
        Scope {
            session_epoch: SessionEpoch(1),
            document_id: "project".into(),
            board_id: "board".into(),
            instance_id: None,
        }
    }

    fn snapshot(document: ProjectDoc) -> AcceptedSnapshot {
        let scene: boardstudio_model::model::SceneDelta =
            serde_json::from_value(serde_json::json!({
                "revision": 0,
                "transactionId": "fixture",
                "changedIds": [],
                "transforms": [],
                "matrixScenes": [],
                "contours": [],
                "boardContours": [],
                "boardReadiness": [],
                "findings": [],
                "readiness": {"layout": true, "outline": true, "pcb": true, "case": false}
            }))
            .expect("fixture scene");
        AcceptedSnapshot {
            session_epoch: SessionEpoch(1),
            token: boardstudio_application::SnapshotToken(7),
            document: Arc::new(document),
            scene: Arc::new(scene),
        }
    }

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Keymap fixture");
        document.boards.push(Board {
            id: "board".into(),
            name: "Main".into(),
            outline_ids: vec![],
            part_ids: vec!["sw1".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id": "switch:base", "name": "MX switch", "kind": "switch",
                "courtyard": [], "pads": []
            }))
            .unwrap(),
        );
        document.parts.push(Part {
            id: "sw1".into(),
            definition_id: "switch:base".into(),
            reference: "SW1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            keycap: None,
            outline: None,
            anchor: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        document.keymap = Some(
            serde_json::from_value(serde_json::json!({
                "layers": [{"id": "base", "name": "Base", "bindings": {"sw1": {"kind": "key-press", "keycode": "A"}}, "sensors": {}}],
                "macros": []
            }))
            .unwrap(),
        );
        document
    }

    #[test]
    fn project_supplies_one_layer_and_key_with_binding_title() {
        let snapshot = snapshot(document());
        let view = project(&snapshot, Some(&scope()), "board", "base").expect("view");
        assert_eq!(view.layers.len(), 1);
        assert_eq!(view.layers[0].id.as_ref(), "base");
        assert_eq!(view.keys.len(), 1);
        assert_eq!(view.keys[0].reference.as_ref(), "SW1");
        assert_eq!(view.keys[0].binding_title.as_ref(), "A");
        assert!(view.keys[0].search_index.contains("sw1"));
    }

    #[test]
    fn project_rejects_stale_scope_or_missing_board() {
        let snapshot = snapshot(document());
        let mut elsewhere = scope();
        elsewhere.board_id = "other".into();
        assert!(project(&snapshot, Some(&elsewhere), "board", "base").is_none());
        assert!(project(&snapshot, None, "board", "base").is_none());
        assert!(project(&snapshot, Some(&scope()), "missing", "base").is_none());
    }

    #[test]
    fn project_falls_back_to_base_layer_without_saved_keymap() {
        let mut bare = document();
        bare.keymap = None;
        let snapshot = snapshot(bare);
        let view = project(&snapshot, Some(&scope()), "board", "ghost").expect("view");
        assert_eq!(view.layers.len(), 1);
        assert_eq!(view.layers[0].id.as_ref(), "base");
        assert_eq!(view.keys[0].binding_title.as_ref(), "Unassigned");
    }
}
