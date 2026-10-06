//! Private canvas composition for the existing mirrored-pair setup form.
use super::{MirroredPairForm, MirroredPairFormProjection, MirroredPairOwner, MirroredPairRequest};
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct MirroredPairCanvasOverlayProps {
    pub projection: MirroredPairFormProjection,
    pub on_cancel: EventHandler<MirroredPairOwner>,
    pub on_preview: EventHandler<MirroredPairRequest>,
}

#[component]
pub fn MirroredPairCanvasOverlay(
    props: MirroredPairCanvasOverlayProps,
) -> Element {
    let owner = props.projection.owner.clone();
    let on_cancel = props.on_cancel;
    rsx! {
        div {
            class: "m1-mirrored-pair-canvas-overlay",
            onkeydown: move |event| {
                if event.key().to_string() == "Escape" {
                    event.prevent_default();
                    event.stop_propagation();
                    on_cancel.call(owner.clone());
                }
            },
            MirroredPairForm {
                key: "{props.projection.owner.open_id}",
                projection: props.projection,
                on_cancel: props.on_cancel,
                on_preview: props.on_preview,
            }
        }
    }
}
