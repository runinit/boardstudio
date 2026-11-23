//! Declarative keycap settings editors backed directly by synchronous `runtime.commit`.
use super::keycaps_scene::{KeycapsKey, KeycapsView};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SnapshotToken};
use boardstudio_model::model::{
    EditOperation, KeycapBoardChange, KeycapBoardSettings, KeycapKeyChange, KeycapKeySettings,
    KeycapMatrixChange, KeycapMatrixSettings, KeycapMount, KeycapProfile, ProjectDoc, Vec2,
};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, NumericField, SelectField, TextCommitField, TextField,
};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonVariant},
    input::InputType,
};
use dioxus::prelude::*;
use std::rc::Rc;

const PROFILES: &[(&str, &str)] = &[
    ("cherry", "Cherry"),
    ("oem", "OEM"),
    ("dcs", "DCS"),
    ("dsa", "DSA"),
    ("sa", "SA"),
    ("hi-pro", "Hi-Pro"),
    ("g20", "G20"),
    ("choc", "Choc"),
];
const MOUNTS: &[(&str, &str)] = &[
    ("mx", "MX cross"),
    ("choc-v1", "Choc v1"),
    ("choc-v2", "Choc v2 cross"),
    ("alps", "Alps"),
];

#[derive(Clone, Debug, PartialEq)]
pub struct KeycapsEditSource {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone)]
pub struct KeycapsSettingsActions {
    runtime: Rc<Runtime>,
    scope: Scope,
    error: Signal<Option<String>>,
}

impl PartialEq for KeycapsSettingsActions {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.runtime, &other.runtime)
            && self.scope == other.scope
            && self.error == other.error
    }
}

impl KeycapsSettingsActions {
    fn commit(&self, target_id: String, op: EditOperation) {
        let scope = self.scope.clone();
        let resolver = EditResolver::new("keycaps", move |accepted: &AcceptedSnapshot| {
            if accepted.session_epoch != scope.session_epoch
                || accepted.document.id != scope.document_id
            {
                return Resolution::Retire("This keycap settings target no longer exists.".into());
            }
            Resolution::submit(vec![target_id.clone()], op.clone())
        });
        let mut err = self.error;
        err.set(self.runtime.commit("keycaps", resolver).err());
    }
}

pub fn use_keycaps_settings_actions(
    runtime: Rc<Runtime>,
    source: Option<KeycapsEditSource>,
    workspace: Signal<Workspace>,
    _scope_generation: Signal<u64>,
) -> Option<KeycapsSettingsActions> {
    let error = use_signal(|| None::<String>);
    let current_scope = runtime.scope();
    let source = source.filter(|s| {
        workspace() == Workspace::Keycaps && current_scope.as_ref() == Some(&s.scope)
    })?;
    Some(KeycapsSettingsActions {
        runtime,
        scope: source.scope,
        error,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectedKeySettings {
    pub key: KeycapsKey,
    pub settings: KeycapKeySettings,
    pub board_color: String,
}

pub fn project_selected_key(
    document: &ProjectDoc,
    view: &KeycapsView,
    selected_key_id: Option<&str>,
) -> Option<SelectedKeySettings> {
    let key_id = selected_key_id?;
    let key = view.keys.iter().find(|k| k.id.as_ref() == key_id)?.clone();
    let settings = document
        .keycaps
        .as_ref()
        .and_then(|k| k.keys.get(key_id))
        .cloned()
        .unwrap_or_default();
    let board_color = document
        .keycaps
        .as_ref()
        .and_then(|k| k.boards.get(view.board_id.as_ref()))
        .map_or_else(|| KeycapBoardSettings::default().color, |s| s.color.clone());
    Some(SelectedKeySettings {
        key,
        settings,
        board_color,
    })
}

fn options_with_default(first: &str, items: &[(&str, &str)]) -> Vec<(String, String)> {
    std::iter::once((String::new(), first.to_owned()))
        .chain(
            items
                .iter()
                .map(|(v, l)| ((*v).to_owned(), (*l).to_owned())),
        )
        .collect()
}

fn profile_str(p: Option<KeycapProfile>) -> String {
    match p {
        Some(KeycapProfile::Cherry) => "cherry",
        Some(KeycapProfile::Oem) => "oem",
        Some(KeycapProfile::Dcs) => "dcs",
        Some(KeycapProfile::Dsa) => "dsa",
        Some(KeycapProfile::Sa) => "sa",
        Some(KeycapProfile::HiPro) => "hi-pro",
        Some(KeycapProfile::G20) => "g20",
        Some(KeycapProfile::Choc) => "choc",
        None => "",
    }
    .to_owned()
}

fn parse_profile(s: &str) -> Option<KeycapProfile> {
    match s {
        "cherry" => Some(KeycapProfile::Cherry),
        "oem" => Some(KeycapProfile::Oem),
        "dcs" => Some(KeycapProfile::Dcs),
        "dsa" => Some(KeycapProfile::Dsa),
        "sa" => Some(KeycapProfile::Sa),
        "hi-pro" => Some(KeycapProfile::HiPro),
        "g20" => Some(KeycapProfile::G20),
        "choc" => Some(KeycapProfile::Choc),
        _ => None,
    }
}

fn mount_str(m: Option<KeycapMount>) -> String {
    match m {
        Some(KeycapMount::Mx) => "mx",
        Some(KeycapMount::ChocV1) => "choc-v1",
        Some(KeycapMount::ChocV2) => "choc-v2",
        Some(KeycapMount::Alps) => "alps",
        None => "",
    }
    .to_owned()
}

fn parse_mount(s: &str) -> Option<KeycapMount> {
    match s {
        "mx" => Some(KeycapMount::Mx),
        "choc-v1" => Some(KeycapMount::ChocV1),
        "choc-v2" => Some(KeycapMount::ChocV2),
        "alps" => Some(KeycapMount::Alps),
        _ => None,
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct KeycapsBoardSettingsEditorProps {
    pub settings: KeycapBoardSettings,
    pub actions: KeycapsSettingsActions,
}

#[component]
pub fn KeycapsBoardSettingsEditor(props: KeycapsBoardSettingsEditorProps) -> Element {
    let (settings, actions) = (props.settings, props.actions);
    let board_id = actions.scope.board_id.clone();
    let (a1, a2, a3) = (actions.clone(), actions.clone(), actions.clone());
    let (b1, b2, b3) = (board_id.clone(), board_id.clone(), board_id);
    rsx! {
        InspectorSection { title: "Board colors", default_open: true,
            section { class: "grid min-w-0 gap-2 p-3 text-foreground", aria_label: "Board colors",
                TextField { id: "keycaps-board-color".to_owned(), label: "Keycap color".to_owned(), aria_label: "Board keycap color", r#type: InputType::Color, value: settings.color,
                    oninput: move |e: FormEvent| a1.commit(b1.clone(), EditOperation::SetKeycapBoard { board_id: b1.clone(), change: KeycapBoardChange::Color { value: e.value() } })
                }
                TextField { id: "keycaps-board-legend".to_owned(), label: "Legend color".to_owned(), aria_label: "Board legend color", r#type: InputType::Color, value: settings.legend_color,
                    oninput: move |e: FormEvent| a2.commit(b2.clone(), EditOperation::SetKeycapBoard { board_id: b2.clone(), change: KeycapBoardChange::LegendColor { value: e.value() } })
                }
                NumericField { label: "Minimum clearance".to_owned(), unit: "mm", aria_label: "Keycap clearance", value: settings.clearance, min: Some(0.0), max: Some(5.0), step: Some(0.1),
                    on_commit: move |v: f64| a3.commit(b3.clone(), EditOperation::SetKeycapBoard { board_id: b3.clone(), change: KeycapBoardChange::Clearance { value: v } })
                }
                if let Some(msg) = (actions.error)() { Alert { variant: AlertVariant::Destructive, "{msg}" } }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct KeycapsMatrixSettingsEditorProps {
    pub matrix_id: String,
    pub matrix_name: String,
    pub settings: KeycapMatrixSettings,
    pub actions: KeycapsSettingsActions,
}

#[component]
pub fn KeycapsMatrixSettingsEditor(props: KeycapsMatrixSettingsEditorProps) -> Element {
    let (mid, mname, settings, actions) = (
        props.matrix_id,
        props.matrix_name,
        props.settings,
        props.actions,
    );
    let (a1, a2, a3, a4) = (
        actions.clone(),
        actions.clone(),
        actions.clone(),
        actions.clone(),
    );
    let (m1, m2, m3, m4) = (mid.clone(), mid.clone(), mid.clone(), mid);
    rsx! {
        fieldset { class: "m-0 mb-2 min-w-0 border-0 p-2",
            legend { class: "px-1 text-sm text-foreground", "{mname}" }
            SelectField::<String> {
                label: "Profile".to_owned(), aria_label: format!("Keycap profile for {mname}"),
                value: profile_str(settings.profile), options: options_with_default("No generated keycap", PROFILES),
                on_commit: move |v: String| a1.commit(m1.clone(), EditOperation::SetMatrixKeycaps { matrix_id: m1.clone(), change: KeycapMatrixChange::Profile { value: parse_profile(&v) } })
            }
            InspectorSection { title: "Profile dimensions & socket", default_open: false,
                NumericField { label: "First profile row".to_owned(), aria_label: format!("First profile row for {mname}"), value: settings.first_row as f64, min: Some(1.0), max: Some(5.0), step: Some(1.0), precision: 0,
                    on_commit: move |v: f64| a2.commit(m2.clone(), EditOperation::SetMatrixKeycaps { matrix_id: m2.clone(), change: KeycapMatrixChange::FirstRow { value: v.round() as u8 } })
                }
                NumericField { label: "Wall thickness".to_owned(), unit: "mm", aria_label: format!("Wall thickness for {mname}"), value: settings.wall_thickness, min: Some(0.8), max: Some(2.0), step: Some(0.1),
                    on_commit: move |v: f64| a3.commit(m3.clone(), EditOperation::SetMatrixKeycaps { matrix_id: m3.clone(), change: KeycapMatrixChange::WallThickness { value: v } })
                }
                SelectField::<String> {
                    label: "Socket".to_owned(), aria_label: format!("Keycap socket for {mname}"),
                    value: mount_str(settings.mount), options: options_with_default("From switch profile", MOUNTS),
                    on_commit: move |v: String| a4.commit(m4.clone(), EditOperation::SetMatrixKeycaps { matrix_id: m4.clone(), change: KeycapMatrixChange::Mount { value: parse_mount(&v) } })
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct KeycapsSettingsEditorProps {
    pub selected: SelectedKeySettings,
    pub actions: KeycapsSettingsActions,
}

#[component]
pub fn KeycapsSettingsEditor(props: KeycapsSettingsEditorProps) -> Element {
    let (selected, actions) = (props.selected, props.actions);
    let (key, s) = (selected.key, selected.settings);
    let key_id = key.id.to_string();
    let commit_key = {
        let (a, id) = (actions.clone(), key_id.clone());
        move |change: KeycapKeyChange| {
            a.commit(
                id.clone(),
                EditOperation::SetKeycapKey {
                    key_id: id.clone(),
                    change,
                },
            );
        }
    };
    let (c_leg, c_inh, c_blk, c_col, c_rst, c_pro, c_mnt, c_row, c_row_rst, c_w, c_d, c_u_rst) = (
        commit_key.clone(),
        commit_key.clone(),
        commit_key.clone(),
        commit_key.clone(),
        commit_key.clone(),
        commit_key.clone(),
        commit_key.clone(),
        commit_key.clone(),
        commit_key.clone(),
        commit_key.clone(),
        commit_key.clone(),
        commit_key,
    );
    let units = s.units;
    rsx! {
        section { class: "grid min-w-0 gap-2 text-foreground", aria_label: "Keycap overrides for {key.reference}",
            TextCommitField {
                label: "Legend".to_owned(), aria_label: format!("Legend for {}", key.reference),
                value: s.legend.unwrap_or_default(),
                placeholder: if key.binding_label.is_empty() { "From binding".to_owned() } else { key.binding_label.to_string() },
                on_commit: move |v: String| c_leg(KeycapKeyChange::Legend { value: Some(v.chars().take(12).collect()) }),
            }
            div { class: "flex flex-wrap gap-1.5",
                Button { variant: ButtonVariant::Secondary, onclick: move |_| c_inh(KeycapKeyChange::Legend { value: None }), "Use binding legend" }
                Button { variant: ButtonVariant::Secondary, onclick: move |_| c_blk(KeycapKeyChange::Legend { value: Some(String::new()) }), "Blank keycap" }
            }
            TextField {
                id: format!("keycaps-color-{}", dioxus::core::current_scope_id().0),
                label: "Keycap color".to_owned(), aria_label: format!("Keycap color for {}", key.reference),
                r#type: InputType::Color, value: s.color.as_deref().unwrap_or(&selected.board_color).to_owned(),
                oninput: move |e: FormEvent| c_col(KeycapKeyChange::Color { value: Some(e.value()) }),
            }
            if s.color.is_some() {
                Button { variant: ButtonVariant::Secondary, onclick: move |_| c_rst(KeycapKeyChange::Color { value: None }), "Use board color" }
            }
            InspectorSection { title: "Keycap overrides", default_open: false,
                SelectField::<String> {
                    label: "Profile".to_owned(), aria_label: format!("Profile override for {}", key.reference),
                    value: profile_str(s.profile), options: options_with_default("From matrix", PROFILES),
                    on_commit: move |v: String| c_pro(KeycapKeyChange::Profile { value: parse_profile(&v) }),
                }
                SelectField::<String> {
                    label: "Socket".to_owned(), aria_label: format!("Socket override for {}", key.reference),
                    value: mount_str(s.mount), options: options_with_default("From matrix / switch", MOUNTS),
                    on_commit: move |v: String| c_mnt(KeycapKeyChange::Mount { value: parse_mount(&v) }),
                }
                NumericField {
                    label: "Profile row".to_owned(), aria_label: format!("Profile row for {}", key.reference),
                    value: s.row.map_or(f64::NAN, f64::from), min: Some(1.0), max: Some(5.0), step: Some(1.0), precision: 0,
                    placeholder: "From matrix",
                    on_commit: move |v: f64| c_row(KeycapKeyChange::Row { value: Some(v.round() as u8) }),
                }
                if s.row.is_some() {
                    Button { variant: ButtonVariant::Secondary, onclick: move |_| c_row_rst(KeycapKeyChange::Row { value: None }), "Use matrix row" }
                }
                NumericField {
                    label: "Width".to_owned(), unit: "u", aria_label: format!("Keycap width for {}", key.reference),
                    value: units.map_or(f64::NAN, |u| u.x), min: Some(0.75), max: Some(7.0), step: Some(0.25),
                    placeholder: "From envelope",
                    on_commit: move |v: f64| c_w(KeycapKeyChange::Units { value: Some(Vec2 { x: v, y: units.map_or(1.0, |u| u.y) }) }),
                }
                NumericField {
                    label: "Depth".to_owned(), unit: "u", aria_label: format!("Keycap depth for {}", key.reference),
                    value: units.map_or(f64::NAN, |u| u.y), min: Some(0.75), max: Some(7.0), step: Some(0.25),
                    placeholder: "From envelope",
                    on_commit: move |v: f64| c_d(KeycapKeyChange::Units { value: Some(Vec2 { x: units.map_or(1.0, |u| u.x), y: v }) }),
                }
                if units.is_some() {
                    Button { variant: ButtonVariant::Secondary, onclick: move |_| c_u_rst(KeycapKeyChange::Units { value: None }), "Use envelope size" }
                }
            }
            if let Some(msg) = (actions.error)() { Alert { variant: AlertVariant::Destructive, "{msg}" } }
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn profile_and_mount_round_trip() {
        for &(val, _) in PROFILES {
            let parsed = parse_profile(val).expect("profile");
            assert_eq!(profile_str(Some(parsed)), val);
        }
        assert_eq!(parse_profile(""), None);
        for &(val, _) in MOUNTS {
            let parsed = parse_mount(val).expect("mount");
            assert_eq!(mount_str(Some(parsed)), val);
        }
        assert_eq!(parse_mount(""), None);
    }

    #[test]
    fn project_selected_key_returns_board_color_and_settings() {
        let mut doc = ProjectDoc::empty("d", "P");
        doc.boards.push(boardstudio_model::model::Board {
            id: "b1".into(),
            name: "B".into(),
            outline_ids: vec![],
            part_ids: vec!["k1".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let view = KeycapsView {
            board_id: Rc::from("b1"),
            board_settings: KeycapBoardSettings::default(),
            keys: vec![KeycapsKey {
                id: Rc::from("k1"),
                reference: Rc::from("K1"),
                pose: boardstudio_model::model::Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
                size: Vec2 { x: 18.0, y: 18.0 },
                color: Rc::from("#ffffff"),
                legend: Rc::from("A"),
                legend_source: crate::keycaps_scene::LegendSource::Binding,
                binding_label: Rc::from("A"),
                search_text: "k1 a".into(),
            }],
            matrices: vec![],
            assigned_count: 1,
        };
        let sel = project_selected_key(&doc, &view, Some("k1")).expect("selected");
        assert_eq!(sel.board_color, KeycapBoardSettings::default().color);
        assert!(project_selected_key(&doc, &view, Some("missing")).is_none());
    }
}
