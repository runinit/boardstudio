//! Workspace-transition invalidation for the mounted Parts create action.

use dioxus::prelude::*;

#[component]
pub(crate) fn PartsViewGenerationOwner(
    workspace: Signal<&'static str>,
    view_generation: Signal<u64>,
) -> Element {
    use_effect(use_reactive((&workspace(),), move |_| {
        // The effect subscribes to workspace only. Reading the generation through `peek` avoids
        // subscribing to the signal that this workspace transition deliberately advances.
        let next_generation = (*view_generation.peek()).saturating_add(1);
        view_generation.set(next_generation);
    }));

    rsx! {}
}

#[cfg(test)]
mod tests {
    use super::PartsViewGenerationOwner;
    use dioxus::prelude::*;
    use std::{cell::RefCell, rc::Rc, time::Duration};

    #[derive(Clone)]
    struct Probe {
        workspace: Rc<RefCell<Option<Signal<&'static str>>>>,
        generation: Rc<RefCell<Option<Signal<u64>>>>,
    }

    impl PartialEq for Probe {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.workspace, &other.workspace)
                && Rc::ptr_eq(&self.generation, &other.generation)
        }
    }

    impl Eq for Probe {}

    fn host() -> Element {
        let probe = use_context::<Probe>();
        let workspace = use_signal(|| "Parts");
        let generation = use_signal(|| 0_u64);
        *probe.workspace.borrow_mut() = Some(workspace);
        *probe.generation.borrow_mut() = Some(generation);
        rsx! {
            PartsViewGenerationOwner { workspace, view_generation: generation }
        }
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

    #[tokio::test]
    async fn mounted_workspace_generation_advances_once_without_self_subscribing() {
        let probe = Probe {
            workspace: Rc::new(RefCell::new(None)),
            generation: Rc::new(RefCell::new(None)),
        };
        let mut dom = VirtualDom::new(host);
        dom.provide_root_context(probe.clone());
        dom.rebuild_in_place();
        settle(&mut dom).await;

        let mut workspace = probe.workspace.borrow().unwrap();
        let generation = probe.generation.borrow().unwrap();
        assert_eq!(
            *generation.peek(),
            1,
            "initial workspace owner is observed once"
        );
        settle(&mut dom).await;
        assert_eq!(
            *generation.peek(),
            1,
            "generation writes do not rerun the effect"
        );

        workspace.set("Layout");
        settle(&mut dom).await;
        assert_eq!(
            *generation.peek(),
            2,
            "one workspace transition advances once"
        );
        settle(&mut dom).await;
        assert_eq!(
            *generation.peek(),
            2,
            "settled workspace does not advance again"
        );

        workspace.set("Parts");
        settle(&mut dom).await;
        assert_eq!(
            *generation.peek(),
            3,
            "returning to Parts is another transition"
        );
        settle(&mut dom).await;
        assert_eq!(*generation.peek(), 3);
    }
}
