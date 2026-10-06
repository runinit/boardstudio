// The page's non-UI layer lives in `boardstudio-web-runtime`; re-export its modules at
// the crate root so presentation code keeps addressing them as `crate::<module>`.
#[cfg(feature = "page")]
#[allow(unused_imports)]
use boardstudio_web_runtime::{
    archive_export, bundled_models, case_generation_lifecycle, case_model_lifecycle, case_preview,
    firmware_position_projection, layout_viewer_source, model_delivery, operation_outcomes,
    parts_preview, pcb_wiring_mode_operation, portable_archive, runtime,
};
#[cfg(all(target_arch = "wasm32", feature = "page"))]
use boardstudio_web_runtime::{case_gesture_preview, renderer_host_page};

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod setup_guide_state;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod cad_presentation;
#[cfg(feature = "page")]
mod outline_settings;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod parts_custom_definition;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod parts_definition_name;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
#[path = "presentation/parts/mechanical_profile.rs"]
mod parts_mechanical_profile;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod parts_new_component;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod parts_view_generation;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod pcb_wiring_remap_operation;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod physical_setup;
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod presentation;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod case_preview_lifecycle;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod parts_assembly_preset_draft;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod case_generation_admission;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod mechanical_feedback;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod firmware_position_choices;

#[cfg(all(feature = "page", target_arch = "wasm32"))]
pub(crate) use boardstudio_web_ui_shared::footprint_forms;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
#[path = "presentation/case_display.rs"]
mod case_display;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
#[path = "presentation/parts_import_footprint.rs"]
#[allow(dead_code)]
mod parts_import_footprint;

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

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
#[path = "presentation/closure_clearance.rs"]
mod closure_clearance;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
#[path = "matrix_transform_operation.rs"]
mod matrix_transform_operation;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod matrix_setup_operation;

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
