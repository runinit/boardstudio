//! Product occurrence links and their source-authored frame placements.
use super::*;
use scene::{PlacementPath, PlacementStep, TracedInstance};
use std::collections::BTreeMap;

pub(super) fn occurrences(
    records: &SourceRecords<'_>,
    scene: &scene::Scene<'_>,
    config_control: bool,
) -> Result<Vec<TracedInstance>, String> {
    let mut nodes = BTreeMap::new();
    let mut shape_definitions = HashMap::new();
    let mut usages = BTreeMap::new();
    let mut context_links = HashMap::new();
    for parts in records.values() {
        for part in parts {
            match part.name.as_str() {
                "SHAPE_DEFINITION_REPRESENTATION" => {
                    let params = parameters(part, 2)?;
                    insert_unique(
                        &mut shape_definitions,
                        reference(&params[0])?,
                        reference(&params[1])?,
                    )?;
                }
                "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION" => {
                    let params = parameters(part, 2)?;
                    insert_unique(
                        &mut context_links,
                        reference(&params[1])?,
                        reference(&params[0])?,
                    )?;
                }
                _ => {}
            }
        }
    }
    for (&id, parts) in records {
        if !parts
            .iter()
            .any(|part| part.name == "PRODUCT_DEFINITION_SHAPE")
        {
            continue;
        }
        let shape = parameters(named_record(records, id, "PRODUCT_DEFINITION_SHAPE")?, 3)?;
        let definition = reference(&shape[2])?;
        let record = simple_record(records, definition)?;
        if record.name == "PRODUCT_DEFINITION" {
            validate_product(records, record, config_control)?;
            let representation = *shape_definitions
                .get(&id)
                .ok_or("product lacks shape definition representation")?;
            scene.factor(representation)?;
            if nodes.insert(definition, representation).is_some() {
                return Err("duplicate product shape".into());
            }
        } else if record.name == "NEXT_ASSEMBLY_USAGE_OCCURRENCE"
            && usages.insert(definition, id).is_none()
        {
            continue;
        } else {
            return Err("unsupported or duplicate product definition shape".into());
        }
    }

    let source_usage_ids: HashSet<_> = records
        .iter()
        .filter_map(|(&id, parts)| {
            parts
                .iter()
                .any(|part| part.name == "NEXT_ASSEMBLY_USAGE_OCCURRENCE")
                .then_some(id)
        })
        .collect();
    if source_usage_ids.len() != usages.len() || context_links.len() != usages.len() {
        return Err("incomplete native assembly links".into());
    }

    let mut edges = HashMap::<u64, Vec<(u64, Matrix4, PlacementStep)>>::new();
    let mut children = HashSet::new();
    let mut used_relationships = HashSet::new();
    for (&usage_id, &shape) in &usages {
        let usage = named_parameters(records, usage_id, "NEXT_ASSEMBLY_USAGE_OCCURRENCE", 6)?;
        let parent = reference(&usage[3])?;
        let child = reference(&usage[4])?;
        let parent_rep = *nodes
            .get(&parent)
            .ok_or("unresolved native assembly parent")?;
        let child_rep = *nodes
            .get(&child)
            .ok_or("unresolved native assembly child")?;
        let relationship = *context_links
            .get(&shape)
            .ok_or("native occurrence lacks context link")?;
        if !used_relationships.insert(relationship) {
            return Err("native assembly relationship is reused".into());
        }
        let (placement, source) = placement(records, scene, relationship, child_rep, parent_rep)?;
        edges
            .entry(parent)
            .or_default()
            .push((child, placement, source));
        children.insert(child);
    }
    if records
        .values()
        .filter(|parts| {
            parts
                .iter()
                .any(|p| p.name == "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION")
        })
        .count()
        != used_relationships.len()
    {
        return Err("unreferenced native placement relationship".into());
    }
    let mut result = Vec::new();
    let mut budget = OccurrenceBudget::default();
    if nodes.is_empty() {
        for root in scene.roots()? {
            result.extend(scene.traced_instances(
                root,
                Matrix4::identity(),
                Some(PlacementPath::default()),
                &mut budget,
            )?);
        }
    } else {
        let mut seen = HashSet::new();
        let mut pending: Vec<_> = nodes
            .keys()
            .rev()
            .filter(|id| !children.contains(id))
            .map(|&id| {
                (
                    id,
                    Matrix4::identity(),
                    PlacementPath::default(),
                    Vec::new(),
                )
            })
            .collect();
        while let Some((node, transform, source, mut path)) = pending.pop() {
            budget.traverse()?;
            if path.len() >= 128 || path.contains(&node) {
                return Err("cyclic or excessively deep native assembly".into());
            }
            path.push(node);
            seen.insert(node);
            units::validate_placement_matrix(transform)?;
            result.extend(scene.traced_instances(
                nodes[&node],
                transform,
                Some(source.clone()),
                &mut budget,
            )?);
            for &(child, placement, step) in edges.get(&node).into_iter().flatten().rev() {
                budget.traverse()?;
                pending.push((
                    child,
                    transform * placement,
                    source.through(step),
                    path.clone(),
                ));
            }
        }
        if seen.len() != nodes.len() {
            return Err("unreachable native assembly node".into());
        }
    }
    scene.require_complete_mappings(&budget.seen_mappings)?;
    Ok(result)
}

fn insert_unique<T>(index: &mut HashMap<u64, T>, id: u64, value: T) -> Result<(), String> {
    if index.insert(id, value).is_some() {
        return Err("duplicate native assembly link".into());
    }
    Ok(())
}

fn validate_product(
    records: &SourceRecords<'_>,
    product: &Record,
    config_control: bool,
) -> Result<(), String> {
    let definition = parameters(product, 4)?;
    let context = simple_record(records, reference(&definition[3])?)?;
    match context.name.as_str() {
        "PRODUCT_DEFINITION_CONTEXT" if !config_control => {}
        "DESIGN_CONTEXT" | "PRODUCT_DEFINITION_CONTEXT" if config_control => {
            let context = parameters(context, 3)?;
            if !matches!(
                (&context[0], &context[2]),
                (Parameter::String(_), Parameter::String(_))
            ) {
                return Err("invalid native design context".into());
            }
            let application =
                named_record(records, reference(&context[1])?, "APPLICATION_CONTEXT")?;
            if !matches!(parameters(application, 1)?, [Parameter::String(_)]) {
                return Err("invalid native application context".into());
            }
        }
        _ => return Err("unsupported native product definition context".into()),
    }
    let formation = simple_record(records, reference(&definition[2])?)?;
    if !matches!(
        formation.name.as_str(),
        "PRODUCT_DEFINITION_FORMATION" | "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE"
    ) {
        return Err("unsupported native product formation".into());
    }
    let count = if formation.name == "PRODUCT_DEFINITION_FORMATION" {
        3
    } else {
        4
    };
    let formation = parameters(formation, count)?;
    named_record(records, reference(&formation[2])?, "PRODUCT")?;
    Ok(())
}

fn placement(
    records: &SourceRecords<'_>,
    scene: &scene::Scene<'_>,
    id: u64,
    child: u64,
    parent: u64,
) -> Result<(Matrix4, PlacementStep), String> {
    let parts = records
        .get(&id)
        .ok_or("missing native assembly placement")?;
    let part = |kind| {
        parts
            .iter()
            .copied()
            .find(|record| record.name == kind)
            .ok_or("missing native placement component")
    };
    if parts.len() != 3 {
        return Err("unsupported native assembly relationship".into());
    }
    parameters(part("SHAPE_REPRESENTATION_RELATIONSHIP")?, 0)?;
    let relationship = parameters(part("REPRESENTATION_RELATIONSHIP")?, 4)?;
    if reference(&relationship[2])? != child || reference(&relationship[3])? != parent {
        return Err("native placement relationship does not match product endpoints".into());
    }
    let operator = parameters(part("REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION")?, 1)?;
    let transform = parameters(
        named_record(
            records,
            reference(&operator[0])?,
            "ITEM_DEFINED_TRANSFORMATION",
        )?,
        4,
    )?;
    let from_frame = reference(&transform[2])?;
    let to_frame = reference(&transform[3])?;
    let from = scene.frame(from_frame, scene.factor(child)?)?;
    let to = scene.frame(to_frame, scene.factor(parent)?)?;
    Ok((
        to * from.invert().ok_or("singular native assembly frame")?,
        PlacementStep {
            from_frame,
            from_rep: child,
            to_frame,
            to_rep: parent,
            mapped: false,
        },
    ))
}
