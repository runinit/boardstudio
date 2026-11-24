use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_model::model::{
    KeyBinding, KeycapBoardSettings, KeycapKeySettings, KeymapLayer, KeymapMacro, Part,
    PartDefinition, PartKind, Pose2, ProjectDoc, Vec2,
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
    let doc = &snapshot.document;
    let board = doc.boards.iter().find(|b| b.id == board_id)?;
    let saved_map = doc.keymap.as_ref().filter(|k| !k.layers.is_empty());
    let saved_layers = saved_map.map_or(&[][..], |m| m.layers.as_slice());
    let layer_idx = saved_layers
        .iter()
        .position(|l| l.id == active_layer_id)
        .unwrap_or(0);
    let layers = if saved_layers.is_empty() {
        vec![KeymapLayerLabel {
            id: Rc::from("base"),
            name: Rc::from("Base"),
        }]
    } else {
        saved_layers
            .iter()
            .map(|l| KeymapLayerLabel {
                id: Rc::from(l.id.as_str()),
                name: Rc::from(l.name.as_str()),
            })
            .collect()
    };
    let default_color = KeycapBoardSettings::default().color;
    let board_color = doc
        .keycaps
        .as_ref()
        .and_then(|k| k.boards.get(board_id))
        .map_or(default_color.as_str(), |s| s.color.as_str());
    let macros = saved_map.map_or(&[][..], |m| m.macros.as_slice());
    let active_layer = saved_layers.get(layer_idx);

    let keys = doc
        .parts
        .iter()
        .filter(|p| board.part_ids.contains(&p.id) && is_supported_key(doc, p))
        .map(|p| {
            let s = doc.keycaps.as_ref().and_then(|k| k.keys.get(&p.id));
            let def = doc.definitions.iter().find(|d| d.id == p.definition_id);
            let pose = snapshot
                .scene
                .transforms
                .iter()
                .find(|t| t.id == p.id)
                .map_or(p.pose, |t| t.pose);
            let title = binding_title_for(
                doc,
                board_id,
                &p.id,
                active_layer,
                layer_idx,
                saved_layers,
                macros,
            );
            KeymapKey {
                id: Rc::from(p.id.as_str()),
                reference: Rc::from(p.reference.as_str()),
                pose,
                binding_title: Rc::from(title.as_str()),
                color: Rc::from(s.and_then(|s| s.color.as_deref()).unwrap_or(board_color)),
                size: keycap_size(p, def, s),
                search_index: format!("{} {title}", p.reference).to_lowercase(),
            }
        })
        .collect();
    Some(Rc::new(KeymapView { layers, keys }))
}

fn is_supported_key(doc: &ProjectDoc, part: &Part) -> bool {
    doc.definitions
        .iter()
        .find(|d| d.id == part.definition_id)
        .is_some_and(|d| d.kind == PartKind::Switch)
        || doc.matrices.iter().any(|m| {
            m.part_ids.contains(&part.id)
                && part
                    .id
                    .strip_prefix(&format!("matrix/{}/", m.id))
                    .is_some_and(|s| !s.contains('/'))
        })
}

fn keycap_size(part: &Part, def: Option<&PartDefinition>, s: Option<&KeycapKeySettings>) -> Vec2 {
    s.and_then(|s| s.units).map_or_else(
        || {
            part.keycap
                .or_else(|| def.and_then(|d| d.keycap))
                .unwrap_or(Vec2 { x: 18.2, y: 18.2 })
        },
        |u| Vec2 {
            x: u.x * 19.05 - 0.85,
            y: u.y * 19.05 - 0.85,
        },
    )
}

fn binding_title_for(
    doc: &ProjectDoc,
    board_id: &str,
    key_id: &str,
    layer: Option<&KeymapLayer>,
    layer_idx: usize,
    layers: &[KeymapLayer],
    macros: &[KeymapMacro],
) -> String {
    if let Some(b) = layer.and_then(|l| l.bindings.get(key_id)) {
        return binding_title(b, layers, macros);
    }
    if layer_idx > 0 {
        return "Transparent".into();
    }
    match doc
        .hardware
        .as_ref()
        .and_then(|h| h.boards.iter().find(|b| b.board_id == board_id))
        .and_then(|b| b.key_bindings.get(key_id))
        .map(String::as_str)
    {
        Some(v) if v.starts_with("&kp ") => v[4..].to_owned(),
        Some("&trans") => "Transparent".into(),
        _ => "Unassigned".into(),
    }
}

fn binding_title(b: &KeyBinding, layers: &[KeymapLayer], macros: &[KeymapMacro]) -> String {
    let layer_name = |id: &str| {
        layers
            .iter()
            .find(|l| l.id == id)
            .map_or("?", |l| l.name.as_str())
            .to_owned()
    };
    match b {
        KeyBinding::KeyPress { keycode } | KeyBinding::StickyKey { keycode } => keycode.clone(),
        KeyBinding::ModTap { hold, tap } => format!("{tap} / {hold}"),
        KeyBinding::LayerTap { layer_id, tap } => format!("{tap} / {}", layer_name(layer_id)),
        KeyBinding::MomentaryLayer { layer_id }
        | KeyBinding::ToggleLayer { layer_id }
        | KeyBinding::ToLayer { layer_id }
        | KeyBinding::StickyLayer { layer_id } => layer_name(layer_id),
        KeyBinding::Macro { macro_id } => macros
            .iter()
            .find(|m| m.id == *macro_id)
            .map_or("Macro", |m| m.name.as_str())
            .to_owned(),
        KeyBinding::Transparent => "Transparent".into(),
        KeyBinding::None => "Unassigned".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;
    use std::sync::Arc;

    #[test]
    fn project_builds_view_and_rejects_mismatched_scope() {
        let mut doc = ProjectDoc::empty("p", "K");
        doc.boards.push(serde_json::from_value(serde_json::json!({
            "id": "b", "name": "Main", "outlineIds": [], "partIds": ["sw1"], "netIds": [], "thickness": 1.6, "traces": [], "vias": []
        })).unwrap());
        doc.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id": "sw", "name": "MX", "kind": "switch", "courtyard": [], "pads": []
            }))
            .unwrap(),
        );
        doc.parts.push(serde_json::from_value(serde_json::json!({
            "id": "sw1", "definitionId": "sw", "reference": "SW1", "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front"
        })).unwrap());
        let scene: boardstudio_model::model::SceneDelta = serde_json::from_value(serde_json::json!({
            "revision": 0, "transactionId": "t", "changedIds": [], "transforms": [], "matrixScenes": [],
            "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
            "readiness": {"layout": true, "outline": true, "pcb": true, "case": false}
        })).unwrap();
        let snap = AcceptedSnapshot {
            session_epoch: SessionEpoch(1),
            token: boardstudio_application::SnapshotToken(1),
            document: Arc::new(doc),
            scene: Arc::new(scene),
        };
        let scope = Scope {
            session_epoch: SessionEpoch(1),
            document_id: "p".into(),
            board_id: "b".into(),
            instance_id: None,
        };
        let view = project(&snap, Some(&scope), "b", "base").expect("view");
        assert_eq!((view.layers.len(), view.keys.len()), (1, 1));
        assert!(project(&snap, None, "b", "base").is_none());
    }
}
