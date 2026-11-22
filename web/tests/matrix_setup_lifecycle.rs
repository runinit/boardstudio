//! Mount the production Matrix Setup owner with deterministic detached async ports.
#![cfg(all(feature = "page", not(target_arch = "wasm32")))]
extern crate self as gloo_timers;
extern crate self as wasm_bindgen_futures;

use boardstudio_application::{Event, OperationId};
use boardstudio_core::model::{Board, ProjectDoc};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[path = "../crates/catalogue/src/matrix_setup_operation.rs"]
mod matrix_setup_operation;
#[path = "support/matrix_setup_presentation.rs"]
mod presentation;

#[path = "support/detached.rs"]
mod detached;
use detached::poll_detached;
pub use detached::{future, spawn_local};

thread_local! { static TEMPLATES: RefCell<Option<Result<Vec<boardstudio_core::model::PartDefinition>, String>>> = const { RefCell::new(None) }; }
use boardstudio_web_runtime::runtime;

fn document(id: &str, revision: u64, with_instance: bool) -> ProjectDoc {
    let mut document = ProjectDoc::empty(id, id);
    document.revision = revision;
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    if with_instance {
        document.hardware = Some(
            serde_json::from_value(serde_json::json!({"instances":[{
                "id":"primary", "name":"Primary", "boardId":"board", "half":"single", "role":"standalone", "flipped":false, "constructionLinked":false
            }]}))
            .unwrap(),
        );
    }
    document
}

fn open_document(runtime: &runtime::Runtime, id: &str, revision: u64, with_instance: bool) {
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: document(id, revision, with_instance),
    });
}

#[path = "support/matrix_catalogue.rs"]
mod matrix_catalogue;
use matrix_catalogue::catalogue;
