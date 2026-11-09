//! Tickets 017/018 mount the actual Case settings. Only accepted values and callbacks live here.
#[cfg(target_arch = "wasm32")]
use boardstudio_core::model::*;
#[cfg(target_arch = "wasm32")]
use boardstudio_web_case::mechanical_settings::*;
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
fn fixture_battery() -> MechanicalBattery {
    MechanicalBattery {
        cable_width: None,
        size: Vec3 {
            x: 30.0,
            y: 20.0,
            z: 6.0,
        },
        at: Vec2 { x: 12.0, y: -4.0 },
        cable_exit: Vec2 { x: 29.0, y: -4.0 },
    }
}

#[cfg(target_arch = "wasm32")]
fn fixture() -> Element {
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
    let sequence = use_signal(|| 0);
    let mut requests = use_signal(|| 0);
    let mut last = use_signal(String::new);
    let mut editable = use_signal(|| true);
    let mut target_count = use_signal(|| 13);
    let mut selected = use_signal(String::new);
    let mut values = use_signal(|| MechanicalSettingsValues {
        board_id: "board".into(),
        transport: HardwareTransport::Wired,
        battery: Some(fixture_battery()),
        suspension_mounts: vec![Mount {
            id: "mount-1".into(),
            at: Vec2 { x: 2.0, y: 3.0 },
            kind: MountKind::Hole,
            hole_diameter: 3.2,
            boss_diameter: None,
            height: None,
        }],
        closure_mounts: Some(
            (1..=13)
                .map(|index| Mount {
                    id: format!("mount-c{index}"),
                    at: Vec2 {
                        x: f64::from(index) * 2.0,
                        y: 3.0,
                    },
                    kind: MountKind::Hole,
                    hole_diameter: 3.2,
                    boss_diameter: None,
                    height: None,
                })
                .collect(),
        ),
        method: PlateMethod::Printed,
        mount: MechanicalMount::Rigid,
        bottom_style: MechanicalBottomStyle::Sheet,
        middle_frame: false,
        integrated_plate_frame: false,
        plate_thickness: 1.5,
        plate_foam_thickness: 0.0,
        pcb_thickness: 1.6,
        bottom_foam_thickness: 0.0,
        bottom_thickness: 2.0,
        wall_thickness: 2.0,
        clearance: 0.2,
        opening_allowance: 0.0,
        openings: vec![MechanicalOpening {
            id: "opening-1".into(),
            vertices: vec![
                MechanicalOpeningVertex {
                    id: "vertex-1".into(),
                    point: Vec2 { x: 0.0, y: 0.0 },
                },
                MechanicalOpeningVertex {
                    id: "vertex-2".into(),
                    point: Vec2 { x: 10.0, y: 0.0 },
                },
                MechanicalOpeningVertex {
                    id: "vertex-3".into(),
                    point: Vec2 { x: 10.0, y: 5.0 },
                },
                MechanicalOpeningVertex {
                    id: "vertex-4".into(),
                    point: Vec2 { x: 0.0, y: 5.0 },
                },
            ],
            z: 0.0,
            height: 10.0,
            next_vertex_identity: 0,
        }],
        internal_gasket: true,
        closure_hardware: Some(InternalClosureHardware {
            drive: ScrewDrive::Hex,
            installation: InsertInstallation::HeatSet,
            thread_diameter: 2.0,
            pitch: 0.4,
            insert_length: 3.0,
            thread_start: 0.5,
            tip_allowance: 0.5,
            seat_lead_depth: 1.0,
            seat_lead_diameter: 2.2,
            bearing_thickness: 1.0,
            fixed_length: None,
            length_datum: ScrewLengthDatum::UnderHead,
            head_profile: ScrewHeadProfile::Flat,
            id: "closure-1".into(),
            thread: "M2".into(),
            screw_lengths: vec![3.0, 4.0],
            head_diameter: 3.8,
            head_height: 2.0,
            hole_diameter: 2.2,
            insert_diameter: 3.2,
            seat_diameter: 4.0,
            seat_depth: 1.0,
            engagement: 2.5,
            bottoming_clearance: 0.5,
            roof: 1.0,
            surround: 1.5,
        }),
        hardware: vec![MechanicalHardwareSpecification {
            id: "hw-1".into(),
            part_id: "bottom".into(),
            feature_id: "mount-c1".into(),
            designation: "M2x4".into(),
            thread: "M2".into(),
            length: 4.0,
            quantity: 4,
            notes: None,
            tolerance: None,
        }],
        critical_fits: vec![MechanicalCriticalFit {
            id: "fit-1".into(),
            part_id: "plate".into(),
            label: "Plate gap".into(),
            from: Vec2 { x: 0.0, y: 0.0 },
            to: Vec2 { x: 10.0, y: 0.0 },
            tolerance: "±0.1".into(),
        }],
        plate_to_pcb: 3.5,
        battery_height: 0.0,
    });
    let mut profile = use_signal(|| MechanicalPartProfile {
        definition_id: "custom".into(),
        source: "KiCad fixture".into(),
        switch_family: None,
        cutouts: vec![vec![
            Vec2 { x: -2.5, y: -2.5 },
            Vec2 { x: 2.5, y: -2.5 },
            Vec2 { x: 2.5, y: 2.5 },
            Vec2 { x: -2.5, y: 2.5 },
        ]],
        plate_to_pcb: 3.5,
        supported_thickness: None,
        clearances: None,
        pcb_holes: None,
        source_geometry: None,
        openings: Some(vec![CaseOpening {
            points: vec![
                Vec2 { x: -2.5, y: -2.5 },
                Vec2 { x: 2.5, y: -2.5 },
                Vec2 { x: 2.5, y: 2.5 },
                Vec2 { x: -2.5, y: 2.5 },
            ],
            z: 0.0,
            height: 10.0,
        }]),
        clearance_volumes: None,
    });
    let identity = MechanicalSettingsIdentity {
        editor_instance_id: 1,
        scope_generation: 1,
        presentation_generation: 1,
        scope: Scope {
            session_epoch: SessionEpoch(1),
            document_id: "fixture".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        snapshot_token: SnapshotToken(1),
        revision: 1,
        active_board_id: "board".into(),
        configuration_board_id: "board".into(),
    };
    let extraction = MechanicalProfileExtractionPort(Rc::new(|_, mappings| {
        Box::pin(async move {
            let request = ArtifactRequest::ExtractMechanical {
            id: "017-real-core-extraction".into(),
            source: r#"(footprint "Custom" (fp_rect (start -9.5 9.5) (end 9.5 -9.5) (stroke (width 0.1) (type solid)) (fill none) (layer "Dwgs.User")))"#.into(),
            mappings, max_deviation_mm: 0.005,
        };
            match serde_json::from_str::<ArtifactReply>(&boardstudio_core::artifact_request(
                &serde_json::to_string(&request).unwrap(),
            ))
            .unwrap()
            {
                ArtifactReply::ExtractMechanical { result, .. } => Ok(result),
                ArtifactReply::Error { error, .. } => Err(error.message),
                other => panic!("unexpected extraction: {other:?}"),
            }
        })
    }));
    rsx! {
        boardstudio_web_ui_shared::ui::button::Button { onclick: move |_| editable.set(!editable()), "Toggle editing" }
        boardstudio_web_ui_shared::ui::button::Button { onclick: move |_| target_count.set(12), "Twelve profile targets" }
        boardstudio_web_ui_shared::ui::button::Button { onclick: move |_| values.with_mut(|values| values.battery = if values.battery.is_some() { None } else { Some(fixture_battery()) }), "Toggle battery" }
        output { role: "status", aria_label: "Requests", "{requests}" }
        output { role: "status", aria_label: "Last request", "{last}" }
        output { role: "status", aria_label: "Accepted plate thickness", "{values().plate_thickness}" }
        output { role: "status", aria_label: "Selected layer", "{selected}" }
        div { class: "flex-1 min-w-0 min-h-0 overflow-auto overscroll-contain bg-card max-[760px]:overflow-visible max-[760px]:overscroll-auto",
        MechanicalSettings {
            identity, request_sequence: sequence, values: Some(values()),
            profiles: Rc::from([MechanicalProfileChoice {
                definition_id: "custom".into(), name: "Custom profile".into(), source: Some("KiCad fixture".into()),
                family: None, plate_to_pcb: Some(3.5), supported_thickness: None,
                switch_family_selectable: true, profile: profile(), kicad_source: Some("fixture".into()),
            }]),
            profile_targets: Rc::from((1..=target_count()).map(|i|MechanicalProfileTargetChoice {
                definition_id:format!("part-{i}"),name:format!("Part {i}"),switch_profile:true,imported_geometry:true,
            }).collect::<Vec<_>>()),
            extract_profile: Some(extraction),
            process_targets: Rc::from([
                MechanicalProcessTarget {
                    part_id: "plate".into(),
                    name: "Plate".into(),
                    default_thickness: 1.5,
                    process: Some(MechanicalPartProcess {
                        part_id: "plate".into(),
                        method: PlateMethod::Printed,
                        material: "PLA".into(),
                        thickness: 1.5,
                        constraints_version: String::new(),
                    }),
                },
                MechanicalProcessTarget {
                    part_id: "bottom".into(),
                    name: "Bottom".into(),
                    default_thickness: 2.0,
                    process: None,
                },
            ]),
            stabilizer_fits: Rc::from([MechanicalStabilizerFit {
                part_id: "plate".into(),
                reference: "Space".into(),
                key_units: 6.25,
                default: MechanicalStabilizerOverride {
                    profile: None,
                    part_id: "plate".into(),
                    kind: MechanicalStabilizerKind::PcbMount,
                    units: 6.25,
                    rotation: None,
                },
                current: MechanicalStabilizerOverride {
                    profile: None,
                    part_id: "plate".into(),
                    kind: MechanicalStabilizerKind::PcbMount,
                    units: 6.25,
                    rotation: None,
                },
            }]),
            layers: Rc::from([
                MechanicalLayerRow { id:"fixture-layer".into(),label:"Fixture layer".into(),z:0.0,thickness:1.5,resolved_body_thickness:None,is_previous:false },
                MechanicalLayerRow { id:"second-layer".into(),label:"Second layer".into(),z:2.0,thickness:1.6,resolved_body_thickness:None,is_previous:false },
            ]),
            gasket_supports: Rc::from([]),fit_parts:Rc::from([]),fit_parts_resolved:false,
            hardware_mounts:Rc::from([]),suggested_mounts:Rc::from([]),
            findings: Rc::from([MechanicalFindingRow {
                id: "finding-1".into(),
                severity: Severity::Error,
                message: "Fixture clearance error".into(),
            }]),
            selected_layer:selected(),mismatch:None,editable:editable(),disabled_reason:None,
            feedback:Rc::from([]),summary_feedback:None,
            on_select_layer:move |id| selected.set(id), on_show_finding:move |_| {},
            on_request: move |request:MechanicalSettingsRequest| {
                requests += 1;
                last.set(format!("{:?}",request.patch));
                values.with_mut(|values| match request.patch.clone() {
                    MechanicalSettingsPatch::SetMethod(v)=> values.method=v,
                    MechanicalSettingsPatch::SetMount(v)=> values.mount=v,
                    MechanicalSettingsPatch::SetBottomStyle(v)=> values.bottom_style=v,
                    MechanicalSettingsPatch::SetMiddleFrame(v)=> values.middle_frame=v,
                    MechanicalSettingsPatch::SetIntegratedPlateFrame(v)=> values.integrated_plate_frame=v,
                    MechanicalSettingsPatch::SetDimension{field:MechanicalDimension::PlateThickness,value} => values.plate_thickness=value,
                    MechanicalSettingsPatch::SetBatteryEnabled(true)=> values.battery = Some(fixture_battery()),
                    MechanicalSettingsPatch::SetBatteryEnabled(false)=> values.battery = None,
                    MechanicalSettingsPatch::UpdateProfile(next)=>profile.set(next),
                    _=>{},
                });
            },
        }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus_web::launch::launch_virtual_dom(VirtualDom::new(fixture), dioxus_web::Config::new());
}
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    panic!("Case settings fixture requires wasm32");
}
