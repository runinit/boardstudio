use super::*;

pub(crate) struct SetupProjectNameEdit {
    pub(crate) draft: Signal<Option<(String, String)>>,
    pub(crate) failure: Signal<Option<String>>,
    pub(crate) on_change: Callback<String>,
    pub(crate) on_commit: Callback<()>,
}

pub(crate) fn use_setup_project_name(runtime: Rc<Runtime>) -> SetupProjectNameEdit {
    use boardstudio_web_runtime::operation_outcomes::{EditOutcome, Settlement};
    let accepted = runtime.model().accepted;
    let owner = accepted
        .as_ref()
        .map(|snapshot| (snapshot.session_epoch, snapshot.document.id.clone()));
    let accepted_name = accepted
        .as_ref()
        .map(|snapshot| (snapshot.document.id.clone(), snapshot.document.name.clone()));
    let mut draft = use_signal(|| accepted_name.clone());
    let mut dirty = use_signal(|| false);
    let mut failure = use_signal(|| None::<String>);
    let mut tickets =
        use_signal(Vec::<((boardstudio_application::SessionEpoch, String), EditOutcome)>::new);
    let mut latest = use_signal(|| None::<boardstudio_application::OperationId>);
    let mut previous_owner = use_signal(|| owner.clone());
    let mut previous_name = use_signal(|| accepted_name.clone());
    let version = use_context::<Signal<u64>>()();
    let operations = tickets
        .read()
        .iter()
        .map(|(_, ticket)| ticket.operation())
        .collect::<Vec<_>>();
    use_effect(use_reactive(
        (&owner, &accepted_name, &version, &operations),
        move |(owner, accepted_name, _, _)| {
            let owner_changed = *previous_owner.peek() != owner;
            let name_changed = *previous_name.peek() != accepted_name;
            previous_owner.set(owner.clone());
            previous_name.set(accepted_name.clone());
            let mut pending = tickets.peek().clone();
            let had_tickets = !pending.is_empty();
            pending.retain(|(ticket_owner, ticket)| {
                match ticket.settlement(owner.as_ref() == Some(ticket_owner)) {
                    Settlement::Pending => true,
                    settlement => {
                        if *latest.peek() == Some(ticket.operation()) {
                            failure.set(match settlement {
                                Settlement::Failed { message } => Some(message),
                                _ => None,
                            });
                        }
                        false
                    }
                }
            });
            if owner_changed
                || (pending.is_empty() && !*dirty.peek() && (name_changed || had_tickets))
            {
                draft.set(accepted_name);
                dirty.set(false);
            }
            if owner_changed {
                failure.set(None);
            }
            if pending.len() != tickets.peek().len() {
                tickets.set(pending);
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
        let Some(current) = runtime.model().accepted else {
            return;
        };
        if current.session_epoch != *epoch || current.document.id != *id {
            return;
        }
        let proposed = draft()
            .filter(|(project_id, _)| project_id == id)
            .map(|(_, name)| name.trim().to_owned())
            .unwrap_or_default();
        if proposed.is_empty() {
            draft.set(Some((id.clone(), current.document.name.clone())));
            return;
        }
        let resolver =
            boardstudio_application::EditResolver::rename_project("setup-project-name", proposed);
        let ticket =
            runtime.resolve_edit("setup-project-name", Some("project name".into()), resolver);
        latest.set(Some(ticket.operation()));
        failure.set(None);
        tickets.write().push(((*epoch, id.clone()), ticket));
    });
    SetupProjectNameEdit {
        draft,
        failure,
        on_change,
        on_commit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::ProjectDoc;
    use boardstudio_web_test_runtime::project_name_test_support as support;
    use boardstudio_web_ui_shared::{
        test_mount::MountedFixture, test_role::by_role_name, test_wait::wait_until,
    };
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn host() -> Element {
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        let edit = use_setup_project_name(use_context::<Rc<Runtime>>());
        rsx! {
            button {
                onclick: move |_| {
                    edit.on_change.call("  ".into());
                    edit.on_commit.call(());
                },
                "Commit blank"
            }
            button {
                onclick: move |_| {
                    edit.on_change.call("  Setup name  ".into());
                    edit.on_commit.call(());
                },
                "Commit name"
            }
        }
    }

    #[wasm_bindgen_test]
    async fn setup_project_name_trims_and_preserves_undo() {
        let runtime = support::new_runtime();
        support::open_document(&runtime, ProjectDoc::empty("setup-name", "Original")).await;
        let initial = runtime.model().accepted.unwrap();
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        let _mounted = MountedFixture::new(root.clone(), dom);
        wait_until("setup name controls mount", || {
            by_role_name(&root, "button", "Commit blank").is_some()
        })
        .await;
        let click = |name| by_role_name(&root, "button", name).unwrap().click();
        click("Commit blank");
        support::run_pending(&runtime).await;
        assert_eq!(runtime.model().accepted.unwrap().token, initial.token);

        click("Commit name");
        support::run_pending(&runtime).await;
        let renamed = runtime.model().accepted.unwrap();
        assert_eq!(renamed.document.name, "Setup name");
        assert_eq!(renamed.document.revision, initial.document.revision + 1);

        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        let undone = runtime.model().accepted.unwrap();
        assert_eq!(undone.document.name, "Original");
        assert_eq!(undone.document.revision, renamed.document.revision + 1);
    }
}
