//! Project rename admission and resolution. The module compiles natively so its tests
//! drive the real Session and Core through the native Runtime; the Library panel binds
//! the resulting resolver to its name field through `PendingEditSignals`.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, SessionEpoch};
use boardstudio_core::model::{EditOperation, ProjectDoc};
use boardstudio_web_runtime::runtime::Runtime;
use std::cell::Cell;

/// The accepted project a rename was drafted against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectNameOwner {
    session_epoch: SessionEpoch,
    document_id: String,
}

impl From<&AcceptedSnapshot> for ProjectNameOwner {
    fn from(snapshot: &AcceptedSnapshot) -> Self {
        Self {
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
        }
    }
}

/// The only logical key of the project name field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectNameKey {
    Name,
}

/// Why a rename commit was not admitted.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ProjectNameRejection {
    /// The Library page left the screen; its retained action must stay silent.
    Unmounted,
    /// No accepted project exists.
    NoProject,
    /// The accepted project is not the one the draft belongs to.
    OwnerChanged,
    /// The trimmed draft is empty.
    Empty,
}

fn renamed_document(document: &ProjectDoc, value: &str) -> Option<ProjectDoc> {
    let name = value.trim();
    if name.is_empty() || name == document.name {
        return None;
    }
    let mut renamed = document.clone();
    renamed.name = name.to_owned();
    Some(renamed)
}

/// Admit a rename commit for `owner` and build the resolver that applies it to whatever
/// document is accepted when the edit runs.
pub(crate) fn project_name_resolver(
    runtime: &Runtime,
    owner: &ProjectNameOwner,
    mounted: &Cell<bool>,
    value: &str,
) -> Result<EditResolver, ProjectNameRejection> {
    if !mounted.get() {
        return Err(ProjectNameRejection::Unmounted);
    }
    let snapshot = runtime
        .model()
        .accepted
        .ok_or(ProjectNameRejection::NoProject)?;
    if *owner != ProjectNameOwner::from(&snapshot) {
        return Err(ProjectNameRejection::OwnerChanged);
    }
    let name = value.trim().to_owned();
    if name.is_empty() {
        return Err(ProjectNameRejection::Empty);
    }
    Ok(EditResolver::new(
        "project-name",
        move |accepted: &AcceptedSnapshot| {
            let Some(document) = renamed_document(&accepted.document, &name) else {
                return Resolution::Unchanged;
            };
            Resolution::submit(
                vec![document.id.clone()],
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            )
        },
    ))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use boardstudio_application::Event;
    use boardstudio_web_runtime::pending_edits::{PendingEditResult, PendingEdits};
    use std::rc::Rc;

    fn open(id: &str, name: &str) -> Rc<Runtime> {
        let runtime = Runtime::new();
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: ProjectDoc::empty(id, name),
        });
        runtime
    }

    fn owner(runtime: &Runtime) -> ProjectNameOwner {
        ProjectNameOwner::from(&runtime.model().accepted.expect("accepted project"))
    }

    fn rename(
        runtime: &Rc<Runtime>,
        edits: &mut PendingEdits<ProjectNameKey>,
        owner: &ProjectNameOwner,
        value: &str,
    ) {
        let resolver = project_name_resolver(runtime, owner, &Cell::new(true), value)
            .expect("rename admitted");
        edits.begin(
            runtime,
            ProjectNameKey::Name,
            "project-name",
            Some("project name".into()),
            resolver,
        );
    }

    #[test]
    fn admission_rejects_unmounted_changed_owner_and_blank_drafts() {
        let runtime = open("rename-admission", "Original");
        let current = owner(&runtime);
        assert_eq!(
            project_name_resolver(&runtime, &current, &Cell::new(false), "New").err(),
            Some(ProjectNameRejection::Unmounted)
        );
        assert_eq!(
            project_name_resolver(&runtime, &current, &Cell::new(true), "   ").err(),
            Some(ProjectNameRejection::Empty)
        );
        let other = open("someone-else", "Other");
        assert_eq!(
            project_name_resolver(&runtime, &owner(&other), &Cell::new(true), "New").err(),
            Some(ProjectNameRejection::OwnerChanged)
        );
        assert_eq!(runtime.model().accepted.unwrap().document.revision, 0);
    }

    #[test]
    fn rename_lands_trimmed_and_same_name_changes_nothing() {
        let runtime = open("rename-lands", "Original");
        let current = owner(&runtime);
        let mut edits = PendingEdits::default();

        rename(&runtime, &mut edits, &current, "  Renamed  ");
        assert_eq!(
            edits.settle(true),
            vec![PendingEditResult::Landed {
                key: ProjectNameKey::Name,
                revision: 1,
            }]
        );
        assert_eq!(runtime.model().accepted.unwrap().document.name, "Renamed");

        rename(&runtime, &mut edits, &current, "Renamed");
        assert_eq!(
            edits.settle(true),
            vec![PendingEditResult::Landed {
                key: ProjectNameKey::Name,
                revision: 1,
            }],
            "an unchanged name lands at the existing revision"
        );
        assert_eq!(runtime.model().accepted.unwrap().document.revision, 1);
    }

    #[test]
    fn rename_resolves_against_the_document_accepted_when_it_runs() {
        let runtime = open("rename-queued", "Original");
        let current = owner(&runtime);
        let mut edits = PendingEdits::default();
        runtime.hold_next_core();
        rename(&runtime, &mut edits, &current, "First");
        rename(&runtime, &mut edits, &current, "Latest");
        runtime.release_core();

        assert_eq!(runtime.model().accepted.unwrap().document.name, "Latest");
        assert_eq!(
            edits.settle(true),
            vec![PendingEditResult::Landed {
                key: ProjectNameKey::Name,
                revision: 2,
            }]
        );
    }

    #[test]
    fn rename_retires_silently_when_its_owner_is_gone() {
        let runtime = open("rename-retired", "Original");
        let current = owner(&runtime);
        let mut edits = PendingEdits::default();
        runtime.hold_next_core();
        rename(&runtime, &mut edits, &current, "Orphan");
        assert_eq!(
            edits.settle(false),
            vec![PendingEditResult::Retired {
                key: ProjectNameKey::Name,
            }]
        );
        runtime.release_core();
    }

    #[test]
    fn rename_failure_surfaces_the_ticket_message() {
        let runtime = open("rename-fails", "Original");
        let current = owner(&runtime);
        let mut edits = PendingEdits::default();
        runtime.fail_next_core("engine died");
        rename(&runtime, &mut edits, &current, "Doomed");
        let results = edits.settle(true);
        assert!(
            matches!(
                results.as_slice(),
                [PendingEditResult::Failed { message, .. }] if message.contains("engine died")
            ),
            "{results:?}"
        );
        assert_eq!(runtime.model().accepted.unwrap().document.name, "Original");
    }
}
