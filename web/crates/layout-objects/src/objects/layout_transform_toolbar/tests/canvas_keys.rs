//! The canvas-owned key listener: one document `keydown`/`keyup` and window `blur` set per
//! canvas, shared by its rail overlays and retired with the last of them.
use super::*;

/// Dispatch one bubbling, cancelable `kind` key event of `key` on `target`.
fn key_event(target: &web_sys::EventTarget, kind: &str, key: &str) -> web_sys::KeyboardEvent {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict(kind, &init).unwrap();
    target.dispatch_event(&event).unwrap();
    event
}

fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

/// Whether some listener claimed a document Escape.
fn escape_claimed() -> bool {
    key_event(&document(), "keydown", "Escape").default_prevented()
}

fn quick_offset(rail: &MountedRail) -> String {
    query(
        &rail.root,
        "[aria-label='Drag to stagger'], [aria-label='Drag to offset column']",
    )
    .and_then(|token| token.get_attribute("data-quick-offset"))
    .expect("the Stagger token draws")
}

/// The Spread token never subscribes a Control target, so it never shows the offset glyph.
fn spread_quick_offset(rail: &MountedRail) -> String {
    rail.token("Drag to spread")
        .get_attribute("data-quick-offset")
        .expect("the Spread token draws")
}

/// Two canvases each route Control, blur and Escape through their own listener, and each
/// retires its listener with its last overlay without touching the other canvas.
#[wasm_bindgen_test]
async fn two_canvases_route_keys_through_their_own_listener_and_retire_it_with_their_overlays() {
    assert!(
        !escape_claimed(),
        "no listener claims Escape before a rail mounts"
    );
    let first_runtime = fixture::open_matrix_runtime().await;
    let second_runtime = fixture::open_matrix_runtime().await;
    // The first canvas mixes a Control target (Stagger) with an Escape-only peer (Spread).
    let first = mount_matrix_rail(
        &first_runtime,
        fixture::MATRIX_ID,
        column(1),
        vec![LayoutTransformTool::Spread, LayoutTransformTool::Stagger],
    )
    .await;
    let second = mount_column_tool(&second_runtime, LayoutTransformTool::Stagger).await;
    frame().await;
    assert_eq!(quick_offset(&first), "false");
    assert_eq!(quick_offset(&second), "false");

    key_event(&document(), "keydown", "Control");
    frame().await;
    assert_eq!(
        quick_offset(&first),
        "true",
        "Control reaches the first canvas"
    );
    assert_eq!(
        quick_offset(&second),
        "true",
        "Control reaches the second canvas"
    );
    assert_eq!(
        spread_quick_offset(&first),
        "false",
        "Spread subscribes no Control"
    );
    key_event(&document(), "keyup", "Control");
    frame().await;
    assert_eq!(quick_offset(&first), "false");
    assert_eq!(quick_offset(&second), "false");

    assert!(escape_claimed(), "an idle rail claims a document Escape");
    let input = document().create_element("input").unwrap();
    document().body().unwrap().append_child(&input).unwrap();
    let typed = key_event(&input, "keydown", "Escape");
    input.remove();
    assert!(!typed.default_prevented(), "a text field keeps its Escape");

    // Retiring the first canvas leaves the second canvas's listener in place.
    let mut first_show = first.show;
    first_show.set(false);
    pump_until(&first_runtime, "the first canvas unmounts", || {
        query(&first.root, "[data-transform-handle]").is_none()
    })
    .await;
    key_event(&document(), "keydown", "Control");
    frame().await;
    assert_eq!(
        quick_offset(&second),
        "true",
        "the second canvas still hears Control"
    );
    web_sys::window()
        .unwrap()
        .dispatch_event(&web_sys::Event::new("blur").unwrap())
        .unwrap();
    frame().await;
    assert_eq!(
        quick_offset(&second),
        "false",
        "window blur releases Control"
    );
    assert!(escape_claimed(), "the second canvas still claims Escape");

    // Retiring the last canvas removes its listener: nothing claims Escape and a late
    // Control reaches no dropped overlay signal.
    let mut second_show = second.show;
    second_show.set(false);
    pump_until(&second_runtime, "the second canvas unmounts", || {
        query(&second.root, "[data-transform-handle]").is_none()
    })
    .await;
    key_event(&document(), "keydown", "Control");
    key_event(&document(), "keyup", "Control");
    frame().await;
    assert!(!escape_claimed(), "the retired canvases claim nothing");
}
