use super::*;
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::{TOUCH_CLS, TextField};

/// Place inside the existing Objects panel when Parts is the active workspace.
#[component]
pub fn PartsLibraryPanel(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    scope_generation: Signal<u64>,
    workspace: Signal<Workspace>,
    mut query: PartsQuery,
    mut selected: PartsSelection,
    on_select: EventHandler<()>,
) -> Element {
    let mut view_generation = use_signal(|| 0_u64);
    let mut generation = use_context::<PartsSelectionGeneration>().0;
    let mut preview_activation = use_context::<PartsPreviewActivation>().0;
    let mut assembly_selection = use_context::<PartsAssemblySelection>().0;
    let mut assembly_orientation = use_context::<PartsAssemblyOrientation>().0;
    let mut selection_generation = use_context::<PartsSelectionGeneration>().0;
    let catalogue = use_catalogue(&snapshot, &scope);
    let content = if let Some(entries) = catalogue.entries {
        let choices = group_choices(&entries);
        let listed_entries = catalogue_choices(&entries);
        let search = query().trim().to_lowercase();
        let groups = choices
            .iter()
            .map(|group| {
                let items = group
                    .entries
                    .iter()
                    .copied()
                    .filter(|entry| entry.matches(&search, group.label))
                    .collect::<Vec<_>>();
                (group, items)
            })
            .filter(|(_, items)| !items.is_empty())
            .collect::<Vec<_>>();
        let result_count = groups.iter().map(|(_, items)| items.len()).sum::<usize>();
        let selected_id = selected_definition_id(&listed_entries, &search, selected(), &scope);
        let visible_assemblies = matching_assembly_presets(&search);
        let matching_assembly = !visible_assemblies.is_empty();
        let no_matches = groups.is_empty() && !matching_assembly;

        rsx! {
            TextField {
                id: format!("parts-library-search-{}", dioxus::core::current_scope_id().0),
                label: "Search parts", plain_label: true, aria_label: "Search footprints",
                class: "", control_class: "max-[960px]:h-control-touch [@media(pointer:coarse)]:h-control-touch",
                aria_invalid: None, r#type: InputType::Search, placeholder: "Name or category", value: query(),
                oninput: move |event: FormEvent| {
                    query.set(event.value());
                    view_generation.set(view_generation() + 1);
                    generation.with_mut(|value| *value = value.wrapping_add(1));
                },
            }
            Collapsible { class: "mt-3 min-h-0 flex-none", default_open: true, keep_mounted: true,
                div { class: "flex items-center justify-between gap-2",
                    CollapsibleTrigger { aria_label: "Key assemblies", class: "flex-1 justify-start py-2 {TOUCH_CLS}", "Key assemblies" small { "{visible_assemblies.len()}" } }
                    AssemblyCreateButton {}
                }
                CollapsibleContent { class: "data-[open=false]:hidden",
                if assembly_selection().is_some() {
                    SelectField::<String> {
                        label: "Switch orientation", aria_label: "Switch orientation",
                        value: if assembly_orientation() == assembly_editor::SwitchOrientation::North { "north" } else { "south" },
                        options: vec![("south".into(), "South-facing LED".into()), ("north".into(), "North-facing LED".into())],
                        on_commit: move |value: String| {
                            assembly_orientation.set(if value == "north" { assembly_editor::SwitchOrientation::North } else { assembly_editor::SwitchOrientation::South });
                            selection_generation.with_mut(|value| *value = value.wrapping_add(1));
                        },
                    }
                    small { "Viewed from the keycap side; rotates the switch and attached parts together." }
                }
                div { role: "group", "aria-label": "Key assemblies",
                    ItemGroup {
                    for preset in visible_assemblies {
                        { let is_selected = assembly_selection() == Some(preset.id);
                          let preset_id = preset.id;
                          let definition_id = preset.definition_id.to_owned();
                          let scope = scope.clone();
                          rsx! { Item { size: ItemSize::Xs, class: "p-0",
                            Button { key: "{preset.name}", variant: ButtonVariant::Ghost,
                                class: "w-full justify-start whitespace-normal text-left aria-pressed:bg-selected aria-pressed:text-selected-foreground {TOUCH_CLS}",
                                aria_pressed: is_selected,
                                onclick: move |_| {
                                    assembly_selection.set(Some(preset_id));
                                    selected.set(Some((scope.clone(), definition_id.clone())));
                                    view_generation.set(view_generation() + 1);
                                    generation.with_mut(|value| *value = value.wrapping_add(1));
                                    preview_activation.with_mut(|value| *value = value.wrapping_add(1));
                                    on_select.call(());
                                },
                                "{preset.name}"
                            }
                          } }
                        }
                    }
                    }
                }
                }
            }
            if entries.is_empty() {
                div { role: "status", Callout { "No component definitions are available." } }
            } else if !groups.is_empty() {
                Collapsible { class: "mt-3 min-h-0 flex-none", default_open: true, keep_mounted: true,
                    CollapsibleTrigger { aria_label: "Components", class: "w-full justify-start py-2 {TOUCH_CLS}", "Components" small { "{result_count}" } }
                    CollapsibleContent { class: "data-[open=false]:hidden", div { role: "group", "aria-label": "Footprint library",
                        for (group, items) in groups {
                            PartsCatalogueCategory { key: "{group.label}", label: group.label,
                                reveal: !query().trim().is_empty() || items.iter().any(|entry| selected_id.as_deref() == Some(entry.definition.id.as_str())),
                                for entry in items {
                                    { let is_selected = assembly_selection().is_none() && selected_id.as_deref() == Some(entry.definition.id.as_str());
                                      let id = entry.definition.id.clone();
                                      let scope = scope.clone();
                                      rsx! {
                                        Item { size: ItemSize::Xs, class: "p-0",
                                        Button {
                                            key: "{entry.definition.id}", variant: ButtonVariant::Ghost,
                                            class: "w-full justify-start whitespace-normal text-left aria-pressed:bg-selected aria-pressed:text-selected-foreground {TOUCH_CLS}",
                                            aria_pressed: is_selected,
                                            onclick: move |_| {
                                                assembly_selection.set(None);
                                                selected.set(Some((scope.clone(), id.clone())));
                                                view_generation.set(view_generation() + 1);
                                                generation.with_mut(|value| *value = value.wrapping_add(1));
                                                preview_activation.with_mut(|value| *value = value.wrapping_add(1));
                                                on_select.call(());
                                            },
                                            "{preferred_label(&entry.definition)}"
                                        } }
                                      }
                                    }
                                }
                            }
                        }
                    } }
                }
            }
            if no_matches && !search.is_empty() {
                div { role: "status", Callout { "No parts match this search." } }
            }
        }
    } else if let Some(error) = catalogue.error {
        rsx! {
            Alert { variant: AlertVariant::Destructive, "Component catalogue could not be loaded: {error}" }
        }
    } else {
        rsx! {
            div { role: "status", Spinner {} "Loading component catalogue…" }
        }
    };

    rsx! {
        section { class: "flex flex-1 flex-col min-h-0 px-3 py-4 text-foreground [&>h2]:mt-0 [&>h2]:mb-4 [&>h2]:text-xl", "aria-label": "Parts library",
            crate::parts_view_generation::PartsViewGenerationOwner {
                workspace,
                view_generation,
            }
            h2 { "Parts library" }
            {content}
            div { class: "grid gap-1 pt-3 mt-3 border-t border-border",
                crate::presentation::parts_import_footprint::ImportKiCadFootprintAction {
                    scope: scope.clone(),
                    view_generation,
                    scope_generation,
                    workspace,
                    selected,
                    query,
                    on_select,
                }
                crate::parts_new_component::NewCustomComponentAction {
                    scope: scope.clone(),
                    view_generation,
                    scope_generation,
                    workspace,
                    query,
                    selected,
                    on_select,
                }
            }
        }
    }
}

/// Focused setup picker; uses the same catalogue and source-backed preview as Parts.
#[component]
pub fn ControllerSetupPicker(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    mut selected: PartsSelection,
    on_place: EventHandler<String>,
    busy: bool,
    enabled: bool,
    error: Option<String>,
) -> Element {
    let catalogue = use_catalogue(&snapshot, &scope);
    let mut generation = use_context::<PartsSelectionGeneration>().0;
    let mut activation = use_context::<PartsPreviewActivation>().0;
    let mut assembly = use_context::<PartsAssemblySelection>().0;
    let mut search = use_signal(String::new);
    let loaded = catalogue.entries.is_some();
    let entries = catalogue.entries.unwrap_or_default();
    let controllers = catalogue_choices(&entries)
        .into_iter()
        .filter(|entry| entry.definition.kind == boardstudio_model::model::PartKind::Controller)
        .collect::<Vec<_>>();
    let selection = selected().filter(|(owner, _)| owner == &scope);
    let needle = search().trim().to_lowercase();
    let matches = controllers
        .iter()
        .copied()
        .filter(|entry| {
            preferred_label(&entry.definition)
                .to_lowercase()
                .contains(&needle)
        })
        .collect::<Vec<_>>();
    let chosen = matches
        .iter()
        .copied()
        .find(|entry| {
            selection
                .as_ref()
                .is_some_and(|(_, id)| id == &entry.definition.id)
        })
        .or_else(|| matches.first().copied());
    let chosen_id = chosen.map(|entry| entry.definition.id.clone());
    let position = matches
        .iter()
        .position(|entry| Some(&entry.definition.id) == chosen_id.as_ref())
        .unwrap_or(0);
    let previous = (!matches.is_empty()).then(|| {
        matches[(position + matches.len() - 1) % matches.len()]
            .definition
            .id
            .clone()
    });
    let next = (!matches.is_empty()).then(|| {
        matches[(position + 1) % matches.len()]
            .definition
            .id
            .clone()
    });
    let owner = scope.clone();
    let choose = EventHandler::new(move |id: String| {
        selected.set(Some((owner.clone(), id)));
        assembly.set(None);
        generation.with_mut(|value| *value = value.wrapping_add(1));
        activation.with_mut(|value| *value = value.wrapping_add(1));
    });
    use_effect(use_reactive((&loaded, &needle, &chosen_id), move |_| {
        let _ = document::eval(
            r#"
            const rail = document.querySelector('[role="group"][aria-label="Controllers"]');
            const card = rail?.querySelector('[aria-pressed="true"]');
            if (rail && card) rail.scrollTo({left: card.offsetLeft - rail.offsetLeft - (rail.clientWidth - card.offsetWidth) / 2,
                behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'instant' : 'smooth'});
        "#,
        );
    }));
    let navigation_disabled = matches.len() < 2;
    rsx! {
        section { class: "grid grid-cols-1 gap-3 min-w-0", aria_label: "Controller selection", aria_roledescription: "carousel",
            TextField {
                id: format!("parts-controller-search-{}", dioxus::core::current_scope_id().0),
                label: "Search controllers", plain_label: true, aria_label: "Search controllers",
                class: "", control_class: "max-[960px]:h-control-touch [@media(pointer:coarse)]:h-control-touch",
                aria_invalid: None, r#type: InputType::Search, placeholder: "Find a controller…", value: search(),
                oninput: move |event: FormEvent| search.set(event.value()),
            }
            div { class: "flex items-center gap-1 text-muted-foreground text-xs [&>span]:flex-1 [&>span]:tabular-nums [&_svg]:size-5 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]",
                span { role: "status", aria_live: "polite", if matches.is_empty() { "0 controllers" } else { "{position + 1} of {matches.len()}" } }
                Tooltip {
                    TooltipTrigger { as: move |attributes| { let previous = previous.clone(); rsx! {
                        Button { attributes, variant: ButtonVariant::Secondary, size: ButtonSize::IconSm,
                            class: "max-[960px]:h-control-touch max-[960px]:min-w-control-touch [@media(pointer:coarse)]:h-control-touch [@media(pointer:coarse)]:min-w-control-touch",
                            aria_label: "Previous controller", disabled: navigation_disabled,
                            onclick: move |_| { if let Some(id) = previous.clone() { choose.call(id); } },
                            boardstudio_web_ui_shared::ui::icons::ChevronDown { class: "rotate-90" }
                        }
                    } } }
                    TooltipContent { "Previous controller" }
                }
                Tooltip {
                    TooltipTrigger { as: move |attributes| { let next = next.clone(); rsx! {
                        Button { attributes, variant: ButtonVariant::Secondary, size: ButtonSize::IconSm,
                            class: "max-[960px]:h-control-touch max-[960px]:min-w-control-touch [@media(pointer:coarse)]:h-control-touch [@media(pointer:coarse)]:min-w-control-touch",
                            aria_label: "Next controller", disabled: navigation_disabled,
                            onclick: move |_| { if let Some(id) = next.clone() { choose.call(id); } },
                            boardstudio_web_ui_shared::ui::icons::ChevronDown { class: "-rotate-90" }
                        }
                    } } }
                    TooltipContent { "Next controller" }
                }
            }
            div { class: "relative flex flex-nowrap overflow-x-auto snap-x snap-mandatory [scrollbar-width:thin] pt-1 pb-3", role: "group", aria_label: "Controllers",
                for entry in &controllers {
                    {
                        let id = entry.definition.id.clone();
                        let active = chosen_id.as_ref() == Some(&id);
                        let matching = preferred_label(&entry.definition).to_lowercase().contains(&needle);
                        rsx! { Button {
                            key: "{id}", class: "flex h-auto flex-[0_0_192px] flex-col items-stretch gap-3 max-w-[192px] min-w-0 mr-2 p-3 rounded-full bg-secondary text-center snap-center overflow-hidden transition-all motion-reduce:transition-none data-[match=false]:basis-0 data-[match=false]:max-w-0 data-[match=false]:px-0 data-[match=false]:mr-0 data-[match=false]:opacity-0 data-[match=false]:invisible aria-pressed:bg-selected aria-pressed:text-selected-foreground [&_strong]:block [&_strong]:min-w-[168px] [&_strong]:text-[11px] [&_strong]:font-semibold [&_strong]:leading-[1.4]", variant: ButtonVariant::Ghost, aria_pressed: active,
                            attributes: {
                                let mut attributes = vec![Attribute::new("data-match", matching, None, false)];
                                if !matching { attributes.push(Attribute::new("aria-hidden", "true", None, false)); }
                                attributes
                            },
                            disabled: !matching,
                            onclick: move |_| choose.call(id.clone()),
                            preview::ControllerFootprintThumbnail { definition: entry.definition.clone() }
                            strong { "{preferred_label(&entry.definition)}" }
                        } }
                    }
                }
            }
            if let Some(entry) = chosen {
                {
                    let id = entry.definition.id.clone();
                    rsx! { Button {
                        class: TOUCH_CLS,
                        disabled: busy || !enabled, onclick: move |_| { choose.call(id.clone()); on_place.call(id.clone()); },
                        if busy { "Preparing controller…" } else { "Place controller" }
                    } }
                }
            } else {
                if !loaded {
                    if let Some(error) = catalogue.error { Alert { variant: AlertVariant::Destructive, "{error}" } }
                    else { div { role: "status", Spinner {} "Loading controllers…" } }
                } else { div { role: "status", Callout { "No controllers match this search." } } }
            }
            if let Some(error) = error { Alert { variant: AlertVariant::Destructive, "{error}" } }
        }
    }
}
