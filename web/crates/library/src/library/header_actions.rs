//! Header presentation of Library operations.
use super::*;
use boardstudio_web_ui_shared::ui::command::{Command, use_commands};
use boardstudio_web_ui_shared::ui::dropdown_menu::{
    DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger,
};
use boardstudio_web_ui_shared::ui::icons::ChevronDown;

/// Shell wiring only. Library remains the sole owner of tool presentation.
pub fn use_project_tools() {
    use_context_provider(|| ProjectToolRequest(use_signal(|| None)));
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
    let _ = use_context::<Signal<u64>>()();
    let project_id = runtime.model().accepted.map(|s| s.document.id.clone());
    let has_current = project_id.is_some();
    let tool_request = try_use_context::<ProjectToolRequest>();
    let (create, copy, import) = (runtime.clone(), runtime.clone(), runtime.clone());
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
        if let Some(id) = &setup_id {
            guide_counter += 1;
            guide.set(Some(SetupGuideRequest {
                project_id: id.clone(),
                request_id: format!("{id}-guide-{}", guide_counter()),
                start_at_project: false,
            }));
            browser.set(false);
        }
    });
    let rename_rt = runtime.clone();
    let rename_id = project_id.clone();
    let rename_project = EventHandler::new(move |()| {
        if let (Some(mut req), Some(id)) = (tool_request, &rename_id)
            && rename_rt
                .model()
                .accepted
                .as_ref()
                .is_some_and(|s| s.document.id == *id)
        {
            req.0.set(Some(ProjectTool::Rename(id.clone())));
            browser.set(true);
        }
    });
    let open_demo = use_callback(move |()| {
        if let Some(mut req) = tool_request {
            req.0.set(Some(ProjectTool::Demo));
            browser.set(true);
        }
    });
    let no_proj = (!has_current).then(|| "Open a project first.".into());
    let cmd = |group, label: &str, reason, on_select| Command {
        group,
        label: label.into(),
        reason,
        hint: None,
        on_select,
    };
    let mut commands = vec![
        cmd(
            "Project",
            "New project",
            pending_new()
                .is_some()
                .then(|| "A new keyboard is being created.".into()),
            new_project,
        ),
        cmd("Project", "Open project…", None, open_project),
        cmd("Project", "Save project copy…", no_proj.clone(), save_copy),
        cmd("Go to", "Setup guide", no_proj.clone(), setup),
        cmd("Project", "Workspace settings", None, on_settings),
    ];
    if tool_request.is_some() {
        commands.push(cmd("Project", "Rename project", no_proj, rename_project));
        commands.push(cmd("Project", "Open demo", None, open_demo));
    }
    use_commands(commands);
    rsx! {
        DropdownMenu {
            DropdownMenuTrigger { class: "header-project-trigger", aria_label: "Project",
                span { class: "header-project-name", title: "{project_name}", "{project_name}" }
                ChevronDown {}
            }
            DropdownMenuContent { aria_label: "Project",
                DropdownMenuItem::<()> { value: (), index: 0usize, disabled: pending_new().is_some(), on_select: move |_| new_project.call(()), "New project" }
                DropdownMenuItem::<()> { value: (), index: 1usize, on_select: move |_| open_project.call(()), "Open project…" }
                DropdownMenuItem::<()> { value: (), index: 2usize, disabled: !has_current, aria_label: "Save project copy…", on_select: move |_| save_copy.call(()), "Save project copy…" }
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
            onmounted: move |e: MountedEvent| file_input.set(e.data().downcast::<web_sys::Element>().and_then(|el| el.dyn_ref::<HtmlInputElement>()).cloned()),
            onchange: move |e: FormEvent| {
                if let Some(input) = e.data().try_as_web_event().and_then(|ev| ev.target()).and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
                    && let Some(file) = input.files().and_then(|f| f.get(0))
                {
                    import.import_file(file);
                    browser.set(false);
                    input.set_value("");
                }
            }
        }
    }
}
