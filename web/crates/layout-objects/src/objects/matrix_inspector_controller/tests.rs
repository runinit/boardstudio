//! Browser and unit coverage for the Matrix Inspector controller.
//! Pure operation cases are table-driven; mounted scenarios share one probe host.
use super::*;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn variant_owner_does_not_claim_a_later_open_even_with_the_same_project_id() {
    let original_operation = boardstudio_application::SessionEpoch(12);
    let reopened_variant = boardstudio_application::SessionEpoch(13);
    assert!(variant_session_matches(
        original_operation,
        "variant-id",
        original_operation,
        "variant-id",
    ));
    assert!(!variant_session_matches(
        original_operation,
        "variant-id",
        reopened_variant,
        "variant-id",
    ));
    assert!(!variant_session_matches(
        original_operation,
        "variant-id",
        original_operation,
        "user-opened-project",
    ));
}

fn synthetic_outline_document() -> ProjectDoc {
    serde_json::from_str(include_str!(
        "../../../../../../core/tests/fixtures/synthetic-outline.json"
    ))
    .expect("synthetic outline fixture should deserialize")
}

fn synthetic_matrix() -> Matrix {
    let document = synthetic_outline_document();
    document
        .matrices
        .into_iter()
        .find(|matrix| matrix.id == "matrix")
        .expect("synthetic fixture has its authored matrix")
}

fn request_matrix(matrix: &Matrix, field: MatrixEditField, value: MatrixEditValue) -> Matrix {
    let document = synthetic_outline_document();
    match build_operation(&document, matrix, &MatrixNameTarget::Matrix, field, value)
        .expect("the selected matrix field should produce a SetMatrix request")
    {
        EditOperation::SetMatrix { matrix, .. } => matrix,
        _ => panic!("matrix fields should use SetMatrix"),
    }
}

#[wasm_bindgen_test]
fn resizing_synthetic_matrix_preserves_cells_and_inherits_only_electrical_members() {
    let mut matrix = synthetic_matrix();
    matrix.rows = 2;
    matrix.columns = 2;
    let old_rows = matrix.rows;
    let old_columns = matrix.columns;
    matrix
        .cells
        .retain(|cell| cell.row < old_rows && cell.column < old_columns);
    let original = matrix.cells[0].clone();
    matrix.cells[0].variant = Some("preset/mx-hotswap/north".into());
    matrix.cells[0].assemblies.push(MatrixAssembly {
        id: "diode".into(),
        definition_id: "fixture-diode".into(),
        offset: boardstudio_model::model::Vec2 { x: 0.0, y: 0.0 },
        rotation: None,
        side: None,
    });
    matrix.cells[0].assemblies.push(MatrixAssembly {
        id: "led".into(),
        definition_id: "fixture-led".into(),
        offset: boardstudio_model::model::Vec2 { x: 2.0, y: -3.0 },
        rotation: Some(27.0),
        side: Some(boardstudio_model::model::Side::Back),
    });
    matrix.cells[0].assemblies.push(MatrixAssembly {
        id: "local-extra".into(),
        definition_id: "fixture-extra".into(),
        offset: boardstudio_model::model::Vec2 { x: 9.0, y: 8.0 },
        rotation: None,
        side: None,
    });
    let template = matrix.cells[0].clone();

    let resized = request_matrix(
        &matrix,
        MatrixEditField::Columns,
        MatrixEditValue::Columns(3),
    );
    assert_eq!(resized.rows, 2);
    assert_eq!(resized.columns, 3);
    assert_eq!(resized.cells[0].row, original.row);
    assert_eq!(resized.cells[0].offset, original.offset);
    assert_eq!(resized.cells[0].enabled, original.enabled);
    let added = resized
        .cells
        .iter()
        .find(|cell| cell.row == 1 && cell.column == 2)
        .unwrap();
    assert!(added.enabled);
    assert_eq!(
        added.definition_id.as_deref(),
        Some(matrix.definition_id.as_str())
    );
    assert_eq!(added.variant.as_deref(), Some("preset/mx-hotswap/north"));
    assert_eq!(added.rotation, Some(180.0));
    assert_eq!(added.offset, None);
    assert_eq!(added.assemblies_local, template.assemblies_local);
    assert_eq!(
        added.assemblies,
        template
            .assemblies
            .iter()
            .filter(|assembly| assembly.id == "diode" || assembly.id == "led")
            .cloned()
            .collect::<Vec<_>>()
    );
    assert_eq!(added.assemblies.len(), 2);
    assert!(
        !added
            .assemblies
            .iter()
            .any(|assembly| assembly.id == "local-extra")
    );
    assert_eq!(matrix.cells[0].assemblies.len(), template.assemblies.len());
}

/// The synthetic matrix bounded to 2x2 with one sparse slot punched out
/// and one far out-of-bounds cell appended.
fn sparse_two_by_two() -> Matrix {
    let mut matrix = synthetic_matrix();
    matrix.rows = 2;
    matrix.columns = 2;
    matrix.cells.retain(|cell| cell.row < 2 && cell.column < 2);
    matrix.cells[0].variant = Some("preset/mx-hotswap/north".into());
    matrix
        .cells
        .retain(|cell| !(cell.row == 1 && cell.column == 1));
    matrix.cells.push(MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        row: 2,
        column: 5,
        enabled: true,
        definition_id: None,
        variant: None,
        offset: None,
        rotation: None,
        assemblies: Vec::new(),
        assemblies_local: None,
    });
    matrix
}

#[wasm_bindgen_test]
fn resize_preserves_sparse_slots_and_trims_only_cells_outside_new_bounds() {
    let matrix = sparse_two_by_two();
    let before = matrix
        .cells
        .iter()
        .filter(|cell| cell.row < 2 && cell.column < 2)
        .cloned()
        .collect::<Vec<_>>();
    let resized = request_matrix(&matrix, MatrixEditField::Rows, MatrixEditValue::Rows(3));
    let retained = resized
        .cells
        .iter()
        .filter(|cell| cell.row < 2 && cell.column < 2)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(retained, before);
    assert!(
        !resized
            .cells
            .iter()
            .any(|cell| cell.row == 1 && cell.column == 1)
    );
    assert!(
        !resized
            .cells
            .iter()
            .any(|cell| cell.row == 2 && cell.column == 5)
    );
    assert!(
        resized
            .cells
            .iter()
            .any(|cell| cell.row == 2 && cell.column == 1)
    );
}

#[wasm_bindgen_test]
fn custom_matrix_resize_keeps_cells_and_rejects_oversized_dimensions() {
    let mut matrix = synthetic_matrix();
    matrix.cells.iter_mut().for_each(|cell| cell.variant = None);
    let cells = matrix.cells.clone();
    let resized = request_matrix(&matrix, MatrixEditField::Rows, MatrixEditValue::Rows(4));
    assert_eq!(resized.cells, cells);
    // Both dimensions reject empty and overflowing sizes through the same
    // validation path.
    for (field, value) in [
        (MatrixEditField::Rows, u32::MAX),
        (MatrixEditField::Rows, 0),
        (MatrixEditField::Columns, u32::MAX),
        (MatrixEditField::Columns, 0),
    ] {
        let edit = match field {
            MatrixEditField::Rows => MatrixEditValue::Rows(value),
            MatrixEditField::Columns => MatrixEditValue::Columns(value),
            _ => unreachable!("rejection table only covers dimensions"),
        };
        assert!(
            matrix_edit(&matrix, field, edit,).is_err(),
            "{field:?} should reject {value}"
        );
    }
}

#[wasm_bindgen_test]
fn matrix_pitch_request_preserves_every_unrelated_accepted_matrix_field() {
    let matrix = synthetic_matrix();
    let next = request_matrix(
        &matrix,
        MatrixEditField::PitchX,
        MatrixEditValue::PitchX(20.0),
    );
    assert_eq!(next.pitch.x, 20.0);
    assert_eq!(next.pitch.y, matrix.pitch.y);
    assert_eq!(next.rows, matrix.rows);
    assert_eq!(next.columns, matrix.columns);
    assert_eq!(next.cells, matrix.cells);
    assert_eq!(next.part_ids, matrix.part_ids);
    assert_eq!(next.board_id, matrix.board_id);
}

#[wasm_bindgen_test]
fn owning_layout_name_request_uses_set_layout_without_renaming_the_matrix() {
    let document = synthetic_outline_document();
    let mut matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "matrix")
        .unwrap()
        .clone();
    matrix.name = Some("Authored matrix name".into());
    let layout = document
        .layouts
        .iter()
        .find(|layout| {
            layout.matrix_ids.iter().any(|id| id == &matrix.id) && layout.board_id == "main"
        })
        .unwrap();
    let operation = build_operation(
        &document,
        &matrix,
        &MatrixNameTarget::Layout {
            id: layout.id.clone(),
        },
        MatrixEditField::Name,
        MatrixEditValue::Name(Some("Renamed layout".into())),
    )
    .unwrap();
    let EditOperation::SetLayout { layout: renamed } = operation else {
        panic!("layout name changes should use SetLayout");
    };
    assert_eq!(renamed.id, layout.id);
    assert_eq!(renamed.matrix_ids, [matrix.id.clone()]);
    assert_eq!(renamed.name, "Renamed layout");
    assert_eq!(matrix.name.as_deref(), Some("Authored matrix name"));
}

mod queued_edits {
    use super::super::*;
    use crate::presentation::objects::matrix_edit_test_support as fixture;
    use boardstudio_web_runtime::edit_ticket::EditTicket;
    use boardstudio_web_runtime::edit_ticket::Settlement;
    use boardstudio_web_test_runtime::project_name_test_support as support;
    use boardstudio_web_ui_shared::test_mount::MountedFixture;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[derive(Clone)]
    struct MountedMatrixInspectorProbe {
        runtime: Rc<Runtime>,
        scope: Scope,
        selected_context: Rc<RefCell<Option<Signal<Option<ScopedTreeContext>>>>>,
        version: Rc<RefCell<Option<Signal<u64>>>>,
        request_sequence: Rc<RefCell<Option<Signal<u64>>>>,
        visible: Rc<RefCell<Option<Signal<bool>>>>,
    }

    fn mounted_matrix_inspector_host() -> Element {
        let probe = use_context::<MountedMatrixInspectorProbe>();
        let version = use_signal(|| 0u64);
        let workspace = use_signal(|| "Layout");
        let generation = use_signal(|| 1u64);
        let visible = use_signal(|| true);
        let selected = use_signal(|| {
            Some(ScopedTreeContext {
                scope: probe.scope.clone(),
                context: TreeContext::Matrix {
                    matrix_id: fixture::MATRIX_ID.into(),
                },
            })
        });
        *probe.selected_context.borrow_mut() = Some(selected);
        *probe.version.borrow_mut() = Some(version);
        let mount = use_matrix_inspector(
            probe.runtime.clone(),
            version,
            selected,
            workspace,
            generation,
        );
        *probe.request_sequence.borrow_mut() = Some(mount.request_sequence);
        *probe.visible.borrow_mut() = Some(visible);
        if !visible() {
            return rsx! {};
        }
        let Some(projection) = mount.projection else {
            return rsx! {};
        };
        rsx! {
            crate::presentation::objects::MatrixInspector {
                projection,
                request_sequence: mount.request_sequence,
                editable: mount.editable,
                busy: mount.busy,
                inspector_mounted: mount.inspector_mounted,
                fields: mount.fields,
                pending_fields: mount.pending_fields,
                feedback: mount.feedback,
                on_edit: mount.on_edit,
                on_apply_preset: mount.on_apply_preset,
                on_delete: mount.on_delete,
                on_unlink: mount.on_unlink,
                on_duplicate: mount.on_duplicate,
            }
        }
    }

    async fn mount_matrix_inspector(
        runtime: Rc<Runtime>,
    ) -> (
        MountedMatrixInspectorProbe,
        web_sys::Element,
        MountedFixture,
    ) {
        let scope = runtime.scope().expect("matrix fixture has active scope");
        let probe = MountedMatrixInspectorProbe {
            runtime,
            scope,
            selected_context: Rc::default(),
            version: Rc::default(),
            request_sequence: Rc::default(),
            visible: Rc::default(),
        };
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("matrix-action-mounted-test");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(mounted_matrix_inspector_host);
        dom.provide_root_context(probe.clone());
        let mounted = MountedFixture::new(root.clone(), dom);
        gloo_timers::future::TimeoutFuture::new(5).await;
        (probe, root, mounted)
    }

    fn button_named(root: &web_sys::Element, name: &str) -> web_sys::HtmlElement {
        boardstudio_web_ui_shared::test_role::by_role_name(root, "button", name)
            .unwrap_or_else(|| panic!("mounted Matrix Inspector has Button {name:?}"))
    }

    fn matrix_input(root: &web_sys::Element, name: &str) -> web_sys::HtmlInputElement {
        let role = if matches!(name, "Matrix name" | "Layout name") {
            "textbox"
        } else {
            "spinbutton"
        };
        boardstudio_web_ui_shared::test_role::by_role_name(root, role, name)
            .unwrap_or_else(|| panic!("mounted Matrix Inspector has {role} {name:?}"))
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap()
    }

    fn type_matrix_input(input: &web_sys::HtmlInputElement, value: &str) {
        use web_sys::{Event as WebEvent, EventInit};

        input.set_value(value);
        let init = EventInit::new();
        init.set_bubbles(true);
        input
            .dispatch_event(
                &WebEvent::new_with_event_init_dict("input", &init)
                    .expect("input event should construct"),
            )
            .expect("input event should dispatch");
    }

    /// Bump the mounted version signal once so the inspector re-projects.
    /// Every mounted scenario waits on this instead of repeating the
    /// borrow/increment/sleep sequence.
    async fn bump_version(probe: &MountedMatrixInspectorProbe) {
        bump_version_for(probe, 5).await;
    }

    async fn bump_version_for(probe: &MountedMatrixInspectorProbe, millis: u32) {
        let mut version = probe
            .version
            .borrow()
            .as_ref()
            .copied()
            .expect("mounted version signal");
        version.set(version() + 1);
        gloo_timers::future::TimeoutFuture::new(millis).await;
    }

    async fn run_and_bump(runtime: &Rc<Runtime>, probe: &MountedMatrixInspectorProbe) {
        run_and_bump_for(runtime, probe, 5).await;
    }

    async fn run_and_bump_for(
        runtime: &Rc<Runtime>,
        probe: &MountedMatrixInspectorProbe,
        millis: u32,
    ) {
        support::run_pending(runtime).await;
        bump_version_for(probe, millis).await;
    }

    /// Type a value into an already-bound inspector input and commit it
    /// with blur.
    fn commit_typed_input(input: &web_sys::HtmlInputElement, value: &str) {
        type_matrix_input(input, value);
        input.focus().expect("field input should focus");
        input.blur().expect("field input should blur");
    }

    /// Click a named action button with the next Core reply held, and drive
    /// the ticket until it reaches Core. Callers await `entered` and fire
    /// `release` on their own schedule.
    fn hold_button_click(
        runtime: &Rc<Runtime>,
        root: &web_sys::Element,
        button: &str,
    ) -> (
        futures_channel::oneshot::Receiver<()>,
        futures_channel::oneshot::Sender<()>,
    ) {
        let (entered, release) = support::gate_next_core_reply(runtime);
        button_named(root, button).click();
        support::drive_pending(runtime);
        (entered, release)
    }

    /// Accepted part poses in document order for stay/undo comparisons.
    fn part_poses(runtime: &Rc<Runtime>) -> Vec<(String, boardstudio_model::model::Pose2)> {
        runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .parts
            .iter()
            .map(|part| (part.id.clone(), part.pose))
            .collect()
    }

    fn field_ticket(
        runtime: &Rc<Runtime>,
        field: MatrixEditField,
        value: MatrixEditValue,
    ) -> EditTicket {
        EditTicket::begin(
            runtime,
            "layout-matrix-inspector",
            Some("matrix".into()),
            matrix_field_resolver(
                fixture::MATRIX_ID.into(),
                MatrixNameTarget::Matrix,
                field,
                value,
                None,
            ),
        )
    }

    #[wasm_bindgen_test]
    async fn variant_snapshot_scope_departure_is_retired_before_recovery() {
        let runtime = support::new_runtime();
        let mut document = fixture::matrix_document();
        document.boards.push(boardstudio_model::model::Board {
            id: "board-other".into(),
            name: "Other".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        support::open_document(&runtime, document).await;
        let accepted = runtime.model().accepted.expect("accepted project");
        let scope = runtime.scope().expect("active board scope");
        let variant_id = accepted.document.id.clone();

        support::navigate(&runtime, "board-other").await;

        assert!(matches!(
            current_variant_snapshot(&runtime, &accepted, &variant_id, &scope),
            Ok(None)
        ));
        // The production preparation path maps this missing snapshot to `Retired`, which
        // bypasses the generic error branch that invokes source-project recovery.
    }

    #[wasm_bindgen_test]
    async fn rapid_rows_then_pitch_both_survive_and_undo_removes_pitch_then_rows() {
        let runtime = fixture::open_matrix_runtime().await;
        let original = fixture::accepted_matrix(&runtime);
        // Hold the first Core reply so the pitch edit queues behind the rows edit.
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let rows = field_ticket(&runtime, MatrixEditField::Rows, MatrixEditValue::Rows(2));
        support::drive_pending(&runtime);
        entered.await.expect("the rows edit reached Core");
        let pitch = field_ticket(
            &runtime,
            MatrixEditField::PitchX,
            MatrixEditValue::PitchX(21.0),
        );
        support::drive_pending(&runtime);
        release.send(()).expect("release the held rows reply");
        fixture::settle_ticket(&runtime, &pitch).await;

        assert!(matches!(rows.settlement(true), Settlement::Landed { .. }));
        assert!(matches!(pitch.settlement(true), Settlement::Landed { .. }));
        let accepted = fixture::accepted_matrix(&runtime);
        assert_eq!(accepted.rows, 2, "the rows edit survived");
        assert_eq!(
            accepted.pitch.x, 21.0,
            "the pitch edit applied on top of it"
        );

        fixture::undo(&runtime).await;
        let after_one = fixture::accepted_matrix(&runtime);
        assert_eq!(
            after_one.pitch.x, original.pitch.x,
            "one Undo removes the pitch"
        );
        assert_eq!(after_one.rows, 2, "the rows edit is still applied");
        fixture::undo(&runtime).await;
        assert_eq!(fixture::accepted_matrix(&runtime).rows, original.rows);
    }

    #[wasm_bindgen_test]
    async fn editing_a_matrix_deleted_before_the_edit_runs_retires_with_a_reason() {
        let runtime = fixture::open_matrix_runtime().await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let delete = EditTicket::begin(
            &runtime,
            "layout-matrix-delete",
            Some("matrix delete".into()),
            matrix_delete_resolver(fixture::MATRIX_ID.into()),
        );
        support::drive_pending(&runtime);
        entered.await.expect("the delete reached Core");
        let rename = field_ticket(
            &runtime,
            MatrixEditField::Name,
            MatrixEditValue::Name(Some("Renamed".into())),
        );
        support::drive_pending(&runtime);
        release.send(()).expect("release the held delete");
        fixture::settle_ticket(&runtime, &rename).await;

        assert!(matches!(delete.settlement(true), Settlement::Landed { .. }));
        match rename.settlement(true) {
            Settlement::Failed { message } => assert!(
                message.contains("no longer exists"),
                "the reason is explained: {message}"
            ),
            other => panic!("expected a retirement with a reason, got {other:?}"),
        }
    }

    #[wasm_bindgen_test]
    async fn mounted_delete_stays_disabled_until_saved_then_clears_its_exact_selection() {
        let runtime = fixture::open_matrix_runtime().await;
        let (probe, root, _mounted) = mount_matrix_inspector(runtime.clone()).await;
        let (entered, release) = hold_button_click(&runtime, &root, "Delete matrix");
        entered.await.expect("the mounted delete reached Core");
        gloo_timers::future::TimeoutFuture::new(5).await;
        assert!(
            button_named(&root, "Delete matrix").has_attribute("disabled"),
            "the production Matrix action stays disabled while its Core request is held"
        );

        release.send(()).expect("release the held delete");
        run_and_bump(&runtime, &probe).await;
        run_and_bump(&runtime, &probe).await;

        assert!(
            probe
                .selected_context
                .borrow()
                .as_ref()
                .copied()
                .expect("mounted selection signal")
                .read()
                .is_none(),
            "the landed delete clears only its captured selected matrix context"
        );
        assert!(
            runtime
                .model()
                .accepted
                .is_some_and(|snapshot| snapshot.document.matrices.is_empty()),
            "the matrix was removed by the real Core edit"
        );
    }

    #[wasm_bindgen_test]
    async fn mounted_delete_does_not_clear_a_newer_outline_selection() {
        let runtime = fixture::open_matrix_runtime().await;
        let (probe, root, _mounted) = mount_matrix_inspector(runtime.clone()).await;
        let (entered, release) = hold_button_click(&runtime, &root, "Delete matrix");
        entered.await.expect("the mounted delete reached Core");
        let newer_selection = ScopedTreeContext {
            scope: probe.scope.clone(),
            context: TreeContext::Outline {
                board_id: probe.scope.board_id.clone(),
            },
        };
        let mut selected = probe
            .selected_context
            .borrow()
            .as_ref()
            .copied()
            .expect("mounted selection signal");
        selected.set(Some(newer_selection.clone()));

        release.send(()).expect("release the held delete");
        run_and_bump(&runtime, &probe).await;
        support::run_pending(&runtime).await;

        assert_eq!(
            selected.read().as_ref(),
            Some(&newer_selection),
            "the old delete cannot clear a newer outline selection"
        );
    }

    #[wasm_bindgen_test]
    async fn current_matrix_owner_still_reports_a_held_delete_save_failure() {
        let runtime = fixture::open_matrix_runtime().await;
        let (probe, root, _mounted) = mount_matrix_inspector(runtime.clone()).await;
        let (core_entered, release_core) = hold_button_click(&runtime, &root, "Delete matrix");
        core_entered
            .await
            .expect("the mounted delete reached the held Core reply");
        support::fail_next_persist(&runtime, "injected matrix delete save failure");
        release_core.send(()).expect("release the held Core reply");
        run_and_bump(&runtime, &probe).await;
        support::run_pending(&runtime).await;

        assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
        assert!(
            runtime.status().starts_with("Could not delete matrix:"),
            "a failure from the current Matrix owner remains visible: {}",
            runtime.status()
        );
    }

    #[wasm_bindgen_test]
    async fn mounted_field_failure_stays_inline_beside_a_newer_draft() {
        let runtime = fixture::open_matrix_runtime().await;
        let (probe, root, _mounted) = mount_matrix_inspector(runtime.clone()).await;
        let rows = matrix_input(&root, "Rows");
        let (entered, release) = support::gate_next_core_reply(&runtime);

        commit_typed_input(&rows, "2");
        assert_eq!(
            probe
                .request_sequence
                .borrow()
                .as_ref()
                .copied()
                .expect("mounted request sequence")(),
            1,
            "the actual Rows input submits through the production callback"
        );
        support::drive_pending(&runtime);
        entered.await.expect("the mounted Rows edit reached Core");
        type_matrix_input(&rows, "3");
        assert_eq!(rows.value(), "3", "the newer draft remains visible");

        support::fail_next_persist(&runtime, "injected matrix field save failure");
        release.send(()).expect("release the held field edit");
        run_and_bump(&runtime, &probe).await;
        run_and_bump(&runtime, &probe).await;

        assert_eq!(rows.value(), "3", "failure does not replace a newer draft");
        let rows_label = rows
            .closest("[data-slot='field']")
            .unwrap()
            .and_then(|field| field.parent_element())
            .and_then(|input_group| input_group.parent_element())
            .expect("the actual Rows field owner exists");
        assert!(
            rows_label
                .query_selector("[role='alert']")
                .unwrap()
                .is_some(),
            "the failed Rows edit explains its failure beside newer draft 3"
        );
        assert!(
            !rows_label
                .query_selector_all("[role='status']")
                .unwrap()
                .item(0)
                .and_then(|status| status.text_content())
                .is_some_and(|status| status.contains("Saving…")),
            "settlement clears Rows saving status even while draft 3 stays visible"
        );
    }

    #[wasm_bindgen_test]
    async fn changing_name_target_resets_the_actual_mounted_name_draft() {
        let runtime = fixture::open_matrix_runtime().await;
        let (probe, root, _mounted) = mount_matrix_inspector(runtime.clone()).await;
        let matrix_name = matrix_input(&root, "Matrix name");
        type_matrix_input(&matrix_name, "Old matrix draft");
        let mut replacement = (*runtime
            .model()
            .accepted
            .expect("accepted fixture document")
            .document)
            .clone();
        replacement.layouts.push(boardstudio_model::model::Layout {
            id: "layout-main".into(),
            name: "Canonical layout".into(),
            board_id: fixture::BOARD_ID.into(),
            matrix_ids: vec![fixture::MATRIX_ID.into()],
            part_ids: vec![],
            mirror_link: None,
        });
        let resolver_document = replacement.clone();
        let ticket = EditTicket::begin(
            &runtime,
            "test-add-layout-target",
            Some("document".into()),
            EditResolver::new("test-add-layout-target", move |_| {
                Resolution::submit(
                    vec![fixture::MATRIX_ID.into()],
                    EditOperation::ReplaceDocument {
                        document: Box::new(resolver_document.clone()),
                    },
                )
            }),
        );
        fixture::settle_ticket(&runtime, &ticket).await;
        assert!(
            runtime.model().accepted.is_some_and(|snapshot| {
                snapshot.document.layouts.iter().any(|layout| {
                    layout.id == "layout-main"
                        && layout.matrix_ids.iter().any(|id| id == fixture::MATRIX_ID)
                })
            }),
            "the real Runtime accepts the replacement Layout"
        );
        bump_version(&probe).await;

        let layout_name = matrix_input(&root, "Layout name");
        assert_eq!(layout_name.value(), "Canonical layout");
        layout_name.focus().expect("Layout name input should focus");
        layout_name.blur().expect("Layout name input should blur");
        assert_eq!(
            probe
                .request_sequence
                .borrow()
                .as_ref()
                .copied()
                .expect("mounted request sequence")(),
            0,
            "the stale matrix draft does not submit against the new layout target"
        );
    }

    #[wasm_bindgen_test]
    async fn equal_baseline_fields_reset_when_the_mounted_matrix_changes() {
        let runtime = support::new_runtime();
        let mut document = fixture::matrix_document();
        document.matrices[0].name = Some("Same".into());
        let mut other_matrix = document.matrices[0].clone();
        other_matrix.id = "matrix-other".into();
        other_matrix.part_ids.clear();
        other_matrix.cells.clear();
        document.matrices.push(other_matrix);
        support::open_document(&runtime, document).await;
        let (probe, root, _mounted) = mount_matrix_inspector(runtime.clone()).await;
        let cases = [
            ("Matrix name", "Draft A", "Same"),
            ("Rows", "8", "1"),
            ("Columns", "9", "3"),
            ("Pitch X", "21.5", "19.05"),
            ("Pitch Y", "22.5", "19.05"),
        ];
        let mut selected = probe
            .selected_context
            .borrow()
            .as_ref()
            .copied()
            .expect("mounted selection signal");
        let request_sequence = probe
            .request_sequence
            .borrow()
            .as_ref()
            .copied()
            .expect("mounted request sequence");

        for (index, (label, draft, accepted)) in cases.into_iter().enumerate() {
            let input = matrix_input(&root, label);
            type_matrix_input(&input, draft);
            if index > 0 {
                selected.set(Some(ScopedTreeContext {
                    scope: probe.scope.clone(),
                    context: TreeContext::Matrix {
                        matrix_id: fixture::MATRIX_ID.into(),
                    },
                }));
                bump_version(&probe).await;
            }
            selected.set(Some(ScopedTreeContext {
                scope: probe.scope.clone(),
                context: TreeContext::Matrix {
                    matrix_id: "matrix-other".into(),
                },
            }));
            bump_version(&probe).await;
            let current = matrix_input(&root, label);
            assert_eq!(current.value(), accepted, "{label} resets on owner change");
            current.focus().expect("field input should focus");
            current.blur().expect("field input should blur");
            assert_eq!(
                request_sequence(),
                0,
                "dirty {label} is not sent to the new owner"
            );
        }
    }

    #[wasm_bindgen_test]
    async fn same_owner_revision_keeps_the_mounted_uncommitted_name_draft() {
        let runtime = fixture::open_matrix_runtime().await;
        let (probe, root, _mounted) = mount_matrix_inspector(runtime.clone()).await;
        let name = matrix_input(&root, "Matrix name");
        type_matrix_input(&name, "Uncommitted matrix name");

        let mut changed_matrix = fixture::accepted_matrix(&runtime);
        changed_matrix.rows = 2;
        let resolver_matrix = changed_matrix.clone();
        let ticket = EditTicket::begin(
            &runtime,
            "test-unrelated-rows-change",
            Some("matrix".into()),
            EditResolver::new("test-unrelated-rows-change", move |_| {
                Resolution::submit(
                    vec![resolver_matrix.id.clone()],
                    EditOperation::SetMatrix {
                        matrix: resolver_matrix.clone(),
                        definitions: None,
                    },
                )
            }),
        );
        fixture::settle_ticket(&runtime, &ticket).await;
        assert_eq!(fixture::accepted_matrix(&runtime).rows, 2);

        bump_version(&probe).await;

        let current_name = matrix_input(&root, "Matrix name");
        assert_eq!(current_name.value(), "Uncommitted matrix name");
        current_name
            .focus()
            .expect("Matrix name input should focus");
        current_name.blur().expect("Matrix name input should blur");
        assert_eq!(
            probe
                .request_sequence
                .borrow()
                .as_ref()
                .copied()
                .expect("mounted request sequence")(),
            1,
            "the retained draft is admitted after the unrelated accepted revision"
        );
    }

    #[wasm_bindgen_test]
    async fn hidden_retired_field_does_not_restore_old_draft_after_real_landing() {
        let runtime = fixture::open_matrix_runtime().await;
        let (probe, root, _mounted) = mount_matrix_inspector(runtime.clone()).await;
        let rows = matrix_input(&root, "Rows");
        let (entered, release) = support::gate_next_core_reply(&runtime);

        commit_typed_input(&rows, "2");
        support::drive_pending(&runtime);
        entered.await.expect("the mounted Rows edit reached Core");

        let mut visible = probe
            .visible
            .borrow()
            .as_ref()
            .copied()
            .expect("mounted visibility signal");
        visible.set(false);
        bump_version(&probe).await;
        assert!(
            boardstudio_web_ui_shared::test_role::by_role_name(&root, "spinbutton", "Rows")
                .is_none()
        );

        release.send(()).expect("release the hidden field edit");
        support::run_pending(&runtime).await;
        for _ in 0..10 {
            if runtime.model().accepted.is_some_and(|snapshot| {
                snapshot
                    .document
                    .matrices
                    .iter()
                    .any(|matrix| matrix.id == fixture::MATRIX_ID && matrix.rows == 2)
            }) {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(20).await;
        }
        assert!(runtime.model().accepted.is_some_and(|snapshot| {
            snapshot
                .document
                .matrices
                .iter()
                .any(|matrix| matrix.id == fixture::MATRIX_ID && matrix.rows == 2)
        }));

        visible.set(true);
        bump_version(&probe).await;
        let rows = matrix_input(&root, "Rows");
        assert_eq!(
            rows.value(),
            "2",
            "remount projects the latest accepted Rows"
        );
        assert!(root.query_selector("[role='alert']").unwrap().is_none());
    }

    fn linked_unlink_document() -> boardstudio_model::model::ProjectDoc {
        use boardstudio_model::model::{
            Layout, LayoutMirrorLink, MirroredCorrespondence, MirroredItemKind,
        };
        let mut document = fixture::matrix_document();
        let mirror_id = "matrix-mirror";
        let mirror_keys = [
            "matrix-mirror/r0c0",
            "matrix-mirror/r0c1",
            "matrix-mirror/r0c2",
        ];
        for (column, id) in mirror_keys.iter().enumerate() {
            document.boards[0].part_ids.push((*id).into());
            document.parts.push(boardstudio_model::model::Part {
                keycap: None,
                outline: None,
                anchor: None,
                id: (*id).into(),
                definition_id: "switch:base".into(),
                reference: format!("SW-M{}", column + 1),
                pose: boardstudio_model::model::Pose2 {
                    at: boardstudio_model::model::Vec2 {
                        x: 100.0 + 19.05 * column as f64,
                        y: 0.0,
                    },
                    rotation: 0.0,
                },
                side: boardstudio_model::model::Side::Front,
                locked: None,
                properties: None,
                generator_parameters: None,
            });
        }
        let source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == fixture::MATRIX_ID)
            .expect("the source matrix exists")
            .clone();
        let mut mirror = source.clone();
        mirror.id = mirror_id.into();
        mirror.part_ids = mirror_keys.iter().map(|id| (*id).into()).collect();
        document.matrices.push(mirror);
        document.layouts.push(Layout {
            id: "layout-left".into(),
            name: "Left".into(),
            board_id: fixture::BOARD_ID.into(),
            matrix_ids: vec![fixture::MATRIX_ID.into()],
            part_ids: vec![],
            mirror_link: None,
        });
        document.layouts.push(Layout {
            id: "layout-right".into(),
            name: "Right".into(),
            board_id: fixture::BOARD_ID.into(),
            matrix_ids: vec![mirror_id.into()],
            part_ids: vec![],
            mirror_link: Some(LayoutMirrorLink {
                source_id: "layout-left".into(),
                axis_x: 50.0,
                split_gap: None,
                spacing: None,
                correspondences: vec![MirroredCorrespondence {
                    companion_mode: None,
                    companion_tweak: None,
                    kind: MirroredItemKind::Matrix,
                    source_id: fixture::MATRIX_ID.into(),
                    mirror_id: mirror_id.into(),
                }],
                exceptions: vec![],
            }),
        });
        document
    }

    /// Ticket 35, RED: unlinking a linked pair through the confirmed UI action
    /// leaves two valid independent halves with no live correspondence, and a
    /// single Undo restores the linked pair with poses unchanged.
    #[wasm_bindgen_test]
    async fn mounted_unlink_halves_confirms_then_clears_link_with_one_undo() {
        let runtime = support::new_runtime();
        support::open_document(&runtime, linked_unlink_document()).await;
        let (probe, root, _mounted) = mount_matrix_inspector(runtime.clone()).await;
        gloo_timers::future::TimeoutFuture::new(10).await;
        let revision_before = runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .revision;
        let poses_before = part_poses(&runtime);
        button_named(&root, "Unlink halves").click();
        gloo_timers::future::TimeoutFuture::new(10).await;
        assert!(
            boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Confirm unlink")
                .is_some(),
            "the first click arms an explicit confirmation"
        );
        assert_eq!(
            runtime
                .model()
                .accepted
                .expect("an accepted document")
                .document
                .revision,
            revision_before,
            "arming the confirmation writes nothing"
        );
        button_named(&root, "Cancel").click();
        gloo_timers::future::TimeoutFuture::new(10).await;
        assert!(
            boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Confirm unlink")
                .is_none(),
            "Cancel drops the confirmation without writing"
        );
        assert_eq!(
            runtime
                .model()
                .accepted
                .expect("an accepted document")
                .document
                .revision,
            revision_before,
            "cancelling writes nothing"
        );
        button_named(&root, "Unlink halves").click();
        gloo_timers::future::TimeoutFuture::new(10).await;
        button_named(&root, "Confirm unlink").click();
        support::run_pending(&runtime).await;
        gloo_timers::future::TimeoutFuture::new(10).await;
        bump_version_for(&probe, 10).await;
        support::run_pending(&runtime).await;
        let after = runtime.model().accepted.expect("an accepted document");
        assert_eq!(
            after.document.revision,
            revision_before + 1,
            "Confirm writes one atomic Core intent"
        );
        assert!(
            after
                .document
                .layouts
                .iter()
                .all(|layout| layout.mirror_link.is_none()),
            "no orphaned correspondences remain: neither half references the former pair"
        );
        assert_eq!(
            after.document.layouts.len(),
            2,
            "both halves stay valid layouts"
        );
        assert!(
            after
                .document
                .layouts
                .iter()
                .any(|layout| layout.matrix_ids == vec![fixture::MATRIX_ID.to_owned()]),
            "the source half keeps its layout membership"
        );
        assert!(
            after
                .document
                .layouts
                .iter()
                .any(|layout| layout.matrix_ids == vec!["matrix-mirror".to_owned()]),
            "the mirror half keeps its layout membership"
        );
        let poses_after: Vec<_> = after
            .document
            .parts
            .iter()
            .map(|part| (part.id.clone(), part.pose))
            .collect();
        assert_eq!(
            poses_after, poses_before,
            "unlink keeps every pose: no silent geometry loss"
        );
        assert!(
            boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Confirm unlink")
                .is_none(),
            "the pair relationship stops showing once unlinked"
        );
        fixture::undo(&runtime).await;
        gloo_timers::future::TimeoutFuture::new(10).await;
        bump_version_for(&probe, 10).await;
        support::run_pending(&runtime).await;
        let undone = runtime.model().accepted.expect("an accepted document");
        assert!(
            undone
                .document
                .layouts
                .iter()
                .any(|layout| layout.id == "layout-right"
                    && layout
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == "layout-left")),
            "one Undo restores the linked pair"
        );
        let poses_undone: Vec<_> = undone
            .document
            .parts
            .iter()
            .map(|part| (part.id.clone(), part.pose))
            .collect();
        assert_eq!(
            poses_undone, poses_before,
            "one Undo restores every pose exactly"
        );
        root.remove();
    }
}
