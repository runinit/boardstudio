use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::{
    keymap::{KeyBinding, KeymapConfiguration, KeymapLayer},
    model::{KeycapBoardSettings, KeycapKeySettings, Part, PartKind, Pose2, Vec2},
};

#[derive(Clone)]
pub(super) struct KeymapView {
    pub board_id: String,
    pub layers: Vec<KeymapLayer>,
    pub active_layer: KeymapLayer,
    pub keys: Vec<KeymapKey>,
}

#[derive(Clone)]
pub(super) struct KeymapKey {
    pub part: Part,
    pub pose: Pose2,
    pub binding_title: String,
    pub color: String,
    pub size: Vec2,
}

/// Build the single accepted-snapshot projection shared by the canvas and panel.
pub(super) fn project(
    snapshot: &AcceptedSnapshot,
    scope: Option<&Scope>,
    board_id: &str,
    active_layer_id: &str,
) -> Option<KeymapView> {
    let scope = scope?;
    if scope.session_epoch != snapshot.session_epoch
        || scope.document_id != snapshot.document.id
        || scope.board_id != board_id
    {
        return None;
    }

    let document = &snapshot.document;
    let board = document.boards.iter().find(|board| board.id == board_id)?;
    let keymap = document
        .keymap
        .as_ref()
        .filter(|keymap| !keymap.layers.is_empty())
        .cloned()
        .unwrap_or_else(KeymapConfiguration::default);
    let active_layer_index = keymap
        .layers
        .iter()
        .position(|layer| layer.id == active_layer_id)
        .unwrap_or(0);
    let active_layer = keymap.layers.get(active_layer_index)?.clone();
    let colors = document
        .keycaps
        .as_ref()
        .and_then(|keycaps| keycaps.boards.get(board_id))
        .cloned()
        .unwrap_or_else(KeycapBoardSettings::default);

    let keys = document
        .parts
        .iter()
        .filter(|part| board.part_ids.iter().any(|id| id == &part.id))
        .filter(|part| {
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
        })
        .map(|part| {
            let binding = binding_for(document, board_id, part, &keymap, active_layer_index);
            let settings = document
                .keycaps
                .as_ref()
                .and_then(|keycaps| keycaps.keys.get(&part.id))
                .cloned()
                .unwrap_or_else(KeycapKeySettings::default);
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id);
            let size = settings.units.map_or_else(
                || {
                    part.keycap
                        .or_else(|| definition.and_then(|definition| definition.keycap))
                        .unwrap_or(Vec2 { x: 18.2, y: 18.2 })
                },
                |units| Vec2 {
                    x: units.x * 19.05 - 0.85,
                    y: units.y * 19.05 - 0.85,
                },
            );
            let pose = snapshot
                .scene
                .transforms
                .iter()
                .find(|transform| transform.id == part.id)
                .map_or(part.pose, |transform| transform.pose);
            KeymapKey {
                part: part.clone(),
                pose,
                binding_title: binding_title(&binding, &keymap),
                color: settings.color.unwrap_or_else(|| colors.color.clone()),
                size,
            }
        })
        .collect();

    Some(KeymapView {
        board_id: board_id.to_owned(),
        layers: keymap.layers,
        active_layer,
        keys,
    })
}

fn binding_for(
    document: &boardstudio_core::model::ProjectDoc,
    board_id: &str,
    part: &Part,
    map: &KeymapConfiguration,
    layer_index: usize,
) -> KeyBinding {
    if let Some(binding) = map.layers[layer_index].bindings.get(&part.id) {
        return binding.clone();
    }
    if layer_index != 0 {
        return KeyBinding::Transparent;
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
        .and_then(|board| board.key_bindings.get(&part.id));
    match legacy.map(String::as_str) {
        Some(value) if value.starts_with("&kp ") => KeyBinding::KeyPress {
            keycode: value[4..].to_owned(),
        },
        Some("&trans") => KeyBinding::Transparent,
        _ => KeyBinding::None,
    }
}

fn binding_title(binding: &KeyBinding, map: &KeymapConfiguration) -> String {
    match binding {
        KeyBinding::KeyPress { keycode } | KeyBinding::StickyKey { keycode } => keycode.clone(),
        KeyBinding::ModTap { hold, tap } => format!("{tap} / {hold}"),
        KeyBinding::LayerTap { layer_id, tap } => format!(
            "{tap} / {}",
            map.layers
                .iter()
                .find(|layer| layer.id == *layer_id)
                .map_or("?", |layer| layer.name.as_str())
        ),
        KeyBinding::MomentaryLayer { layer_id }
        | KeyBinding::ToggleLayer { layer_id }
        | KeyBinding::ToLayer { layer_id }
        | KeyBinding::StickyLayer { layer_id } => map
            .layers
            .iter()
            .find(|layer| layer.id == *layer_id)
            .map_or_else(|| "?".to_owned(), |layer| layer.name.clone()),
        KeyBinding::Macro { macro_id } => map
            .macros
            .iter()
            .find(|item| item.id == *macro_id)
            .map_or_else(|| "Macro".to_owned(), |item| item.name.clone()),
        KeyBinding::Transparent => "Transparent".to_owned(),
        KeyBinding::None => "Unassigned".to_owned(),
    }
}
