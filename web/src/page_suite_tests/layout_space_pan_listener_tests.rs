//! Real-browser regression for the Layout Space-pan window listener that `ui-model` owns.
//!
//! It dispatches genuine `KeyboardEvent`s on the browser `window`, so it runs in the page suite.
use std::{cell::Cell, rc::Rc};

use boardstudio_web_ui_model::canvas_interaction::LayoutSpacePanWindowListener;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn layout_space_pan_window_listener_handles_keyboard_events() {
    let space_down = Rc::new(Cell::new(false));
    let is_layout = Rc::new(Cell::new(true));
    let listener = LayoutSpacePanWindowListener::install(
        {
            let is_layout = is_layout.clone();
            Rc::new(move || is_layout.get())
        },
        space_down.clone(),
    )
    .expect("window listeners install");
    let window = web_sys::window().expect("window");
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(" ");
    init.set_code("Space");
    init.set_bubbles(true);
    init.set_cancelable(true);
    let key_event = |kind| {
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict(kind, &init).expect("Space event")
    };
    let consumed = key_event("keydown");
    consumed.prevent_default();
    window.dispatch_event(&consumed).unwrap();
    assert!(!space_down.get(), "consumed Space must not acquire pan");

    let down = key_event("keydown");
    window.dispatch_event(&down).unwrap();
    assert!(space_down.get(), "window Space keydown enables pan");
    assert!(down.default_prevented(), "Space does not scroll the page");
    window.dispatch_event(&consumed).unwrap();
    assert!(
        space_down.get(),
        "consumed repeats do not cancel admitted pan"
    );

    is_layout.set(false);
    let up = key_event("keyup");
    up.prevent_default();
    window.dispatch_event(&up).unwrap();
    assert!(
        !space_down.get(),
        "keyup releases pan even after workspace changes"
    );
    let other_workspace = key_event("keydown");
    window.dispatch_event(&other_workspace).unwrap();
    assert!(
        !space_down.get(),
        "other workspaces do not acquire Layout pan"
    );
    assert!(!other_workspace.default_prevented());

    is_layout.set(true);
    let document = window.document().expect("document");
    for tag in ["input", "textarea", "select"] {
        let input = document.create_element(tag).expect("typing control");
        document.body().unwrap().append_child(&input).unwrap();
        let typing = key_event("keydown");
        input.dispatch_event(&typing).unwrap();
        input.remove();
        assert!(
            !space_down.get(),
            "typing Space does not acquire pan: {tag}"
        );
        assert!(!typing.default_prevented());
    }
    drop(listener);
    let after_drop = key_event("keydown");
    window.dispatch_event(&after_drop).unwrap();
    assert!(
        !space_down.get(),
        "dropped keydown listener cannot acquire pan"
    );
    assert!(!after_drop.default_prevented());
    space_down.set(true);
    window.dispatch_event(&key_event("keyup")).unwrap();
    assert!(
        space_down.get(),
        "dropped keyup listener cannot release pan"
    );
}
