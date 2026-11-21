#![allow(unused_imports)]
use super::tests_support::*;
use crate::mechanical_pair::{MechanicalPairFeatureRow, MechanicalPairLinkOption, feature_label};
use boardstudio_model::model::*;
use boardstudio_web_ui_shared::ui::button::Button;
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use super::*;
use crate::mechanical_settings_controller::{
    MechanicalSettingsController, MechanicalSettingsCurrent, MechanicalSettingsPorts,
};
use crate::runtime::Runtime;
use boardstudio_application::Event;
use boardstudio_model::model::{
    GasketConstructionVersion, InternalGasketConfiguration, PartDefinition, ProjectDoc,
};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::{
    test_role::by_role_name,
    test_wait::{settle_until, wait_until, wait_until_future},
};
use futures_channel::oneshot;
use std::{cell::RefCell, collections::VecDeque};
use wasm_bindgen_test::wasm_bindgen_test;

#[derive(Clone, Default)]
struct TestCatalogue(Rc<RefCell<VecDeque<CatalogueReply>>>);

type CatalogueReply = oneshot::Receiver<Result<(), String>>;

/// The normalized mounting-hole definition Core validates on every supplied
/// template: the `ceoloide/mounting_hole_npth` source with a drilled pad.
fn template() -> Rc<PartDefinition> {
    Rc::new(
        serde_json::from_value(serde_json::json!({
        "id": "generator:ceoloide/mounting_hole_npth", "name": "mounting hole npth",
        "kind": "custom",
        "pads": [{
            "id": "pad-0", "number": "", "at": { "x": 0, "y": 0 },
            "size": { "x": 2.2, "y": 2.2 }, "shape": "circle", "drill": 2.2,
            "plated": false, "rotation": 0
        }],
        "courtyard": [
            { "x": -1.1, "y": -1.1 }, { "x": 1.1, "y": -1.1 },
            { "x": 1.1, "y": 1.1 }, { "x": -1.1, "y": 1.1 }
        ],
        "envelopeSource": { "courtyard": "generated" },
        "envelopeNotice": "No closed courtyard is available; the outline uses physical graphics and pad extents.",
        "generator": { "source": "ceoloide/mounting_hole_npth", "version": "bundled-1", "parameters": {} }
    }))
        .unwrap(),
    )
}

fn profile() -> MechanicalPartProfile {
    MechanicalPartProfile {
        definition_id: "part-profile".into(),
        source: "Authored".into(),
        source_geometry: None,
        pcb_holes: None,
        clearance_volumes: None,
        openings: Some(vec![CaseOpening {
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 5.0, y: 0.0 },
                Vec2 { x: 5.0, y: 5.0 },
                Vec2 { x: 0.0, y: 5.0 },
            ],
            z: 0.0,
            height: 10.0,
        }]),
        clearances: None,
        supported_thickness: None,
        switch_family: None,
        cutouts: vec![],
        plate_to_pcb: 3.5,
    }
}

fn host() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_signal(|| 0_u64);
    use_hook({
        let runtime = runtime.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }))
        }
    });
    let _ = version();
    let feedback = use_signal(Vec::<MechanicalSettingsFeedback>::new);
    let sequence = use_signal(|| 0_u64);
    let mut show_dimension = use_signal(|| true);
    let mut show_case = use_signal(|| true);
    let mut presentation_generation = use_signal(|| 1_u64);
    let catalogue = use_context::<TestCatalogue>();
    let current = {
        let runtime = runtime.clone();
        Rc::new(move || {
            let model = runtime.model();
            let accepted = model.accepted?;
            Some(MechanicalSettingsCurrent {
                identity: MechanicalSettingsIdentity {
                    editor_instance_id: 1,
                    scope_generation: 1,
                    presentation_generation: presentation_generation(),
                    scope: runtime.scope()?,
                    snapshot_token: accepted.token,
                    revision: accepted.document.revision,
                    active_board_id: "board".into(),
                    configuration_board_id: "board".into(),
                },
                configuration: accepted.document.mechanical.clone().map(Rc::new),
                accepted,
                editable: show_case(),
                lifecycle: model.lifecycle,
                durability: model.durability,
            })
        })
    };
    let controller = use_hook({
        let current = current.clone();
        let runtime = runtime.clone();
        move || {
            MechanicalSettingsController::new(MechanicalSettingsPorts {
                current,
                load_mounting_hole: Rc::new(move || {
                    let reply = catalogue.0.borrow_mut().pop_front();
                    Box::pin(async move {
                        if let Some(reply) = reply {
                            reply.await.unwrap()?;
                        }
                        Ok(template())
                    })
                }),
                resolve_edit: Rc::new(move |resolver| {
                    runtime.resolve_edit(
                        "mechanical-settings",
                        Some("mechanical settings".into()),
                        resolver,
                    )
                }),
                publish: Rc::new(move |entry| {
                    let mut feedback = feedback;
                    feedback.write().push(entry);
                }),
            })
        }
    });
    use_context_provider(|| controller.clone());
    use_effect(use_reactive(
        (&version(), &presentation_generation(), &show_case()),
        {
            let controller = controller.clone();
            move |_| controller.settle()
        },
    ));
    let current = current().unwrap();
    let configuration = current.configuration.unwrap();
    let dimension_controller = controller.clone();
    let text_controller = controller.clone();
    let profile_controller = controller.clone();
    let profile = configuration.profiles[0].clone();
    rsx! {
        span { class: "test-revision", "{current.identity.revision}" }
        Button { class: "test-toggle-dimension", onclick: move |_| show_dimension.toggle(), "Toggle dimension" }
        Button { class: "test-toggle-case", onclick: move |_| {
            presentation_generation += 1;
            show_case.toggle();
        }, "Toggle Case" }
        if show_case() {
        if show_dimension() {
        div { class: "test-mechanical-dimension",
            DimensionField {
                identity: current.identity.clone(), request_sequence: sequence,
                feedback: Rc::from(feedback().as_slice()),
                field: MechanicalDimension::WallThickness, label: "Wall thickness",
                value: configuration.wall_thickness, editable: true,
                on_request: move |request| { dimension_controller.submit(request); },
            }
        }
        }
        div { class: "test-mechanical-text",
            TextDraftField {
                identity: current.identity.clone(), request_sequence: sequence,
                feedback: Rc::from(feedback().as_slice()), label: "Thread",
                value: configuration.internal_gasket.as_ref().unwrap().hardware.thread.clone(),
                intent: MechanicalTextIntent::ClosureThread, editable: true,
                on_request: move |request| { text_controller.submit(request); },
            }
        }
        div { class: "test-mechanical-profile",
            ProfileOpeningListEditor {
                identity: current.identity, request_sequence: sequence,
                feedback: Rc::from(feedback().as_slice()),
                profile: profile.clone(),
                clearance_volume: false, title: "Access openings",
                openings: profile.openings.clone().unwrap(), editable: true,
                on_request: move |request| { profile_controller.submit(request); },
                on_change: move |_| {},
            }
        }
        }
    }
}

async fn mounted_fields(project_id: &str) -> (Rc<Runtime>, web_sys::Element, MountedFixture) {
    mounted_fields_with_catalogue(project_id, TestCatalogue::default()).await
}

async fn mounted_fields_with_catalogue(
    project_id: &str,
    catalogue: TestCatalogue,
) -> (Rc<Runtime>, web_sys::Element, MountedFixture) {
    let runtime = support::new_runtime();
    let mut document = ProjectDoc::empty(project_id, "Mechanical field settlement");
    document.boards.push(
        serde_json::from_value(serde_json::json!({
            "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
            "netIds": [], "thickness": 1.6, "traces": [], "vias": []
        }))
        .unwrap(),
    );
    let hardware = InternalClosureHardware {
        id: "custom-m2".into(),
        thread: "M2".into(),
        screw_lengths: vec![8.0, 10.0, 12.0],
        thread_diameter: 2.0,
        pitch: 0.4,
        drive: ScrewDrive::Hex,
        installation: InsertInstallation::HeatSet,
        length_datum: ScrewLengthDatum::UnderHead,
        head_profile: ScrewHeadProfile::Flat,
        fixed_length: None,
        head_diameter: 4.0,
        head_height: 1.0,
        hole_diameter: 2.2,
        insert_diameter: 3.2,
        insert_length: 3.0,
        seat_diameter: 2.8,
        seat_depth: 3.5,
        engagement: 2.5,
        thread_start: 0.2,
        tip_allowance: 0.1,
        bottoming_clearance: 0.5,
        roof: 1.5,
        surround: 1.5,
        seat_lead_depth: 0.25,
        seat_lead_diameter: 3.0,
        bearing_thickness: 1.5,
    };
    let gasket = InternalGasketConfiguration {
        version: GasketConstructionVersion::InternalV1,
        minimum_wall: 2.0,
        support_clearance: None,
        tolerance: 0.05,
        support_count: 4,
        auto_count: Some(true),
        hardware,
    };
    document.mechanical = Some(
        serde_json::from_value(serde_json::json!({
            "boardId": "board", "method": "printed", "mount": "rigid",
            "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
            "bottomFoamThickness": 0.4, "batteryHeight": 4.0, "bottomThickness": 3.0,
            "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3, "profiles": [profile()],
            "closureMounts": [], "internalGasket": gasket,
        }))
        .unwrap(),
    );
    support::open_document(&runtime, document).await;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(catalogue);
    let mounted = MountedFixture::new(root.clone(), dom);
    wait_until("the mounted mechanical fields render", || {
        has(&root, ".test-mechanical-dimension input") && has(&root, ".test-mechanical-text input")
    })
    .await;
    (runtime, root, mounted)
}

async fn rendered() {
    gloo_timers::future::TimeoutFuture::new(40).await;
}

fn wall_thickness(runtime: &Runtime) -> f64 {
    runtime
        .model()
        .accepted
        .unwrap()
        .document
        .mechanical
        .as_ref()
        .unwrap()
        .wall_thickness
}

fn accepted_revision(runtime: &Runtime) -> u64 {
    runtime.model().accepted.unwrap().document.revision
}

/// Whether the host has rendered the revision Core accepted last: Core state reaches the
/// fields only through a render, so a wait on the revision alone can pass before it.
fn rendered_accepted(runtime: &Runtime, root: &web_sys::Element) -> bool {
    root.query_selector(".test-revision")
        .ok()
        .flatten()
        .and_then(|marker| marker.text_content())
        .is_some_and(|text| text == accepted_revision(runtime).to_string())
}

fn has(root: &web_sys::Element, selector: &str) -> bool {
    root.query_selector(selector).ok().flatten().is_some()
}

/// Short drain, kept only where the test asserts a stale outcome does not land.
async fn settle_fields(runtime: &Rc<Runtime>) {
    for _ in 0..2 {
        support::run_pending(runtime).await;
        rendered().await;
    }
}

fn input(root: &web_sys::Element, field: &str) -> HtmlInputElement {
    let (role, name) = if field == "dimension" {
        ("spinbutton", "Wall thickness")
    } else {
        ("textbox", "Thread")
    };
    if field == "dimension" {
        by_role_name(root, role, name).unwrap().dyn_into().unwrap()
    } else {
        // The 018 text owner is still native and retains its implicit wrapping label.
        root.query_selector(".test-mechanical-text input")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }
}

fn type_value(input: &HtmlInputElement, value: &str) {
    input.focus().unwrap();
    input.set_value(value);
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
        .unwrap();
}

fn inline_failure(root: &web_sys::Element, field: &str) -> Option<String> {
    root.query_selector(&format!(".test-mechanical-{field} [role='alert']"))
        .unwrap()
        .and_then(|alert| alert.text_content())
}

async fn dispose(runtime: &Rc<Runtime>, root: web_sys::Element, project_id: &str) {
    runtime
        .store
        .delete_project(project_id.into())
        .await
        .unwrap();
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn a_newer_original_mechanical_draft_survives_an_older_held_landing() {
    for (field, submitted, original) in [("dimension", "3", "2"), ("text", "M3", "M2")] {
        let project_id = format!("mechanical-original-{field}");
        let (runtime, root, _mounted) = mounted_fields(&project_id).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let input = input(&root, field);
        type_value(&input, submitted);
        input.blur().unwrap();
        rendered().await;
        support::drive_pending(&runtime);
        wait_until_future("the held edit reaches the Core executor", entered)
            .await
            .unwrap();
        type_value(&input, original);
        let landed = accepted_revision(&runtime);
        release.send(()).unwrap();
        settle_until(
            &runtime,
            async |runtime| support::drive_pending(runtime),
            "the held landing is accepted and the newer original draft stays",
            || {
                accepted_revision(&runtime) > landed
                    && rendered_accepted(&runtime, &root)
                    && input.value() == original
            },
        )
        .await;
        assert_eq!(
            input.value(),
            original,
            "returning to the original accepted text remains a newer draft"
        );
        assert!(inline_failure(&root, field).is_none());
        dispose(&runtime, root, &project_id).await;
    }
}

#[wasm_bindgen_test]
async fn mounted_mechanical_latest_commit_keeps_both_edits_and_undo_steps() {
    let project_id = "mechanical-mounted-latest";
    let (runtime, root, _mounted) = mounted_fields(project_id).await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let input = input(&root, "dimension");
    type_value(&input, "3");
    input.blur().unwrap();
    rendered().await;
    support::drive_pending(&runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    type_value(&input, "4");
    input.blur().unwrap();
    rendered().await;
    support::drive_pending(&runtime);
    release.send(()).unwrap();
    settle_until(
        &runtime,
        async |runtime| support::drive_pending(runtime),
        "the latest mechanical commit is accepted",
        || wall_thickness(&runtime) == 4.0 && input.value() == "4",
    )
    .await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .mechanical
            .as_ref()
            .unwrap()
            .wall_thickness,
        4.0
    );
    assert_eq!(input.value(), "4");
    for (accepted, expected_text) in [(3.0, "3"), (2.0, "2")] {
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        settle_until(
            &runtime,
            async |runtime| support::drive_pending(runtime),
            "the undo restores the earlier accepted step",
            || wall_thickness(&runtime) == accepted && input.value() == expected_text,
        )
        .await;
        assert_eq!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .mechanical
                .as_ref()
                .unwrap()
                .wall_thickness,
            accepted
        );
        assert_eq!(input.value(), expected_text);
    }
    dispose(&runtime, root, project_id).await;
}
#[wasm_bindgen_test]
async fn catalogue_failure_restores_only_unchanged_drafts_and_reports_beside_newer_text() {
    for (field, submitted, newer, expected) in [
        ("dimension", "3", None, "2"),
        ("dimension", "3", Some("4"), "4"),
        ("dimension", "3", Some("2"), "2"),
        ("text", "M3", None, "M2"),
        ("text", "M3", Some("M4"), "M4"),
        ("text", "M3", Some("M2"), "M2"),
    ] {
        let (release, reply) = oneshot::channel();
        let catalogue = TestCatalogue::default();
        catalogue.0.borrow_mut().push_back(reply);
        let project_id = format!("mechanical-catalogue-{field}-{expected}");
        let (runtime, root, _mounted) = mounted_fields_with_catalogue(&project_id, catalogue).await;
        let input = input(&root, field);
        type_value(&input, submitted);
        input.blur().unwrap();
        rendered().await;
        if let Some(newer) = newer {
            type_value(&input, newer);
        }
        release.send(Err("catalogue unavailable".into())).unwrap();
        settle_until(
            &runtime,
            async |runtime| support::drive_pending(runtime),
            "the catalogue failure restores or keeps the draft",
            || {
                input.value() == expected
                    && wall_thickness(&runtime) == 2.0
                    && inline_failure(&root, field)
                        .is_some_and(|message| message.contains("catalogue unavailable"))
            },
        )
        .await;
        assert_eq!(input.value(), expected);
        assert!(
            inline_failure(&root, field)
                .is_some_and(|message| message.contains("catalogue unavailable"))
        );
        assert_eq!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .mechanical
                .as_ref()
                .unwrap()
                .wall_thickness,
            2.0
        );
        dispose(&runtime, root, &project_id).await;
    }
}

#[wasm_bindgen_test]
async fn preparation_from_an_unmounted_field_cannot_bind_a_replacement_field() {
    let (release, reply) = oneshot::channel();
    let catalogue = TestCatalogue::default();
    catalogue.0.borrow_mut().push_back(reply);
    let project_id = "mechanical-preparation-remount";
    let (runtime, root, _mounted) = mounted_fields_with_catalogue(project_id, catalogue).await;
    support::fail_next_core_reply(&runtime, "old preparation failed");
    let old_input = input(&root, "dimension");
    type_value(&old_input, "3");
    old_input.blur().unwrap();
    rendered().await;
    let toggle = root
        .query_selector(".test-toggle-dimension")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    toggle.click();
    rendered().await;
    assert!(
        root.query_selector(".test-mechanical-dimension")
            .unwrap()
            .is_none()
    );
    toggle.click();
    rendered().await;
    let replacement = input(&root, "dimension");
    type_value(&replacement, "3");
    release.send(Ok(())).unwrap();
    settle_fields(&runtime).await;
    assert_eq!(
        replacement.value(),
        "3",
        "the old submission must not restore the replacement's identical draft"
    );
    assert!(
        inline_failure(&root, "dimension").is_none(),
        "the old outcome must not reach the replacement binding"
    );
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .mechanical
            .as_ref()
            .unwrap()
            .wall_thickness,
        2.0
    );
    dispose(&runtime, root, project_id).await;
}

fn profile_input(root: &web_sys::Element, dimension: &str) -> HtmlInputElement {
    by_role_name(
        root,
        "spinbutton",
        &format!("Access openings volume 1 {dimension}"),
    )
    .unwrap()
    .dyn_into()
    .unwrap()
}

fn failure_at(input: &HtmlInputElement) -> Option<String> {
    input
        .closest("[data-slot=field]")
        .unwrap()
        .unwrap()
        .query_selector("[role='alert']")
        .unwrap()
        .and_then(|alert| alert.text_content())
}

#[wasm_bindgen_test]
async fn profile_volume_siblings_bind_distinct_fields_for_the_same_domain_request() {
    let project_id = "mechanical-profile-siblings";
    let (runtime, root, _mounted) = mounted_fields(project_id).await;
    support::fail_next_core_reply(&runtime, "profile volume failed");
    let bottom = profile_input(&root, "Bottom Z");
    let height = profile_input(&root, "Height");
    type_value(&bottom, "2");
    bottom.blur().unwrap();
    rendered().await;
    type_value(&height, "15");
    settle_until(
        &runtime,
        async |runtime| support::drive_pending(runtime),
        "the profile volume failure restores its field",
        || {
            bottom.value() == "0"
                && failure_at(&bottom)
                    .is_some_and(|message| message.contains("profile volume failed"))
        },
    )
    .await;
    assert_eq!(bottom.value(), "0");
    assert!(failure_at(&bottom).is_some_and(|message| message.contains("profile volume failed")));
    assert_eq!(
        height.value(),
        "15",
        "an untouched sibling's newer draft stays visible"
    );
    assert!(
        failure_at(&height).is_none(),
        "failure belongs to the submitted profile dimension"
    );
    dispose(&runtime, root, project_id).await;
}

#[wasm_bindgen_test]
async fn catalogue_latest_preparation_error_survives_older_failure_arriving_last() {
    let (release_a, reply_a) = oneshot::channel();
    let (release_b, reply_b) = oneshot::channel();
    let catalogue = TestCatalogue::default();
    catalogue.0.borrow_mut().extend([reply_a, reply_b]);
    let project_id = "mechanical-catalogue-latest-error";
    let (runtime, root, _mounted) = mounted_fields_with_catalogue(project_id, catalogue).await;
    let input = input(&root, "dimension");
    for _ in 0..2 {
        type_value(&input, "3");
        input.blur().unwrap();
        rendered().await;
    }
    release_b
        .send(Err("newer catalogue failure".into()))
        .unwrap();
    settle_until(
        &runtime,
        async |runtime| support::drive_pending(runtime),
        "the newer catalogue failure is reported",
        || {
            input.value() == "2"
                && inline_failure(&root, "dimension")
                    .is_some_and(|message| message.contains("newer catalogue failure"))
        },
    )
    .await;
    assert_eq!(input.value(), "2");
    assert!(
        inline_failure(&root, "dimension")
            .is_some_and(|message| message.contains("newer catalogue failure"))
    );
    release_a
        .send(Err("older catalogue failure".into()))
        .unwrap();
    settle_fields(&runtime).await;
    assert_eq!(input.value(), "2");
    assert!(
        inline_failure(&root, "dimension")
            .is_some_and(|message| message.contains("newer catalogue failure")),
        "the bounded latest admission still owns feedback after its preparation failed"
    );
    dispose(&runtime, root, project_id).await;
}

#[wasm_bindgen_test]
async fn presentation_departure_retires_field_observation_without_canceling_session_edit() {
    let project_id = "mechanical-presentation-return";
    let (runtime, root, _mounted) = mounted_fields(project_id).await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let first_input = input(&root, "dimension");
    type_value(&first_input, "3");
    first_input.blur().unwrap();
    rendered().await;
    support::drive_pending(&runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    let toggle = root
        .query_selector(".test-toggle-case")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    toggle.click();
    rendered().await;
    assert!(
        root.query_selector(".test-mechanical-dimension")
            .unwrap()
            .is_none()
    );
    toggle.click();
    rendered().await;
    let returned = input(&root, "dimension");
    assert_eq!(returned.value(), "2");
    assert!(
        root.query_selector(".test-mechanical-dimension [role='status']")
            .unwrap()
            .is_none(),
        "returning to Case must not revive the old presentation's pending field observation"
    );
    release.send(()).unwrap();
    settle_until(
        &runtime,
        async |runtime| support::drive_pending(runtime),
        "the held Session edit lands and the returned field shows it",
        || wall_thickness(&runtime) == 3.0 && returned.value() == "3",
    )
    .await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .mechanical
            .as_ref()
            .unwrap()
            .wall_thickness,
        3.0,
        "the already submitted Session edit still runs"
    );
    assert_eq!(returned.value(), "3");
    assert!(inline_failure(&root, "dimension").is_none());
    dispose(&runtime, root, project_id).await;
}
#[wasm_bindgen_test]
async fn newer_catalogue_failure_and_draft_survive_an_older_actual_ticket_landing() {
    for newer in ["5", "2"] {
        let (ready, reply_a) = oneshot::channel();
        let (release_b, reply_b) = oneshot::channel();
        ready.send(Ok(())).unwrap();
        let catalogue = TestCatalogue::default();
        catalogue.0.borrow_mut().extend([reply_a, reply_b]);
        let project_id = format!("mechanical-mixed-phase-{newer}");
        let (runtime, root, _mounted) = mounted_fields_with_catalogue(&project_id, catalogue).await;
        let (entered, release_a) = support::gate_next_core_reply(&runtime);
        let input = input(&root, "dimension");
        type_value(&input, "3");
        input.blur().unwrap();
        rendered().await;
        support::drive_pending(&runtime);
        wait_until_future("the held edit reaches the Core executor", entered)
            .await
            .unwrap();
        type_value(&input, "4");
        input.blur().unwrap();
        rendered().await;
        type_value(&input, newer);
        release_b
            .send(Err("newer preparation failed".into()))
            .unwrap();
        settle_until(
            &runtime,
            async |runtime| support::drive_pending(runtime),
            "the newer preparation failure is reported",
            || {
                input.value() == newer
                    && inline_failure(&root, "dimension")
                        .is_some_and(|message| message.contains("newer preparation failed"))
            },
        )
        .await;
        assert_eq!(input.value(), newer);
        assert!(
            inline_failure(&root, "dimension")
                .is_some_and(|message| message.contains("newer preparation failed"))
        );
        assert!(
            root.query_selector(".test-mechanical-dimension [role='status']")
                .unwrap()
                .is_none(),
            "only the latest admitted request drives field pending"
        );
        release_a.send(()).unwrap();
        settle_until(
            &runtime,
            async |runtime| support::drive_pending(runtime),
            "the held Session edit lands beside the newer draft",
            || {
                wall_thickness(&runtime) == 3.0
                    && rendered_accepted(&runtime, &root)
                    && input.value() == newer
                    && inline_failure(&root, "dimension")
                        .is_some_and(|message| message.contains("newer preparation failed"))
            },
        )
        .await;
        assert_eq!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .mechanical
                .as_ref()
                .unwrap()
                .wall_thickness,
            3.0
        );
        assert_eq!(
            input.value(),
            newer,
            "the newer draft survives, including the original accepted value"
        );
        assert!(
            inline_failure(&root, "dimension")
                .is_some_and(|message| message.contains("newer preparation failed")),
            "an older actual ticket cannot clear the newer preparation failure"
        );
        dispose(&runtime, root, &project_id).await;
    }
}
