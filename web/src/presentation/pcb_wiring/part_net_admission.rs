use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};

pub(crate) fn use_part_net_owner_lifetime() -> Rc<Cell<bool>> {
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    alive
}

/// Check lifetime first so a detached classifier never reads signals after its owner is dropped.
pub(crate) fn part_net_owner_context_is_current(
    alive: &Cell<bool>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    generation: u64,
) -> bool {
    alive.get() && workspace() == "PCB" && scope_generation() == generation
}
