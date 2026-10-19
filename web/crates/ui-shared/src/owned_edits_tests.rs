use super::OwnedEdits;
use boardstudio_application::{EditResolver, Event, Resolution};
use boardstudio_core::model::{EditOperation, ProjectDoc};
use boardstudio_web_runtime::runtime::Runtime;
use dioxus::prelude::*;
use std::rc::Rc;

fn opened_runtime() -> Rc<Runtime> {
    let runtime = Runtime::new();
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
fn owner_replacement_retires_observations_and_rebinds_mounted_signals() {
    let dom = VirtualDom::new(VNode::empty);
    dom.in_scope(ScopeId::ROOT, || {
        let runtime = opened_runtime();
        let scope = runtime.scope();
        let mut edits = OwnedEdits::default();
        let draft = Signal::new("typed".to_owned());
        let failure = Signal::new(Some("previous failure".to_owned()));
        edits.bind_field("name", draft, failure);
        assert!(edits.owner_changed(scope.as_ref(), 1));
        edits.follow_owner(scope.as_ref(), 1);
        runtime.hold_next_core();
        edits.helper.begin_field(
            &runtime,
            "name",
            "rename",
            None,
            rename("Old owner"),
            "typed",
        );
        edits.remember("name");

        edits.follow_owner(scope.as_ref(), 1);
        assert!(
            edits.is_pending(&"name"),
            "the same owner keeps its observation"
        );
        assert!(edits.owner_changed(scope.as_ref(), 2));
        edits.follow_owner(scope.as_ref(), 2);
        assert!(!edits.owner_changed(scope.as_ref(), 2));
        assert!(!edits.is_pending(&"name"));
        assert_eq!(edits.pending().count(), 0);
        assert!(!edits.has_terminal());
        runtime.release_core();
        assert_eq!(runtime.model().accepted.unwrap().document.name, "Old owner");
        assert!(edits.helper.settle(true, |_| "ignored".into()).is_empty());
        assert_eq!(&*draft.peek(), "typed");
        assert_eq!(failure.peek().as_deref(), Some("previous failure"));

        edits.helper.begin_field(
            &runtime,
            "name",
            "rename",
            None,
            rename("New owner"),
            "typed",
        );
        edits.remember("name");
        assert_eq!(
            edits
                .helper
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
fn bound_draft_tracks_live_rebindings_and_forgets_unmounted_fields() {
    let dom = VirtualDom::new(VNode::empty);
    dom.in_scope(ScopeId::ROOT, || {
        let mut edits = OwnedEdits::default();
        let mut draft = Signal::new(" 1.50 ".to_owned());
        let failure = Signal::new(None);
        assert_eq!(edits.bound_draft(&"missing"), None);
        edits.bind_field("name", draft, failure);
        assert_eq!(edits.bound_draft(&"name").as_deref(), Some(" 1.50 "));
        draft.set("2.00".into());
        assert_eq!(edits.bound_draft(&"name").as_deref(), Some("2.00"));
        let replacement = Signal::new("remounted".to_owned());
        edits.bind_field("name", replacement, failure);
        edits.follow_owner(None, 1);
        edits.follow_owner(None, 2);
        assert_eq!(edits.bound_draft(&"name").as_deref(), Some("remounted"));
        edits.unbind_field(&"name");
        edits.unbind_field(&"name");
        edits.follow_owner(None, 3);
        assert_eq!(edits.bound_draft(&"name"), None);

        let runtime = opened_runtime();
        edits.helper.begin_field(
            &runtime,
            "name",
            "rename",
            None,
            rename("Unbound"),
            "remounted",
        );
        edits
            .helper
            .settle(true, |_| "must not reach unmounted field".into());
        assert_eq!(&*replacement.peek(), "remounted");
    });
}

#[test]
fn pending_requests_keep_latest_order_until_terminal_requests_are_pruned() {
    let runtime = opened_runtime();
    let mut edits = OwnedEdits::default();
    edits.follow_owner(runtime.scope().as_ref(), 1);
    runtime.hold_next_core();
    edits
        .helper
        .begin_one_shot(&runtime, "first", "rename", None, rename("First"));
    edits.remember("first");
    edits
        .helper
        .begin_one_shot(&runtime, "second", "rename", None, rename("Second"));
    edits.remember("second");
    edits
        .helper
        .begin_one_shot(&runtime, "first", "rename", None, rename("Latest"));
    edits.remember("first");
    assert!(edits.is_pending(&"first"));
    assert!(!edits.has_terminal());
    edits.prune();
    assert_eq!(
        edits.pending().copied().collect::<Vec<_>>(),
        vec!["second", "first"]
    );
    runtime.release_core();
    assert!(!edits.is_pending(&"first"));
    assert_eq!(edits.pending().count(), 0);
    assert!(edits.has_terminal());
    edits.prune();
    assert!(!edits.has_terminal());
    assert_eq!(runtime.model().accepted.unwrap().document.name, "Latest");
}
