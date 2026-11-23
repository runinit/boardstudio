use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn a_save_completion_notifies_only_the_durability_subscriber() {
    let runtime = Runtime::new_with_restoration(false).unwrap();
    let counts = Rc::new([Cell::new(0), Cell::new(0)]);
    for (index, subscribe) in [Runtime::subscribe, Runtime::subscribe_durability]
        .into_iter()
        .enumerate()
    {
        let counts = counts.clone();
        subscribe(
            &runtime,
            Rc::new(move || counts[index].set(counts[index].get() + 1)),
        );
    }
    let persist = |event, result| {
        let effects = runtime.session.borrow_mut().submit(event);
        let Effect::Persist {
            save_attempt_id, ..
        } = effects.last().unwrap()
        else {
            panic!("{effects:?}")
        };
        runtime.complete(Completion::Persist {
            save_attempt_id: *save_attempt_id,
            result,
        });
    };
    let open = Event::Open {
        operation_id: runtime.operation(),
        document: ProjectDoc::empty("project", "Original"),
    };
    persist(open, SaveResult::Committed);
    assert_eq!(counts.each_ref().map(Cell::get), [1, 0]);
    runtime.take_notifications();

    for (name, error) in [("Edited", None), ("Unsaved", Some("disk full"))] {
        let edit = Event::ResolveEdit {
            operation_id: runtime.operation(),
            label: "rename".into(),
            resolver: EditResolver::rename_project("rename", name.into()),
        };
        let result = error.map_or(SaveResult::Committed, |error| {
            SaveResult::Aborted(error.into())
        });
        persist(edit, result);
        assert_eq!(runtime.status(), error.unwrap_or("Saved locally."));
    }
    assert_eq!(counts.each_ref().map(Cell::get), [1, 2]);
    assert_eq!(runtime.take_notifications().len(), 1);
}
