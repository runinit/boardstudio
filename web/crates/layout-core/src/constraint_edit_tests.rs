use crate::{
    component_offset::LayoutConstraintValues,
    layout_component_edits::{
        constraint_resolver, offset_axes_resolver, remove_constraint_resolver,
    },
};
use boardstudio_application::Event;
use boardstudio_core::model::{
    Constraint, ConstraintEndpoint, ConstraintRelation, OffsetAxes, Vec2,
};
use boardstudio_web_runtime::{operation_outcomes::Settlement, runtime::Runtime};
use boardstudio_web_test_runtime::{
    layout_fixtures::{part_position, position_edit_document},
    new_native_runtime,
};
use std::rc::Rc;

fn open() -> Rc<Runtime> {
    let runtime = new_native_runtime();
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: position_edit_document(),
    });
    runtime
}

#[test]
fn typed_constraint_update_preserves_id_and_endpoints_for_every_relation_then_removes() {
    let runtime = open();
    let values = [
        ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 17.0,
        },
        ConstraintRelation::PartOffset {
            offset: Vec2 { x: -3.0, y: 4.0 },
            rotation: -11.0,
        },
        ConstraintRelation::MirrorLine {
            at: Vec2 { x: 12.0, y: -9.0 },
            angle: 31.0,
        },
    ];

    for (index, relation) in values.into_iter().enumerate() {
        let ticket = runtime.resolve_edit(
            "layout-inspector-constraint",
            Some("layout".into()),
            constraint_resolver(
                "b".into(),
                "board".into(),
                "a".into(),
                LayoutConstraintValues::Framed {
                    relation: relation.clone(),
                },
                if index == 0 { 9 } else { 77 },
            ),
        );
        assert!(matches!(ticket.settlement(true), Settlement::Landed { .. }));

        let accepted = runtime.model().accepted.unwrap();
        assert!(
            matches!(
                accepted.document.constraints.as_slice(),
                [Constraint::Framed { id, source: ConstraintEndpoint::Part { part_id: source }, target: ConstraintEndpoint::Part { part_id: target }, relation: saved }]
                    if id == "layout-component-constraint-9"
                        && source == "a"
                        && target == "b"
                        && saved == &relation
            ),
            "typed update preserves the durable id, endpoint frames and exact relation: {:?}",
            accepted.document.constraints
        );
    }

    let remove = runtime.resolve_edit(
        "layout-inspector-remove-constraint",
        Some("layout".into()),
        remove_constraint_resolver("b".into(), "layout-component-constraint-9".into()),
    );
    assert!(matches!(remove.settlement(true), Settlement::Landed { .. }));
    assert!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .constraints
            .is_empty(),
        "the same stable record can be removed through the facade-backed resolver"
    );
}

#[test]
fn converting_a_constraint_between_axes_leaves_the_target_where_it_was() {
    let runtime = open();
    let set = runtime.resolve_edit(
        "layout-inspector-constraint",
        Some("layout".into()),
        constraint_resolver(
            "b".into(),
            "board".into(),
            "a".into(),
            LayoutConstraintValues::Offset {
                offset: Vec2 { x: 20.0, y: 0.0 },
                rotation: 0.0,
            },
            9,
        ),
    );
    assert!(matches!(set.settlement(true), Settlement::Landed { .. }));
    let target_before = part_position(&runtime, "b");
    let convert = runtime.resolve_edit(
        "layout-inspector-offset-axes",
        Some("layout".into()),
        offset_axes_resolver(
            "b".into(),
            "layout-component-constraint-9".into(),
            OffsetAxes::Part,
        ),
    );
    assert!(matches!(
        convert.settlement(true),
        Settlement::Landed { .. }
    ));
    assert_eq!(part_position(&runtime, "b"), target_before);
    let document = runtime.model().accepted.unwrap().document;
    assert!(
        matches!(
            document.constraints.as_slice(),
            [Constraint::PartOffset { id, .. }] if id == "layout-component-constraint-9"
        ),
        "the constraint did not become part-offset with its id: {:?}",
        document.constraints
    );
}
