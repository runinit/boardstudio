use super::*;

pub(crate) struct SetupProjectNameEdit {
    pub(crate) draft: Signal<Option<(String, String)>>,
    pub(crate) failure: Signal<Option<String>>,
    pub(crate) on_change: Callback<String>,
    pub(crate) on_commit: Callback<()>,
}

pub(crate) fn use_setup_project_name(runtime: Rc<Runtime>) -> SetupProjectNameEdit {
    use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
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
        use_signal(Vec::<((boardstudio_application::SessionEpoch, String), EditTicket)>::new);
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
        let resolver = boardstudio_application::EditResolver::new(
            "setup-project-name",
            move |accepted: &boardstudio_application::AcceptedSnapshot| {
                if accepted.document.name == proposed {
                    return boardstudio_application::Resolution::Unchanged;
                }
                let mut document = accepted.document.as_ref().clone();
                document.name = proposed.clone();
                boardstudio_application::Resolution::submit(
                    vec![document.id.clone()],
                    EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                )
            },
        );
        let ticket = EditTicket::begin(
            &runtime,
            "setup-project-name",
            Some("project name".into()),
            resolver,
        );
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
