//! Scoped controls for an already accepted routed-board reference.
use boardstudio_core::model::{Asset, BoardReference};
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Action {
    SetEnabled(bool),
    SetPositionX(f64),
    SetPositionY(f64),
    SetRotation(f64),
    SetElevation(f64),
    SetModelAsset {
        path: String,
        asset_id: Option<String>,
    },
    Remove,
}

#[component]
pub(super) fn Editor(
    reference: BoardReference,
    assets: Vec<Asset>,
    disabled: bool,
    on_action: EventHandler<Action>,
) -> Element {
    let options = assets
        .iter()
        .filter(|asset| {
            [".step", ".stp", ".stl", ".wrl"]
                .iter()
                .any(|extension| asset.name.to_ascii_lowercase().ends_with(extension))
        })
        .collect::<Vec<_>>();

    rsx! {
        details { class: "m1-case-physical-setup m1-board-reference",
            summary { "Routed PCB reference" }
            p { "Preview a routed KiCad board with the case. Replace this reference after editing routing in KiCad." }
            label { class: "m1-board-reference-enabled",
                input {
                    r#type: "checkbox",
                    checked: reference.enabled,
                    disabled,
                    aria_label: "Use routed PCB in assembly",
                    onchange: move |event: FormEvent| on_action.call(Action::SetEnabled(event.checked())),
                }
                "Use routed PCB in assembly"
            }
            div { class: "m1-board-reference-transform",
                label { "X (mm)"
                    input {
                        r#type: "number", step: "0.1", value: "{reference.pose.at.x}",
                        disabled,
                        oninput: move |event: FormEvent| {
                            if let Ok(value) = event.value().parse::<f64>()
                                && value.is_finite()
                            {
                                on_action.call(Action::SetPositionX(value));
                            }
                        }
                    }
                }
                label { "Y (mm)"
                    input {
                        r#type: "number", step: "0.1", value: "{reference.pose.at.y}",
                        disabled,
                        oninput: move |event: FormEvent| {
                            if let Ok(value) = event.value().parse::<f64>()
                                && value.is_finite()
                            {
                                on_action.call(Action::SetPositionY(value));
                            }
                        }
                    }
                }
                label { "Z (mm)"
                    input {
                        r#type: "number", step: "0.1", value: "{reference.elevation}",
                        disabled,
                        oninput: move |event: FormEvent| {
                            if let Ok(value) = event.value().parse::<f64>()
                                && value.is_finite()
                            {
                                on_action.call(Action::SetElevation(value));
                            }
                        }
                    }
                }
            }
            label { "Rotation (°)"
                input {
                    r#type: "number", value: "{reference.pose.rotation}",
                    disabled,
                    oninput: move |event: FormEvent| {
                        if let Ok(value) = event.value().parse::<f64>()
                            && value.is_finite()
                        {
                            on_action.call(Action::SetRotation(value));
                        }
                    }
                }
            }
            for (path, asset_id) in reference.model_assets.iter() {
                {
                    let path_for_change = path.clone();
                    let current_asset = asset_id.clone();
                    rsx! {
                        label { key: "{path}", "Model asset for {path}"
                            select {
                                aria_label: "Model asset for {path}",
                                value: "{asset_id}",
                                disabled,
                                onchange: move |event: FormEvent| {
                                    let selected = event.value();
                                    on_action.call(Action::SetModelAsset {
                                        path: path_for_change.clone(),
                                        asset_id: if selected.is_empty() { None } else { Some(selected) },
                                    });
                                },
                                option { value: "", "Resolve bundled model" }
                                if !current_asset.is_empty() && !options.iter().any(|asset| asset.id == *asset_id) {
                                    option { value: "{current_asset}", "Saved model asset is unavailable" }
                                }
                                for asset in &options {
                                    option { key: "{asset.id}", value: "{asset.id}", "{asset.name}" }
                                }
                            }
                        }
                    }
                }
            }
            button {
                r#type: "button",
                disabled,
                onclick: move |_| on_action.call(Action::Remove),
                "Remove PCB reference"
            }
        }
    }
}
