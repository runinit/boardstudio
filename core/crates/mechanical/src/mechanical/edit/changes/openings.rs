//! Access-opening and opening-vertex changes.
/// Applies one openings change. Expands in [`super::apply_change`] scope and uses its
/// `configuration`, `change`, `document`, `board_id`, `identity_floor` and
/// `changed` bindings; exactly one arm runs because domains partition variants.
macro_rules! opening_arms {
    ($configuration:expr, $change:expr, $document:expr, $board_id:expr, $identity_floor:expr, $changed:expr) => {
        match $change {
            MechanicalChange::AddOpening => {
                let id = reserve_opening_identity($configuration, $identity_floor.opening)?;
                let mut opening = MechanicalOpening {
                    id: id.clone(),
                    vertices: Vec::new(),
                    z: 0.0,
                    height: 10.0,
                    next_vertex_identity: 0,
                };
                for point in [
                    Vec2 { x: -2.5, y: -2.5 },
                    Vec2 { x: 2.5, y: -2.5 },
                    Vec2 { x: 2.5, y: 2.5 },
                    Vec2 { x: -2.5, y: 2.5 },
                ] {
                    let vertex_id = reserve_vertex_identity(&mut opening, $identity_floor.vertex)?;
                    opening.vertices.push(MechanicalOpeningVertex {
                        id: vertex_id,
                        point,
                    });
                }
                $configuration
                    .openings
                    .get_or_insert_with(Vec::new)
                    .push(opening);
                $changed.push(id);
            }
            MechanicalChange::RemoveOpening { opening_id } => {
                let removed = remove_item(
                    $configuration.openings.as_mut(),
                    |item| item.id == *opening_id,
                    OPENING_UNAVAILABLE,
                )?;
                $changed.push(removed.id);
            }
            MechanicalChange::AddOpeningVertex { opening_id } => {
                let openings = $configuration.openings.as_deref_mut().unwrap_or_default();
                let opening = opening_mut(openings, opening_id)?;
                // Reserved from the opening's own monotonic counter, so a removed vertex
                // id can never be handed out again within that opening.
                let vertex_id = reserve_vertex_identity(opening, $identity_floor.vertex)?;
                opening.vertices.push(MechanicalOpeningVertex {
                    id: vertex_id.clone(),
                    point: Vec2 { x: 0.0, y: 0.0 },
                });
                let opening_id = opening.id.clone();
                validate_openings(openings)?;
                $changed.push(opening_id);
                $changed.push(vertex_id);
            }
            MechanicalChange::RemoveOpeningVertex {
                opening_id,
                vertex_id,
            } => {
                let openings = $configuration.openings.as_deref_mut().unwrap_or_default();
                let opening = opening_mut(openings, opening_id)?;
                let matches = |vertex: &MechanicalOpeningVertex| vertex.id == *vertex_id;
                if opening.vertices.len() <= 3 && opening.vertices.iter().any(matches) {
                    return Err("An opening footprint needs at least three vertices.".into());
                }
                let removed =
                    remove_item(Some(&mut opening.vertices), matches, VERTEX_UNAVAILABLE)?;
                let opening_id = opening.id.clone();
                validate_openings(openings)?;
                $changed.extend([opening_id, removed.id]);
            }
            MechanicalChange::SetOpeningDimension {
                opening_id,
                vertex_id,
                field,
                value,
            } => {
                if !matches!(
                    (field, vertex_id),
                    (
                        MechanicalDimension::OpeningBottomZ | MechanicalDimension::OpeningHeight,
                        None
                    ) | (
                        MechanicalDimension::OpeningPointX | MechanicalDimension::OpeningPointY,
                        Some(_)
                    )
                ) {
                    return Err("The selected opening field is unavailable.".into());
                }
                validate_dimension(*field, *value)?;
                let opening = opening_mut(
                    $configuration.openings.as_deref_mut().unwrap_or_default(),
                    opening_id,
                )?;
                if let Some(vertex_id) = vertex_id {
                    let vertex = find_item_mut(
                        &mut opening.vertices,
                        |vertex| vertex.id == *vertex_id,
                        VERTEX_UNAVAILABLE,
                    )?;
                    if *field == MechanicalDimension::OpeningPointX {
                        vertex.point.x = *value;
                    } else {
                        vertex.point.y = *value;
                    }
                    $changed.extend([opening_id.clone(), vertex_id.clone()]);
                } else {
                    if *field == MechanicalDimension::OpeningBottomZ {
                        opening.z = *value;
                    } else {
                        opening.height = *value;
                    }
                    $changed.push(opening_id.clone());
                }
            }
            _ => {}
        }
    };
}
