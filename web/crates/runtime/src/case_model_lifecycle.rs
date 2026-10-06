//! Private native Case worker and viewer input identities.
use std::{cell::RefCell, rc::Rc};

pub fn case_model_worker<K: Clone + PartialEq, W>(
    slot: &RefCell<Option<(K, Rc<W>)>>,
    scope: &K,
    usable: impl FnOnce(&W) -> bool,
    create: impl FnOnce() -> Result<Rc<W>, String>,
) -> Result<Rc<W>, String> {
    let current = slot
        .borrow()
        .as_ref()
        .and_then(|(current, worker)| (current == scope && usable(worker)).then(|| worker.clone()));
    if let Some(worker) = current {
        return Ok(worker);
    }
    let worker = create()?;
    *slot.borrow_mut() = Some((scope.clone(), worker.clone()));
    Ok(worker)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionInputs {
    pub source: usize,
    pub preview: usize,
    pub models: Vec<(String, usize)>,
    pub theme: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[derive(Default)]
    struct Worker {
        closed: Cell<bool>,
    }

    #[test]
    fn first_step_decode_creates_a_worker_without_retaining_the_cache_borrow() {
        let slot = RefCell::new(None);
        let worker = case_model_worker(
            &slot,
            &1,
            |_: &Worker| true,
            || Ok(Rc::new(Worker::default())),
        )
        .unwrap();
        assert!(Rc::ptr_eq(&slot.borrow().as_ref().unwrap().1, &worker));
    }

    #[test]
    fn changed_scope_and_closed_worker_are_replaced_but_current_worker_is_reused() {
        let old = Rc::new(Worker::default());
        let slot = RefCell::new(Some((1, old.clone())));
        let fresh = case_model_worker(
            &slot,
            &2,
            |worker| !worker.closed.get(),
            || Ok(Rc::new(Worker::default())),
        )
        .unwrap();
        assert!(!Rc::ptr_eq(&old, &fresh));
        let reused = case_model_worker(
            &slot,
            &2,
            |worker| !worker.closed.get(),
            || panic!("current worker must be reused"),
        )
        .unwrap();
        assert!(Rc::ptr_eq(&fresh, &reused));
        fresh.closed.set(true);
        let replacement = case_model_worker(
            &slot,
            &2,
            |worker| !worker.closed.get(),
            || Ok(Rc::new(Worker::default())),
        )
        .unwrap();
        assert!(!Rc::ptr_eq(&fresh, &replacement));
    }

    #[test]
    fn failed_replacement_keeps_the_existing_cached_worker() {
        let original = Rc::new(Worker::default());
        let slot = RefCell::new(Some((1, original.clone())));
        assert!(
            case_model_worker(
                &slot,
                &2,
                |worker| !worker.closed.get(),
                || Err("unavailable".into())
            )
            .is_err()
        );
        assert!(Rc::ptr_eq(&slot.borrow().as_ref().unwrap().1, &original));
    }

    #[test]
    fn arriving_board_preview_invalidates_an_unchanged_cad_projection_before_models_finish() {
        let before = ProjectionInputs {
            source: 1,
            preview: 0,
            models: vec![],
            theme: "light".into(),
        };
        let board_first = ProjectionInputs {
            preview: 2,
            ..before.clone()
        };
        assert_ne!(before, board_first);
        let replacement = ProjectionInputs {
            preview: 3,
            ..board_first.clone()
        };
        assert_ne!(board_first, replacement);
        assert_ne!(board_first, before); // withdrawn preview must remove old board data
    }

    #[test]
    fn exact_model_identity_and_theme_invalidate_projection_without_pointer_hash_collisions() {
        let first = ProjectionInputs {
            source: 1,
            preview: 2,
            models: vec![("a".into(), 3)],
            theme: "light".into(),
        };
        let new_row = ProjectionInputs {
            models: vec![("b".into(), 3)],
            ..first.clone()
        };
        assert_ne!(first, new_row);
        assert_ne!(
            first,
            ProjectionInputs {
                theme: "dark".into(),
                ..first.clone()
            }
        );
        assert_eq!(first, first.clone());
    }
}
