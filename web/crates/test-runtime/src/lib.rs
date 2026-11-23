//! Concrete Core engine wiring for web tests.
pub mod layout_fixtures;
use boardstudio_core::CoreEngine;
use boardstudio_web_runtime::core_services::CoreServices;

pub fn engine_core_services() -> CoreServices {
    CoreServices {
        validate_open: CoreEngine::validate_open,
        open_scratch: || {
            let mut engine = CoreEngine::new();
            Box::new(move |request| engine.handle(request))
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn new_native_runtime() -> std::rc::Rc<boardstudio_web_runtime::runtime::Runtime> {
    boardstudio_web_runtime::runtime::Runtime::with_core(engine_core_services())
}

#[cfg(target_arch = "wasm32")]
pub use boardstudio_web_runtime::{gate_driver, runtime};
#[cfg(target_arch = "wasm32")]
mod fixtures;
#[cfg(target_arch = "wasm32")]
pub use fixtures::{firmware_export_test_support, in_process_support, project_name_test_support};
