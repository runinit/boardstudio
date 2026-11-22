use super::*;
use boardstudio_application::{AcceptedSnapshot, Event, Scope};
use boardstudio_model::model::{Asset, Board, BoardReference, Pose2, Vec2};
use boardstudio_web_test_runtime::in_process_support;
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_role::by_role_name;
use js_sys::{Array, Uint8Array};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::File;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct Fixture {
    runtime: Rc<Runtime>,
    accepted: AcceptedSnapshot,
    scope: Scope,
    reference: BoardReference,
    asset: Asset,
}

impl PartialEq for Fixture {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.runtime, &other.runtime)
            && self.accepted.token == other.accepted.token
            && self.scope == other.scope
            && self.reference == other.reference
            && self.asset == other.asset
    }
}

async fn fixture() -> Fixture {
    let runtime = support::new_runtime();
    let missing_sha = sha256_hex(b"absent routed board browser-store fixture");
    let asset = Asset {
        id: "routed-board-asset".into(),
        name: "Left_PCB.kicad_pcb".into(),
        media_type: "application/x-kicad_pcb".into(),
        sha256: missing_sha,
        license: None,
        source: None,
    };
    let mut document = in_process_support::board_document();
    // A second board for the owner-change test to navigate to.
    document.boards.push(Board {
        id: "changed-routed-board-target".into(),
        name: "Changed target board".into(),
        outline_ids: Vec::new(),
        part_ids: Vec::new(),
        net_ids: Vec::new(),
        thickness: 1.6,
        traces: Vec::new(),
        vias: Vec::new(),
    });
    let reference = BoardReference {
        id: "routed-board-reference".into(),
        board_id: document.boards[0].id.clone(),
        asset_id: asset.id.clone(),
        enabled: true,
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        elevation: 0.0,
        model_assets: Default::default(),
    };
    document.assets.push(asset.clone());
    document.board_references.push(reference.clone());
    support::open_document(&runtime, document).await;
    let accepted = runtime
        .model()
        .accepted
        .expect("the routed-board project is accepted");
    let scope = runtime
        .scope()
        .expect("the accepted routed-board project has a scope");
    Fixture {
        runtime,
        accepted,
        scope,
        reference,
        asset,
    }
}

#[component]
fn missing_asset_host(fixture: Fixture) -> Element {
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    let workspace = use_signal(|| "Layout");
    let selected_context = use_signal(|| None);
    let anchor_scope = use_signal(|| None);
    let generation = use_signal(|| 1u64);
    let adapter = use_hook(|| {
        super::super::SelectionAdapter::new(selected_context, anchor_scope, generation)
    });
    let owner = super::super::current_layout_owner(&fixture.runtime, workspace, &adapter);
    rsx! {
        Editor {
            reference: Some(fixture.reference.clone()),
            assets: vec![fixture.asset.clone()],
            disabled: false,
            runtime: BoardReferenceRuntimeHandle::new(fixture.runtime.clone()),
            workspace,
            adapter: BoardReferenceAdapterHandle::new(adapter),
            owner,
        }
    }
}

#[component]
fn field_edit_host(fixture: Fixture) -> Element {
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    use_hook(|| {
        fixture.runtime.subscribe(Rc::new(move || {
            let mut version = version;
            version.set(version() + 1);
        }));
    });
    let _ = version();
    let workspace = use_signal(|| "Layout");
    let selected_context = use_signal(|| None);
    let anchor_scope = use_signal(|| None);
    let generation = use_signal(|| 1u64);
    let adapter = use_hook(|| {
        super::super::SelectionAdapter::new(selected_context, anchor_scope, generation)
    });
    let owner = super::super::current_layout_owner(&fixture.runtime, workspace, &adapter);
    let disabled = !super::super::board_reference_owner_is_current(
        &fixture.runtime,
        workspace,
        &adapter,
        &owner,
    );
    let snapshot = fixture.runtime.model().accepted.unwrap();
    rsx! {
        Editor {
            reference: snapshot.document.board_references.first().cloned(),
            assets: snapshot.document.assets.clone(),
            disabled,
            runtime: BoardReferenceRuntimeHandle::new(fixture.runtime.clone()),
            workspace,
            adapter: BoardReferenceAdapterHandle::new(adapter),
            owner,
        }
    }
}

#[wasm_bindgen_test]
async fn routed_reference_has_named_file_action_and_editable_fields() {
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    let fixture = fixture().await;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new_with_props(
        field_edit_host,
        field_edit_hostProps {
            fixture: fixture.clone(),
        },
    );
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("routed reference disclosure mounts", || {
        by_role_name(&root, "button", "Routed PCB reference").is_some()
    })
    .await;
    let disclosure = by_role_name(&root, "button", "Routed PCB reference").unwrap();
    disclosure.click();
    wait_until("routed reference opens", || {
        disclosure.get_attribute("aria-expanded").as_deref() == Some("true")
    })
    .await;
    wait_until("routed reference controls mount", || {
        by_role_name(&root, "button", "Replace KiCad board").is_some()
            && by_role_name(&root, "spinbutton", "X (mm)").is_some()
            && by_role_name(&root, "checkbox", "Use routed PCB in assembly").is_some()
    })
    .await;
    assert_eq!(
        disclosure.get_attribute("aria-expanded").as_deref(),
        Some("true")
    );
    let picker_clicked = Rc::new(std::cell::Cell::new(false));
    let observed = picker_clicked.clone();
    let listener = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::new(
        move |event: web_sys::Event| {
            if let Some(input) = event
                .target()
                .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                && input.type_() == "file"
                && input.accept() == ".kicad_pcb"
            {
                observed.set(true);
            }
        },
    );
    root.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
        .unwrap();
    let file_button = by_role_name(&root, "button", "Replace KiCad board").unwrap();
    wait_until("reference discovery settles before replacing", || {
        !file_button.has_attribute("disabled")
    })
    .await;
    file_button.click();
    assert!(
        picker_clicked.get(),
        "the named action opens its KiCad file input"
    );
    root.remove_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
        .unwrap();
    fixture.runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_missing_stored_board_blob_reports_retry_and_keeps_reference() {
    let fixture = fixture().await;
    assert!(
        fixture
            .runtime
            .store
            .load_asset(fixture.asset.sha256.clone())
            .await
            .expect("IndexedDB asset lookup succeeds")
            .is_none(),
        "fixture SHA must be absent from browser storage"
    );
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("pcb-reference-missing-asset-test-root");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new_with_props(
        missing_asset_host,
        missing_asset_hostProps {
            fixture: fixture.clone(),
        },
    );
    let _mounted = MountedFixture::new(root.clone(), dom);

    for _ in 0..100 {
        if by_role_name(
            &root,
            "alert",
            "Saved routed-board file is missing from browser storage.",
        )
        .is_some()
        {
            break;
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
    let alert = by_role_name(
        &root,
        "alert",
        "Saved routed-board file is missing from browser storage.",
    )
    .expect("missing stored blob produces a mounted alert");
    assert!(
        alert
            .text_content()
            .unwrap()
            .contains("Saved routed-board file is missing from browser storage")
    );
    let retry = by_role_name(&root, "button", "Retry model-path discovery")
        .expect("failed discovery offers retry");
    assert!(retry.get_attribute("disabled").is_none());
    retry.click();
    for _ in 0..100 {
        if by_role_name(
            &root,
            "alert",
            "Saved routed-board file is missing from browser storage.",
        )
        .is_none()
        {
            break;
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
    for _ in 0..100 {
        if by_role_name(
            &root,
            "alert",
            "Saved routed-board file is missing from browser storage.",
        )
        .is_some()
        {
            break;
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
    assert!(
        by_role_name(
            &root,
            "alert",
            "Saved routed-board file is missing from browser storage."
        )
        .expect("retry runs the real missing-asset lookup again")
        .text_content()
        .unwrap()
        .contains("Saved routed-board file is missing from browser storage")
    );
    assert!(support::take_held_effects(&fixture.runtime).is_empty());
    assert!(
        fixture
            .runtime
            .model()
            .accepted
            .unwrap()
            .document
            .board_references
            .iter()
            .any(|reference| reference == &fixture.reference)
    );
    root.remove();
}

#[derive(Clone)]
struct ReplaceHostProbe {
    fixture: Fixture,
    file: File,
    file_sha: String,
    busy: Rc<RefCell<Option<Signal<bool>>>>,
    error: Rc<RefCell<Option<Signal<Option<String>>>>>,
}

impl PartialEq for ReplaceHostProbe {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.busy, &other.busy)
    }
}

#[component]
fn replace_host(probe: ReplaceHostProbe) -> Element {
    let workspace = use_signal(|| "Layout");
    let selected_context = use_signal(|| None);
    let anchor_scope = use_signal(|| None);
    let generation = use_signal(|| 1u64);
    let adapter = use_hook(|| {
        super::super::SelectionAdapter::new(selected_context, anchor_scope, generation)
    });
    let owner = super::super::current_layout_owner(&probe.fixture.runtime, workspace, &adapter);
    let request_generation = use_signal(|| 0u64);
    let busy = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let paths = use_signal(Vec::<String>::new);
    let paths_asset_id = use_signal(|| None::<String>);
    *probe.busy.borrow_mut() = Some(busy);
    *probe.error.borrow_mut() = Some(error);
    let runtime = probe.fixture.runtime.clone();
    let adapter_for_import = adapter.clone();
    let reference = probe.fixture.reference.clone();
    let file = probe.file.clone();
    rsx! {
        button {
            id: "pcb-reference-test-begin-replace",
            disabled: busy(),
            onclick: move |_| begin_board_import(
                BoardReferenceContext {
                    runtime: runtime.clone(),
                    workspace,
                    adapter: adapter_for_import.clone(),
                    owner: owner.clone(),
                    generation: request_generation,
                    request_generation: 0,
                },
                Some(reference.clone()),
                file.clone(),
                busy,
                error,
                paths,
                paths_asset_id,
            ),
            "Begin routed-board replacement"
        }
        if busy() { p { role: "status", "Reading routed board" } }
        if let Some(message) = error() { p { role: "alert", "{message}" } }
    }
}

#[derive(Clone, Copy)]
enum OwnerChange {
    Project,
    Board,
    Revision,
}

/// Change the accepted owner through the real Session: open another project, navigate to
/// another board, or land another revision of the same board.
async fn changed_owner(fixture: &Fixture, change: OwnerChange) {
    let runtime = &fixture.runtime;
    match change {
        OwnerChange::Project => {
            let mut replacement = (*fixture.accepted.document).clone();
            replacement.id = "changed-routed-board-project".into();
            runtime.submit(Event::Open {
                operation_id: runtime.operation(),
                document: replacement,
            });
            support::run_pending(runtime).await;
        }
        OwnerChange::Board => support::navigate(runtime, "changed-routed-board-target").await,
        OwnerChange::Revision => {
            let mut document = (*fixture.accepted.document).clone();
            document.name.push_str(" revised");
            support::replace_document(
                runtime,
                "routed-board-revision-change",
                &fixture.scope.board_id,
                document,
            )
            .await;
        }
    }
}

fn replacement_file(salt: usize) -> (File, String) {
    let mut bytes = include_bytes!("../../../../tests/fixtures/replacement.kicad_pcb").to_vec();
    // Trailing whitespace preserves the fixture as KiCad text while giving
    // each stale-owner case a unique BrowserStore identity.
    bytes.extend(std::iter::repeat_n(b'\n', salt));
    let sha256 = sha256_hex(&bytes);
    let contents = Uint8Array::from(bytes.as_slice());
    let file = File::new_with_u8_array_sequence(
        &Array::of1(&contents.into()),
        "Replacement_PCB.kicad_pcb",
    )
    .expect("supported routed-board File fixture constructs");
    (file, sha256)
}

#[wasm_bindgen_test]
async fn replace_discards_async_file_read_after_project_board_or_revision_changes() {
    for (index, change) in [
        OwnerChange::Project,
        OwnerChange::Board,
        OwnerChange::Revision,
    ]
    .into_iter()
    .enumerate()
    {
        let fixture = fixture().await;
        let (file, file_sha) = replacement_file(index + 1);
        let probe = ReplaceHostProbe {
            fixture: fixture.clone(),
            file,
            file_sha,
            busy: Rc::new(RefCell::new(None)),
            error: Rc::new(RefCell::new(None)),
        };
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id(&format!("pcb-reference-replace-stale-test-{index}"));
        document.body().unwrap().append_child(&root).unwrap();
        let _mounted = MountedFixture::new(
            root.clone(),
            VirtualDom::new_with_props(
                replace_host,
                replace_hostProps {
                    probe: probe.clone(),
                },
            ),
        );
        let (entered, release) = test_support::install_import_gate();
        for _ in 0..100 {
            if by_role_name(&root, "button", "Begin routed-board replacement").is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        by_role_name(&root, "button", "Begin routed-board replacement")
            .unwrap()
            .click();
        entered
            .await
            .expect("real File.array_buffer read reaches gate");
        changed_owner(&fixture, change).await;
        let accepted_after_change = fixture
            .runtime
            .model()
            .accepted
            .expect("project stays accepted");
        release.send(()).expect("release file-read admission gate");

        let busy = probe.busy.borrow().expect("mounted busy signal");
        let error = probe.error.borrow().expect("mounted error signal");
        for _ in 0..100 {
            if !busy() && error().is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            !busy(),
            "stale operation settles and re-enables replacement"
        );
        let message = error().expect("changed owner reports a retryable failure");
        match change {
            OwnerChange::Project | OwnerChange::Board => {
                assert!(message.contains("result was discarded"));
            }
            OwnerChange::Revision => {
                assert!(message.contains("changed while the routed-board file was read"));
            }
        }
        assert!(
            by_role_name(&root, "button", "Begin routed-board replacement")
                .is_some_and(|button| button.get_attribute("disabled").is_none()),
            "replacement action remains usable for retry"
        );
        assert!(
            support::take_held_effects(&fixture.runtime).is_empty(),
            "no project edit may be submitted after the stale file read"
        );
        let accepted_after_read = fixture
            .runtime
            .model()
            .accepted
            .expect("project stays accepted");
        assert_eq!(accepted_after_read.token, accepted_after_change.token);
        assert_eq!(
            accepted_after_read.document.revision, accepted_after_change.document.revision,
            "the stale file read changed no accepted document"
        );
        let stored = fixture
            .runtime
            .store
            .load_asset(probe.file_sha.clone())
            .await
            .expect("IndexedDB asset lookup succeeds");
        assert!(
            stored.is_none(),
            "stale read must not store replacement bytes"
        );
        root.remove();
    }
}
