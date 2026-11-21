use super::OwnedEdits;
use crate::draft_edits::FieldView;
use boardstudio_application::{EditResolver, Event, Resolution};
use boardstudio_model::model::{EditOperation, ProjectDoc};
use boardstudio_web_runtime::runtime::Runtime;
use dioxus::prelude::*;
use std::rc::Rc;

fn opened_runtime() -> Rc<Runtime> {
    let runtime = Runtime::with_core(boardstudio_web_test_runtime::engine_core_services());
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: ProjectDoc::empty("owned-edits", "Original"),
    });
    assert!(runtime.model().accepted.is_some());
    runtime
}

fn rename(name: &'static str) -> EditResolver {
    EditResolver::new("owned-edits-test", move |accepted| {
        let mut document = (*accepted.document).clone();
        document.name = name.into();
        Resolution::submit(
            vec![document.id.clone()],
            EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        )
    })
}

#[test]
fn owner_replacement_retires_observations_and_preserves_mounted_signals() {
    let dom = VirtualDom::new(VNode::empty);
    dom.in_scope(ScopeId::ROOT, || {
        let runtime = opened_runtime();
        let scope = runtime.scope();
        let mut edits = OwnedEdits::default();
        let draft = Signal::new("typed".to_owned());
        let failure = Signal::new(Some("previous failure".to_owned()));
        let field = FieldView::new(draft, failure);
        assert!(edits.owner_changed(scope.as_ref(), 1));
        edits.follow_owner(scope.as_ref(), 1);
        runtime.hold_next_core();
        edits.edits.submit(
            &runtime,
            "name",
            "rename",
            None,
            rename("Old owner"),
            field.submitted("typed".to_owned()),
        );

        edits.follow_owner(scope.as_ref(), 1);
        assert!(
            edits.edits.is_pending(&"name"),
            "the same owner keeps its observation"
        );
        assert!(edits.owner_changed(scope.as_ref(), 2));
        edits.follow_owner(scope.as_ref(), 2);
        assert!(!edits.owner_changed(scope.as_ref(), 2));
        assert!(!edits.edits.is_pending(&"name"));
        assert_eq!(edits.edits.pending_keys().len(), 0);
        assert!(!edits.edits.has_terminal());
        runtime.release_core();
        assert_eq!(runtime.model().accepted.unwrap().document.name, "Old owner");
        assert!(edits.edits.settle(true, |_| "ignored".into()).is_empty());
        assert_eq!(&*draft.peek(), "typed");
        assert_eq!(failure.peek().as_deref(), Some("previous failure"));

        edits.edits.submit(
            &runtime,
            "name",
            "rename",
            None,
            rename("New owner"),
            field.submitted("typed".to_owned()),
        );
        assert_eq!(
            edits
                .edits
                .settle(true, |_| "accepted projection".into())
                .len(),
            1
        );
        assert_eq!(&*draft.peek(), "accepted projection");
        assert_eq!(*failure.peek(), None);
        assert!(
            edits.owner_changed(None, 2),
            "scope changes also replace the owner"
        );
    });
}

#[test]
fn pending_payloads_follow_latest_submission_order_and_terminals_wait_for_settlement() {
    #[derive(Clone, Debug)]
    struct Request(&'static str, &'static str);

    impl PartialEq for Request {
        fn eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    let runtime = opened_runtime();
    let mut edits = OwnedEdits::default();
    edits.follow_owner(runtime.scope().as_ref(), 1);
    runtime.hold_next_core();
    let submit = |key, payload| {
        edits.edits.begin(
            &runtime,
            Request(key, payload),
            "rename",
            None,
            rename(payload),
        );
    };
    submit("first", "Original pending");
    submit("second", "Second");
    submit("first", "Latest");
    assert!(!edits.edits.has_terminal());
    assert_eq!(
        edits
            .edits
            .pending_keys()
            .into_iter()
            .map(|key| key.1)
            .collect::<Vec<_>>(),
        ["Second", "Latest"]
    );

    runtime.release_core();
    assert_eq!(edits.edits.pending_keys().len(), 0);
    assert!(edits.edits.has_terminal());
    assert!(
        edits.edits.has_terminal(),
        "checking does not consume terminals"
    );

    submit("first", "Replaced unobserved terminal");
    let results = edits.edits.settle(true, |_| unreachable!());
    let payloads = results
        .iter()
        .map(|result| match result {
            crate::draft_edits::PendingEditResult::Landed { key, .. } => key.1,
            other => panic!("expected a landing, got {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(payloads, ["Replaced unobserved terminal", "Second"]);
    assert!(!edits.edits.has_terminal());
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Replaced unobserved terminal"
    );
}
