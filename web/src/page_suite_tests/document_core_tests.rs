use crate::runtime::Runtime;
use boardstudio_application::EditResolver;
use boardstudio_application::{Durability, Event};
use boardstudio_core::model::ProjectDoc;
use boardstudio_web_runtime::runtime::test_hooks;
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_test::wasm_bindgen_test;

async fn wait_for_save(runtime: &Runtime, revision: u64) {
    for _ in 0..200 {
        match runtime.model().durability {
            Durability::Saved { revision: landed } if landed == revision => return,
            Durability::Failed { reason, .. } => panic!("IndexedDB save failed: {reason}"),
            _ => TimeoutFuture::new(1).await,
        }
    }
    panic!("revision {revision} was not saved");
}

#[wasm_bindgen_test]
async fn commits_publish_before_the_latest_revision_is_durable() {
    let runtime = test_hooks::new_runtime().unwrap();
    let store = test_hooks::store(&runtime);
    let id = "coalesced-rename";
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: ProjectDoc::empty(id, "Original"),
    });
    wait_for_save(&runtime, 0).await;

    for (revision, name) in [(1, "First"), (2, "Second"), (3, "Latest")] {
        let accepted = runtime
            .commit(
                "rename",
                EditResolver::rename_project("rename", name.into()),
            )
            .expect("rename commits synchronously");
        assert_eq!(accepted.document.name, name);
        assert_eq!(accepted.document.revision, revision);
        assert_eq!(runtime.model().durability, Durability::Saving { revision });
    }

    wait_for_save(&runtime, 3).await;
    let reloaded = store.load_document(id.to_owned()).await.unwrap().unwrap();
    assert_eq!(reloaded.name, "Latest");
    assert_eq!(reloaded.revision, 3);
    store.delete_project(id.to_owned()).await.unwrap();
    assert!(store.load_document(id.to_owned()).await.unwrap().is_none());
}
