//! Switch fit profiles with custom geometry editing, and manufacturing overrides.
use crate::mechanical_projection::{
    MechanicalProcessTarget, MechanicalProfileChoice, MechanicalProfileTargetChoice,
    MechanicalStabilizerFit, process_material_options,
};
use crate::mechanical_settings::{
    Fields, MechanicalDimension, MechanicalSettingsPatch, Sender, dimension, method_options,
};
use boardstudio_model::model::{
    CaseOpening, MechanicalBuiltinProfile, MechanicalExtraction, MechanicalGeometry,
    MechanicalPartProfile, MechanicalPurpose, MechanicalPurposeMapping, MechanicalStabilizerKind,
    MechanicalSwitchFamily, Vec2,
};
use boardstudio_web_ui_shared::inspector_fields::{InspectorSection, NumericField, SelectField};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonSize, ButtonVariant},
    callout::{Callout, CalloutVariant},
    checkbox::{Checkbox, CheckboxState},
};
use dioxus::prelude::*;
use std::{future::Future, pin::Pin, rc::Rc};

pub type ExtractionFuture = Pin<Box<dyn Future<Output = Result<MechanicalExtraction, String>>>>;

/// Reads a KiCad part's geometry for the given purpose mappings; the first argument is the definition id.
#[derive(Clone)]
pub struct MechanicalProfileExtractionPort(
    pub Rc<dyn Fn(String, Vec<MechanicalPurposeMapping>) -> ExtractionFuture>,
);

impl PartialEq for MechanicalProfileExtractionPort {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

const FAMILIES: [(MechanicalSwitchFamily, &str); 3] = [
    (MechanicalSwitchFamily::Mx, "MX"),
    (MechanicalSwitchFamily::ChocV1, "Choc v1"),
    (MechanicalSwitchFamily::ChocV2, "Choc v2"),
];
fn constraint_version(version: &str) -> &str {
    if version.is_empty() {
        CONSTRAINT_VERSION
    } else {
        version
    }
}

const CONSTRAINT_VERSION: &str = "2026-09-24";

fn family_options() -> Vec<(Option<MechanicalSwitchFamily>, String)> {
    FAMILIES
        .iter()
        .map(|(family, label)| (Some(*family), (*label).to_owned()))
        .collect()
}

fn family_label(family: MechanicalSwitchFamily) -> &'static str {
    FAMILIES
        .iter()
        .find(|(candidate, _)| *candidate == family)
        .map_or("", |(_, label)| label)
}

const PURPOSES: [(MechanicalPurpose, &str); 4] = [
    (MechanicalPurpose::PlateCutout, "Plate cutout"),
    (
        MechanicalPurpose::ElectricalPcbMountingHole,
        "PCB mounting hole",
    ),
    (MechanicalPurpose::ClearanceEnvelope, "Clearance envelope"),
    (MechanicalPurpose::DrawingGuide, "Drawing guide"),
];

#[component]
pub(crate) fn ProfileSection(
    profiles: Rc<[MechanicalProfileChoice]>,
    targets: Rc<[MechanicalProfileTargetChoice]>,
    extract: Option<MechanicalProfileExtractionPort>,
    plate_thickness: f64,
    plate_to_pcb: f64,
    editable: bool,
    send: Sender,
) -> Element {
    let mut family = use_signal(|| None::<MechanicalSwitchFamily>);
    let mut stabilizer = use_signal(|| None::<MechanicalBuiltinProfile>);
    let switch_targets: Vec<_> = targets
        .iter()
        .filter(|target| target.switch_profile)
        .map(|target| (target.definition_id.clone(), target.name.clone()))
        .collect();
    rsx! {
        Fields { editable,
            if profiles.is_empty() {
                Callout { "No explicit switch fit profile is assigned. The supported plate thickness range is unavailable until a fit profile is assigned in Parts." }
                p { "Plate underside to PCB top: {plate_to_pcb:.2} mm · current configured gap." }
            }
            for profile in profiles.iter() {
                article { key: "{profile.definition_id}", class: "grid gap-3 rounded-md bg-card p-3", role: "group", aria_label: "Fit profile {profile.name}",
                    strong { "{profile.name}" }
                    p { "Profile source: {profile.source}." }
                    if profile.switch_family_selectable {
                        SelectField::<Option<MechanicalSwitchFamily>> { label: "Switch fit family", placeholder: "Choose switch family…",
                            value: profile.family, options: family_options(),
                            on_commit: { let id = profile.definition_id.clone(); move |family: Option<MechanicalSwitchFamily>| if let Some(family) = family { send.send(MechanicalSettingsPatch::SetSwitchFamily { definition_id: id.clone(), family }) } } }
                    } else if let Some(family) = profile.family {
                        p { "Switch fit: {family_label(family)}" }
                    }
                    p { "Plate underside to PCB top: {profile.plate_to_pcb:.2} mm · derived from switch fit." }
                    if let Some(range) = profile.supported_thickness {
                        p { "Supported plate thickness: {range.x:.2}–{range.y:.2} mm" }
                    } else {
                        div { role: "status", Callout { variant: CalloutVariant::Warning, "Supplier review needed for the supported thickness range." } }
                    }
                    if profile.family.is_none() { div { role: "status", Callout { "Choose a supported switch family to resolve the plate gap." } } }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Remove profile {profile.name}",
                        onclick: { let id = profile.definition_id.clone(); move |_| send.send(MechanicalSettingsPatch::RemoveProfile { definition_id: id.clone() }) }, "Remove profile" }
                    InspectorSection { title: "Profile geometry", default_open: false,
                        ProfileGeometryEditor { profile: profile.profile.clone(), extract: extract.clone().filter(|_| profile.has_kicad_source), editable, send }
                    }
                }
            }
            if !switch_targets.is_empty() {
                SelectField::<Option<MechanicalSwitchFamily>> { label: "Library fit profile", placeholder: "Choose a switch family…", value: family(), options: family_options(),
                    on_commit: move |next| family.set(next) }
                SelectField::<String> { label: "Assign library fit profile to", placeholder: "Choose a placed switch type…", value: String::new(), disabled: family().is_none(), options: switch_targets.clone(),
                    on_commit: move |definition_id: String| if let Some(family) = family() { send.send(MechanicalSettingsPatch::AssignSwitchProfile { definition_id, family }) } }
                SelectField::<Option<MechanicalBuiltinProfile>> { label: "Library stabilizer profile", placeholder: "Choose a stabilizer size…", value: stabilizer(),
                    options: vec![(Some(MechanicalBuiltinProfile::MxStab2u), "MX stabilizer · 2u".into()), (Some(MechanicalBuiltinProfile::MxStab625u), "MX stabilizer · 6.25u".into())],
                    on_commit: move |next| stabilizer.set(next) }
                SelectField::<String> { label: "Assign library stabilizer profile to", placeholder: "Choose a placed switch type…", value: String::new(), disabled: stabilizer().is_none(),
                    options: targets.iter().map(|target| (target.definition_id.clone(), target.name.clone())).collect::<Vec<_>>(),
                    on_commit: move |definition_id: String| if let Some(source) = stabilizer() { send.send(MechanicalSettingsPatch::AssignStabilizerProfile { definition_id, source }) } }
            }
            if targets.iter().any(|target| target.imported_geometry) {
                SelectField::<String> { label: "Custom KiCad geometry", aria_label: "Assign custom geometry profile to", placeholder: "Choose imported part type…", value: String::new(),
                    options: targets.iter().filter(|target| target.imported_geometry).map(|target| (target.definition_id.clone(), target.name.clone())).collect::<Vec<_>>(),
                    on_commit: move |definition_id: String| send.send(MechanicalSettingsPatch::AssignImportedGeometryProfile { definition_id }) }
            }
            p { "Current plate thickness: {plate_thickness:.2} mm." }
        }
    }
}

/// Custom KiCad layer extraction plus hand-edited cutouts, clearances and volumes.
#[component]
fn ProfileGeometryEditor(
    profile: MechanicalPartProfile,
    extract: Option<MechanicalProfileExtractionPort>,
    editable: bool,
    send: Sender,
) -> Element {
    let mut geometry = use_signal(|| None::<MechanicalGeometry>);
    let mut selected = use_signal(Vec::<String>::new);
    let mut purposes = use_signal(Vec::<MechanicalPurposeMapping>::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(String::new);
    let update = move |change: Rc<dyn Fn(&mut MechanicalPartProfile)>,
                       base: MechanicalPartProfile| {
        let mut next = base;
        change(&mut next);
        send.send(MechanicalSettingsPatch::UpdateProfile(next));
    };
    rsx! {
        section { class: "grid gap-3", aria_label: "Profile geometry",
            if let Some(port) = extract {
                section { aria_label: "Custom KiCad geometry",
                    Button { variant: ButtonVariant::Secondary, aria_label: "Read KiCad layers", disabled: !editable || busy(),
                        onclick: { let (port, id, existing) = (port.clone(), profile.definition_id.clone(), profile.source_geometry.clone()); move |_| {
                            busy.set(true);
                            error.set(String::new());
                            let (port, id, existing) = (port.clone(), id.clone(), existing.clone());
                            spawn(async move {
                                match (port.0)(id, Vec::new()).await {
                                    Ok(result) => {
                                        selected.set(existing.as_ref().map_or_else(Vec::new, |source| source.source_ids.clone()));
                                        purposes.set(existing.as_ref().map_or_else(|| result.source_geometry.mappings.clone(), |source| source.mappings.clone()));
                                        geometry.set(Some(result.geometry));
                                    }
                                    Err(message) => error.set(message),
                                }
                                busy.set(false);
                            });
                        } },
                        if geometry().is_some() { "Reload layers" } else { "Read KiCad layers" } }
                    if let Some(loaded) = geometry() {
                        p { class: "text-muted-foreground text-sm", "Select exact primitives for plate cutouts, PCB holes, clearances or drawing guides, then apply." }
                        for primitive in loaded.primitives.iter() {
                            div { key: "{primitive.id}", role: "group", aria_label: "Geometry {primitive.id}", class: "grid gap-1",
                                label { class: "flex items-center gap-2",
                                    Checkbox { aria_label: "Select geometry {primitive.id}", disabled: !editable || busy(),
                                        checked: Some(if selected().contains(&primitive.id) { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                                        on_checked_change: { let id = primitive.id.clone(); move |state: CheckboxState| {
                                            let mut ids = selected();
                                            ids.retain(|entry| *entry != id);
                                            if bool::from(state) { ids.push(id.clone()); }
                                            selected.set(ids);
                                        } } }
                                    span { strong { "{primitive.layer.as_deref().unwrap_or(\"geometry\")}" } small { " {primitive.kind:?} · {primitive.source_group_id}" } }
                                }
                                SelectField::<MechanicalPurpose> { label: "Purpose", aria_label: "Purpose for {primitive.layer.as_deref().unwrap_or(\"geometry\")}",
                                    value: purposes().iter().find(|mapping| mapping.source_id.as_deref() == Some(primitive.id.as_str())).map_or(MechanicalPurpose::PlateCutout, |mapping| mapping.purpose),
                                    options: PURPOSES.iter().map(|(purpose, label)| (*purpose, (*label).to_owned())).collect::<Vec<_>>(),
                                    on_commit: { let id = primitive.id.clone(); move |purpose| {
                                        let mut entries = purposes();
                                        entries.retain(|mapping| mapping.source_id.as_deref() != Some(id.as_str()));
                                        entries.push(MechanicalPurposeMapping { source_id: Some(id.clone()), kind: None, layer: None, purpose });
                                        purposes.set(entries);
                                    } } }
                            }
                        }
                        Button { variant: ButtonVariant::Secondary, aria_label: "Apply selected geometry", disabled: !editable || busy() || selected().is_empty(),
                            onclick: { let (port, profile) = (port.clone(), profile.clone()); move |_| {
                                let ids = selected();
                                let mappings = geometry().iter().flat_map(|geometry| geometry.primitives.iter()).filter(|primitive| ids.contains(&primitive.id)).map(|primitive| {
                                    let purpose = purposes().iter().find(|mapping| mapping.source_id.as_deref() == Some(primitive.id.as_str())).map_or(MechanicalPurpose::PlateCutout, |mapping| mapping.purpose);
                                    MechanicalPurposeMapping { source_id: Some(primitive.id.clone()), kind: None, layer: None, purpose }
                                }).collect::<Vec<_>>();
                                busy.set(true);
                                error.set(String::new());
                                let (port, profile) = (port.clone(), profile.clone());
                                spawn(async move {
                                    match (port.0)(profile.definition_id.clone(), mappings).await {
                                        Ok(result) => update(Rc::new(move |next: &mut MechanicalPartProfile| {
                                            next.cutouts = result.plate_cutouts.clone();
                                            next.clearances = Some(result.clearance_envelopes.clone());
                                            next.pcb_holes = Some(result.pcb_holes.clone());
                                            next.source_geometry = Some(result.source_geometry.clone());
                                        }), profile),
                                        Err(message) => error.set(message),
                                    }
                                    busy.set(false);
                                });
                            } }, "Apply selected geometry" }
                    }
                    if !error().is_empty() { Alert { variant: AlertVariant::Destructive, "{error()}" } }
                }
            }
            ContourList { title: "Plate cutout geometry", contours: profile.cutouts.clone(), editable,
                on_change: { let profile = profile.clone(); move |cutouts: Vec<Vec<Vec2>>| update(Rc::new(move |next: &mut MechanicalPartProfile| next.cutouts = cutouts.clone()), profile.clone()) } }
            ContourList { title: "Component clearance zones", contours: profile.clearances.clone().unwrap_or_default(), editable,
                on_change: { let profile = profile.clone(); move |clearances: Vec<Vec<Vec2>>| update(Rc::new(move |next: &mut MechanicalPartProfile| next.clearances = Some(clearances.clone())), profile.clone()) } }
            VolumeList { title: "Access openings", volumes: profile.openings.clone().unwrap_or_default(), editable,
                on_change: { let profile = profile.clone(); move |openings: Vec<CaseOpening>| update(Rc::new(move |next: &mut MechanicalPartProfile| next.openings = Some(openings.clone())), profile.clone()) } }
            VolumeList { title: "Clearance volumes", volumes: profile.clearance_volumes.clone().unwrap_or_default(), editable,
                on_change: { let profile = profile.clone(); move |volumes: Vec<CaseOpening>| update(Rc::new(move |next: &mut MechanicalPartProfile| next.clearance_volumes = Some(volumes.clone())), profile.clone()) } }
        }
    }
}

fn default_square() -> Vec<Vec2> {
    vec![
        Vec2 { x: -2.5, y: -2.5 },
        Vec2 { x: 2.5, y: -2.5 },
        Vec2 { x: 2.5, y: 2.5 },
        Vec2 { x: -2.5, y: 2.5 },
    ]
}

/// One polygon's vertices; removing is refused below three vertices.
#[component]
fn PointsEditor(
    label: String,
    points: Vec<Vec2>,
    editable: bool,
    on_change: EventHandler<Vec<Vec2>>,
) -> Element {
    rsx! {
        for (index, point) in points.iter().copied().enumerate() {
            div { key: "{index}", class: "flex items-end gap-2",
                for (axis, value) in [("X", point.x), ("Y", point.y)] {
                    NumericField { key: "{axis}", label: axis, aria_label: format!("{label} vertex {} {axis}", index + 1), unit: "mm", step: 0.1, value,
                        on_commit: { let mut points = points.clone(); move |value| { if axis == "X" { points[index].x = value } else { points[index].y = value } on_change.call(points.clone()) } } }
                }
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Remove {label} vertex {index + 1}", disabled: !editable || points.len() <= 3,
                    onclick: { let mut points = points.clone(); move |_| { points.remove(index); on_change.call(points.clone()) } }, "Remove vertex" }
            }
        }
        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Add vertex to {label}", disabled: !editable,
            onclick: { let mut points = points.clone(); move |_| { points.push(Vec2 { x: 0.0, y: 0.0 }); on_change.call(points.clone()) } }, "Add vertex" }
    }
}

#[component]
fn ContourList(
    title: &'static str,
    contours: Vec<Vec<Vec2>>,
    editable: bool,
    on_change: EventHandler<Vec<Vec<Vec2>>>,
) -> Element {
    rsx! {
        section { aria_label: "{title}", class: "grid gap-2",
            strong { "{title}" }
            Button { variant: ButtonVariant::Secondary, size: ButtonSize::Sm, aria_label: "Add polygon to {title}", disabled: !editable,
                onclick: { let mut contours = contours.clone(); move |_| { contours.push(default_square()); on_change.call(contours.clone()) } }, "Add polygon" }
            if contours.is_empty() { Callout { "No contours assigned." } }
            for (index, contour) in contours.iter().cloned().enumerate() {
                fieldset { key: "{index}", aria_label: "{title} contour {index + 1}", class: "grid gap-2", disabled: !editable,
                    strong { "Contour {index + 1}" }
                    PointsEditor { label: format!("{title} contour {}", index + 1), points: contour, editable,
                        on_change: { let contours = contours.clone(); move |points| { let mut next = contours.clone(); next[index] = points; on_change.call(next) } } }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Remove contour {index + 1} from {title}",
                        onclick: { let mut contours = contours.clone(); move |_| { contours.remove(index); on_change.call(contours.clone()) } }, "Remove" }
                }
            }
        }
    }
}

#[component]
fn VolumeList(
    title: &'static str,
    volumes: Vec<CaseOpening>,
    editable: bool,
    on_change: EventHandler<Vec<CaseOpening>>,
) -> Element {
    rsx! {
        section { aria_label: "{title}", class: "grid gap-2",
            strong { "{title}" }
            Button { variant: ButtonVariant::Secondary, size: ButtonSize::Sm, aria_label: "Add volume to {title}", disabled: !editable,
                onclick: { let mut volumes = volumes.clone(); move |_| { volumes.push(CaseOpening { points: default_square(), z: 0.0, height: 10.0 }); on_change.call(volumes.clone()) } }, "Add volume" }
            if volumes.is_empty() { Callout { "No volumes configured." } }
            for (index, volume) in volumes.iter().cloned().enumerate() {
                fieldset { key: "{index}", aria_label: "{title} volume {index + 1}", class: "grid gap-2", disabled: !editable,
                    strong { "Volume {index + 1}" }
                    for (label, field, value) in [("Bottom Z", MechanicalDimension::OpeningBottomZ, volume.z), ("Height", MechanicalDimension::OpeningHeight, volume.height)] {
                        {dimension(label, field, value, { let volumes = volumes.clone(); move |value| {
                            let mut next = volumes.clone();
                            if field == MechanicalDimension::OpeningBottomZ { next[index].z = value } else { next[index].height = value }
                            on_change.call(next)
                        } })}
                    }
                    PointsEditor { label: format!("{title} volume {}", index + 1), points: volume.points, editable,
                        on_change: { let volumes = volumes.clone(); move |points| { let mut next = volumes.clone(); next[index].points = points; on_change.call(next) } } }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Remove {title} volume {index + 1}",
                        onclick: { let mut volumes = volumes.clone(); move |_| { volumes.remove(index); on_change.call(volumes.clone()) } }, "Remove" }
                }
            }
        }
    }
}

#[component]
pub(crate) fn ManufacturingSection(
    targets: Rc<[MechanicalProcessTarget]>,
    stabilizers: Rc<[MechanicalStabilizerFit]>,
    editable: bool,
    send: Sender,
) -> Element {
    rsx! {
        Fields { editable,
            h4 { "Per-part process overrides" }
            for target in targets.iter() {
                article { key: "{target.part_id}", class: "grid gap-1.5",
                    strong { "{target.name}" }
                    if let Some(process) = target.process.clone() {
                        if target.part_id.ends_with("foam") {
                            output { "Cut sheet" }
                        } else {
                            SelectField::<boardstudio_model::model::PlateMethod> { label: "Method", value: process.method.clone(), options: method_options(),
                                on_commit: { let id = target.part_id.clone(); move |method| send.send(MechanicalSettingsPatch::SetProcessMethod { part_id: id.clone(), method }) } }
                        }
                        SelectField::<String> { label: "Material", value: process.material.clone(),
                            options: process_material_options(&target.part_id, &process.method).iter().map(|material| ((*material).to_owned(), (*material).to_owned())).collect::<Vec<_>>(),
                            on_commit: { let id = target.part_id.clone(); move |material| send.send(MechanicalSettingsPatch::SetProcessMaterial { part_id: id.clone(), material }) } }
                        {dimension("Finished thickness", MechanicalDimension::PlateThickness, if process.thickness >= 0.0 { process.thickness } else { target.default_thickness },
                            { let id = target.part_id.clone(); move |thickness| send.send(MechanicalSettingsPatch::SetProcessThickness { part_id: id.clone(), thickness }) })}
                        p { class: "text-muted-foreground text-sm", "Constraint set " code { "{constraint_version(&process.constraints_version)}" } }
                        if !process.constraints_version.is_empty() && process.constraints_version != CONSTRAINT_VERSION {
                            Alert { variant: AlertVariant::Destructive, "Unsupported constraint set. Diagnostics will block export." }
                        }
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Remove process override for {target.name}",
                            onclick: { let id = target.part_id.clone(); move |_| send.send(MechanicalSettingsPatch::RemoveProcessOverride { part_id: id.clone() }) }, "Remove" }
                    } else {
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Add process override for {target.name}",
                            onclick: { let id = target.part_id.clone(); move |_| send.send(MechanicalSettingsPatch::AddProcessOverride { part_id: id.clone() }) }, "Add process override" }
                    }
                }
            }
            p { class: "text-muted-foreground text-sm", "Supported constraint set: {CONSTRAINT_VERSION}." }
            h4 { "Stabilizer fit · {stabilizers.len()} wide keys" }
            for stabilizer in stabilizers.iter() {
                article { key: "{stabilizer.part_id}", class: "grid gap-1.5",
                    strong { "{stabilizer.reference} · {stabilizer.key_units}u" }
                    SelectField::<MechanicalStabilizerKind> { label: "Stabilizer", aria_label: "Stabilizer for {stabilizer.reference}", value: stabilizer.current.kind.clone(),
                        options: {
                            let mut options = vec![(MechanicalStabilizerKind::PcbMount, format!("Cherry PCB mount · {}u", stabilizer.default.units)), (MechanicalStabilizerKind::None, "None".into())];
                            if stabilizer.current.profile.is_some() { options.push((MechanicalStabilizerKind::PlateMount, "Custom plate mount".into())); }
                            options
                        },
                        on_commit: { let current = stabilizer.current.clone(); move |kind| { let mut next = current.clone(); next.kind = kind; send.send(MechanicalSettingsPatch::SetStabilizerKind(next)) } } }
                    p { class: "text-muted-foreground text-sm", "Size and orientation follow the key layout." }
                }
            }
            if stabilizers.is_empty() { p { class: "text-muted-foreground text-sm", "No keys in this layout need stabilizers." } }
        }
    }
}
