//! Header presentation of Library operations. Drafts and the project browser stay in Library.
use super::*;
use boardstudio_web_ui_shared::ui::command::{Command, use_commands};
use boardstudio_web_ui_shared::ui::dropdown_menu::{
    DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger,
};
use boardstudio_web_ui_shared::ui::icons::ChevronDown;

/// Shell wiring only. Library remains the sole owner of drafts and tool presentation.
pub fn use_project_tools() {
    let request = ProjectToolRequest(use_signal(|| None));
    use_context_provider(|| request);
}

#[component]
pub fn ProjectHeaderMenu(
    project_name: String,
    on_undo: EventHandler<()>,
    on_redo: EventHandler<()>,
    on_export: EventHandler<()>,
    on_settings: EventHandler<()>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let mut browser = use_context::<ProjectBrowserState>().0;
    let pending_new = use_context::<Signal<Option<PendingNewKeyboard>>>();
    let new_error = use_context::<Signal<String>>();
    let mut guide = use_context::<Signal<Option<SetupGuideRequest>>>();
    let mut guide_counter = use_signal(|| 0_u64);
    let mut file_input = use_signal(|| None::<HtmlInputElement>);
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let current = runtime.model().accepted;
    let has_current = current.is_some();
    let project_id = current.map(|snapshot| snapshot.document.id.clone());
    let create = runtime.clone();
    let copy = runtime.clone();
    let import = runtime.clone();
    let tool_request = try_use_context::<ProjectToolRequest>();
    let new_project = use_callback(move |()| {
        start_new_keyboard(
            create.clone(),
            Some(ProjectBrowserState(browser)),
            pending_new,
            new_error,
        )
    });
    let open_project = use_callback(move |()| {
        if let Some(input) = file_input() {
            input.click();
        }
    });
    let save_copy = use_callback(move |()| copy.export_project_copy());
    let setup_id = project_id.clone();
    let setup = use_callback(move |()| {
        if let Some(project_id) = &setup_id {
            guide_counter += 1;
            guide.set(Some(SetupGuideRequest {
                project_id: project_id.clone(),
                request_id: format!("{}-guide-{}", project_id, guide_counter()),
                start_at_project: false,
            }));
            browser.set(false);
        }
    });
    let rename_project = use_memo(use_reactive((&project_id,), {
        let rename_runtime = runtime.clone();
        move |(rename_id,)| {
            let rename_runtime = rename_runtime.clone();
            EventHandler::new(move |()| {
                if let Some(mut request) = tool_request
                    && let Some(id) = &rename_id
                    && rename_runtime
                        .model()
                        .accepted
                        .as_ref()
                        .is_some_and(|s| s.document.id == *id)
                {
                    request.0.set(Some(ProjectTool::Rename(id.clone())));
                    browser.set(true);
                }
            })
        }
    }));
    let open_demo = use_callback(move |()| {
        if let Some(mut request) = tool_request {
            request.0.set(Some(ProjectTool::Demo));
            browser.set(true);
        }
    });
    let mut commands = vec![
        Command {
            group: "Project",
            label: "New project".into(),
            reason: pending_new()
                .is_some()
                .then(|| "A new keyboard is being created.".into()),
            hint: None,
            on_select: new_project,
        },
        Command {
            group: "Project",
            label: "Open project…".into(),
            reason: None,
            hint: None,
            on_select: open_project,
        },
        Command {
            group: "Project",
            label: "Save project copy…".into(),
            reason: (!has_current).then(|| "Open a project first.".into()),
            hint: None,
            on_select: save_copy,
        },
        Command {
            group: "Go to",
            label: "Setup guide".into(),
            reason: (!has_current).then(|| "Open a project first.".into()),
            hint: None,
            on_select: setup,
        },
        Command {
            group: "Project",
            label: "Workspace settings".into(),
            reason: None,
            hint: None,
            on_select: on_settings,
        },
    ];
    if tool_request.is_some() {
        commands.push(Command {
            group: "Project",
            label: "Rename project".into(),
            reason: (!has_current).then(|| "Open a project first.".into()),
            hint: None,
            on_select: rename_project(),
        });
        commands.push(Command {
            group: "Project",
            label: "Open demo".into(),
            reason: None,
            hint: None,
            on_select: open_demo,
        });
    }
    use_commands(commands);
    rsx! {
        DropdownMenu {
            DropdownMenuTrigger { class: "header-project-trigger", aria_label: "Project",
                span { class: "header-project-name", title: "{project_name}", "{project_name}" }
                ChevronDown {}
            }
            DropdownMenuContent { aria_label: "Project",
                DropdownMenuItem::<()> { value: (), index: 0usize, disabled: pending_new().is_some(),
                    on_select: move |_| new_project.call(()),
                    "New project"
                }
                DropdownMenuItem::<()> { value: (), index: 1usize, on_select: move |_| open_project.call(()), "Open project…" }
                DropdownMenuItem::<()> { value: (), index: 2usize, disabled: !has_current,
                    aria_label: "Save project copy…", on_select: move |_| save_copy.call(()), "Save project copy…" }
                DropdownMenuItem::<()> { value: (), index: 3usize, on_select: move |_| browser.set(true), "Browse projects" }
                if browser() && has_current {
                    DropdownMenuItem::<()> { value: (), index: 4usize, on_select: move |_| browser.set(false), "Return to editor" }
                }
                DropdownMenuItem::<()> { value: (), index: 5usize, disabled: !has_current, on_select: move |_| setup.call(()), "Setup guide" }
                DropdownMenuSeparator {}
                DropdownMenuItem::<()> { value: (), index: 6usize, disabled: !has_current, on_select: move |_| on_undo.call(()), "Undo" }
                DropdownMenuItem::<()> { value: (), index: 7usize, disabled: !has_current, on_select: move |_| on_redo.call(()), "Redo" }
                DropdownMenuItem::<()> { value: (), index: 8usize, on_select: move |_| on_export.call(()), "Export" }
                DropdownMenuItem::<()> { value: (), index: 9usize, on_select: move |_| on_settings.call(()), "Workspace settings" }
            }
        }
        input { id: "header-open-project", r#type: "file", hidden: true, accept: ".boardstudio",
            onmounted: move |event: MountedEvent| {
                file_input.set(event.data().downcast::<web_sys::Element>().and_then(|e| e.dyn_ref::<HtmlInputElement>()).cloned());
            },
            onchange: move |event: FormEvent| {
                let Some(input) = event.data().try_as_web_event().and_then(|e| e.target()).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) else { return; };
                let Some(file) = input.files().and_then(|files| files.get(0)) else { return; };
                import.import_file(file);
                browser.set(false);
                input.set_value("");
            }
        }
    }
}
