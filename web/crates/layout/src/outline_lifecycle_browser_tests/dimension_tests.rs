//! Dimension and coordinate field tests for the outline inspector.
use super::*;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

#[derive(Clone)]
struct DimensionProbe {
    committed: Rc<RefCell<Vec<f64>>>,
}

fn mounted_dimension_host() -> Element {
    let probe = use_context::<DimensionProbe>();
    let committed = probe.committed.clone();
    let on_commit = use_callback(move |value: f64| committed.borrow_mut().push(value));
    rsx! {
        div { id: "outline-dimension-test-root",
            OutlineDimension {
                label: "Outline margin",
                value: 4.0,
                minimum: 0.0,
                editable: true,
                on_commit,
            }
        }
    }
}

fn coordinate_history_host() -> Element {
    let mut accepted = use_signal(|| 0.0);
    rsx! {
        OutlineCoordinate { label: "Point 1 X mm".to_owned(), value: accepted(), editable: true, on_commit: |_| {} }
        OutlineCoordinate { label: "Point 1 Y mm".to_owned(), value: 10.0, editable: true, on_commit: |_| {} }
        button { id: "coordinate-accepted", onclick: move |_| accepted.set(1.5), "Accept coordinate" }
        button { id: "coordinate-undo", onclick: move |_| accepted.set(0.0), "Undo coordinate" }
    }
}

#[wasm_bindgen_test]
async fn coordinate_undo_restores_accepted_value_without_reviving_a_submitted_draft() {
    let root = new_root("coordinate-history-mount");
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(coordinate_history_host));
    wait_until("the coordinate inputs mount", || {
        by_role_name(&root, "spinbutton", "Point 1 X mm").is_some()
    })
    .await;
    let x = by_role_name(&root, "spinbutton", "Point 1 X mm")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    let y = by_role_name(&root, "spinbutton", "Point 1 Y mm")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    send_input(&y, "");
    x.focus().unwrap();
    send_input(&x, "1.5");
    send_key(&x, "Enter");
    root.query_selector("#coordinate-accepted")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    settle_dimension().await;
    assert_eq!(x.value(), "1.5");
    assert_eq!(
        y.value(),
        "",
        "another coordinate's dirty draft must remain local"
    );
    root.query_selector("#coordinate-undo")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    wait_until("Undo shows the restored accepted coordinate", || {
        x.value() == "0"
    })
    .await;
    assert_eq!(
        x.value(),
        "0",
        "Undo must show the restored accepted coordinate"
    );
    assert_eq!(
        y.value(),
        "",
        "Undo on X must preserve the unrelated Y draft"
    );
    root.remove();
}

fn mount_dimension() -> (DimensionProbe, web_sys::Element, MountedFixture) {
    let probe = DimensionProbe {
        committed: Rc::default(),
    };
    let (root, mounted) = mount_probe(
        "outline-dimension-mount",
        VirtualDom::new(mounted_dimension_host),
        &probe,
    );
    (probe, root, mounted)
}

fn dimension_input() -> web_sys::HtmlInputElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(
            "#outline-dimension-mount #outline-dimension-test-root input[aria-label='Outline margin']",
        )
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
}

#[wasm_bindgen_test]
async fn outline_dimension_enter_commits_and_escape_restores_accepted_value() {
    let (probe, root, _mounted) = mount_dimension();
    wait_until("the outline margin input mounts", || {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(
                "#outline-dimension-mount #outline-dimension-test-root input[aria-label='Outline margin']",
            )
            .unwrap()
            .is_some()
    })
    .await;
    let input = dimension_input();

    input.focus().unwrap();
    send_input(&input, "5.25");
    send_key(&input, "Enter");
    wait_until("Enter commits 5.25", || {
        probe.committed.borrow().as_slice() == [5.25]
    })
    .await;
    assert_eq!(probe.committed.borrow().as_slice(), &[5.25]);

    input.focus().unwrap();
    send_input(&input, "9");
    send_key(&input, "Escape");
    wait_until("Escape restores the accepted baseline", || {
        input.value() == "4"
    })
    .await;
    assert_eq!(input.value(), "4");
    input.blur().unwrap();
    // Negative check (no commit): one event-loop turn, recorded as unproven in the Outcome.
    yield_turn().await;
    assert_eq!(
        probe.committed.borrow().as_slice(),
        &[5.25],
        "Escape restores the accepted baseline without submitting"
    );
    root.remove();
}
