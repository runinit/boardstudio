use super::*;

pub(super) fn current_settings(owner: &CurrentSettingsOwner) -> Option<MechanicalSettingsCurrent> {
    let CurrentSettingsOwner {
        runtime,
        alive,
        source_projection,
        editor_instance_id,
        base_generation,
        presentation_generation,
        presentation_key,
        workspace,
        instance_selection,
    } = owner;
    if !alive.get() {
        return None;
    }
    let scope = runtime.scope();
    let observed_key = (workspace(), base_generation(), scope.clone());
    let presentation_generation = {
        let mut previous = presentation_key.borrow_mut();
        if previous.as_ref() != Some(&observed_key) {
            if previous.is_some() {
                presentation_generation.set(
                    presentation_generation
                        .get()
                        .checked_add(1)
                        .expect("Case presentation identity exhausted"),
                );
            }
            *previous = Some(observed_key);
        }
        presentation_generation.get()
    };
    let model = runtime.model();
    let accepted = model.accepted.clone()?;
    let scope = scope?;
    if scope.session_epoch != accepted.session_epoch
        || scope.document_id != accepted.document.id
        || scope.board_id != model.active_board_id
        || scope.instance_id != model.active_instance_id
    {
        return None;
    }
    let key = SettingsSourceKey {
        scope: scope.clone(),
        token: accepted.token,
        revision: accepted.document.revision,
    };
    let projection = source_projection.read();
    let projection = projection
        .as_ref()
        .filter(|projection| projection.key == key)?;
    let configuration = projection.configuration.clone();
    let configuration_board_id = configuration.as_ref().map_or_else(
        || scope.board_id.clone(),
        |configuration| configuration.board_id.clone(),
    );
    let saved = matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) && matches!(
        model.durability,
        Durability::Saved { .. } | Durability::Saving { .. }
    );
    let editable = workspace() == "Case"
        && instance_selection.is_current(&model)
        && saved
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && configuration_board_id == scope.board_id;
    Some(MechanicalSettingsCurrent {
        identity: MechanicalSettingsIdentity {
            editor_instance_id: *editor_instance_id,
            scope_generation: base_generation(),
            presentation_generation,
            scope: scope.clone(),
            snapshot_token: accepted.token,
            revision: accepted.document.revision,
            active_board_id: model.active_board_id.clone(),
            configuration_board_id,
        },
        accepted,
        configuration,
        editable,
        lifecycle: model.lifecycle,
        durability: model.durability,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SettingsSourceKey {
    pub(super) scope: Scope,
    pub(super) token: boardstudio_application::SnapshotToken,
    pub(super) revision: u64,
}

#[derive(Clone, PartialEq)]
pub(super) struct MechanicalSettingsSourceProjection {
    pub(super) key: SettingsSourceKey,
    pub(super) configuration: Option<Rc<MechanicalConfiguration>>,
    pub(super) profiles: Rc<[MechanicalProfileChoice]>,
}

#[derive(Clone)]
pub(super) struct MechanicalSettingsResolvedProjection {
    pub(super) key: SettingsSourceKey,
    pub(super) identity: MechanicalSettingsIdentity,
    pub(super) resolution: Rc<MechanicalResolution>,
}

pub(super) fn projection_matches_identity(
    projection: &MechanicalSettingsResolvedProjection,
    identity: &MechanicalSettingsIdentity,
) -> bool {
    projection.identity == *identity
        && projection.key.scope == identity.scope
        && projection.key.token == identity.snapshot_token
        && projection.key.revision == identity.revision
}

pub(super) fn mechanical_assembly_has_layer(assembly: &MechanicalAssembly, id: &str) -> bool {
    assembly.stack.iter().any(|layer| layer.id == id)
        || (id == "gaskets" && !assembly.gasket_supports.is_empty())
        || (assembly.gasket_supports.iter().any(|support| {
            id == format!("gasket:{}:lower", support.id)
                || id == format!("gasket:{}:upper", support.id)
        }) && assembly.case.bodies.iter().any(|body| body.body.id == id))
}

pub(super) fn resolved_projection_layer_is_selectable(
    projection: &MechanicalSettingsResolvedProjection,
    identity: &MechanicalSettingsIdentity,
    id: &str,
) -> bool {
    // The resolved projection is admitted to the mounted settings only for this exact
    // scope/token/revision. Recheck the full identity because a retained row action can
    // otherwise outlive the render that supplied its layer id.
    projection_matches_identity(projection, identity)
        && mechanical_assembly_has_layer(&projection.resolution.assembly, id)
}

impl PartialEq for MechanicalSettingsResolvedProjection {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.identity == other.identity
            && Rc::ptr_eq(&self.resolution, &other.resolution)
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct MechanicalSceneRows {
    pub(super) layers: Rc<[MechanicalLayerRow]>,
    pub(super) gasket_supports: Rc<[MechanicalGasketSupportRow]>,
    pub(super) fit_parts: Rc<[MechanicalFitPart]>,
    pub(super) fit_parts_resolved: bool,
    pub(super) hardware_mounts: Rc<[MechanicalHardwareMount]>,
    pub(super) suggested_mounts: Rc<[Mount]>,
    pub(super) findings: Rc<[MechanicalFindingRow]>,
}

pub(super) fn project_scene_rows(
    display_scene: Option<&Rc<CadScene>>,
    current_scene: Option<&Rc<CadScene>>,
    current_resolution: Option<&Rc<MechanicalResolution>>,
    internal_gasket: bool,
    is_previous: bool,
) -> Option<MechanicalSceneRows> {
    let display_assembly = display_scene.and_then(|scene| scene.mechanical.as_ref());
    let current_assembly: Option<&MechanicalAssembly> = current_resolution
        .map(|resolution| &resolution.assembly)
        .or_else(|| current_scene.and_then(|scene| scene.mechanical.as_ref()));
    if display_assembly.is_none() && current_assembly.is_none() {
        return None;
    }
    let layers = project_layer_rows(
        display_assembly,
        current_assembly,
        internal_gasket,
        is_previous,
    );
    let support_assembly = current_assembly.or(display_assembly);
    let supports_are_previous = current_assembly.is_none() && is_previous;
    let gasket_supports: Vec<_> = support_assembly
        .into_iter()
        .flat_map(|assembly| assembly.gasket_supports.iter())
        .map(|support| MechanicalGasketSupportRow {
            id: support.id.clone(),
            region_id: support.region_id.clone(),
            outline_key: support.outline_key.clone(),
            anchor: support.anchor,
            pair_id: support.pair_id.clone(),
            length: support.length,
            width: support.width,
            unlinked: support.unlinked,
            fit_error: support.fit_error.clone(),
            is_previous: supports_are_previous,
        })
        .collect();
    let findings: Vec<_> = current_assembly
        .into_iter()
        .flat_map(|assembly| assembly.diagnostics.iter())
        .map(|finding| MechanicalFindingRow {
            id: finding.id.clone(),
            severity: finding.severity.clone(),
            message: finding.message.clone(),
        })
        .collect();
    let suggested_mounts = current_assembly
        .map(|assembly| assembly.suggested_mounts.clone())
        .unwrap_or_default();
    let fit_parts = current_assembly
        .into_iter()
        .flat_map(|assembly| assembly.case.bodies.iter())
        .map(|body| MechanicalFitPart {
            id: body.body.id.clone(),
            name: if body.body.name.is_empty() {
                body.body.id.clone()
            } else {
                body.body.name.clone()
            },
        })
        .collect::<Vec<_>>();
    let fit_parts_resolved = current_assembly.is_some();
    let hardware_mounts = current_assembly
        .into_iter()
        .flat_map(|assembly| assembly.case.bodies.iter())
        .flat_map(|body| {
            let part_id = body.body.id.clone();
            let part_name = if body.body.name.is_empty() {
                part_id.clone()
            } else {
                body.body.name.clone()
            };
            body.body
                .mounts
                .iter()
                .flatten()
                .map(move |mount| MechanicalHardwareMount {
                    part_id: part_id.clone(),
                    part_name: part_name.clone(),
                    feature_id: mount.id.clone(),
                    kind: mount.kind.clone(),
                    x: mount.at.x,
                    y: mount.at.y,
                })
        })
        .collect::<Vec<_>>();
    Some(MechanicalSceneRows {
        layers,
        gasket_supports: Rc::from(gasket_supports),
        fit_parts: Rc::from(fit_parts),
        fit_parts_resolved,
        hardware_mounts: Rc::from(hardware_mounts),
        suggested_mounts: Rc::from(suggested_mounts),
        findings: Rc::from(findings),
    })
}

pub(super) fn project_layer_rows(
    display_assembly: Option<&MechanicalAssembly>,
    current_assembly: Option<&MechanicalAssembly>,
    internal_gasket: bool,
    is_previous: bool,
) -> Rc<[MechanicalLayerRow]> {
    // Current assemblies have already passed the exact scope/token/revision admission above.
    // Prefer their rows so the settings view can show the accepted resolved stack before the
    // viewport publishes a current scene; only fall back to display geometry as explicitly
    // previous rows when no current assembly is available.
    let (assembly, rows_are_previous) = current_assembly
        .map(|assembly| (Some(assembly), false))
        .unwrap_or((display_assembly, is_previous));
    let layers: Vec<_> = assembly
        .into_iter()
        .flat_map(|assembly| assembly.stack.iter().map(move |layer| (assembly, layer)))
        .map(|(assembly, layer)| MechanicalLayerRow {
            id: layer.id.clone(),
            label: mechanical_layer_label(&layer.id, internal_gasket),
            z: layer.z,
            thickness: layer.thickness,
            resolved_body_thickness: assembly
                .case
                .bodies
                .iter()
                .find(|body| body.body.id == layer.id)
                .map(|body| body.body.thickness),
            is_previous: rows_are_previous,
        })
        .collect();
    Rc::from(layers)
}

pub(super) fn current_source_key(runtime: &Runtime) -> Option<SettingsSourceKey> {
    let model = runtime.model();
    let accepted = model.accepted.as_ref()?;
    let scope = runtime.scope()?;
    (scope.session_epoch == accepted.session_epoch
        && scope.document_id == accepted.document.id
        && scope.board_id == model.active_board_id
        && scope.instance_id == model.active_instance_id
        && accepted.scene.revision == accepted.document.revision)
        .then(|| SettingsSourceKey {
            scope,
            token: accepted.token,
            revision: accepted.document.revision,
        })
}

pub(super) fn project_settings_source(
    runtime: &Runtime,
    key: Option<&SettingsSourceKey>,
) -> Option<MechanicalSettingsSourceProjection> {
    let key = key?;
    let model = runtime.model();
    let accepted = model.accepted.as_ref()?;
    if current_source_key(runtime).as_ref() != Some(key) {
        return None;
    }
    #[cfg(test)]
    SOURCE_PROJECTIONS.set(SOURCE_PROJECTIONS.get() + 1);
    let effective =
        boardstudio_web_host::cad_jobs::captured_case_document(accepted, &key.scope).ok()?;
    let configuration = effective.mechanical.clone().map(Rc::new);
    let profiles = configuration
        .as_ref()
        .map_or_else(Vec::new, |configuration| {
            profile_choices(accepted, configuration)
        });
    Some(MechanicalSettingsSourceProjection {
        key: key.clone(),
        configuration,
        profiles: Rc::from(profiles),
    })
}

pub(super) fn profile_targets(
    accepted: &AcceptedSnapshot,
    board_id: &str,
    configuration: Option<&MechanicalConfiguration>,
) -> Vec<MechanicalProfileTargetChoice> {
    let Some(board) = accepted
        .document
        .boards
        .iter()
        .find(|board| board.id == board_id)
    else {
        return Vec::new();
    };
    if configuration.is_none() {
        return Vec::new();
    }
    let assigned = configuration
        .map(|configuration| {
            configuration
                .profiles
                .iter()
                .map(|profile| profile.definition_id.as_str())
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();
    let mut seen = std::collections::HashSet::new();
    accepted
        .document
        .parts
        .iter()
        .filter(|part| {
            board.part_ids.contains(&part.id) && seen.insert(part.definition_id.as_str())
        })
        .filter(|part| !assigned.contains(part.definition_id.as_str()))
        .filter_map(|part| {
            let definition = accepted
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)?;
            Some(MechanicalProfileTargetChoice {
                definition_id: definition.id.clone(),
                name: definition.name.clone(),
                switch_profile: definition.kind == boardstudio_model::model::PartKind::Switch
                    || definition
                        .generator
                        .as_ref()
                        .is_some_and(|generator| generator.source.ends_with("/switch_choc_v1_v2")),
                imported_geometry: definition.kicad_source.is_some(),
            })
        })
        .collect()
}

pub(super) fn settings_values(
    configuration: &MechanicalConfiguration,
    transport: HardwareTransport,
) -> MechanicalSettingsValues {
    MechanicalSettingsValues {
        board_id: configuration.board_id.clone(),
        transport,
        battery: configuration.battery.clone(),
        suspension_mounts: configuration.mounts.clone(),
        closure_mounts: configuration.closure_mounts.clone(),
        method: configuration.method.clone(),
        mount: configuration.mount.clone(),
        bottom_style: configuration
            .bottom_style
            .clone()
            .unwrap_or(MechanicalBottomStyle::Shell),
        middle_frame: configuration.middle_frame.unwrap_or(false),
        integrated_plate_frame: configuration.integrated_plate_frame,
        plate_thickness: configuration.plate_thickness,
        plate_foam_thickness: configuration.plate_foam_thickness,
        pcb_thickness: configuration.pcb_thickness,
        bottom_foam_thickness: configuration.bottom_foam_thickness,
        bottom_thickness: configuration.bottom_thickness,
        wall_thickness: configuration.wall_thickness,
        clearance: configuration.clearance,
        opening_allowance: configuration.opening_allowance.unwrap_or(0.0),
        openings: configuration.openings.clone().unwrap_or_default(),
        internal_gasket: configuration.internal_gasket.is_some(),
        closure_hardware: configuration
            .internal_gasket
            .as_ref()
            .map(|settings| settings.hardware.clone()),
        hardware: configuration.hardware.clone().unwrap_or_default(),
        critical_fits: configuration.critical_fits.clone().unwrap_or_default(),
        plate_to_pcb: configuration.plate_to_pcb,
        battery_height: configuration.battery_height,
    }
}

pub(super) fn process_targets(
    document: &ProjectDoc,
    configuration: &MechanicalConfiguration,
) -> Vec<MechanicalProcessTarget> {
    let mut targets = ["plate", "plate-foam", "bottom-foam", "bottom"]
        .into_iter()
        .map(|part_id| process_target(configuration, part_id, part_id.replace('-', " ")))
        .collect::<Vec<_>>();
    targets.extend(document.parts.iter().map(|part| {
        let name = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
            .map_or_else(
                || part.definition_id.clone(),
                |definition| definition.name.clone(),
            );
        process_target(
            configuration,
            &part.id,
            format!("{} · {name}", part.reference),
        )
    }));
    targets
}

pub(super) fn process_target(
    configuration: &MechanicalConfiguration,
    part_id: &str,
    name: String,
) -> MechanicalProcessTarget {
    let default_thickness = match part_id {
        "plate" => configuration.plate_thickness,
        "plate-foam" => configuration.plate_foam_thickness,
        "bottom-foam" => configuration.bottom_foam_thickness,
        "bottom" => configuration.bottom_thickness,
        _ => configuration.plate_thickness,
    };
    MechanicalProcessTarget {
        part_id: part_id.to_owned(),
        name,
        default_thickness,
        process: configuration
            .part_processes
            .iter()
            .flatten()
            .find(|process| process.part_id == part_id)
            .cloned(),
    }
}

pub(super) fn stabilizer_fits(
    document: &ProjectDoc,
    configuration: &MechanicalConfiguration,
) -> Vec<MechanicalStabilizerFit> {
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == configuration.board_id)
    else {
        return Vec::new();
    };
    let imported_stabilizers = document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
        .filter(|part| {
            document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
                .and_then(|definition| definition.kicad_source.as_ref())
                .is_some_and(|source| source.source.contains("(footprint \"STAB_MX_"))
        })
        .collect::<Vec<_>>();
    document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
        .filter_map(|part| {
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)?;
            let size = part.keycap.or(definition.keycap)?;
            let is_mx_switch = definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source.to_lowercase() == "ceoloide/switch_mx");
            if !is_mx_switch
                || size.x.max(size.y) < 37.0
                || imported_stabilizers.iter().any(|stabilizer| {
                    (stabilizer.pose.at.x - part.pose.at.x)
                        .hypot(stabilizer.pose.at.y - part.pose.at.y)
                        < 0.01
                })
            {
                return None;
            }
            let key_units = (((size.x.max(size.y) + 1.0) / 19.05 * 4.0).round()) / 4.0;
            let default = MechanicalStabilizerOverride {
                profile: None,
                part_id: part.id.clone(),
                kind: MechanicalStabilizerKind::PcbMount,
                units: if key_units <= 2.75 { 2.0 } else { key_units },
                rotation: Some(if size.y > size.x { 90.0 } else { 0.0 }),
            };
            let mut current = configuration
                .stabilizers
                .iter()
                .flatten()
                .find(|stabilizer| stabilizer.part_id == part.id)
                .cloned()
                .unwrap_or_else(|| default.clone());
            current.units = default.units;
            current.rotation = default.rotation;
            Some(MechanicalStabilizerFit {
                part_id: part.id.clone(),
                reference: part.reference.clone(),
                key_units,
                default,
                current,
            })
        })
        .collect()
}

pub(super) fn mechanical_layer_label(id: &str, internal_gasket: bool) -> String {
    if id == "retainer" && internal_gasket {
        return "Top case".into();
    }
    if id == "pcb" {
        return "PCB".into();
    }
    let label = id.replace('-', " ");
    let mut chars = label.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

pub(super) fn profile_choices(
    accepted: &AcceptedSnapshot,
    configuration: &MechanicalConfiguration,
) -> Vec<MechanicalProfileChoice> {
    let Some(board) = accepted
        .document
        .boards
        .iter()
        .find(|board| board.id == configuration.board_id)
    else {
        return Vec::new();
    };
    configuration
        .profiles
        .iter()
        .map(|profile| {
            let definition = accepted
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == profile.definition_id);
            let placed_switch = accepted.document.parts.iter().any(|part: &Part| {
                board.part_ids.contains(&part.id)
                    && part.definition_id == profile.definition_id
                    && definition.is_some_and(|definition| {
                        definition.kind == PartKind::Switch
                            || definition.generator.as_ref().is_some_and(|generator| {
                                generator.source.ends_with("/switch_choc_v1_v2")
                            })
                    })
            });
            MechanicalProfileChoice {
                definition_id: profile.definition_id.clone(),
                name: definition.map_or_else(
                    || profile.definition_id.clone(),
                    |definition| definition.name.clone(),
                ),
                source: Some(profile.source.clone()),
                family: profile.switch_family,
                plate_to_pcb: Some(profile.plate_to_pcb),
                supported_thickness: profile.supported_thickness,
                switch_family_selectable: placed_switch,
                profile: profile.clone(),
                kicad_source: definition.and_then(|definition| {
                    definition
                        .kicad_source
                        .as_ref()
                        .map(|source| source.source.clone())
                }),
            }
        })
        .collect()
}

pub(super) fn disabled_reason(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    instance_selection: InstanceSelection,
    current: &MechanicalSettingsCurrent,
) -> Option<String> {
    if workspace() != "Case" {
        return Some("Mechanical settings are available in the Case workspace.".into());
    }
    if !instance_selection.is_current(&runtime.model()) {
        return Some(
            "Wait for physical assembly selection before changing mechanical settings.".into(),
        );
    }
    if current.identity.configuration_board_id != current.identity.active_board_id {
        return Some("Show the configured board before changing its mechanical settings.".into());
    }
    if !matches!(
        current.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) {
        return Some(
            "Wait for the accepted document to finish opening before editing mechanical settings."
                .into(),
        );
    }
    if !matches!(
        current.durability,
        Durability::Saved { .. } | Durability::Saving { .. }
    ) {
        return Some("Recover the document before changing mechanical settings.".into());
    }
    let model = runtime.model();
    if model.display_preview.is_some() || model.gesture.is_some() {
        return Some(
            "Finish or cancel the active edit before changing mechanical settings.".into(),
        );
    }
    None
}
