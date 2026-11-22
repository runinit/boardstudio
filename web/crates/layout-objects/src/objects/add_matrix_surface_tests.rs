//! Browser coverage for the shared Add-matrix surface and its mode toggle.
use super::super::mirrored_pair::{
    MirroredPairFormProjection, MirroredPairFormValues, MirroredPairOwner,
};
use super::*;
use crate::objects::matrix_setup::{MatrixSetupOwner, MatrixSetupProjection};
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_role::by_role_name;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn matrix_owner() -> MatrixSetupOwner {
    MatrixSetupOwner {
        editor_instance_id: 3,
        open_id: 5,
        scope_generation: 7,
        scope: boardstudio_application::Scope {
            session_epoch: boardstudio_application::SessionEpoch(13),
            document_id: "add-matrix-surface-fixture".into(),
            board_id: "board-current".into(),
            instance_id: None,
        },
        board_id: "board-current".into(),
        snapshot_token: boardstudio_application::SnapshotToken(17),
        revision: 19,
    }
}

fn pair_owner() -> MirroredPairOwner {
    MirroredPairOwner {
        editor_instance_id: 3,
        open_id: 5,
        scope_generation: 7,
        scope: boardstudio_application::Scope {
            session_epoch: boardstudio_application::SessionEpoch(13),
            document_id: "add-matrix-surface-fixture".into(),
            board_id: "board-current".into(),
            instance_id: None,
        },
        board_id: "board-current".into(),
        snapshot_token: boardstudio_application::SnapshotToken(17),
        revision: 19,
    }
}

fn single_mount() -> MatrixSetupMount {
    MatrixSetupMount {
        projection: Some(MatrixSetupProjection {
            owner: matrix_owner(),
            editable: true,
            can_cancel: true,
            status: None,
            error: None,
        }),
        can_open: false,
        on_open: EventHandler::default(),
        on_cancel: EventHandler::default(),
        on_create: EventHandler::default(),
    }
}

fn pair_mount() -> MirroredPairMount {
    MirroredPairMount {
        form: Some(MirroredPairFormProjection {
            owner: pair_owner(),
            values: MirroredPairFormValues::default(),
            editable: true,
            error: None,
            status: None,
        }),
        placement: None,
        can_open: false,
        owns_canvas: true,
        on_open: EventHandler::default(),
        on_cancel: EventHandler::default(),
        on_preview: EventHandler::default(),
        on_move: EventHandler::default(),
        on_commit: EventHandler::default(),
        on_created: EventHandler::default(),
    }
}

fn pair_composition() -> Element {
    rsx! {
        div { id: "add-matrix-surface-pair-root",
            AddMatrixSurface {
                matrix_setup: MatrixSetupMount {
                    projection: None,
                    can_open: true,
                    on_open: EventHandler::default(),
                    on_cancel: EventHandler::default(),
                    on_create: EventHandler::default(),
                },
                mirrored_pair: pair_mount(),
                guide_covering_single: false,
            }
        }
    }
}

fn single_composition() -> Element {
    rsx! {
        div { id: "add-matrix-surface-single-root",
            AddMatrixSurface {
                matrix_setup: single_mount(),
                mirrored_pair: MirroredPairMount {
                    form: None,
                    placement: None,
                    can_open: true,
                    owns_canvas: false,
                    on_open: EventHandler::default(),
                    on_cancel: EventHandler::default(),
                    on_preview: EventHandler::default(),
                    on_move: EventHandler::default(),
                    on_commit: EventHandler::default(),
                    on_created: EventHandler::default(),
                },
                guide_covering_single: false,
            }
        }
    }
}

async fn rendered() {
    gloo_timers::future::TimeoutFuture::new(80).await;
}

fn mounted(query: &str) -> bool {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(query)
        .unwrap()
        .is_some()
}

fn mode_pressed(scope: &str, label: &str) -> Option<String> {
    let document = web_sys::window().unwrap().document().unwrap();
    let group = document
        .query_selector(&format!(
            "{scope} [role='group'][aria-label='Add matrix mode']"
        ))
        .unwrap()?;
    by_role_name(&group, "button", label)?.get_attribute("aria-pressed")
}

async fn mount_root(id: &str, make: fn() -> Element) -> MountedFixture {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id(id);
    document.body().unwrap().append_child(&root).unwrap();
    let mounted = MountedFixture::new(root.clone(), VirtualDom::new(make));
    rendered().await;
    mounted
}

#[wasm_bindgen_test]
async fn shared_add_matrix_surface_pair_mode_shows_shared_sliders_and_preview() {
    let _mounted = mount_root("add-matrix-surface-pair-root", pair_composition).await;

    let mode_group =
        mounted("#add-matrix-surface-pair-root [role='group'][aria-label='Add matrix mode']");
    let pair_pressed = mode_pressed("#add-matrix-surface-pair-root", "Mirrored pair");
    let rows_slider =
        mounted("#add-matrix-surface-pair-root [role='slider'][aria-label='Rows per half']");
    let columns_slider =
        mounted("#add-matrix-surface-pair-root [role='slider'][aria-label='Columns per half']");
    let assembly =
        mounted("#add-matrix-surface-pair-root [role='group'][aria-label='Switch mounting']");
    let preview = mounted(
        "#add-matrix-surface-pair-root [role='img'][aria-label='3 rows by 5 columns matrix preview']",
    );
    let second_name =
        mounted("#add-matrix-surface-pair-root input[aria-label='Right layout name']");
    let gap =
        mounted("#add-matrix-surface-pair-root input[aria-label='Gap between key edges (mm)']");
    let single_heading = mounted("#add-matrix-surface-pair-root section[aria-label='New matrix']");

    document_cleanup("add-matrix-surface-pair-root");

    assert!(
        mode_group,
        "pair mode renders the shared Single | Mirrored pair toggle"
    );
    assert_eq!(
        pair_pressed.as_deref(),
        Some("true"),
        "the surface marks Mirrored pair as the active mode"
    );
    assert!(
        rows_slider && columns_slider,
        "pair mode reuses the shared rows/columns sliders"
    );
    assert!(assembly, "pair mode reuses the shared assembly toggles");
    assert!(preview, "pair mode reuses the shared SVG matrix preview");
    assert!(
        second_name && gap,
        "pair mode adds the second layout name and gap fields"
    );
    assert!(
        !single_heading,
        "pair mode replaces the single-matrix form instead of stacking it"
    );
}

#[wasm_bindgen_test]
async fn shared_add_matrix_surface_single_mode_marks_toggle_and_shares_sliders() {
    let _mounted = mount_root("add-matrix-surface-single-root", single_composition).await;

    let single_pressed = mode_pressed("#add-matrix-surface-single-root", "Single");
    let pair_toggle = mode_pressed("#add-matrix-surface-single-root", "Mirrored pair");
    let rows_slider = mounted("#add-matrix-surface-single-root [role='slider'][aria-label='Rows']");
    let preview = mounted(
        "#add-matrix-surface-single-root [role='img'][aria-label='5 rows by 6 columns matrix preview']",
    );

    document_cleanup("add-matrix-surface-single-root");

    assert_eq!(
        single_pressed.as_deref(),
        Some("true"),
        "the surface marks Single as the active mode"
    );
    assert!(
        pair_toggle.is_some(),
        "single mode offers the Mirrored pair toggle"
    );
    assert!(rows_slider, "single mode keeps the shared rows slider");
    assert!(preview, "single mode keeps the shared SVG matrix preview");
}

fn document_cleanup(id: &str) {
    let document = web_sys::window().unwrap().document().unwrap();
    if let Some(root) = document.get_element_by_id(id) {
        root.remove();
    }
}
