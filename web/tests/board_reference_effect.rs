use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc, time::Duration};

#[path = "../src/presentation/board_reference_effect.rs"]
mod board_reference_effect;

#[derive(Clone)]
struct Probe {
    runs: Rc<Cell<usize>>,
    epoch: Rc<Cell<Option<Signal<u64>>>>,
    paths: Rc<Cell<Option<Signal<Vec<String>>>>>,
    error: Rc<Cell<Option<Signal<Option<String>>>>>,
}

impl PartialEq for Probe {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.runs, &other.runs)
    }
}

#[component]
fn MissingReferenceEffect(probe: Probe) -> Element {
    let model_paths = use_signal(|| vec!["stale.step".to_owned()]);
    let paths_asset_id = use_signal(|| Some("old-asset".to_owned()));
    let attempted_discovery = use_signal(|| Some("old-attempt".to_owned()));
    let request_generation = use_signal(|| 0u64);
    let error = use_signal(|| Some("stale error".to_owned()));
    probe.epoch.set(Some(request_generation));
    probe.paths.set(Some(model_paths));
    probe.error.set(Some(error));

    use_effect(use_reactive((&false, &false), {
        let runs = probe.runs.clone();
        move |(has_reference, already_busy)| {
            if !has_reference && !already_busy {
                runs.set(runs.get() + 1);
                board_reference_effect::clear_missing_reference(
                    model_paths,
                    paths_asset_id,
                    attempted_discovery,
                    request_generation,
                    error,
                );
            }
        }
    }));

    rsx! { div { "Paths: {model_paths().len()}" } }
}

#[tokio::test]
async fn mounted_missing_reference_reset_invalidates_once_and_settles() {
    let probe = Probe {
        runs: Rc::new(Cell::new(0)),
        epoch: Rc::new(Cell::new(None)),
        paths: Rc::new(Cell::new(None)),
        error: Rc::new(Cell::new(None)),
    };
    let mut dom = VirtualDom::new_with_props(
        MissingReferenceEffect,
        MissingReferenceEffectProps {
            probe: probe.clone(),
        },
    );
    dom.rebuild_in_place();
    for _ in 0..8 {
        if tokio::time::timeout(Duration::from_millis(10), dom.wait_for_work())
            .await
            .is_err()
        {
            break;
        }
        dom.render_immediate_to_vec();
    }

    assert_eq!(probe.runs.get(), 1, "absent reference reset must settle");
    assert_eq!(*probe.epoch.get().unwrap().peek(), 1);
    assert!(probe.paths.get().unwrap().peek().is_empty());
    assert!(probe.error.get().unwrap().peek().is_none());
}
