//! Project rename admission and resolution. The module compiles natively so its tests
//! drive the real Session and Core through the native Runtime; the Library panel binds
//! the resulting resolver to its name field.

use boardstudio_application::{AcceptedSnapshot, EditResolver, SessionEpoch};
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
    Ok(EditResolver::rename_project("project-name", name))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use boardstudio_application::Event;
    use boardstudio_model::model::{EditOperation, ProjectDoc};
    use std::rc::Rc;

    fn open(id: &str, name: &str) -> Rc<Runtime> {
        let runtime = Runtime::with_core(boardstudio_web_test_runtime::engine_core_services());
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: ProjectDoc::empty(id, name),
        });
        runtime
    }

    fn owner(runtime: &Runtime) -> ProjectNameOwner {
        ProjectNameOwner::from(&runtime.model().accepted.expect("accepted project"))
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
        let accepted = runtime.model().accepted.unwrap();
        let resolver = project_name_resolver(&runtime, &current, &Cell::new(true), "  Renamed  ")
            .expect("rename admitted");
        assert!(matches!(
            resolver.resolve(&accepted),
            boardstudio_application::Resolution::Submit(command)
                if command.target_ids == [accepted.document.id.clone()]
                    && command.operation == (EditOperation::RenameProject { name: "Renamed".into() })
        ));
        runtime.submit(Event::ResolveEdit {
            operation_id: runtime.operation(),
            label: "project-name".into(),
            resolver,
        });
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(accepted.document.name, "Renamed");
        assert_eq!(accepted.document.revision, 1);
    }
}
