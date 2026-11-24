use boardstudio_core::model::ProjectDoc;
use boardstudio_web_test_runtime::{in_process_support, project_name_test_support};
use js_sys::Object;
use js_sys::{Array, Reflect, Uint8Array};
use std::collections::BTreeMap;
use wasm_bindgen::{JsValue, closure::Closure};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn archive_exactly_at_size_limit_reaches_normal_validation() {
    let window = web_sys::window().unwrap();
    let original = Reflect::get(&window, &"confirm".into()).unwrap();
    let confirm = Closure::<dyn FnMut(JsValue) -> bool>::new(|_| true);
    Reflect::set(&window, &"confirm".into(), confirm.as_ref()).unwrap();
    let runtime = project_name_test_support::new_runtime();
    project_name_test_support::open_document(&runtime, in_process_support::board_document()).await;
    let imported = ProjectDoc::empty("archive-at-limit", "At limit");
    let prepared = boardstudio_web_runtime::portable_archive::prepare_archive(
        &imported,
        &BTreeMap::new(),
        vec![],
        false,
    )
    .unwrap();
    let (_, buffers) = boardstudio_core::archive::request(&prepared.metadata, &prepared.buffers);
    let parts = Array::new();
    parts.push(&Uint8Array::from(buffers[0].as_slice()));
    let done = project_name_test_support::track_import_file(&runtime);
    let file = web_sys::File::new_with_u8_array_sequence(&parts, "at-limit.boardstudio").unwrap();
    let descriptor = Object::new();
    Reflect::set(
        &descriptor,
        &"value".into(),
        &JsValue::from_f64(128.0 * 1024.0 * 1024.0),
    )
    .unwrap();
    Object::define_property(&Object::from(file.clone()), &"size".into(), &descriptor);
    runtime.import_file(file);
    done.await.unwrap();
    Reflect::set(&window, &"confirm".into(), &original).unwrap();
    project_name_test_support::run_pending(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.id,
        "archive-at-limit"
    );
}

#[wasm_bindgen_test]
async fn oversized_import_is_rejected_before_opening_or_reading() {
    let runtime = project_name_test_support::new_runtime();
    project_name_test_support::open_document(&runtime, in_process_support::board_document()).await;
    let before = runtime.model().accepted.unwrap();
    let file =
        web_sys::File::new_with_u8_array_sequence(&Array::new(), "oversized.boardstudio").unwrap();
    let descriptor = Object::new();
    Reflect::set(
        &descriptor,
        &"value".into(),
        &JsValue::from_f64(128.0 * 1024.0 * 1024.0 + 1.0),
    )
    .unwrap();
    Object::define_property(&Object::from(file.clone()), &"size".into(), &descriptor);
    runtime.import_file(file);
    assert!(
        runtime.status().contains("128 MiB"),
        "oversized files must be rejected synchronously: {}",
        runtime.status()
    );
    assert_eq!(runtime.model().accepted.unwrap().token, before.token);
}
