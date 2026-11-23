use super::*;

pub(crate) struct SetupProjectNameEdit {
    pub(crate) draft: Signal<Option<(String, String)>>,
    pub(crate) failure: Signal<Option<String>>,
    pub(crate) on_change: Callback<String>,
    pub(crate) on_commit: Callback<()>,
}

pub(crate) fn use_setup_project_name(runtime: Rc<Runtime>) -> SetupProjectNameEdit {
    let accepted = runtime.model().accepted;
    let owner = accepted
        .as_ref()
        .map(|s| (s.session_epoch, s.document.id.clone()));
    let accepted_name = accepted
        .as_ref()
        .map(|s| (s.document.id.clone(), s.document.name.clone()));
    let mut draft = use_signal(|| accepted_name.clone());
    let mut dirty = use_signal(|| false);
    let mut failure = use_signal(|| None::<String>);
    let mut prev_owner = use_signal(|| owner.clone());
    let version = use_context::<Signal<u64>>()();
    use_effect(use_reactive(
        (&owner, &accepted_name, &version),
        move |(owner, name, _)| {
            let changed = *prev_owner.peek() != owner;
            prev_owner.set(owner);
            if changed || !*dirty.peek() {
                draft.set(name);
                dirty.set(false);
            }
            if changed {
                failure.set(None);
            }
        },
    ));
    let on_change = use_callback({
        let owner = owner.clone();
        move |name: String| {
            if let Some((_, id)) = &owner {
                failure.set(None);
                dirty.set(true);
                draft.set(Some((id.clone(), name)));
            }
        }
    });
    let on_commit = use_callback(move |_| {
        if !dirty() {
            return;
        }
        dirty.set(false);
        let Some((epoch, id)) = owner.as_ref() else {
            return;
        };
        let Some(cur) = runtime.model().accepted else {
            return;
        };
        if cur.session_epoch != *epoch || cur.document.id != *id {
            return;
        }
        let proposed = draft()
            .filter(|(pid, _)| pid == id)
            .map(|(_, n)| n.trim().to_owned())
            .unwrap_or_default();
        if proposed.is_empty() {
            draft.set(Some((id.clone(), cur.document.name.clone())));
            return;
        }
        let res = runtime.commit(
            "setup-project-name",
            boardstudio_application::EditResolver::rename_project("setup-project-name", proposed),
        );
        if let Some(acc) = runtime.model().accepted {
            draft.set(Some((acc.document.id.clone(), acc.document.name.clone())));
        }
        failure.set(res.err());
    });
    SetupProjectNameEdit {
        draft,
        failure,
        on_change,
        on_commit,
    }
}
