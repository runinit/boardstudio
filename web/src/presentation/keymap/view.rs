use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::{
    keymap::{KeyBinding, KeymapLayer, KeymapMacro},
    model::{KeycapBoardSettings, KeycapKeySettings, Part, PartKind, Pose2, ProjectDoc, Vec2},
};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) struct KeymapView {
    pub layers: Vec<KeymapLayerLabel>,
    pub keys: Vec<KeymapKey>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) struct KeymapLayerLabel {
    pub id: Rc<str>,
    pub name: Rc<str>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) struct KeymapKey {
    pub id: Rc<str>,
    pub reference: Rc<str>,
    pub pose: Pose2,
    pub binding_title: Rc<str>,
    pub color: Rc<str>,
    pub size: Vec2,
    pub search_index: String,
}

/// Build the single accepted-snapshot projection shared by the canvas and panel.
pub(in crate::presentation) fn project(
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
    definition: Option<&boardstudio_core::model::PartDefinition>,
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
