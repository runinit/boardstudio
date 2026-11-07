//! Ticket 008: the Runtime-owned registry of live draft owners.
//!
//! Each live explicit-submit draft owner registers one entry per admission key
//! while it is mounted. Both Update Now checkpoints (and the post-takeover
//! reload) poll the registry synchronously through
//! [`Runtime::update_safety_inputs`](super::runtime::Runtime::update_safety_inputs),
//! so there is no cached or event-ordered view of draft state to go stale.
//!
//! Registration is RAII: holding the returned [`DraftOwnerGuard`] keeps the
//! entry live; dropping it (genuine unmount, or a scope-change effect replacing
//! it) marks the entry dead and the next poll prunes it. Polling also prunes
//! entries whose project scope no longer matches the current accepted document,
//! which is what makes a project or scope switch retire the old owner.
//! Pruned dead entries are dropped with a one-time diagnostic instead of being
//! treated as blockers; stale-scope entries retire silently as normal lifecycle.
//! Hidden, keep-mounted and portal-retained owners stay registered and keep
//! blocking, because their guard is still alive.

use super::update_safety::UnresolvedDraft;
use boardstudio_application::SessionEpoch;
use std::rc::{Rc, Weak};

/// The project scope an entry belongs to: the accepted document it was
/// registered against. Poll drops entries whose scope no longer matches.
pub type DraftOwnerScope = (String, SessionEpoch);

/// One live draft owner's registration, evaluated synchronously at each poll.
pub struct DraftOwnerRegistration {
    /// The scope the owner is registered against; stale scopes prune silently.
    pub project_scope: DraftOwnerScope,
    /// Workspace that owns the form, e.g. "Layout". Diagnostic context only.
    pub workspace: &'static str,
    /// Owner identity, e.g. "geometry-scripts". Diagnostic context only.
    pub owner: &'static str,
    /// Admission key within the owner, e.g. "apply". Diagnostic context only.
    pub admission_key: &'static str,
    /// The editing area named in the deferral.
    pub area: String,
    /// The per-key remedy named in the deferral.
    pub remedy: String,
    /// Whether this admission key currently holds an unresolved Draft value.
    /// Read synchronously (signal peeks, never subscriptions) on every poll.
    pub is_unresolved: Rc<dyn Fn() -> bool>,
}

/// RAII liveness for one registration. Retire it explicitly with
/// [`DraftOwnerRegistry::remove`] on unmount or scope change; dropping the
/// guard without retiring leaves a dead entry that the next poll prunes with a
/// one-time diagnostic instead of treating it as a blocker.
pub struct DraftOwnerGuard {
    id: u64,
    _alive: Rc<()>,
}

struct RegisteredDraftOwner {
    id: u64,
    project_scope: DraftOwnerScope,
    workspace: &'static str,
    owner: &'static str,
    admission_key: &'static str,
    area: String,
    remedy: String,
    alive: Weak<()>,
    is_unresolved: Rc<dyn Fn() -> bool>,
}

/// The live draft-owner inventory polled by the update-safety gate.
#[derive(Default)]
pub struct DraftOwnerRegistry {
    next_id: u64,
    entries: Vec<RegisteredDraftOwner>,
    diagnostics: Vec<String>,
}

impl DraftOwnerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one admission key. The entry stays live while the returned
    /// guard is held; dropping the guard retires it at the next poll.
    pub fn register(&mut self, registration: DraftOwnerRegistration) -> DraftOwnerGuard {
        let alive = Rc::new(());
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(RegisteredDraftOwner {
            id,
            project_scope: registration.project_scope,
            workspace: registration.workspace,
            owner: registration.owner,
            admission_key: registration.admission_key,
            area: registration.area,
            remedy: registration.remedy,
            alive: Rc::downgrade(&alive),
            is_unresolved: registration.is_unresolved,
        });
        DraftOwnerGuard { id, _alive: alive }
    }

    /// Poll every live entry against the current scope: prune dead and
    /// stale-scope entries, then collect the unresolved keys. Dead entries are
    /// never blockers; each records a one-time diagnostic for
    /// [`take_diagnostics`](Self::take_diagnostics).
    pub fn poll(&mut self, current_scope: Option<&DraftOwnerScope>) -> Vec<UnresolvedDraft> {
        let mut unresolved = Vec::new();
        let mut index = 0;
        while index < self.entries.len() {
            let entry = &self.entries[index];
            if entry.alive.upgrade().is_none() {
                let retired = self.entries.remove(index);
                self.diagnostics.push(format!(
                    "draft owner retired without unregistering: {}/{}/{}; dropped, not blocking",
                    retired.workspace, retired.owner, retired.admission_key
                ));
                continue;
            }
            if current_scope.is_none_or(|scope| *scope != entry.project_scope) {
                self.entries.remove(index);
                continue;
            }
            if (entry.is_unresolved)() {
                unresolved.push(UnresolvedDraft {
                    area: entry.area.clone(),
                    remedy: entry.remedy.clone(),
                });
            }
            index += 1;
        }
        unresolved
    }

    /// Retire one registration silently by guard identity: the normal path for
    /// genuine unmount and scope change. Dropping the guard without retiring
    /// leaves a dead entry for [`poll`](Self::poll) to prune with a diagnostic.
    pub fn remove(&mut self, guard: &DraftOwnerGuard) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id != guard.id);
        before != self.entries.len()
    }

    /// Drain the one-time prune diagnostics recorded by [`poll`](Self::poll).
    pub fn take_diagnostics(&mut self) -> Vec<String> {
        std::mem::take(&mut self.diagnostics)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl std::fmt::Debug for DraftOwnerRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DraftOwnerRegistry")
            .field("entries", &self.entries.len())
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_safety::UnresolvedDraft;
    use std::cell::Cell;

    fn scope(id: &str, epoch: u64) -> DraftOwnerScope {
        (id.to_owned(), SessionEpoch(epoch))
    }

    fn register_flag(
        registry: &mut DraftOwnerRegistry,
        project_scope: DraftOwnerScope,
        unresolved: Rc<Cell<bool>>,
    ) -> DraftOwnerGuard {
        registry.register(DraftOwnerRegistration {
            project_scope,
            workspace: "Layout",
            owner: "geometry-scripts",
            admission_key: "apply",
            area: "Geometry scripts".to_owned(),
            remedy: "Apply the script edit, or cancel the edit in the form".to_owned(),
            is_unresolved: Rc::new(move || unresolved.get()),
        })
    }

    #[test]
    fn unresolved_entries_block_with_area_and_remedy() {
        let mut registry = DraftOwnerRegistry::new();
        let unresolved = Rc::new(Cell::new(true));
        let _guard = register_flag(&mut registry, scope("doc", 1), unresolved);
        let blocked = registry.poll(Some(&scope("doc", 1)));
        assert_eq!(
            blocked,
            vec![UnresolvedDraft {
                area: "Geometry scripts".to_owned(),
                remedy: "Apply the script edit, or cancel the edit in the form".to_owned(),
            }]
        );
    }

    #[test]
    fn resolved_entries_do_not_block() {
        let mut registry = DraftOwnerRegistry::new();
        let unresolved = Rc::new(Cell::new(false));
        let _guard = register_flag(&mut registry, scope("doc", 1), unresolved);
        assert!(registry.poll(Some(&scope("doc", 1))).is_empty());
    }

    #[test]
    fn dropped_guards_prune_with_a_one_time_diagnostic_and_never_block() {
        let mut registry = DraftOwnerRegistry::new();
        let unresolved = Rc::new(Cell::new(true));
        let guard = register_flag(&mut registry, scope("doc", 1), unresolved);
        assert_eq!(registry.len(), 1);
        drop(guard);
        let blocked = registry.poll(Some(&scope("doc", 1)));
        assert!(blocked.is_empty(), "a dead entry never blocks");
        assert_eq!(registry.len(), 0);
        let diagnostics = registry.take_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0].contains("geometry-scripts") && diagnostics[0].contains("not blocking"),
            "the diagnostic names the owner and the fail-open: {diagnostics:?}"
        );
        assert!(
            registry.take_diagnostics().is_empty(),
            "the diagnostic fires once"
        );
    }

    #[test]
    fn scope_switches_retire_stale_entries_silently() {
        let mut registry = DraftOwnerRegistry::new();
        let unresolved = Rc::new(Cell::new(true));
        let _guard = register_flag(&mut registry, scope("doc-a", 1), unresolved);
        let blocked = registry.poll(Some(&scope("doc-b", 1)));
        assert!(blocked.is_empty(), "a stale scope never blocks");
        assert_eq!(registry.len(), 0);
        assert!(
            registry.take_diagnostics().is_empty(),
            "a scope change is normal lifecycle, not a diagnostic"
        );
    }

    #[test]
    fn closing_the_project_retires_every_entry() {
        let mut registry = DraftOwnerRegistry::new();
        let unresolved = Rc::new(Cell::new(true));
        let _guard = register_flag(&mut registry, scope("doc", 1), unresolved);
        assert!(registry.poll(None).is_empty());
        assert_eq!(registry.len(), 0);
    }
}
