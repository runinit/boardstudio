//! The shared harness for the mounted matrix edit tests: a small real-Core matrix document
//! (three keys in one matrix, opened through the in-process Session and Core), fixture
//! builders, runtime pumping, and the DOM event and mount helpers the Inspector, toolbar and
//! rail overlay tests drive.
#![cfg(all(test, target_arch = "wasm32"))]
use crate::runtime::Runtime;
use boardstudio_model::model::{
    Board, Follow, Layout, LayoutMirrorLink, Matrix, MatrixAnchor, MatrixCell,
    MirroredCorrespondence, MirroredItemKind, OutlineFeature, OutlineSettings, Part, PartAnchor,
    Pose2, ProjectDoc, Side, Vec2,
};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use std::rc::Rc;
use wasm_bindgen::JsCast;

pub const MATRIX_ID: &str = "matrix";
pub const BOARD_ID: &str = "board-main";

pub fn matrix_document() -> ProjectDoc {
    let key_ids = [
        "matrix/matrix/r0c0",
        "matrix/matrix/r0c1",
        "matrix/matrix/r0c2",
    ];
    let mut document = ProjectDoc::empty("project", "Matrix edits");
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: OutlineSettings::default(),
        id: "envelope-main".into(),
        part_ids: vec![],
        margin: 4.0,
        operation: boardstudio_model::model::Operation::Add,
    });
    document.boards.push(Board {
        id: BOARD_ID.into(),
        name: "Main".into(),
        outline_ids: vec!["envelope-main".into()],
        part_ids: key_ids.iter().map(|id| (*id).into()).collect(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id": "switch:base",
            "name": "MX switch",
            "kind": "switch",
            "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
            "pads": []
        }))
        .unwrap(),
    );
    for (column, id) in key_ids.iter().enumerate() {
        document.parts.push(Part {
            keycap: None,
            outline: None,
            anchor: None,
            id: (*id).into(),
            definition_id: "switch:base".into(),
            reference: format!("SW{}", column + 1),
            pose: Pose2 {
                at: Vec2 {
                    x: 19.05 * column as f64,
                    y: 0.0,
                },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
    }
    document.matrices.push(
        serde_json::from_value(serde_json::json!({
            "id": MATRIX_ID,
            "rows": 1,
            "columns": 3,
            "pitch": {"x": 19.05, "y": 19.05},
            "origin": {"x": 0.0, "y": 0.0},
            "definitionId": "switch:base",
            "partIds": key_ids,
            "boardId": BOARD_ID,
            "cells": (0..3).map(|column| serde_json::json!({
                "row": 0, "column": column, "enabled": true,
                "definitionId": "switch:base", "assemblies": []
            })).collect::<Vec<_>>()
        }))
        .unwrap(),
    );
    document
}

pub async fn open_matrix_runtime() -> Rc<Runtime> {
    let runtime = support::new_runtime();
    support::open_document(&runtime, matrix_document()).await;
    runtime
}

/// Ticket 07: the matrix document plus a layout homing the matrix and one
/// anchored independent part (the MCU), following the split-board-editing
/// column vocabulary: local Offset/Rotation plus the Spread and Stagger
/// carried from anchor key r0c1.
pub fn anchored_part(step: (f64, f64), anchored: bool, locked: bool) -> Part {
    Part {
        keycap: None,
        outline: None,
        id: "mcu".into(),
        definition_id: "switch:base".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2 { x: 60.0, y: 40.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: anchored.then(|| PartAnchor {
            matrix_id: MATRIX_ID.into(),
            row: 0,
            column: 1,
            follow: Follow {
                x: true,
                y: true,
                rotation: true,
            },
            spread: step.0,
            stagger: step.1,
            splay: 0.0,
            offset: Vec2::default(),
            rotation: 0.0,
        }),
        locked: locked.then_some(true),
        properties: None,
        generator_parameters: None,
    }
}

/// Ticket 07: one layout homing the matrix and the independent parts, so
/// part anchor edits resolve.
pub fn part_document(step: (f64, f64)) -> ProjectDoc {
    let mut document = matrix_document();
    document.boards[0].part_ids.push("mcu".into());
    document.layouts.push(Layout {
        id: "layout".into(),
        name: "Layout".into(),
        board_id: BOARD_ID.into(),
        matrix_ids: vec![MATRIX_ID.into()],
        part_ids: vec!["mcu".into()],
        mirror_link: None,
    });
    document.parts.push(anchored_part(step, true, false));
    document
}

pub async fn open_part_runtime(step: (f64, f64)) -> Rc<Runtime> {
    let runtime = support::new_runtime();
    support::open_document(&runtime, part_document(step)).await;
    runtime
}

/// Ticket 07: the single-layout part document with a detached (and unlocked)
/// MCU, for the detached rail.
pub fn detached_part_document() -> ProjectDoc {
    let mut document = part_document((0.0, 0.0));
    let part = document
        .parts
        .iter_mut()
        .find(|part| part.id == "mcu")
        .expect("the mcu part");
    part.anchor = None;
    document
}

pub async fn open_detached_part_runtime() -> Rc<Runtime> {
    let runtime = support::new_runtime();
    support::open_document(&runtime, detached_part_document()).await;
    runtime
}

/// Ticket 07: a mirrored pair whose halves each carry an anchored independent
/// part, with a Part correspondence so a rail drag on one half fans out to
/// the other per ADR-0038.
pub fn part_pair_document() -> ProjectDoc {
    let mut document = matrix_document();
    document.boards[0].part_ids.push("mcu".into());
    document.boards[0].part_ids.push("mcu-mirror".into());
    document.matrices.push(
        serde_json::from_value(serde_json::json!({
            "id": "matrix-mirror",
            "rows": 1,
            "columns": 3,
            "pitch": {"x": 19.05, "y": 19.05},
            "origin": {"x": 0.0, "y": 0.0},
            "definitionId": "switch:base",
            "partIds": ["matrix/matrix-mirror/r0c0", "matrix/matrix-mirror/r0c1", "matrix/matrix-mirror/r0c2"],
            "mirror": "x",
            "boardId": BOARD_ID,
            "cells": (0..3).map(|column| serde_json::json!({
                "row": 0, "column": column, "enabled": true,
                "definitionId": "switch:base", "assemblies": []
            })).collect::<Vec<_>>()
        }))
        .unwrap(),
    );
    for (index, column) in [0, 1, 2].iter().enumerate() {
        document.parts.push(Part {
            keycap: None,
            outline: None,
            anchor: None,
            id: format!("matrix/matrix-mirror/r0c{column}"),
            definition_id: "switch:base".into(),
            reference: format!("SW{}M", index + 1),
            pose: Pose2 {
                at: Vec2 {
                    x: 19.05 * *column as f64,
                    y: 0.0,
                },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        document.boards[0]
            .part_ids
            .push(format!("matrix/matrix-mirror/r0c{column}"));
    }
    document.layouts.push(Layout {
        id: "left".into(),
        name: "Left".into(),
        board_id: BOARD_ID.into(),
        matrix_ids: vec![MATRIX_ID.into()],
        part_ids: vec!["mcu".into()],
        mirror_link: None,
    });
    document.layouts.push(Layout {
        id: "right".into(),
        name: "Right".into(),
        board_id: BOARD_ID.into(),
        matrix_ids: vec!["matrix-mirror".into()],
        part_ids: vec!["mcu-mirror".into()],
        mirror_link: Some(LayoutMirrorLink {
            source_id: "left".into(),
            axis_x: 40.0,
            split_gap: Some(80.0),
            spacing: None,
            correspondences: vec![
                MirroredCorrespondence {
                    kind: MirroredItemKind::Matrix,
                    source_id: MATRIX_ID.into(),
                    mirror_id: "matrix-mirror".into(),
                    companion_mode: None,
                    companion_tweak: None,
                },
                MirroredCorrespondence {
                    kind: MirroredItemKind::Part,
                    source_id: "mcu".into(),
                    mirror_id: "mcu-mirror".into(),
                    companion_mode: None,
                    companion_tweak: None,
                },
            ],
            exceptions: vec![],
        }),
    });
    let mut mcu = anchored_part((19.0, 0.0), true, false);
    document.parts.push(mcu.clone());
    mcu.id = "mcu-mirror".into();
    mcu.reference = "U1M".into();
    if let Some(anchor) = mcu.anchor.as_mut() {
        anchor.matrix_id = "matrix-mirror".into();
        anchor.spread = -19.0;
    }
    document.parts.push(mcu);
    document
}

pub async fn open_part_pair_runtime() -> Rc<Runtime> {
    let runtime = support::new_runtime();
    support::open_document(&runtime, part_pair_document()).await;
    runtime
}

pub fn accepted_matrix(runtime: &Runtime) -> boardstudio_model::model::Matrix {
    runtime
        .model()
        .accepted
        .expect("an accepted document")
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == MATRIX_ID)
        .cloned()
        .expect("the matrix exists")
}

/// Drive the in-process runtime until `ticket` settles, polling the parked reply task.
pub async fn settle_ticket(
    runtime: &Rc<Runtime>,
    ticket: &boardstudio_web_runtime::operation_outcomes::EditOutcome,
) {
    for _ in 0..100 {
        support::run_pending(runtime).await;
        if !ticket.is_pending() {
            return;
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
}

pub async fn undo(runtime: &Rc<Runtime>) {
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    support::run_pending(runtime).await;
}

pub async fn redo(runtime: &Rc<Runtime>) {
    runtime.submit(boardstudio_application::Event::Redo {
        operation_id: runtime.operation(),
    });
    support::run_pending(runtime).await;
}

/// The accepted document revision.
pub fn revision(runtime: &Runtime) -> u64 {
    runtime
        .model()
        .accepted
        .expect("an accepted document")
        .document
        .revision
}

/// The accepted matrix named `id`.
pub fn matrix_named(runtime: &Runtime, id: &str) -> boardstudio_model::model::Matrix {
    runtime
        .model()
        .accepted
        .expect("an accepted document")
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .cloned()
        .expect("the matrix exists")
}

/// The accepted part named `id`.
pub fn part_named(runtime: &Runtime, id: &str) -> Part {
    runtime
        .model()
        .accepted
        .expect("an accepted document")
        .document
        .parts
        .iter()
        .find(|part| part.id == id)
        .cloned()
        .expect("the part exists")
}

/// Drive the runtime until `condition` holds, failing with `what` at the shared bound.
pub async fn pump_until(runtime: &Rc<Runtime>, what: &str, condition: impl Fn() -> bool) {
    boardstudio_web_ui_shared::test_wait::settle_until(
        runtime,
        support::run_pending,
        what,
        condition,
    )
    .await;
}

/// Drive the runtime until the accepted revision passes `revision`.
pub async fn pump_past(runtime: &Rc<Runtime>, revision: u64) {
    pump_until(runtime, "the edit lands", || {
        self::revision(runtime) > revision
    })
    .await;
}

/// Drive the runtime until the display preview exists.
pub async fn pump_preview(runtime: &Rc<Runtime>) {
    pump_until(runtime, "the gesture previews", || {
        runtime.model().display_preview.is_some()
    })
    .await;
}

/// Drive the runtime until it is Ready with its accepted revision saved.
pub async fn wait_until_editable(runtime: &Rc<Runtime>) {
    use boardstudio_application::{Durability, Lifecycle};
    pump_until(runtime, "the runtime becomes editable", || {
        let model = runtime.model();
        model.lifecycle == Lifecycle::Ready
            && model.accepted.as_ref().is_some_and(|accepted| {
                model.durability
                    == (Durability::Saved {
                        revision: accepted.document.revision,
                    })
            })
    })
    .await;
}

/// A key or standalone part at `at`, unlocked, front side.
pub fn key_part(id: &str, reference: &str, definition_id: &str, at: Vec2) -> Part {
    Part {
        keycap: None,
        outline: None,
        anchor: None,
        id: id.into(),
        definition_id: definition_id.into(),
        reference: reference.into(),
        pose: Pose2 { at, rotation: 0.0 },
        side: Side::Front,
        locked: Some(false),
        properties: None,
        generator_parameters: None,
    }
}

/// One enabled matrix cell with its own definition and nothing local.
pub fn matrix_cell(row: u32, column: u32, definition_id: &str) -> MatrixCell {
    MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        row,
        column,
        enabled: true,
        definition_id: Some(definition_id.into()),
        variant: None,
        offset: None,
        rotation: None,
        assemblies: vec![],
        assemblies_local: None,
    }
}

/// A 1x1 matrix of `key_id` at `origin` on `board_id`.
pub fn single_key_matrix(
    id: &str,
    key_id: &str,
    definition_id: &str,
    board_id: &str,
    pitch: f64,
    origin: Vec2,
) -> Matrix {
    Matrix {
        id: id.into(),
        name: None,
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: pitch, y: pitch },
        origin,
        definition_id: definition_id.into(),
        part_ids: vec![key_id.into()],
        board_id: Some(board_id.into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![matrix_cell(0, 0, definition_id)],
    }
}

/// An anchor on key r0c0 of `matrix_id` following x, y and (when asked) rotation.
pub fn key_anchor(matrix_id: &str, rotation: bool) -> MatrixAnchor {
    MatrixAnchor {
        matrix_id: matrix_id.into(),
        row: 0,
        column: 0,
        follow: Follow {
            x: true,
            y: true,
            rotation,
        },
        aggregate: None,
    }
}

/// Mount `dom` into a fresh root appended to the shared document body.
pub fn mount_root(dom: dioxus::prelude::VirtualDom) -> (web_sys::Element, MountedFixture) {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let mounted = MountedFixture::new(root.clone(), dom);
    (root, mounted)
}

/// The first element under `root` matching `selector`.
pub fn query(root: &web_sys::Element, selector: &str) -> Option<web_sys::Element> {
    root.query_selector(selector).unwrap()
}

/// Every element under `root` matching `selector`.
pub fn query_all(root: &web_sys::Element, selector: &str) -> Vec<web_sys::Element> {
    let list = root.query_selector_all(selector).unwrap();
    (0..list.length())
        .filter_map(|index| list.item(index))
        .filter_map(|node| node.dyn_into().ok())
        .collect()
}

/// The text of the first element matching `selector`, or "" when none matches.
pub fn text_of(root: &web_sys::Element, selector: &str) -> String {
    query(root, selector)
        .and_then(|element| element.text_content())
        .unwrap_or_default()
}

/// The `(x, y)` of an element's `translate(x y)` transform.
pub fn translate_of(element: &web_sys::Element) -> (f64, f64) {
    let transform = element.get_attribute("transform").unwrap_or_default();
    let inner = transform
        .strip_prefix("translate(")
        .and_then(|rest| rest.split(')').next())
        .unwrap_or("");
    let mut parts = inner.split(' ');
    let mut next = || {
        parts
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(f64::NAN)
    };
    (next(), next())
}

/// Dispatch a script-created pointer event. The returned event is never trusted: mounted drag
/// coverage proves wiring and math through this seam, while real trusted-pointer proof lives in
/// the packaged-page mouse smoke.
pub fn dispatch_pointer(
    target: &web_sys::Element,
    kind: &str,
    ctrl: bool,
    alt: bool,
    buttons: u16,
    client: (i32, i32),
) -> web_sys::PointerEvent {
    let init = js_sys::Object::new();
    for (name, value) in [
        ("bubbles", wasm_bindgen::JsValue::TRUE),
        ("cancelable", wasm_bindgen::JsValue::TRUE),
        ("pointerId", 17.into()),
        ("pointerType", "mouse".into()),
        ("isPrimary", wasm_bindgen::JsValue::TRUE),
        ("button", 0.into()),
        ("buttons", buttons.into()),
        ("ctrlKey", ctrl.into()),
        ("altKey", alt.into()),
        ("clientX", client.0.into()),
        ("clientY", client.1.into()),
    ] {
        js_sys::Reflect::set(init.as_ref(), &name.into(), &value).unwrap();
    }
    let constructor = js_sys::Reflect::get(&js_sys::global(), &"PointerEvent".into())
        .unwrap()
        .dyn_into::<js_sys::Function>()
        .unwrap();
    let arguments = js_sys::Array::of2(&kind.into(), init.as_ref());
    let event = js_sys::Reflect::construct(&constructor, &arguments)
        .unwrap()
        .dyn_into::<web_sys::PointerEvent>()
        .unwrap();
    target.dispatch_event(&event).unwrap();
    event
}

/// Dispatch a bubbling `keydown` of `key` on `target`.
pub fn press_key(target: &web_sys::EventTarget, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    target
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
}

/// Type `value` into `input` with one bubbling `input` event.
pub fn type_value(input: &web_sys::HtmlInputElement, value: &str) {
    input.set_value(value);
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
        .unwrap();
}

/// Type `value` into `input` and commit it with Enter.
pub fn commit_value(input: &web_sys::HtmlInputElement, value: &str) {
    type_value(input, value);
    press_key(input, "Enter");
}

/// Stub pointer capture on every element under `root` matching `selector`: a synthetic pointer
/// id is not an active pointer, so the browser refuses real capture. Everything else stays the
/// production pointer path.
pub fn stub_pointer_capture(root: &web_sys::Element, selector: &str) {
    let stub = js_sys::Function::new_no_args("return true;");
    for handle in query_all(root, selector) {
        for method in [
            "setPointerCapture",
            "hasPointerCapture",
            "releasePointerCapture",
        ] {
            js_sys::Reflect::set(handle.as_ref(), &method.into(), stub.as_ref()).unwrap();
        }
    }
}

/// World point to client pixels for the mounted 100-unit, 400 px overlay svg (4 px per mm,
/// y up).
pub fn world_to_client(svg: &web_sys::Element) -> impl Fn(f64, f64) -> (i32, i32) + use<> {
    let rect = svg.get_bounding_client_rect();
    move |x, y| {
        (
            (rect.left() + x * 4.0).round() as i32,
            (rect.top() - y * 4.0).round() as i32,
        )
    }
}
