//! The shipped keyboards that native tests read. The recipes build them once per test binary,
//! and `demo_projects` and `included_keyboards` read the same documents.

use crate::demo_projects::demo_projects;
use boardstudio_core::CoreEngine;
use serde_json::Value;
use std::{collections::BTreeMap, sync::OnceLock};

/// The ten shipped split keyboards (`demo_projects::split_projects`) and the reused-PCB
/// demonstration, built on first use.
pub(crate) fn projects() -> &'static BTreeMap<String, Value> {
    static PROJECTS: OnceLock<BTreeMap<String, Value>> = OnceLock::new();
    PROJECTS.get_or_init(|| {
        let mut engine = CoreEngine::new();
        let mut built = demo_projects::split_projects().unwrap();
        built.insert(
            "reused-pcb".to_owned(),
            demo_projects::reused_pcb_project(&mut engine).unwrap(),
        );
        built
    })
}
