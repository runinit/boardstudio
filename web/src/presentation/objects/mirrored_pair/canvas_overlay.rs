//! Private canvas composition for the existing mirrored-pair setup form.
use super::{MirroredPairForm, MirroredPairFormProjection, MirroredPairOwner, MirroredPairRequest};
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub(in crate::presentation) struct MirroredPairCanvasOverlayProps {
    pub projection: MirroredPairFormProjection,
    pub on_cancel: EventHandler<MirroredPairOwner>,
    pub on_preview: EventHandler<MirroredPairRequest>,
}

#[component]
pub(in crate::presentation) fn MirroredPairCanvasOverlay(
    props: MirroredPairCanvasOverlayProps,
) -> Element {
    rsx! {
        div { class: "m1-mirrored-pair-canvas-overlay",
            MirroredPairForm {
                key: "{props.projection.owner.open_id}",
                projection: props.projection,
                on_cancel: props.on_cancel,
                on_preview: props.on_preview,
            }
        }
    }
}
