//! Per-feature actions for a physical half that belongs to a mechanical mirror pair.
use crate::mechanical_pair::{MechanicalPairFeatureRow, feature_label};
use crate::mechanical_settings::MechanicalSettingsPatch;
use boardstudio_model::model::MechanicalFeatureState;
use boardstudio_web_ui_shared::inspector_fields::InspectorSection;
use boardstudio_web_ui_shared::ui::button::{Button, ButtonSize, ButtonVariant};
use dioxus::prelude::*;
use std::rc::Rc;

/// The patch a pair button sends: make a linked feature local, rejoin an exception, or unlink.
fn pair_patch(row: &MechanicalPairFeatureRow, rejoin_from_own: bool) -> MechanicalSettingsPatch {
    let (feature_id, instance_id) = (row.feature_id.clone(), row.instance_id.clone());
    match row.state {
        MechanicalFeatureState::Linked => MechanicalSettingsPatch::MakeFeatureLocal {
            feature_id,
            instance_id,
        },
        _ => MechanicalSettingsPatch::RejoinFeatureMember {
            feature_id,
            source_instance_id: if rejoin_from_own {
                instance_id.clone()
            } else {
                row.counterpart_instance_id.clone()
            },
            instance_id,
        },
    }
}

#[component]
pub fn MechanicalPairControls(
    features: Rc<[MechanicalPairFeatureRow]>,
    instance_id: String,
    editable: bool,
    send_patch: EventHandler<MechanicalSettingsPatch>,
) -> Element {
    rsx! {
        InspectorSection { title: "Mirrored pair", badge: features.len().to_string(),
            for row in features.iter().cloned() {
                div { key: "{row.feature_id}",
                    span { "{feature_label(row.kind)}: {row.state:?}" }
                    if let Some(preview) = row.preview.clone().filter(|_| row.state != MechanicalFeatureState::Linked) {
                        span { " (partner: {preview})" }
                    }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, disabled: !editable,
                        onclick: { let row = row.clone(); move |_| send_patch.call(pair_patch(&row, false)) },
                        if row.state == MechanicalFeatureState::Linked { "Make local" } else { "Rejoin" }
                    }
                    if row.state == MechanicalFeatureState::Local {
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, disabled: !editable,
                            onclick: { let row = row.clone(); move |_| send_patch.call(pair_patch(&row, true)) }, "Rejoin from this half" }
                    }
                }
            }
            Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, disabled: !editable,
                onclick: move |_| send_patch.call(MechanicalSettingsPatch::UnlinkMirrorPair { instance_id: instance_id.clone() }),
                "Unlink pair" }
        }
    }
}
