//! Accepted-snapshot projection and physical 2D scene for the Keycaps workspace.
//!
//! The page parent owns route composition and canonical selection. This module is a
//! controlled view over the accepted document and emits stable part IDs only.
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::model::{
    Contour, KeyBinding, KeycapBoardSettings, KeycapKeySettings, Part, PartDefinition, PartKind,
    Pose2, ProjectDoc, Vec2,
};
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct KeycapsView {
    pub board_id: Rc<str>,
    pub keys: Vec<KeycapsKey>,
    pub matrices: Vec<KeycapsMatrix>,
    pub assigned_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct KeycapsKey {
    pub id: Rc<str>,
    pub reference: Rc<str>,
    pub pose: Pose2,
    pub size: Vec2,
    pub color: Rc<str>,
    pub legend: Rc<str>,
    pub legend_source: LegendSource,
    pub binding_label: Rc<str>,
    pub search_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LegendSource {
    Binding,
    Explicit,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct KeycapsMatrix {
    pub id: Rc<str>,
    pub name: Rc<str>,
}

/// Build the physical Keycaps view from one accepted snapshot and its active board scope.
pub(super) fn project(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
    active_board_id: &str,
) -> Option<Rc<KeycapsView>> {
    if scope.session_epoch != snapshot.session_epoch
        || scope.document_id != snapshot.document.id
        || scope.board_id != active_board_id
    {
        return None;
    }

    let document = &snapshot.document;
    let board = document
        .boards
        .iter()
        .find(|board| board.id == active_board_id)?;
    let defaults = KeycapBoardSettings::default();
    let board_settings = document
        .keycaps
        .as_ref()
        .and_then(|settings| settings.boards.get(active_board_id));
    let board_color =
        board_settings.map_or(defaults.color.as_str(), |settings| settings.color.as_str());
    let keys: Vec<_> = document
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
            let binding_label = key_binding_label(document, active_board_id, &part.id);
            let (legend, legend_source) = settings
                .and_then(|settings| settings.legend.as_deref())
                .map_or_else(
                    || (binding_label.clone(), LegendSource::Binding),
                    |legend| (legend.to_owned(), LegendSource::Explicit),
                );
            let color = settings
                .and_then(|settings| settings.color.as_deref())
                .unwrap_or(board_color);
            let pose = snapshot
                .scene
                .transforms
                .iter()
                .find(|transform| transform.id == part.id)
                .map_or(part.pose, |transform| transform.pose);
            let reference = part.reference.as_str();
            KeycapsKey {
                id: Rc::from(part.id.as_str()),
                reference: Rc::from(reference),
                pose,
                size: keycap_size(part, definition, settings),
                color: Rc::from(color),
                legend: Rc::from(legend.as_str()),
                legend_source,
                binding_label: Rc::from(binding_label.as_str()),
                search_text: format!("{reference} {binding_label}").to_lowercase(),
            }
        })
        .collect();
    let assigned_count = keys
        .iter()
        .filter(|key| key_binding(document, active_board_id, &key.id) != "&none")
        .count();
    let matrices = document
        .matrices
        .iter()
        .filter(|matrix| {
            matrix.board_id.as_deref() == Some(active_board_id)
                || matrix.part_ids.iter().any(|part_id| {
                    board
                        .part_ids
                        .iter()
                        .any(|board_part_id| board_part_id == part_id)
                })
        })
        .map(|matrix| KeycapsMatrix {
            id: Rc::from(matrix.id.as_str()),
            name: Rc::from(matrix.name.as_deref().unwrap_or(&matrix.id)),
        })
        .collect();

    Some(Rc::new(KeycapsView {
        board_id: Rc::from(active_board_id),
        keys,
        matrices,
        assigned_count,
    }))
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
    definition: Option<&PartDefinition>,
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

fn key_binding(document: &ProjectDoc, board_id: &str, key_id: &str) -> String {
    let saved_binding = document
        .keymap
        .as_ref()
        .and_then(|keymap| keymap.layers.first())
        .and_then(|layer| layer.bindings.get(key_id));
    if let Some(binding) = saved_binding {
        return match binding {
            KeyBinding::KeyPress { keycode } | KeyBinding::StickyKey { keycode } => {
                format!("&kp {keycode}")
            }
            KeyBinding::ModTap { tap, .. } | KeyBinding::LayerTap { tap, .. } => {
                format!("&kp {tap}")
            }
            KeyBinding::Transparent => "&trans".into(),
            _ => "&none".into(),
        };
    }
    document
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|board| board.board_id == board_id)
        })
        .and_then(|board| board.key_bindings.get(key_id))
        .cloned()
        .unwrap_or_else(|| "&none".into())
}

fn key_binding_label(document: &ProjectDoc, board_id: &str, key_id: &str) -> String {
    let binding = key_binding(document, board_id, key_id);
    if binding == "&none" || binding == "&trans" {
        return String::new();
    }
    if let Some(code) = binding.strip_prefix("&kp ") {
        return match code {
            "SPACE" => "Space".into(),
            "ENTER" => "Enter".into(),
            "ESC" => "Esc".into(),
            "TAB" => "Tab".into(),
            "BSPC" => "Backspace".into(),
            "LSHFT" => "Shift".into(),
            "LCTRL" => "Ctrl".into(),
            "LALT" => "Alt".into(),
            "LGUI" => "Super".into(),
            "UP" => "Up".into(),
            "DOWN" => "Down".into(),
            "LEFT" => "Left".into(),
            "RIGHT" => "Right".into(),
            "MINUS" => "-".into(),
            "EQUAL" => "=".into(),
            "LBKT" => "[".into(),
            "RBKT" => "]".into(),
            "BSLH" => "\\".into(),
            "SEMI" => ";".into(),
            "SQT" => "'".into(),
            "COMMA" => ",".into(),
            "DOT" => ".".into(),
            "FSLH" => "/".into(),
            "GRAVE" => "`".into(),
            code if code.len() == 1 && code.as_bytes()[0].is_ascii_uppercase() => code.into(),
            code if code.len() == 2
                && code.starts_with('N')
                && code.as_bytes()[1].is_ascii_digit() =>
            {
                code[1..].into()
            }
            code if code.strip_prefix('F').is_some_and(|suffix| {
                suffix
                    .parse::<u8>()
                    .is_ok_and(|number| (1..=12).contains(&number))
            }) =>
            {
                code.into()
            }
            _ => code.into(),
        };
    }
    binding
}

/// Render physical keycaps into the active board SVG coordinate system.
#[component]
pub(super) fn KeycapsCanvas(
    view: Rc<KeycapsView>,
    contours: Rc<[Contour]>,
    selected_ids: BTreeSet<String>,
    on_select_key: EventHandler<String>,
) -> Element {
    rsx! {
        g { class: "m1-keycaps-layout", "data-board-id": "{view.board_id}",
            g { class: "m1-keycaps-outline", "aria-hidden": "true", "pointer-events": "none",
                for (index, contour) in contours.iter().enumerate() {
                    polygon {
                        key: "outline-{index}",
                        points: contour_points(&contour.points),
                        class: if contour.hole { "m1-outline is-hole" } else { "m1-outline" },
                    }
                }
            }
            for key in &view.keys {
                {
                    let id = key.id.clone();
                    let click_id = id.clone();
                    let keyboard_id = id.clone();
                    let reference = key.reference.clone();
                    let selected = selected_ids.contains(id.as_ref());
                    let color = key.color.clone();
                    let pose = key.pose;
                    let size = key.size;
                    let legend = key.legend.clone();
                    let foreground = foreground_color(&color);
                    let font_size = (4.0_f64).min(
                        13.0 / (legend.encode_utf16().count().max(1) as f64) * 1.3,
                    );
                    rsx! {
                        g {
                            key: "{id}",
                            class: if selected { "m1-keymap-key is-selected" } else { "m1-keymap-key" },
                            transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation})",
                            role: "button",
                            tabindex: "0",
                            "aria-pressed": selected,
                            "aria-label": "Edit key {reference}",
                            onclick: move |event| {
                                event.stop_propagation();
                                on_select_key.call(click_id.to_string());
                            },
                            onkeydown: move |event: KeyboardEvent| {
                                let pressed = event.data().key().to_string();
                                if pressed == "Enter" || pressed == " " {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    on_select_key.call(keyboard_id.to_string());
                                }
                            },
                            rect {
                                x: "{-size.x / 2.0}",
                                y: "{-size.y / 2.0}",
                                width: "{size.x}",
                                height: "{size.y}",
                                rx: "1.3",
                                fill: "{color}",
                            }
                            g { transform: "scale(1 -1)",
                                text {
                                    text_anchor: "middle",
                                    dominant_baseline: "central",
                                    fill: "{foreground}",
                                    font_size: "{font_size}",
                                    if legend.is_empty() { "—" } else { "{legend}" }
                                }
                                text {
                                    text_anchor: "middle",
                                    y: "6",
                                    font_size: "2.1",
                                    fill: "{foreground}",
                                    "{reference}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn contour_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ")
}

#[component]
pub(super) fn KeycapsKeyList(
    view: Rc<KeycapsView>,
    selected_key_id: Option<String>,
    on_select_key: EventHandler<String>,
) -> Element {
    let mut query = use_signal(String::new);
    let normalized_query = query().to_lowercase();
    // Match React's resolved selected key: an ID outside this accepted projection
    // has no selected option, without emitting a selection change.
    let selected_value = selected_key_id
        .filter(|id| view.keys.iter().any(|key| key.id.as_ref() == id.as_str()))
        .unwrap_or_default();
    let visible_keys = view
        .keys
        .iter()
        .filter(|key| key.search_text.contains(normalized_query.as_str()))
        .collect::<Vec<_>>();
    let no_visible_keys = visible_keys.is_empty();
    rsx! {
        section { class: "m1-keycaps-key-list", "aria-label": "Keycaps key selection",
            label {
                "Find a key",
                input {
                    r#type: "search",
                    "aria-label": "Find a key",
                    value: "{query}",
                    oninput: move |event| query.set(event.value()),
                }
            }
            label {
                "Selected key",
                select {
                    "aria-label": "Selected key",
                    value: "{selected_value}",
                    onchange: move |event| on_select_key.call(event.value()),
                    option { value: "", selected: selected_value.is_empty(), "Choose on the layout…" }
                    for key in &visible_keys {
                        {
                            let binding_label = if key.binding_label.is_empty() {
                                "Unassigned"
                            } else {
                                key.binding_label.as_ref()
                            };
                            let selected = selected_value.as_str() == key.id.as_ref();
                            rsx! {
                                option {
                                    key: "{key.id}",
                                    value: "{key.id}",
                                    selected,
                                    "{key.reference} · {binding_label}"
                                }
                            }
                        }
                    }
                }
            }
            if !query().is_empty() && no_visible_keys {
                p { role: "status", "No matching keys." }
            }
        }
    }
}

#[component]
pub(super) fn KeycapsMatrixList(view: Rc<KeycapsView>) -> Element {
    if view.matrices.is_empty() {
        return rsx! {
            p { class: "m1-keycaps-empty-note", "Standalone switches use their individual profile override." }
        };
    }
    rsx! {
        section { class: "m1-keycaps-matrix-list", "aria-label": "Keycaps matrices",
            h2 { "Matrices" }
            ul {
                for matrix in &view.matrices {
                    li { key: "{matrix.id}", "{matrix.name}" }
                }
            }
        }
    }
}

#[component]
pub(super) fn KeycapsSelectedSummary(
    view: Rc<KeycapsView>,
    selected_key_id: Option<String>,
) -> Element {
    let selected_key = selected_key_id
        .as_deref()
        .and_then(|id| view.keys.iter().find(|key| key.id.as_ref() == id));
    let Some(key) = selected_key else {
        return rsx! {
            section { class: "m1-keycaps-selected-summary", "aria-label": "Selected keycap",
                h2 { "Select a key" }
                p { "Choose a key on the physical view or from the key list." }
            }
        };
    };
    let legend_status = match key.legend_source {
        LegendSource::Binding => "From binding",
        LegendSource::Explicit if key.legend.is_empty() => "Blank keycap",
        LegendSource::Explicit => "Explicit legend",
    };
    let dimensions = format!("{} × {} mm", key.size.x, key.size.y);
    rsx! {
        section { class: "m1-keycaps-selected-summary", "aria-label": "Selected keycap",
            h2 { "{key.reference} · key" }
            dl {
                dt { "Physical size" }
            dd { "{dimensions}" }
            dt { "Keycap color" }
            dd { span { "aria-hidden": "true", class: "m1-keycaps-color-swatch", style: "background-color: {key.color}" } "{key.color}" }
            dt { "Legend" }
            dd { if key.legend.is_empty() { "—" } else { "{key.legend}" } }
            dt { "Legend source" }
            dd { "{legend_status}" }
            }
        }
    }
}

fn foreground_color(color: &str) -> &'static str {
    let Some(hex) = color.strip_prefix('#') else {
        return "#ffffff";
    };
    let Ok(rgb) = u32::from_str_radix(hex, 16) else {
        return "#ffffff";
    };
    let red = ((rgb >> 16) & 255) as f64;
    let green = ((rgb >> 8) & 255) as f64;
    let blue = (rgb & 255) as f64;
    if red * 0.299 + green * 0.587 + blue * 0.114 > 150.0 {
        "#182331"
    } else {
        "#ffffff"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{
        Board, KeyBinding, KeycapConfiguration, KeycapKeySettings, KeymapConfiguration,
        KeymapLayer, Matrix, SceneDelta, Transform,
    };
    use std::{collections::BTreeMap, sync::Arc};

    fn part(id: &str, definition_id: &str, reference: &str) -> Part {
        Part {
            keycap: None,
            outline: None,
            id: id.into(),
            definition_id: definition_id.into(),
            reference: reference.into(),
            pose: Pose2 {
                at: Vec2 { x: 1.0, y: 2.0 },
                rotation: 0.0,
            },
            side: boardstudio_core::model::Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn definition(id: &str, kind: &str, keycap: Option<Vec2>) -> PartDefinition {
        let mut value = serde_json::json!({
            "id": id,
            "name": id,
            "kind": kind,
            "courtyard": [],
            "pads": [],
        });
        if let Some(size) = keycap {
            value["keycap"] = serde_json::json!(size);
        }
        serde_json::from_value(value).unwrap()
    }

    fn board(part_ids: &[&str]) -> Board {
        Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: part_ids.iter().map(|id| (*id).into()).collect(),
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        }
    }

    fn matrix(id: &str, board_id: Option<&str>, part_ids: &[&str], name: Option<&str>) -> Matrix {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": name,
            "rows": 1,
            "columns": 1,
            "pitch": {"x": 19.05, "y": 19.05},
            "origin": {"x": 0.0, "y": 0.0},
            "definitionId": "matrix-switch",
            "partIds": part_ids,
            "boardId": board_id,
        }))
        .unwrap()
    }

    fn snapshot(document: ProjectDoc, transforms: Vec<Transform>) -> AcceptedSnapshot {
        let revision = document.revision;
        AcceptedSnapshot {
            token: SnapshotToken(4),
            session_epoch: SessionEpoch(7),
            document: Arc::new(document),
            scene: Arc::new(SceneDelta {
                module_scenes: vec![],
                revision,
                transaction_id: "fixture".into(),
                changed_ids: vec![],
                transforms,
                matrix_scenes: vec![],
                contours: vec![],
                board_contours: vec![],
                board_readiness: vec![],
                board_outline_scenes: vec![],
                finding_markers: vec![],
                findings: vec![],
                readiness: boardstudio_core::model::Readiness {
                    layout: true,
                    outline: false,
                    pcb: false,
                    case_ready: false,
                },
            }),
        }
    }

    fn scope(board_id: &str) -> Scope {
        Scope {
            session_epoch: SessionEpoch(7),
            document_id: "doc".into(),
            board_id: board_id.into(),
            instance_id: None,
        }
    }

    #[test]
    fn accepted_board_projection_preserves_membership_pose_and_display_fallbacks() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board(&[
            "switch",
            "matrix/m/member",
            "matrix/m/",
            "matrix/m/nested/member",
            "other",
        ]));
        document.definitions.push(definition(
            "switch-def",
            "switch",
            Some(Vec2 { x: 22.0, y: 21.0 }),
        ));
        document
            .definitions
            .push(definition("plain-def", "custom", None));
        document.parts = vec![
            part("switch", "switch-def", "SW1"),
            part("matrix/m/member", "plain-def", "K1"),
            part("matrix/m/", "plain-def", "K2"),
            part("matrix/m/nested/member", "plain-def", "K3"),
            part("not-on-board", "switch-def", "SW2"),
            part("other", "plain-def", "Q1"),
        ];
        document.matrices = vec![matrix(
            "m",
            None,
            &["matrix/m/member", "matrix/m/", "matrix/m/nested/member"],
            None,
        )];
        let accepted = snapshot(
            document,
            vec![Transform {
                id: "switch".into(),
                pose: Pose2 {
                    at: Vec2 { x: 12.5, y: -4.0 },
                    rotation: 30.0,
                },
            }],
        );

        let view = project(&accepted, &scope("board"), "board").unwrap();
        assert_eq!(
            view.keys
                .iter()
                .map(|key| key.id.as_ref())
                .collect::<Vec<_>>(),
            ["switch", "matrix/m/member", "matrix/m/"]
        );
        assert_eq!(view.keys[0].pose.at, Vec2 { x: 12.5, y: -4.0 });
        assert_eq!(view.keys[0].pose.rotation, 30.0);
        assert_eq!(view.keys[0].size, Vec2 { x: 22.0, y: 21.0 });
        assert_eq!(view.keys[1].size, Vec2 { x: 18.2, y: 18.2 });
        assert_eq!(view.matrices[0].name.as_ref(), "m");
        assert_eq!(view.assigned_count, 0);
    }

    #[test]
    fn explicit_blank_keeps_inherited_binding_search_and_settings_are_not_materialized() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board(&["switch"]));
        document
            .definitions
            .push(definition("switch-def", "switch", None));
        document.parts.push(part("switch", "switch-def", "SW1"));
        document.keymap = Some(KeymapConfiguration {
            layers: vec![KeymapLayer {
                id: "base".into(),
                name: "Base".into(),
                bindings: BTreeMap::from([(
                    "switch".into(),
                    KeyBinding::KeyPress {
                        keycode: "SPACE".into(),
                    },
                )]),
                sensors: BTreeMap::new(),
            }],
            macros: vec![],
        });
        document.keycaps = Some(KeycapConfiguration {
            keys: BTreeMap::from([(
                "switch".into(),
                KeycapKeySettings {
                    legend: Some(String::new()),
                    color: Some("#102030".into()),
                    units: Some(Vec2 { x: 1.5, y: 1.25 }),
                    ..KeycapKeySettings::default()
                },
            )]),
            ..KeycapConfiguration::default()
        });
        let before = document.clone();
        let accepted = snapshot(document, vec![]);

        let view = project(&accepted, &scope("board"), "board").unwrap();
        let key = &view.keys[0];
        assert_eq!(key.legend.as_ref(), "");
        assert_eq!(key.legend_source, LegendSource::Explicit);
        assert_eq!(key.binding_label.as_ref(), "Space");
        assert_eq!(key.search_text, "sw1 space");
        assert_eq!(key.color.as_ref(), "#102030");
        assert!((key.size.x - 27.725).abs() < 1e-9);
        assert!((key.size.y - 22.9625).abs() < 1e-9);
        assert_eq!(view.assigned_count, 1);
        assert_eq!(*accepted.document, before);
        assert!(
            accepted
                .document
                .keycaps
                .as_ref()
                .unwrap()
                .boards
                .is_empty()
        );
    }

    #[test]
    fn physical_legend_inheritance_uses_legacy_binding_and_active_scope_is_required() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board(&["switch"]));
        document
            .definitions
            .push(definition("switch-def", "switch", None));
        document.parts.push(part("switch", "switch-def", "SW1"));
        let mut hardware = boardstudio_core::model::HardwareConfiguration::default();
        hardware
            .boards
            .push(boardstudio_core::model::ElectricalBoardConfiguration {
                board_id: "board".into(),
                key_bindings: BTreeMap::from([("switch".into(), "&kp A".into())]),
                ..Default::default()
            });
        document.hardware = Some(hardware);
        let accepted = snapshot(document, vec![]);
        let view = project(&accepted, &scope("board"), "board").unwrap();
        assert_eq!(view.keys[0].legend.as_ref(), "A");
        assert_eq!(view.keys[0].legend_source, LegendSource::Binding);
        assert!(project(&accepted, &scope("wrong-board"), "board").is_none());
        let mut stale = scope("board");
        stale.session_epoch = SessionEpoch(8);
        assert!(project(&accepted, &stale, "board").is_none());
    }

    #[test]
    fn key_binding_labels_match_existing_choices_and_fallbacks() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board(&[]));
        let mut hardware = boardstudio_core::model::HardwareConfiguration::default();
        hardware
            .boards
            .push(boardstudio_core::model::ElectricalBoardConfiguration {
                board_id: "board".into(),
                key_bindings: BTreeMap::from([
                    ("space".into(), "&kp SPACE".into()),
                    ("custom".into(), "&kp CUSTOM".into()),
                    ("transparent".into(), "&trans".into()),
                    ("none".into(), "&none".into()),
                ]),
                ..Default::default()
            });
        document.hardware = Some(hardware);
        assert_eq!(key_binding_label(&document, "board", "space"), "Space");
        assert_eq!(key_binding_label(&document, "board", "custom"), "CUSTOM");
        assert_eq!(key_binding_label(&document, "board", "transparent"), "");
        assert_eq!(key_binding_label(&document, "board", "none"), "");
        assert_eq!(key_binding_label(&document, "board", "missing"), "");
    }

    #[test]
    fn canvas_text_contrast_uses_effective_cap_color_luminance() {
        assert_eq!(foreground_color("#ffffff"), "#182331");
        assert_eq!(foreground_color("#000000"), "#ffffff");
        assert_eq!(foreground_color("#808080"), "#ffffff");
        assert_eq!(foreground_color("not-a-color"), "#ffffff");
    }
}
