use super::{CadScene, current_cad_scene, physical_case_fingerprint};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Event, Scope};
use boardstudio_core::model::{Board, EditOperation, PreparedCaseAssemblyIR, ProjectDoc};
use boardstudio_web_host::cad_jobs::CadResult;
use std::rc::Rc;

fn runtime_with_scene(exact: bool) -> (Rc<Runtime>, Scope, Rc<CadScene>) {
    let runtime = Runtime::new();
    let mut document = ProjectDoc::empty("cad-rebind", "CAD rebind");
    document.boards.push(board("first-board"));
    document.boards.push(board("other-board"));
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document,
    });
    let accepted = runtime
        .model()
        .accepted
        .expect("the Case project is accepted");
    let scope = runtime
        .scope()
        .expect("the accepted Case project has a scope");
    let fingerprint = physical_case_fingerprint(&accepted, &scope)
        .expect("the accepted board fixture has physical inputs");
    let scene = Rc::new(CadScene {
        scope: scope.clone(),
        token: accepted.token,
        snapshot: accepted.clone(),
        result: CadResult {
            revision: accepted.document.revision,
            ..CadResult::default()
        },
        prepared: PreparedCaseAssemblyIR {
            revision: accepted.document.revision,
            bodies: Vec::new(),
        },
        physical_fingerprint: Some(fingerprint),
        mechanical: None,
        exact,
        contours: Vec::new(),
    });
    (runtime, scope, scene)
}

fn board(id: &str) -> Board {
    Board {
        id: id.into(),
        name: id.into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    }
}

/// Land an edit through the real Session: the project name changes and, when given, so
/// does the first board's thickness (a physical Case input).
fn edited(runtime: &Rc<Runtime>, board_thickness: Option<f64>) -> AcceptedSnapshot {
    let mut document = (*runtime
        .model()
        .accepted
        .expect("the Case project is accepted")
        .document)
        .clone();
    document.name.push_str(" updated");
    if let Some(thickness) = board_thickness {
        document.boards[0].thickness = thickness;
    }
    runtime
        .commit(
            "rebind edit",
            boardstudio_application::EditResolver::new("rebind edit", move |_| {
                boardstudio_application::Resolution::submit(
                    vec![],
                    EditOperation::ReplaceDocument {
                        document: Box::new(document.clone()),
                    },
                )
            }),
        )
        .unwrap()
}

#[test]
fn cad_scene_rebinds_only_exact_output_with_matching_physical_inputs() {
    for (exact, thickness, rebind) in [
        (true, None, true),
        (false, None, false),
        (true, Some(2.0), false),
    ] {
        let (runtime, scope, original) = runtime_with_scene(exact);
        let current = edited(&runtime, thickness);
        let shown = current_cad_scene(&mut Some(original.clone()), &scope, &current)
            .expect("same-owner output remains displayable");
        if rebind {
            assert_eq!(shown.token, current.token);
            let revision = current.document.revision;
            assert_eq!(shown.snapshot.document.revision, revision);
            assert_eq!(shown.result.revision, revision);
            assert_eq!(shown.prepared.revision, revision);
        } else {
            assert!(Rc::ptr_eq(&shown, &original));
            assert_eq!(shown.exact, exact);
            assert_eq!(shown.token, original.token);
            assert_eq!(
                shown.snapshot.document.revision,
                original.snapshot.document.revision
            );
        }
    }
}

#[test]
fn cad_scene_retires_old_scope_instead_of_rebinding_across_physical_owner() {
    let (runtime, scope, original) = runtime_with_scene(true);
    let current = edited(&runtime, None);
    let mut cached = Some(original);
    assert!(current_cad_scene(&mut cached, &scope, &current).is_some());
    let changed_scope = Scope {
        board_id: "other-board".into(),
        ..scope
    };
    assert!(current_cad_scene(&mut cached, &changed_scope, &current).is_none());
}
