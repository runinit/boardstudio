use super::*;

pub(super) fn layout_assignment(
    projection: &LayoutComponentInspectorProjection,
    layout_choice: Memo<Option<String>>,
    latest_capture: Signal<LayoutComponentInspectorOwner>,
    on_action: EventHandler<LayoutComponentInspectorAction>,
) -> Element {
    let choices = std::iter::once((String::new(), "Board / ungrouped".to_owned()))
        .chain(
            projection
                .layouts
                .iter()
                .map(|layout| (layout.id.to_string(), layout.name.to_string())),
        )
        .collect::<Vec<_>>();
    rsx! {
        SelectField::<String> {
            id: format!("layout-choice-{}-Component-layout", dioxus::core::current_scope_id().0),
            label: "Layout",
            aria_label: "Component layout",
            value: layout_choice().unwrap_or_default(),
            options: choices,
            on_commit: move |choice: String| {
                on_action.call(LayoutComponentInspectorAction::AssignLayout {
                    owner: latest_capture(),
                    layout_id: (!choice.is_empty()).then_some(choice),
                })
            },
        }
    }
}

pub(super) fn position_fields(
    projection: &LayoutComponentInspectorProjection,
    commit: impl FnMut(ComponentPositionAxis, f64) + Copy + 'static,
) -> Element {
    let position = projection.position;
    let field = |axis| {
        let (id_suffix, label, aria_label, value) = match axis {
            ComponentPositionAxis::X => (10, "X (mm)", "X mm", position.x),
            ComponentPositionAxis::Y => (9, "Y (mm)", "Y mm", position.y),
        };
        let id = format!(
            "layout-input-{}-{id_suffix}",
            dioxus::core::current_scope_id().0
        );
        let mut commit = commit;
        rsx! {
            NumericField {
                id,
                label,
                aria_label,
                value: format!("{value:.2}").parse::<f64>().unwrap_or(value),
                precision: 2,
                step: Some(0.1),
                disabled_reason: position_unavailable(projection).then(|| "Position is unavailable".to_owned()),
                on_commit: move |value| commit(axis, value),
            }
        }
    };
    rsx! {
        h3 { class: "flex items-baseline justify-between text-[13px] [&_small]:text-[11px] [&_small]:font-normal [&_small]:text-muted-foreground",
            "Position"
            small { "millimetres" }
        }
        div { class: "grid grid-cols-2 gap-2",
            {field(ComponentPositionAxis::X)}
            {field(ComponentPositionAxis::Y)}
        }
    }
}
