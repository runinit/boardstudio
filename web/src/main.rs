// The page's non-UI layer lives in `boardstudio-web-runtime`; re-export its modules at
// the crate root so presentation code keeps addressing them as `crate::<module>`.
#[cfg(feature = "page")]
#[allow(unused_imports)]
use boardstudio_web_runtime::{
    archive_export, bundled_models, case_generation_lifecycle, case_model_lifecycle, case_preview,
    firmware_position_projection, layout_viewer_source, model_delivery, operation_outcomes,
    parts_preview, pcb_wiring_mode_operation, portable_archive, runtime,
};

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod setup_guide_state;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub(crate) use boardstudio_web_case::cad_presentation;
#[cfg(feature = "page")]
mod outline_settings;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod pcb_wiring_remap_operation;
#[cfg(all(feature = "page", target_arch = "wasm32"))]
pub(crate) use boardstudio_web_catalogue::matrix_setup_operation;
#[cfg(all(feature = "page", target_arch = "wasm32"))]
pub(crate) use boardstudio_web_catalogue::physical_setup;
#[cfg(all(feature = "page", target_arch = "wasm32", test))]
pub(crate) use boardstudio_web_ui_shared::footprint_forms;
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod presentation;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod firmware_position_choices;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
mod presentation {
    pub(crate) use crate::model_delivery;

    #[path = "outline_grid_rounding.rs"]
    mod outline_grid_rounding_tests;

    pub(crate) mod objects {
        #[path = "keycap_resize.rs"]
        mod keycap_resize;
    }

    pub(crate) mod pcb_wiring {
        use crate::firmware_position_projection::FirmwarePlanIdentity;
        use boardstudio_application::Scope;
        use boardstudio_core::electrical::ElectricalPlan;
        use boardstudio_core::model::ProjectDoc;
        use std::{rc::Rc, sync::Arc};

        pub(crate) type WiringPlanIdentity = FirmwarePlanIdentity;

        #[allow(dead_code)]
        #[derive(Clone, Debug, PartialEq)]
        pub(crate) enum PcbWiringResolution {
            Idle,
            Current {
                identity: WiringPlanIdentity,
                plan: Rc<ElectricalPlan>,
            },
            Pending {
                identity: WiringPlanIdentity,
            },
            Failed {
                identity: WiringPlanIdentity,
                message: String,
            },
        }

        #[derive(Clone)]
        pub(crate) struct PcbWiringSource {
            pub(crate) identity: WiringPlanIdentity,
            pub(crate) ui_scope: Scope,
            pub(crate) scope_generation: u64,
            pub(crate) active_part_id: Option<String>,
            pub(crate) document: Arc<ProjectDoc>,
        }

        #[path = "mode.rs"]
        mod mode;

        #[path = "connections.rs"]
        mod connections;

        #[path = "apply.rs"]
        mod apply;

        #[path = "pins.rs"]
        mod pins;

        #[path = "remap.rs"]
        mod remap;

        #[cfg(test)]
        #[path = "mode_owner_tests.rs"]
        mod mode_owner_tests;
    }
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
#[path = "matrix_transform_operation.rs"]
mod matrix_transform_operation;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod mirrored_pair_lifecycle;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod matrix_transform_lifecycle;

fn main() {
    #[cfg(all(target_arch = "wasm32", feature = "page"))]
    dioxus::launch(app);
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
fn app() -> dioxus::prelude::Element {
    dioxus::prelude::use_effect(|| {
        if let Some(root) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.document_element())
        {
            let _ = root.set_attribute("lang", "en");
        }
    });
    presentation::App()
}

#[cfg(feature = "page")]
mod mirrored_pair_geometry;
