#[cfg(feature = "page")]
mod archive_export;
#[cfg(feature = "page")]
mod bundled_models;
#[cfg(feature = "page")]
mod case_generation_lifecycle;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod case_model_lifecycle;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
#[path = "macro_accessible_names.rs"]
mod macro_accessible_names;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod setup_guide_state;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod cad_presentation;
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
mod pcb_wiring_mode_operation;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod pcb_wiring_remap_operation;
#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod physical_setup;
#[cfg(feature = "page")]
mod portable_archive;
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod presentation;
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod renderer_host_page;
#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod runtime;
#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
#[path = "runtime_test_stub.rs"]
mod runtime;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod firmware_request_adapter;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod preview_generator;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
mod case_preview_lifecycle;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod case_preview;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod operation_outcomes;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod case_generation_admission;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod mechanical_feedback;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod firmware_position_projection;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
mod firmware_position_choices;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
#[path = "presentation/instance_selection.rs"]
mod instance_selection;

#[cfg(all(feature = "page", any(test, target_arch = "wasm32")))]
#[path = "presentation/footprint_forms.rs"]
mod footprint_forms;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
mod renderer_host_source_sync;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
#[path = "presentation/case_display.rs"]
mod case_display;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
#[path = "presentation/model_delivery.rs"]
mod model_delivery;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
#[path = "presentation/layout_viewer_source.rs"]
mod layout_viewer_source;

#[cfg(all(feature = "page", test, not(target_arch = "wasm32")))]
mod presentation {
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
