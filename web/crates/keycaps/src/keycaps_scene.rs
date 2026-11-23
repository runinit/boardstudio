//! Accepted-snapshot projection and physical 2D scene for the Keycaps workspace.
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_model::model::{
    Contour, KeyBinding, KeycapBoardSettings, KeycapKeySettings, KeycapMatrixSettings, Part,
    PartDefinition, PartKind, Pose2, ProjectDoc, Vec2,
};
use boardstudio_web_ui_shared::inspector_fields::{SelectField, TextField};
use boardstudio_web_ui_shared::ui::input::InputType;
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapsView {
    pub board_id: Rc<str>,
    pub board_settings: KeycapBoardSettings,
    pub keys: Vec<KeycapsKey>,
    pub matrices: Vec<KeycapsMatrix>,
    pub assigned_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapsKey {
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
pub enum LegendSource {
    Binding,
    Explicit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapsMatrix {
    pub id: Rc<str>,
    pub name: Rc<str>,
    pub settings: KeycapMatrixSettings,
}

pub fn project(
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
    let doc = &snapshot.document;
    let board = doc.boards.iter().find(|b| b.id == active_board_id)?;
    let board_settings = doc
        .keycaps
        .as_ref()
        .and_then(|k| k.boards.get(active_board_id))
        .cloned()
        .unwrap_or_default();
    let keys: Vec<_> = doc
        .parts
        .iter()
        .filter(|p| board.part_ids.contains(&p.id) && is_supported_key(doc, p))
        .map(|p| {
            let s = doc.keycaps.as_ref().and_then(|k| k.keys.get(&p.id));
            let def = doc.definitions.iter().find(|d| d.id == p.definition_id);
            let binding_label = key_binding_label(doc, active_board_id, &p.id);
            let (legend, legend_source) = s.and_then(|s| s.legend.as_deref()).map_or_else(
                || (binding_label.clone(), LegendSource::Binding),
                |l| (l.to_owned(), LegendSource::Explicit),
            );
            let color = s
                .and_then(|s| s.color.as_deref())
                .unwrap_or(&board_settings.color);
            let pose = snapshot
                .scene
                .transforms
                .iter()
                .find(|t| t.id == p.id)
                .map_or(p.pose, |t| t.pose);
            KeycapsKey {
                id: Rc::from(p.id.as_str()),
                reference: Rc::from(p.reference.as_str()),
                pose,
                size: keycap_size(p, def, s),
                color: Rc::from(color),
                legend: Rc::from(legend.as_str()),
                legend_source,
                binding_label: Rc::from(binding_label.as_str()),
                search_text: format!("{} {binding_label}", p.reference).to_lowercase(),
            }
        })
        .collect();
    let assigned_count = keys
        .iter()
        .filter(|k| key_binding(doc, active_board_id, &k.id) != "&none")
        .count();
    let matrices = doc
        .matrices
        .iter()
        .filter(|m| {
            m.board_id.as_deref() == Some(active_board_id)
                || m.part_ids.iter().any(|id| board.part_ids.contains(id))
        })
        .map(|m| KeycapsMatrix {
            id: Rc::from(m.id.as_str()),
            name: Rc::from(m.name.as_deref().unwrap_or(&m.id)),
            settings: doc
                .keycaps
                .as_ref()
                .and_then(|k| k.matrices.get(&m.id))
                .cloned()
                .unwrap_or_default(),
        })
        .collect();
    Some(Rc::new(KeycapsView {
        board_id: Rc::from(active_board_id),
        board_settings,
        keys,
        matrices,
        assigned_count,
    }))
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

fn key_binding(doc: &ProjectDoc, board_id: &str, key_id: &str) -> String {
    if let Some(b) = doc
        .keymap
        .as_ref()
        .and_then(|k| k.layers.first())
        .and_then(|l| l.bindings.get(key_id))
    {
        return match b {
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
    doc.hardware
        .as_ref()
        .and_then(|h| h.boards.iter().find(|b| b.board_id == board_id))
        .and_then(|b| b.key_bindings.get(key_id))
        .cloned()
        .unwrap_or_else(|| "&none".into())
}

fn key_binding_label(doc: &ProjectDoc, board_id: &str, key_id: &str) -> String {
    let b = key_binding(doc, board_id, key_id);
    if b == "&none" || b == "&trans" {
        return String::new();
    }
    let Some(c) = b.strip_prefix("&kp ") else {
        return b;
    };
    match c {
        "SPACE" => "Space",
        "ENTER" => "Enter",
        "ESC" => "Esc",
        "TAB" => "Tab",
        "BSPC" => "Backspace",
        "LSHFT" => "Shift",
        "LCTRL" => "Ctrl",
        "LALT" => "Alt",
        "LGUI" => "Super",
        "UP" => "Up",
        "DOWN" => "Down",
        "LEFT" => "Left",
        "RIGHT" => "Right",
        "MINUS" => "-",
        "EQUAL" => "=",
        "LBKT" => "[",
        "RBKT" => "]",
        "BSLH" => "\\",
        "SEMI" => ";",
        "SQT" => "'",
        "COMMA" => ",",
        "DOT" => ".",
        "FSLH" => "/",
        "GRAVE" => "`",
        _ if c.len() == 2 && c.starts_with('N') && c.as_bytes()[1].is_ascii_digit() => &c[1..],
        _ => c,
    }
    .to_owned()
}

#[component]
pub fn KeycapsCanvas(
    view: Rc<KeycapsView>,
    contours: Rc<[Contour]>,
    selected_ids: BTreeSet<String>,
    on_select_key: EventHandler<String>,
) -> Element {
    rsx! {
        g { "data-board-id": "{view.board_id}",
            g { "aria-hidden": "true", "pointer-events": "none",
                for (i, c) in contours.iter().enumerate() {
                    polygon {
                        key: "outline-{i}",
                        points: c.points.iter().map(|p| format!("{},{}", p.x, p.y)).collect::<Vec<_>>().join(" "),
                        class: if c.hole { "fill-canvas stroke-destructive stroke-[0.3] [stroke-dasharray:0.8_0.55]" } else { "fill-geometry-fill stroke-geometry-board stroke-[0.3]" },
                    }
                }
            }
            for key in &view.keys {
                {
                    let (id, sel) = (key.id.clone(), selected_ids.contains(key.id.as_ref()));
                    let (click_id, kb_id) = (id.clone(), id.clone());
                    let fg = foreground_color(&key.color);
                    let font_size = (4.0_f64).min(13.0 / (key.legend.encode_utf16().count().max(1) as f64) * 1.3);
                    rsx! {
                        g {
                            key: "{id}",
                            class: if sel { "cursor-pointer outline-none [&_rect]:stroke-primary [&_rect]:stroke-[0.8] [&_text]:pointer-events-none" } else { "cursor-pointer outline-none [&_rect]:stroke-border-strong [&_rect]:stroke-[0.2] hover:[&_rect]:stroke-primary hover:[&_rect]:stroke-[0.5] [&_text]:pointer-events-none" },
                            transform: "translate({key.pose.at.x} {key.pose.at.y}) rotate({key.pose.rotation})",
                            role: "button", tabindex: "0", "aria-pressed": sel, "aria-label": "Edit key {key.reference}",
                            onclick: move |e| { e.stop_propagation(); on_select_key.call(click_id.to_string()); },
                            onkeydown: move |e: KeyboardEvent| {
                                let k = e.data().key().to_string();
                                if k == "Enter" || k == " " { e.prevent_default(); e.stop_propagation(); on_select_key.call(kb_id.to_string()); }
                            },
                            rect { x: "{-key.size.x / 2.0}", y: "{-key.size.y / 2.0}", width: "{key.size.x}", height: "{key.size.y}", rx: "1.3", fill: "{key.color}" }
                            g { transform: "scale(1 -1)",
                                text { text_anchor: "middle", dominant_baseline: "central", fill: "{fg}", font_size: "{font_size}", if key.legend.is_empty() { "—" } else { "{key.legend}" } }
                                text { text_anchor: "middle", y: "6", font_size: "2.1", fill: "{fg}", "{key.reference}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn KeycapsKeyList(
    view: Rc<KeycapsView>,
    selected_key_id: Option<String>,
    on_select_key: EventHandler<String>,
) -> Element {
    let mut query = use_signal(String::new);
    let q = query().to_lowercase();
    let selected = selected_key_id
        .filter(|id| view.keys.iter().any(|k| k.id.as_ref() == id.as_str()))
        .unwrap_or_default();
    let mut options = vec![(String::new(), "Choose on the layout…".to_owned())];
    for k in view
        .keys
        .iter()
        .filter(|k| k.search_text.contains(q.as_str()))
    {
        let lbl = if k.binding_label.is_empty() {
            "Unassigned"
        } else {
            k.binding_label.as_ref()
        };
        options.push((k.id.to_string(), format!("{} · {lbl}", k.reference)));
    }
    let no_matches = !query().is_empty() && options.len() == 1;
    rsx! {
        section { class: "grid min-w-0 gap-3", "aria-label": "Keycaps key selection",
            TextField {
                id: format!("keycaps-key-search-{}", dioxus::core::current_scope_id().0),
                label: "Find a key".to_owned(), aria_label: "Find a key", r#type: InputType::Search,
                value: query(), oninput: move |e: FormEvent| query.set(e.value()),
            }
            SelectField::<String> {
                label: "Selected key".to_owned(), aria_label: "Selected key",
                value: selected, options, placeholder: "Choose on the layout…",
                on_commit: move |v: String| on_select_key.call(v),
            }
            if no_matches { p { role: "status", "No matching keys." } }
        }
    }
}

fn foreground_color(color: &str) -> &'static str {
    let Some(rgb) = color
        .strip_prefix('#')
        .and_then(|h| u32::from_str_radix(h, 16).ok())
    else {
        return "#ffffff";
    };
    let (r, g, b) = (
        ((rgb >> 16) & 255) as f64,
        ((rgb >> 8) & 255) as f64,
        (rgb & 255) as f64,
    );
    if r * 0.299 + g * 0.587 + b * 0.114 > 150.0 {
        "#182331"
    } else {
        "#ffffff"
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn foreground_color_picks_contrast_and_keycap_size_scales_units() {
        assert_eq!(foreground_color("#ffffff"), "#182331");
        assert_eq!(foreground_color("#101010"), "#ffffff");
        let part = Part {
            id: "k1".into(),
            reference: "K1".into(),
            definition_id: "sw".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: boardstudio_model::model::Side::Front,
            keycap: None,
            outline: None,
            anchor: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        };
        let s = KeycapKeySettings {
            units: Some(Vec2 { x: 2.0, y: 1.0 }),
            ..Default::default()
        };
        let sz = keycap_size(&part, None, Some(&s));
        assert!((sz.x - (2.0 * 19.05 - 0.85)).abs() < 1e-6);
    }
}
