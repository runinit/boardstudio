#![allow(unused_imports)]
use super::*;
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_model::model::*;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::ui::button::Button;
use boardstudio_web_ui_shared::{
    test_role::{all_by_role_name, by_role_name},
    test_wait::wait_until,
};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
    static NEXT_RENDER_PROBE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static ACTIVE_RENDER_PROBE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

#[derive(Clone)]
struct RenderProbeId(String);

pub(crate) fn test_page_dom(page: fn() -> Element) -> VirtualDom {
    let id = NEXT_RENDER_PROBE.with(|next| {
        let id = format!("case-render-{}", next.get());
        next.set(next.get() + 1);
        id
    });
    ACTIVE_RENDER_PROBE.with(|active| *active.borrow_mut() = Some(id.clone()));
    let dom = VirtualDom::new(page_with_render_probe);
    dom.provide_root_context(page);
    dom.provide_root_context(RenderProbeId(id));
    dom
}

#[component]
pub(crate) fn page_with_render_probe() -> Element {
    let page = use_context::<fn() -> Element>();
    let id = use_context::<RenderProbeId>().0;
    let mut generation = use_signal(|| 0u64);
    let content = page();
    rsx! {
        {content}
        Button { id: "{id}-advance", onclick: move |_| generation += 1, "render" }
        output { id: "{id}-generation", "{generation()}" }
    }
}

pub(crate) fn test_identity_with_instance(instance_id: &str) -> MechanicalSettingsIdentity {
    let mut identity = test_identity();
    identity.scope.instance_id = Some(instance_id.to_owned());
    identity
}

pub(crate) fn test_identity() -> MechanicalSettingsIdentity {
    MechanicalSettingsIdentity {
        editor_instance_id: 1,
        scope_generation: 1,
        presentation_generation: 1,
        scope: Scope {
            session_epoch: SessionEpoch(1),
            document_id: "document".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        snapshot_token: SnapshotToken(1),
        revision: 1,
        active_board_id: "board".into(),
        configuration_board_id: "board".into(),
    }
}

pub(crate) fn test_values(
    transport: HardwareTransport,
    battery: Option<MechanicalBattery>,
) -> MechanicalSettingsValues {
    MechanicalSettingsValues {
        board_id: "board".into(),
        transport,
        battery,
        suspension_mounts: vec![],
        closure_mounts: Some(vec![]),
        method: PlateMethod::Printed,
        mount: MechanicalMount::Rigid,
        bottom_style: MechanicalBottomStyle::Shell,
        middle_frame: false,
        integrated_plate_frame: false,
        plate_thickness: 1.5,
        plate_foam_thickness: 0.0,
        pcb_thickness: 1.6,
        bottom_foam_thickness: 0.0,
        bottom_thickness: 2.0,
        wall_thickness: 2.0,
        clearance: 0.2,
        opening_allowance: 0.0,
        openings: vec![],
        internal_gasket: true,
        closure_hardware: None,
        hardware: vec![],
        critical_fits: vec![],
        plate_to_pcb: 3.5,
        battery_height: 0.0,
    }
}

pub(crate) fn test_process(
    part_id: &str,
    method: PlateMethod,
    material: &str,
    thickness: f64,
) -> MechanicalProcessTarget {
    MechanicalProcessTarget {
        part_id: part_id.into(),
        name: part_id.replace('-', " "),
        default_thickness: thickness,
        process: Some(MechanicalPartProcess {
            part_id: part_id.into(),
            method,
            material: material.into(),
            thickness,
            constraints_version: "2026-09-24".into(),
        }),
    }
}

pub(crate) fn element(selector: &str) -> web_sys::HtmlElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(selector)
        .unwrap_or_else(|error| panic!("invalid selector {selector}: {error:?}"))
        .unwrap_or_else(|| panic!("no element matches {selector}"))
        .dyn_into()
        .unwrap_or_else(|_| panic!("element matching {selector} is not an HTMLElement"))
}

pub(crate) async fn rendered() {
    let id = ACTIVE_RENDER_PROBE.with(|active| {
        active
            .borrow()
            .clone()
            .expect("the current test page has a render probe")
    });
    let document = web_sys::window().unwrap().document().unwrap();
    let label_selector = format!("#{id}-generation");
    let advance_selector = format!("#{id}-advance");
    wait_until("the Case test page mounts", || {
        document.query_selector(&label_selector).unwrap().is_some()
    })
    .await;
    let label = document.query_selector(&label_selector).unwrap().unwrap();
    let expected = label
        .text_content()
        .unwrap()
        .parse::<u64>()
        .expect("render probe generation is numeric")
        + 1;
    document
        .query_selector(&advance_selector)
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    wait_until("the Case test page renders its next generation", || {
        label.text_content().as_deref() == Some(expected.to_string().as_str())
    })
    .await;
}

/// The primitives commit an option on primary pointerdown/up, not on click.
pub(crate) fn tap_option(option: &web_sys::HtmlElement) {
    for kind in ["pointerdown", "pointerup"] {
        let event = js_sys::eval(&format!(
            "new PointerEvent('{kind}', {{bubbles:true, cancelable:true, button:0}})"
        ))
        .unwrap()
        .dyn_into::<web_sys::Event>()
        .unwrap();
        option.dispatch_event(&event).unwrap();
    }
}

pub(crate) fn mount_battery_test_page(
    root_id: &'static str,
    page: fn() -> Element,
) -> MountedFixture {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id(root_id);
    document.body().unwrap().append_child(&root).unwrap();
    MountedFixture::new(root.clone(), test_page_dom(page))
}
