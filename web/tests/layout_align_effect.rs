use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc, time::Duration};

#[path = "../src/presentation/objects/layout_align_geometry.rs"]
mod layout_align_geometry;

#[derive(Clone)]
struct Probe {
    selected_reference: Rc<Cell<Option<Signal<Option<String>>>>>,
    workspace: Rc<Cell<Option<Signal<bool>>>>,
    references: Rc<Cell<Option<Signal<Vec<String>>>>>,
    target_available: Rc<Cell<Option<Signal<bool>>>>,
    effect_runs: Rc<Cell<usize>>,
    initial_reference: Option<String>,
    initial_workspace: bool,
    initial_references: Vec<String>,
}

impl PartialEq for Probe {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.selected_reference, &other.selected_reference)
            && Rc::ptr_eq(&self.workspace, &other.workspace)
            && Rc::ptr_eq(&self.references, &other.references)
            && Rc::ptr_eq(&self.target_available, &other.target_available)
            && Rc::ptr_eq(&self.effect_runs, &other.effect_runs)
            && self.initial_reference == other.initial_reference
            && self.initial_workspace == other.initial_workspace
            && self.initial_references == other.initial_references
    }
}

impl Eq for Probe {}

#[component]
fn AlignReferenceEffectProbe(probe: Probe) -> Element {
    let mut selected_reference = use_signal(|| probe.initial_reference.clone());
    let workspace = use_signal(|| probe.initial_workspace);
    let references = use_signal(|| probe.initial_references.clone());
    let target_available = use_signal(|| true);
    probe.selected_reference.set(Some(selected_reference));
    probe.workspace.set(Some(workspace));
    probe.references.set(Some(references));
    probe.target_available.set(Some(target_available));

    use_effect(use_reactive(
        (
            &workspace(),
            &references.read().clone(),
            &selected_reference.read().clone(),
            &target_available(),
        ),
        {
            let effect_runs = probe.effect_runs.clone();
            move |(workspace, references, _current, target_available)| {
                effect_runs.set(effect_runs.get() + 1);
                let current = selected_reference.read().clone();
                layout_align_geometry::reconcile_reference_choice(
                    current.as_deref(),
                    &references,
                    workspace && target_available,
                    |next| selected_reference.set(next),
                );
            }
        },
    ));

    rsx! { div { "Align reference effect" } }
}

async fn settle(dom: &mut VirtualDom) {
    for _ in 0..30 {
        if tokio::time::timeout(Duration::from_millis(10), dom.wait_for_work())
            .await
            .is_err()
        {
            break;
        }
        dom.render_immediate_to_vec();
    }
}

fn probe(
    initial_reference: Option<&str>,
    initial_workspace: bool,
    initial_references: &[&str],
) -> Probe {
    Probe {
        selected_reference: Rc::new(Cell::new(None)),
        workspace: Rc::new(Cell::new(None)),
        references: Rc::new(Cell::new(None)),
        target_available: Rc::new(Cell::new(None)),
        effect_runs: Rc::new(Cell::new(0)),
        initial_reference: initial_reference.map(str::to_owned),
        initial_workspace,
        initial_references: initial_references
            .iter()
            .map(|id| (*id).to_owned())
            .collect(),
    }
}

fn mount(probe: Probe) -> VirtualDom {
    let mut dom = VirtualDom::new_with_props(
        AlignReferenceEffectProbe,
        AlignReferenceEffectProbeProps {
            probe: probe.clone(),
        },
    );
    dom.rebuild_in_place();
    dom
}

#[tokio::test]
async fn mounted_empty_reference_effect_does_not_self_trigger() {
    let probe = probe(None, true, &[]);
    let mut dom = mount(probe.clone());
    settle(&mut dom).await;

    assert_eq!(probe.effect_runs.get(), 1);
    assert_eq!(
        probe.selected_reference.get().unwrap().peek().as_deref(),
        None
    );
}

#[tokio::test]
async fn mounted_reference_survives_hidden_workspace_and_falls_back_only_when_ineligible() {
    let probe = probe(Some("B"), false, &["A"]);
    let mut dom = mount(probe.clone());
    settle(&mut dom).await;
    let selected_reference = probe.selected_reference.get().unwrap();
    let mut workspace = probe.workspace.get().unwrap();
    let mut references = probe.references.get().unwrap();
    let mut target_available = probe.target_available.get().unwrap();
    assert_eq!(selected_reference.peek().as_deref(), Some("B"));

    workspace.set(true);
    references.set(vec!["A".into(), "B".into()]);
    settle(&mut dom).await;
    assert_eq!(selected_reference.peek().as_deref(), Some("B"));

    references.set(vec!["A".into()]);
    target_available.set(false);
    settle(&mut dom).await;
    assert_eq!(selected_reference.peek().as_deref(), Some("B"));

    target_available.set(true);
    references.set(vec!["A".into(), "B".into()]);
    settle(&mut dom).await;
    assert_eq!(selected_reference.peek().as_deref(), Some("B"));

    references.set(vec!["A".into()]);
    settle(&mut dom).await;
    assert_eq!(selected_reference.peek().as_deref(), Some("A"));
    let settled_runs = probe.effect_runs.get();
    settle(&mut dom).await;
    assert_eq!(probe.effect_runs.get(), settled_runs);
}
