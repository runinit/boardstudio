//! Board, module, outline and circuit mirroring helpers used to build a mirrored half.

use super::*;

/// The mirror's stored configuration: the source's, for the board it is placed on.
///
/// A new board starts without the lists that name the source board's parts or one physical item
/// (hardware specifications, critical fits, a battery). Openings, mounts, closure mounts and
/// gasket supports are linked through the pair, so they reach the mirror reflected.
pub(super) fn mirror_configuration(
    source: &MechanicalConfiguration,
    board_id: &str,
    new_board: bool,
) -> MechanicalConfiguration {
    let mut configuration = source.clone();
    configuration.board_id = board_id.into();
    if new_board {
        configuration.hardware = None;
        configuration.critical_fits = None;
        configuration.battery = None;
    }
    configuration
}

/// What a mounted module's mirror takes from the board mirror: the two boards and instances, the
/// pair map, and the source-to-mirror id maps of the board's parts and modules.
pub(super) struct ModuleMirror<'a> {
    pub(super) from: &'a str,
    pub(super) to: &'a str,
    pub(super) source_instance: &'a str,
    pub(super) mirror_instance: &'a str,
    pub(super) mapping: Pose2,
    pub(super) parts: &'a BTreeMap<String, String>,
    /// Source parts whose mirror is Omitted. A module plugged into one keeps everything except
    /// its connection, which is cleared as deleting the connector clears it.
    pub(super) omitted_parts: &'a BTreeSet<String>,
    pub(super) modules: &'a BTreeMap<String, String>,
}

/// A mounted module on the mirror (ADR-0010 amendment, modules; ticket 27). The pose goes through the
/// pair map and the heading takes the full rule `rotation + 180 - r`, the independent-part rule. The
/// module is not flipped, so its faces, gap, attachment, supports, definition and every parameter are
/// unchanged. Its own id and bus id take the board-prefix rule, its connector and upstream module take
/// their mirror ids, and a pin to the source instance moves to the mirror instance. A reference the
/// mirror cannot follow refuses the record by name (the caller discards the whole edit).
pub(super) fn mirror_module(
    module: &MountedModule,
    context: &ModuleMirror,
) -> Result<MountedModule, String> {
    let host_instance_id = match module.host_instance_id.as_deref() {
        None => None,
        Some(id) if id == context.source_instance => Some(context.mirror_instance.to_owned()),
        Some(other) => {
            return Err(format!(
                "Module {:?} is pinned to physical instance {other:?}, which is not the source {:?}; its mirror has no instance to be pinned to.",
                module.id, context.source_instance
            ));
        }
    };
    let pose = mirror_pose(
        context.mapping,
        Pose2 {
            at: module.at,
            rotation: module.rotation,
        },
    );
    let mut mirrored = module.clone();
    mirrored.id = context.modules[&module.id].clone();
    mirrored.host_board_id = context.to.into();
    mirrored.host_instance_id = host_instance_id;
    mirrored.at = pose.at;
    mirrored.rotation = pose.rotation;
    if mirrored.connection.as_ref().is_some_and(|connection| {
        context
            .omitted_parts
            .contains(&connection.host_connector_part_id)
    }) {
        mirrored.connection = None;
    }
    if let Some(connection) = mirrored.connection.as_mut() {
        let connector = connection.host_connector_part_id.clone();
        if !connector.is_empty() {
            connection.host_connector_part_id =
                context.parts.get(&connector).cloned().ok_or_else(|| {
                    format!(
                        "Module {:?} connects to {connector:?}, which is not a part of board {:?}.",
                        module.id, context.from
                    )
                })?;
        }
        connection.bus_id = mirror_id(context.from, context.to, &connection.bus_id);
        if let Some(upstream) = connection.upstream_module_id.clone() {
            connection.upstream_module_id = Some(context.modules.get(&upstream).cloned().ok_or_else(|| {
                format!(
                    "Module {:?} takes its output from {upstream:?}, which is not a module of board {:?}.",
                    module.id, context.from
                )
            })?);
        }
    }
    Ok(mirrored)
}

/// Every mechanical feature the configuration carries, matched to itself on the other half.
pub(super) fn feature_correspondences(
    configuration: &MechanicalConfiguration,
) -> Vec<MechanicalFeatureCorrespondence> {
    let same = |kind: MechanicalFeatureKind, id: &str| MechanicalFeatureCorrespondence {
        kind,
        first_feature_id: id.into(),
        second_feature_id: id.into(),
        source_instance_id: None,
    };
    let mut features = Vec::new();
    features.extend(
        configuration
            .mounts
            .iter()
            .map(|mount| same(MechanicalFeatureKind::Mount, &mount.id)),
    );
    features.extend(
        configuration
            .closure_mounts
            .iter()
            .flatten()
            .map(|mount| same(MechanicalFeatureKind::ClosureMount, &mount.id)),
    );
    features.extend(
        configuration
            .openings
            .iter()
            .flatten()
            .map(|opening| same(MechanicalFeatureKind::Opening, &opening.id)),
    );
    features.extend(
        configuration
            .gasket_layout
            .iter()
            .flat_map(|layout| &layout.supports)
            .map(|support| same(MechanicalFeatureKind::GasketSupport, &support.id)),
    );
    features
}

/// The second half's instance: the opposite half label with the recipes' central/peripheral
/// roles, owning its configuration until the link promotes both members to the pair.
pub(super) fn mirror_instance(
    source: &PhysicalBoardInstance,
    id: &str,
    board_id: &str,
    flipped: bool,
    controller_part_id: Option<String>,
    configuration: MechanicalConfiguration,
) -> PhysicalBoardInstance {
    let half = mirror_half_label(&source.half).to_owned();
    PhysicalBoardInstance {
        id: id.into(),
        name: match half.as_str() {
            "left" => "Left half".into(),
            "right" => "Right half".into(),
            _ => format!("{} mirror", source.name),
        },
        board_id: board_id.into(),
        role: if half == "right" {
            "peripheral"
        } else {
            "central"
        }
        .into(),
        half,
        flipped,
        controller_part_id,
        mechanical: Some(configuration),
        construction_ownership: MechanicalConstructionOwnership::Local,
        mechanical_identity_reservation: None,
    }
}

/// Links the two members through the mechanical `LinkMirrorPair` operation: the pair, its
/// shared construction, every linked feature authored by the source, the generated clearance
/// holes, and the mirrored-half record. The mirror instance must already be in the document with
/// a configuration that carries the same feature ids as the source.
pub(super) fn link_halves(
    doc: &mut ProjectDoc,
    request: &Request,
    mirror_id: &str,
    halves: MirroredHalves,
) -> Result<Vec<String>, String> {
    let mirror = effective_inputs(
        doc,
        &[],
        &MechanicalTarget::Instance {
            instance_id: mirror_id.into(),
        },
    )?
    .configuration;
    let edit = MechanicalEdit {
        target: MechanicalTarget::Instance {
            instance_id: request.source_id.into(),
        },
        change: MechanicalChange::LinkMirrorPair {
            pair_id: request.pair_id.into(),
            members: [request.source_id.into(), mirror_id.into()],
            mapping: request.mapping,
            construction_source: request.source_id.into(),
            correspondences: feature_correspondences(&mirror),
        },
        mounting_hole: request
            .mounting_hole
            .map(|definition| Box::new(definition.clone())),
    };
    let changed = mechanical::edit::apply(doc, &[], &edit)?;
    doc.hardware
        .as_mut()
        .and_then(|hardware| {
            hardware
                .pairs
                .iter_mut()
                .find(|pair| pair.id == request.pair_id)
        })
        .ok_or("The mechanical pair was not recorded.")?
        .halves = Some(halves);
    Ok(changed)
}

pub(super) fn reuse_board(
    doc: &mut ProjectDoc,
    request: &Request,
    instance_id: &str,
) -> Result<Vec<String>, String> {
    if request.companion_tweak.is_some() {
        return Err("reuse-board copies no companions; companionTweak requires new-board".into());
    }
    let (source, configuration) = source_half(doc, request)?;
    if source.flipped {
        return Err(format!(
            "Physical instance {:?} is already flipped; a reused board needs an unflipped source.",
            request.source_id
        ));
    }
    // A flipped instance of the same board is the physical reflection `q = (-p.x, p.y)`, which is
    // the pair map with no translation and no turn; any other map describes a different part.
    if request.mapping.at.x != 0.0 || request.mapping.at.y != 0.0 || request.mapping.rotation != 0.0
    {
        return Err(format!(
            "A flipped instance of board {:?} is the plain reflection: its map must be zero \
             translation and zero rotation, not {:?}. Create a new board for any other map.",
            source.board_id, request.mapping
        ));
    }
    ensure_board_unclaimed(doc, &source.board_id)?;
    ensure_instance_fresh(doc, instance_id)?;
    // A module pinned to the source instance is not active on the flipped instance, so it would
    // vanish from the mirror. Copying it is not supported yet: refuse it by name.
    let pinned: Vec<&str> = doc
        .modules
        .iter()
        .filter(|module| {
            module.host_board_id == source.board_id
                && module.host_instance_id.as_deref() == Some(request.source_id)
        })
        .map(|module| module.id.as_str())
        .collect();
    if !pinned.is_empty() {
        return Err(format!(
            "Modules {} are pinned to physical instance {:?}; reuse-board cannot copy a pinned module yet. Unpin them or create a new board.",
            pinned.join(", "),
            request.source_id
        ));
    }
    // An embedded circuit belongs to the board, not to an instance (it has no instance pin), so the
    // flipped instance shows the circuit without a copy, as it shows the board's own parts.
    let board = source.board_id.clone();
    let mirror = mirror_instance(
        &source,
        instance_id,
        &board,
        true,
        None,
        mirror_configuration(&configuration, &board, false),
    );
    doc.hardware
        .as_mut()
        .ok_or("Physical instances are unavailable")?
        .instances
        .push(mirror);
    link_halves(
        doc,
        request,
        instance_id,
        MirroredHalves {
            companion_mode: request.companion_mode,
            source_board_id: board.clone(),
            mirror_board_id: board,
            correspondences: vec![],
            exceptions: vec![],
        },
    )
}

/// How a feature's ids are renamed. Outline identities share one document namespace, so every
/// identity the rename makes is recorded and checked once the whole outline is built.
pub(super) struct Renames<'a> {
    /// Maps an outline identity (a feature, connection, gap or bridge) and records the result.
    pub(super) identity: &'a dyn Fn(&str) -> String,
    /// Maps a reference to a part of the source board to its mirror; foreign ids stay as they are.
    pub(super) part: &'a dyn Fn(&str) -> String,
    /// Whether a source part takes the full heading rule (an independent part, ticket 30) rather
    /// than the switch rule (a key or a companion). It decides how an outline anchored to it is
    /// reflected in the part's frame.
    pub(super) full_rule_part: &'a dyn Fn(&str) -> bool,
}

pub(super) fn mirror_control(
    point: &OutlineControlPoint,
    mapping: Pose2,
    renames: &Renames,
) -> OutlineControlPoint {
    match &point.part_id {
        // Attached to a part: the point is in that part's frame.
        Some(part) => OutlineControlPoint {
            at: anchored_local(point.at, (renames.full_rule_part)(part)),
            part_id: Some((renames.part)(part)),
        },
        None => OutlineControlPoint {
            at: pair::forward(mapping, point.at),
            part_id: None,
        },
    }
}

pub(super) fn mirror_gap(
    gap: &ProtectedOutlineGap,
    mapping: Pose2,
    renames: &Renames,
) -> ProtectedOutlineGap {
    ProtectedOutlineGap {
        id: (renames.identity)(&gap.id),
        points: gap
            .points
            .iter()
            .map(|point| mirror_control(point, mapping, renames))
            .collect(),
    }
}

pub(super) fn mirror_settings(
    settings: &OutlineSettings,
    mapping: Pose2,
    renames: &Renames,
) -> OutlineSettings {
    let mut mirrored = settings.clone();
    if let Some(repair) = mirrored.repair.as_mut() {
        repair.keep_gaps = repair
            .keep_gaps
            .iter()
            .map(|gap| mirror_gap(gap, mapping, renames))
            .collect();
    }
    mirrored
}

/// Reflects a feature. A point in world coordinates goes through the pair map; a point anchored
/// to a part is in that part's frame and only flips handedness. A reflected polygon keeps its
/// orientation by reversing its point order.
pub(super) fn mirror_feature(
    feature: &OutlineFeature,
    mapping: Pose2,
    renames: &Renames,
) -> OutlineFeature {
    match feature {
        OutlineFeature::Polygon {
            anchor_part_id,
            id,
            points,
            operation,
        } => OutlineFeature::Polygon {
            anchor_part_id: anchor_part_id.as_deref().map(renames.part),
            id: (renames.identity)(id),
            points: points
                .iter()
                .rev()
                .map(|point| match anchor_part_id.as_deref() {
                    Some(anchor) => anchored_local(*point, (renames.full_rule_part)(anchor)),
                    None => pair::forward(mapping, *point),
                })
                .collect(),
            operation: *operation,
        },
        OutlineFeature::Rect {
            rotation,
            anchor_part_id,
            id,
            center,
            size,
            radius,
            operation,
        } => OutlineFeature::Rect {
            rotation: if anchor_part_id.is_some() {
                rotation.map(|angle| -angle)
            } else {
                (rotation.is_some() || mapping.rotation != 0.0)
                    .then(|| pair::forward_angle(mapping, rotation.unwrap_or(0.0)))
            },
            anchor_part_id: anchor_part_id.as_deref().map(renames.part),
            id: (renames.identity)(id),
            center: match anchor_part_id.as_deref() {
                Some(anchor) => anchored_local(*center, (renames.full_rule_part)(anchor)),
                None => pair::forward(mapping, *center),
            },
            size: *size,
            radius: *radius,
            operation: *operation,
        },
        OutlineFeature::PartEnvelope {
            connections,
            settings,
            id,
            part_ids,
            margin,
            operation,
        } => OutlineFeature::PartEnvelope {
            connections: connections
                .iter()
                .map(|connection| OutlineConnection {
                    id: (renames.identity)(&connection.id),
                    width: connection.width,
                    points: connection
                        .points
                        .iter()
                        .map(|point| mirror_control(point, mapping, renames))
                        .collect(),
                })
                .collect(),
            settings: mirror_settings(settings, mapping, renames),
            id: (renames.identity)(id),
            part_ids: part_ids
                .iter()
                .filter(|part| !part.starts_with(GENERATED_CLOSURE_PREFIX))
                .map(|part| (renames.part)(part))
                .collect(),
            margin: *margin,
            operation: *operation,
        },
    }
}

/// A fixed outline version's snapshot, reflected: features, authored bridges and protected gaps.
pub(super) fn mirror_snapshot(
    snapshot: &OutlineSnapshot,
    mapping: Pose2,
    renames: &Renames,
    matrix_map: &BTreeMap<String, String>,
) -> OutlineSnapshot {
    OutlineSnapshot {
        features: snapshot
            .features
            .iter()
            .map(|feature| mirror_feature(feature, mapping, renames))
            .collect(),
        settings: mirror_settings(&snapshot.settings, mapping, renames),
        expected_regions: snapshot.expected_regions,
        bridges: snapshot
            .bridges
            .iter()
            .map(|bridge| OutlineBridge {
                id: (renames.identity)(&bridge.id),
                width: bridge.width,
                points: bridge
                    .points
                    .iter()
                    .map(|point| pair::forward(mapping, *point))
                    .collect(),
                part_ids: bridge
                    .part_ids
                    .iter()
                    .map(|part| (renames.part)(part))
                    .collect(),
                matrix_ids: bridge
                    .matrix_ids
                    .iter()
                    .map(|matrix| {
                        matrix_map
                            .get(matrix)
                            .cloned()
                            .unwrap_or_else(|| matrix.clone())
                    })
                    .collect(),
                authored: bridge.authored,
            })
            .collect(),
        protected_gaps: snapshot
            .protected_gaps
            .iter()
            .map(|gap| mirror_gap(gap, mapping, renames))
            .collect(),
    }
}

/// A mirror's reference: the board token substituted, or `to-reference` when none matches.
pub(super) fn mirror_reference(from: &str, to: &str, reference: &str) -> String {
    substitute_token(reference, from, to).unwrap_or_else(|| format!("{to}-{reference}"))
}

/// Renames a wiring function key or value that names a part or net of the source board
/// (`peripheral/left/TRRS/split-rx`, a net id). The longest whole-segment match wins; a
/// terminal such as `P21` names nothing and is kept.
pub(super) fn remap_wiring_key(text: &str, renames: &BTreeMap<String, String>) -> String {
    let bounded = |start: usize, end: usize| {
        (start == 0 || text.as_bytes()[start - 1] == b'/')
            && (end == text.len() || text.as_bytes()[end] == b'/')
    };
    let mut best: Option<(usize, usize, &String)> = None;
    for (source, mirror) in renames {
        let mut from = 0;
        while let Some(found) = text[from..].find(source.as_str()) {
            let start = from + found;
            let end = start + source.len();
            if bounded(start, end) && best.is_none_or(|(_, length, _)| source.len() > length) {
                best = Some((start, source.len(), mirror));
                break;
            }
            from = start + 1;
            while !text.is_char_boundary(from) {
                from += 1;
            }
        }
    }
    match best {
        Some((start, length, mirror)) => {
            format!("{}{}{}", &text[..start], mirror, &text[start + length..])
        }
        None => text.to_owned(),
    }
}

pub(super) fn corresponds(
    kind: MirroredItemKind,
    source: &str,
    mirror: &str,
) -> MirroredCorrespondence {
    MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind,
        source_id: source.to_owned(),
        mirror_id: mirror.to_owned(),
    }
}

/// An embedded circuit's own members, private nets and component definitions, and where each one
/// mirrors (ticket 28). `modules::circuit` places a circuit's members under `embedded/<id>/` and reads
/// that prefix back (`embed`, `remove_circuit`, the electrical checks), so the board-prefix rule would
/// break it (`embedded/left-haptic/U1` would become `right/embedded/...`). A mirror keeps the prefix and
/// takes its own circuit id: `embedded/left-haptic/U1` becomes `embedded/right-haptic/U1`. The circuit
/// id itself takes the board-prefix rule. A host net a port joins is not in the namespace and keeps the
/// board-prefix rule.
pub(super) struct CircuitMirror {
    /// Source circuit id to mirror circuit id.
    pub(super) ids: BTreeMap<String, String>,
    /// Source id of a circuit member or private net to its mirror id.
    pub(super) owned: BTreeMap<String, String>,
    /// Source id of a circuit component definition to its mirror copy.
    pub(super) definitions: BTreeMap<String, String>,
}

/// Moves `id` from the namespace `from` to the namespace `to`, or `None` when `id` is outside `from`.
pub(super) fn reprefix(id: &str, from: &str, to: &str) -> Option<String> {
    id.strip_prefix(from).map(|rest| format!("{to}{rest}"))
}

/// `reference` when the document does not use it yet, otherwise its prefix with the next free number
/// (`right-U1` becomes `right-U2`), as `modules::allocate_reference` does for circuit members.
pub(super) fn free_reference(reference: &str, used: &mut BTreeSet<String>) -> String {
    if used.insert(reference.to_owned()) {
        return reference.to_owned();
    }
    let prefix = reference.trim_end_matches(|c: char| c.is_ascii_digit());
    let mut index = 1;
    loop {
        let candidate = format!("{prefix}{index}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        index += 1;
    }
}

/// The mirror of each embedded circuit hosted on `from`. A mirror circuit id that is already a circuit
/// on the document, or that another hosted circuit already takes, refuses the record by name, and so
/// does a mirror namespace that already holds a part, net or definition.
pub(super) fn circuit_mirror(
    doc: &ProjectDoc,
    hosted: &[EmbeddedCircuit],
    from: &str,
    to: &str,
) -> Result<CircuitMirror, String> {
    let mut mirror = CircuitMirror {
        ids: BTreeMap::new(),
        owned: BTreeMap::new(),
        definitions: BTreeMap::new(),
    };
    for circuit in hosted {
        let id = mirror_id(from, to, &circuit.id);
        if doc.embedded_circuits.iter().any(|other| other.id == id)
            || mirror.ids.values().any(|taken| *taken == id)
        {
            return Err(format!(
                "The mirrored embedded circuit {id:?} already exists."
            ));
        }
        let source_prefix = format!("embedded/{}/", circuit.id);
        let prefix = format!("embedded/{id}/");
        if doc.parts.iter().any(|p| p.id.starts_with(&prefix))
            || doc.definitions.iter().any(|d| d.id.starts_with(&prefix))
            || doc.nets.iter().any(|n| n.id.starts_with(&prefix))
        {
            return Err(format!(
                "The circuit namespace {prefix:?} is already in use."
            ));
        }
        for part in &circuit.part_ids {
            let mirrored = reprefix(part, &source_prefix, &prefix).ok_or_else(|| {
                format!(
                    "Embedded circuit {:?} has member {part:?} outside its namespace.",
                    circuit.id
                )
            })?;
            mirror.owned.insert(part.clone(), mirrored);
        }
        for net in &circuit.net_ids {
            if let Some(mirrored) = reprefix(net, &source_prefix, &prefix) {
                mirror.owned.insert(net.clone(), mirrored);
            }
        }
        let (source_definitions, mirror_definitions) = (
            format!("{source_prefix}definition/"),
            format!("{prefix}definition/"),
        );
        for definition in &doc.definitions {
            if let Some(mirrored) =
                reprefix(&definition.id, &source_definitions, &mirror_definitions)
            {
                mirror.definitions.insert(definition.id.clone(), mirrored);
            }
        }
        mirror.ids.insert(circuit.id.clone(), id);
    }
    Ok(mirror)
}

/// Repairs only absent matrix companions on the draft. Generate into a scratch document so
/// existing source parts, authored poses, sides and generator parameters remain untouched.
/// Return the repaired ids: their mirrors keep a durable literal fallback instead of the
/// requested duplicate/tweak mode. A missing switch is a validation error, never a repair.
pub(super) fn materialise_missing_companions(
    doc: &mut ProjectDoc,
    board_id: &str,
) -> Result<BTreeSet<String>, String> {
    let matrices: Vec<_> = doc
        .matrices
        .iter()
        .filter(|matrix| matrix.board_id.as_deref() == Some(board_id))
        .cloned()
        .collect();
    let mut repaired = BTreeSet::new();
    for matrix in matrices {
        crate::matrix::valid_matrix(&matrix, doc)?;
        let mut missing = BTreeSet::new();
        for row in 0..matrix.rows {
            for column in 0..matrix.columns {
                let cell = matrix
                    .cells
                    .iter()
                    .find(|cell| cell.row == row && cell.column == column);
                if cell.is_some_and(|cell| !cell.enabled || cell.deleted) {
                    continue;
                }
                let key_id = crate::matrix::member_id(&matrix.id, row, column);
                if !doc.parts.iter().any(|part| part.id == key_id) {
                    return Err(format!(
                        "A companion's source switch {key_id:?} is unavailable"
                    ));
                }
                for assembly in cell.into_iter().flat_map(|cell| &cell.assemblies) {
                    let id = crate::matrix::companion_id(&key_id, &assembly.id);
                    if !doc.parts.iter().any(|part| part.id == id) {
                        missing.insert(id);
                    }
                }
            }
        }
        if missing.is_empty() {
            continue;
        }
        let mut generated = doc.clone();
        crate::matrix::set_matrix(&mut generated, &matrix)?;
        doc.parts.extend(
            generated
                .parts
                .into_iter()
                .filter(|part| missing.contains(&part.id)),
        );
        let source_matrix = doc
            .matrices
            .iter_mut()
            .find(|source| source.id == matrix.id)
            .ok_or("The source matrix is unavailable")?;
        let board = doc
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .ok_or("The source board is unavailable")?;
        for id in &missing {
            if !source_matrix.part_ids.contains(id) {
                source_matrix.part_ids.push(id.clone());
            }
            if !board.part_ids.contains(id) {
                board.part_ids.push(id.clone());
            }
        }
        repaired.extend(missing);
    }
    Ok(repaired)
}
