use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn a_save_completion_notifies_only_the_durability_subscriber() {
    let runtime = Runtime::new_with_restoration(false, CoreServices::engine()).unwrap();
    let switchboard = Rc::new([Cell::new(0), Cell::new(0)]);
    let listeners = [Runtime::subscribe, Runtime::subscribe_durability];
    for (index, subscribe) in listeners.into_iter().enumerate() {
        let counts = switchboard.clone();
        subscribe(
            &runtime,
            Rc::new(move || counts[index].set(counts[index].get() + 1)),
        );
    }
    let persist = |event, result| {
        let effects = runtime.session.borrow_mut().submit(event);
        let attempt = effects
            .into_iter()
            .find_map(|effect| match effect {
                Effect::Persist {
                    save_attempt_id, ..
                } => Some(save_attempt_id),
                _ => None,
            })
            .expect("persistence effect");
        runtime.complete(Completion::Persist {
            save_attempt_id: attempt,
            result,
        });
    };
    persist(
        Event::Open {
            operation_id: runtime.operation(),
            document: ProjectDoc::empty("project", "Original"),
        },
        SaveResult::Committed,
    );
    assert_eq!(switchboard.each_ref().map(Cell::get), [1, 0]);
    runtime.take_notifications();

    for (name, result, expected_status) in [
        ("Edited", SaveResult::Committed, "Saved locally."),
        (
            "Unsaved",
            SaveResult::Aborted("disk full".into()),
            "disk full",
        ),
    ] {
        persist(
            Event::ResolveEdit {
                operation_id: runtime.operation(),
                label: "rename".into(),
                resolver: EditResolver::rename_project("rename", name.into()),
            },
            result,
        );
        assert_eq!(runtime.status(), expected_status);
    }
    assert_eq!(switchboard.each_ref().map(Cell::get), [1, 2]);
    assert_eq!(runtime.take_notifications().len(), 1);
}
