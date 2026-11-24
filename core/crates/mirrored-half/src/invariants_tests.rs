use crate::model::{Board, Part, Pose2, ProjectDoc, Side, Vec2};
use crate::{MirrorDestination, MirroredPair, project_document};

fn split_document() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("project", "Project");
    for id in ["source", "destination"] {
        doc.boards.push(Board {
            id: id.into(),
            name: id.into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
    }
    doc.parts.push(Part {
        id: "authored".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 2.0, y: 3.0 },
            rotation: 15.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        anchor: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    doc.boards[0].part_ids.push("authored".into());
    doc.mirrored_pairs.push(MirroredPair {
        id: "split".into(),
        source: MirrorDestination::SecondBoard("source".into()),
        destination: MirrorDestination::SecondBoard("destination".into()),
        mapping: Pose2 {
            at: Vec2 { x: 10.0, y: 0.0 },
            rotation: 0.0,
        },
        overrides: vec![],
    });

    doc
}

#[test]
fn board_projection_is_repeatable_and_keeps_source_canonical() {
    let doc = split_document();
    let resolved = project_document(&doc);
    assert_eq!(resolved, project_document(&doc));
    let reflected = &resolved.geometry().parts[1];
    assert_eq!(reflected.pose.at, Vec2 { x: 8.0, y: 3.0 });
    assert_eq!(reflected.pose.rotation, -15.0);
    assert_eq!(
        resolved.geometry().boards[1].part_ids,
        std::slice::from_ref(&reflected.id)
    );
    assert_eq!(doc.parts.len(), 1);
    assert!(doc.boards[1].part_ids.is_empty());
}

#[test]
fn layout_projection_local_omission_rejoin_and_unlink_use_the_same_geometry() {
    use crate::model::{HalfExceptionState, Layout};
    use crate::{set_exception, unlink_pair};

    let mut doc = split_document();
    doc.boards.pop();
    for id in ["left", "right"] {
        doc.layouts.push(Layout {
            id: id.into(),
            name: id.into(),
            board_id: "source".into(),
            matrix_ids: vec![],
            part_ids: if id == "left" {
                vec!["authored".into()]
            } else {
                vec![]
            },
            mirror_link: None,
        });
    }
    let pair = &mut doc.mirrored_pairs[0];
    pair.source = MirrorDestination::SameBoardLayout("left".into());
    pair.destination = MirrorDestination::SameBoardLayout("right".into());
    let target = "5/split/part/authored";
    let expected = project_document(&doc).geometry().parts[1].clone();
    assert_eq!(expected.id, target);
    set_exception(&mut doc, "split", target, HalfExceptionState::Omitted).unwrap();
    assert_eq!(project_document(&doc).geometry().parts.len(), 1);
    set_exception(&mut doc, "split", target, HalfExceptionState::Linked).unwrap();
    assert_eq!(project_document(&doc).geometry().parts[1], expected);
    set_exception(&mut doc, "split", target, HalfExceptionState::Local).unwrap();
    doc.parts[0].pose.at.x = 40.0;
    assert_eq!(project_document(&doc).geometry().parts[1], expected);
    doc.parts.clear();
    doc.layouts[0].part_ids.clear();
    doc.boards[0].part_ids.clear();
    assert_eq!(project_document(&doc).geometry().parts[0], expected);
    unlink_pair(&mut doc, "split").unwrap();
    assert!(doc.mirrored_pairs.is_empty());
    assert_eq!(doc.parts, [expected]);
    assert_eq!(doc.layouts[1].part_ids, [target]);
}

#[test]
fn reversible_split_shares_its_board_without_projecting_extra_parts() {
    use crate::create_half;
    use crate::model::{HardwareConfiguration, MirroredHalfMode, PhysicalBoardInstance};

    let mut doc = split_document();
    doc.mirrored_pairs.clear();
    doc.boards.pop();
    doc.hardware = Some(HardwareConfiguration {
        instances: vec![PhysicalBoardInstance {
            id: "left".into(),
            name: "Left".into(),
            board_id: "source".into(),
            half: "left".into(),
            role: "primary".into(),
            flipped: false,
            controller_part_id: None,
            mechanical: None,
            construction_ownership: Default::default(),
            mechanical_identity_reservation: None,
        }],
        ..Default::default()
    });
    create_half(
        &mut doc,
        "split",
        "left",
        &MirroredHalfMode::ReuseBoard {
            instance_id: "right".into(),
        },
        None,
        None,
        Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
    )
    .unwrap();

    let resolved = project_document(&doc);
    let geometry = resolved.geometry();
    assert_eq!(geometry.parts.len(), 1);
    assert_eq!(geometry.boards.len(), 1);
    let instances = &geometry.hardware.as_ref().unwrap().instances;
    assert_eq!(instances.len(), 2);
    assert_eq!(instances[0].board_id, instances[1].board_id);
    assert_ne!(instances[0].flipped, instances[1].flipped);
}

// Deterministic seeded operation-sequence fuzzer over mirrored-pair
// workflows: 32 seeds x 40 steps across two-board and same-board layouts,
// covering CreatePair (via create_half and pair creation), EditSource,
// refused linked edits (with document atomicity), MakeLocal/Omit/Rejoin,
// Unlink and an exact snapshot-history Undo/Redo stack, with format-8 JSON
// round-trip determinism checked after every step.
use crate::MirrorEditRefused;
use crate::model::{EditOperation, HalfExceptionState, Layout, MirroredHalfMode, Position};
use crate::{admit_edit, create_half, create_pair, set_exception, unlink_pair, validate_pairs};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 11) as f64 / (1u64 << 53) as f64)
    }
}

fn fuzz_mapping() -> Pose2 {
    Pose2 {
        at: Vec2 { x: 10.0, y: 0.0 },
        rotation: 0.0,
    }
}

fn two_board_fixture() -> ProjectDoc {
    let mut doc = split_document();
    doc.hardware = Some(crate::model::HardwareConfiguration {
        instances: vec![crate::model::PhysicalBoardInstance {
            id: "left".into(),
            name: "Left".into(),
            board_id: "source".into(),
            half: "left".into(),
            role: "primary".into(),
            flipped: false,
            controller_part_id: None,
            mechanical: None,
            construction_ownership: Default::default(),
            mechanical_identity_reservation: None,
        }],
        ..Default::default()
    });
    doc
}

fn same_board_fixture() -> ProjectDoc {
    let mut doc = split_document();
    doc.boards.pop();
    for id in ["left", "right"] {
        doc.layouts.push(Layout {
            id: id.into(),
            name: id.into(),
            board_id: "source".into(),
            matrix_ids: vec![],
            part_ids: if id == "left" {
                vec!["authored".into()]
            } else {
                vec![]
            },
            mirror_link: None,
        });
    }
    let pair = &mut doc.mirrored_pairs[0];
    pair.source = MirrorDestination::SameBoardLayout("left".into());
    pair.destination = MirrorDestination::SameBoardLayout("right".into());
    doc
}

fn check_invariants(doc: &ProjectDoc) {
    use crate::MirrorOverride;

    validate_pairs(doc).unwrap();
    let resolved = project_document(doc);
    assert_eq!(resolved, project_document(doc));
    let projected = resolved.geometry();
    for pair in &doc.mirrored_pairs {
        for source in doc
            .parts
            .iter()
            .filter(|part| boardstudio_model::mirrored_half::owns_part(doc, &pair.source, &part.id))
        {
            let target_id =
                boardstudio_model::mirrored_half::mirrored_part_id(doc, pair, &source.id);
            let override_state = pair
                .overrides
                .iter()
                .find(|item| item.source_id == source.id);
            match override_state.map(|item| &item.state) {
                None => {
                    let target = projected
                        .parts
                        .iter()
                        .find(|part| part.id == target_id)
                        .expect("linked mirror target projected");
                    assert_eq!(target.pose, crate::reflect::pose(pair.mapping, source.pose));
                    assert_eq!(target.properties, source.properties);
                }
                Some(MirrorOverride::Omitted) => {
                    assert!(
                        !projected.parts.iter().any(|part| part.id == target_id),
                        "omitted target {target_id} must stay absent"
                    );
                }
                Some(MirrorOverride::Local(local)) => {
                    let target = projected
                        .parts
                        .iter()
                        .find(|part| part.id == target_id)
                        .expect("local target projected");
                    assert_eq!(target.pose, local.pose);
                    assert_eq!(target.properties, local.properties);
                }
                Some(MirrorOverride::CompanionPose(pose)) => {
                    let target = projected
                        .parts
                        .iter()
                        .find(|part| part.id == target_id)
                        .expect("companion pose target projected");
                    assert_eq!(target.pose, *pose);
                    assert_eq!(target.properties, source.properties);
                }
            }
        }
    }
    let bytes = serde_json::to_vec(doc).unwrap();
    let back: ProjectDoc = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(&back, doc);
    assert_eq!(project_document(&back), resolved);
}

/// First linked (non-overridden) mirror target, if any.
fn linked_target(doc: &ProjectDoc) -> Option<(String, String, String)> {
    let pair = doc.mirrored_pairs.first()?;
    for source in &doc.parts {
        if !boardstudio_model::mirrored_half::owns_part(doc, &pair.source, &source.id)
            || pair
                .overrides
                .iter()
                .any(|item| item.source_id == source.id)
        {
            continue;
        }
        return Some((
            pair.id.clone(),
            boardstudio_model::mirrored_half::mirrored_part_id(doc, pair, &source.id),
            source.id.clone(),
        ));
    }
    None
}

fn hardware_ready(doc: &ProjectDoc, fresh: u64) -> bool {
    doc.hardware.as_ref().is_some_and(|hardware| {
        !hardware
            .instances
            .iter()
            .any(|item| item.id == format!("right{fresh}"))
            && !doc
                .boards
                .iter()
                .any(|board| board.id == format!("destination{fresh}"))
            && hardware.instances.iter().any(|item| {
                item.id == "left" && doc.boards.iter().any(|board| board.id == item.board_id)
            })
    })
}

#[derive(Clone, Copy)]
enum Op {
    Create,
    Edit,
    Refuse,
    Local,
    Omit,
    Rejoin,
    Unlink,
    Undo,
    Redo,
}

struct Fuzz {
    rng: Rng,
    two_board: bool,
    doc: ProjectDoc,
    history: Vec<ProjectDoc>,
    redo: Vec<ProjectDoc>,
    undone: Vec<Vec<u8>>,
    fresh: u64,
    counts: [usize; 9],
}

impl Fuzz {
    fn push_history(&mut self, snapshot: ProjectDoc) {
        self.history.push(snapshot);
        self.redo.clear();
        self.undone.clear();
    }
    fn create_pair(&mut self) {
        assert!(self.doc.mirrored_pairs.is_empty());
        let snapshot = self.doc.clone();
        let n = self.fresh;
        self.fresh += 1;
        let id = format!("split{n}");
        if self.two_board && self.rng.below(2) == 0 && hardware_ready(&self.doc, n) {
            create_half(
                &mut self.doc,
                &id,
                "left",
                &MirroredHalfMode::NewBoard {
                    board_id: format!("destination{n}"),
                    instance_id: format!("right{n}"),
                },
                None,
                None,
                fuzz_mapping(),
            )
            .unwrap();
        } else {
            let (source, destination) = if self.two_board {
                (
                    MirrorDestination::SecondBoard("source".into()),
                    MirrorDestination::SecondBoard("destination".into()),
                )
            } else {
                (
                    MirrorDestination::SameBoardLayout("left".into()),
                    MirrorDestination::SameBoardLayout("right".into()),
                )
            };
            create_pair(
                &mut self.doc,
                MirroredPair {
                    id,
                    source,
                    destination,
                    mapping: fuzz_mapping(),
                    overrides: vec![],
                },
            )
            .unwrap();
        }
        assert_eq!(self.doc.mirrored_pairs.len(), 1);
        self.push_history(snapshot);
    }
    fn edit_source(&mut self) {
        use crate::MirrorOverride;

        let local_before: Vec<(String, Pose2)> = self
            .doc
            .mirrored_pairs
            .iter()
            .flat_map(|pair| &pair.overrides)
            .filter_map(|item| match &item.state {
                MirrorOverride::Local(part) => Some((part.id.clone(), part.pose)),
                MirrorOverride::Omitted | MirrorOverride::CompanionPose(_) => None,
            })
            .collect();
        let (id, at) = self
            .doc
            .parts
            .iter()
            .find(|part| part.id == "authored")
            .map(|part| (part.id.clone(), part.pose.at))
            .unwrap();
        admit_edit(
            &self.doc,
            &EditOperation::MoveParts {
                positions: vec![Position { id: id.clone(), at }],
            },
        )
        .unwrap();
        let snapshot = self.doc.clone();
        let part = self
            .doc
            .parts
            .iter_mut()
            .find(|part| part.id == id)
            .unwrap();
        part.pose.at.x += self.rng.range(-8.0, 8.0);
        part.pose.at.y += self.rng.range(-8.0, 8.0);
        part.pose.rotation = (part.pose.rotation + self.rng.range(-10.0, 10.0)).rem_euclid(360.0);
        part.properties.get_or_insert_default().insert(
            format!("fuzz{}", self.rng.below(4)),
            serde_json::Value::from(self.rng.next() as i64),
        );
        if self.rng.below(4) == 0 {
            part.reference = format!("SW{}", self.rng.below(64));
        }
        let (pose, properties) = (part.pose, part.properties.clone());
        self.push_history(snapshot);
        let projected_doc = project_document(&self.doc);
        let projected_parts = &projected_doc.geometry().parts;
        let projected = projected_parts
            .iter()
            .find(|part| part.id == id)
            .unwrap()
            .clone();
        assert_eq!(projected.pose, pose);
        assert_eq!(projected.properties, properties);
        for (local_id, prev_pose) in local_before {
            let after = projected_parts
                .iter()
                .find(|part| part.id == local_id)
                .expect("local target stays projected across source edits");
            assert_eq!(after.pose, prev_pose);
        }
    }
    fn refuse_linked_edit(&mut self, pair_id: &str, target: &str, source: &str) {
        let before = self.doc.clone();
        let refused = admit_edit(
            &self.doc,
            &EditOperation::MoveParts {
                positions: vec![Position {
                    id: target.into(),
                    at: Vec2 { x: 1.0, y: 2.0 },
                }],
            },
        );
        assert!(
            matches!(&refused, Err(MirrorEditRefused::LinkedTarget { target_id, source_id, pair_id: refused_pair }) if target_id == target && source_id == source && refused_pair == pair_id),
            "{refused:?}"
        );
        assert_eq!(self.doc, before);
    }
    fn set_state(&mut self, pair_id: &str, target: &str, state: HalfExceptionState) {
        let snapshot = self.doc.clone();
        set_exception(&mut self.doc, pair_id, target, state).unwrap();
        self.push_history(snapshot);
        let present = project_document(&self.doc)
            .geometry()
            .parts
            .iter()
            .any(|part| part.id == target);
        assert_eq!(present, state != HalfExceptionState::Omitted);
    }
    fn rejoin(&mut self, pair_id: &str, target: &str) {
        let snapshot = self.doc.clone();
        set_exception(&mut self.doc, pair_id, target, HalfExceptionState::Linked).unwrap();
        self.push_history(snapshot);
        assert!(
            self.doc
                .mirrored_pairs
                .iter()
                .all(|pair| pair.overrides.is_empty())
        );
        assert!(
            project_document(&self.doc)
                .geometry()
                .parts
                .iter()
                .any(|part| part.id == target)
        );
    }
    fn unlink(&mut self) {
        let pair_id = self.doc.mirrored_pairs.first().unwrap().id.clone();
        let snapshot = self.doc.clone();
        unlink_pair(&mut self.doc, &pair_id).unwrap();
        assert!(self.doc.mirrored_pairs.is_empty());
        self.push_history(snapshot);
    }
    fn undo(&mut self) {
        if let Some(restored) = self.history.pop() {
            let current = std::mem::replace(&mut self.doc, restored);
            self.undone.push(serde_json::to_vec(&current).unwrap());
            self.redo.push(current);
        }
    }
    fn redo(&mut self) {
        if let Some(restored) = self.redo.pop() {
            let json = self.undone.pop().unwrap();
            self.history.push(self.doc.clone());
            self.doc = restored;
            assert_eq!(serde_json::to_vec(&self.doc).unwrap(), json);
        }
    }
    fn step(&mut self) {
        let linked = linked_target(&self.doc);
        let overridden = self.doc.mirrored_pairs.first().and_then(|pair| {
            pair.overrides.first().map(|item| {
                (
                    pair.id.clone(),
                    boardstudio_model::mirrored_half::mirrored_part_id(
                        &self.doc,
                        pair,
                        &item.source_id,
                    ),
                )
            })
        });
        let mut ops = vec![Op::Edit];
        ops.push(if self.doc.mirrored_pairs.is_empty() {
            Op::Create
        } else {
            Op::Unlink
        });
        if linked.is_some() {
            ops.extend([Op::Refuse, Op::Local, Op::Omit]);
        }
        if overridden.is_some() {
            ops.push(Op::Rejoin);
        }
        if !self.history.is_empty() {
            ops.push(Op::Undo);
        }
        if !self.redo.is_empty() {
            ops.push(Op::Redo);
        }
        let op = ops[self.rng.below(ops.len())];
        self.counts[op as usize] += 1;
        match op {
            Op::Create => self.create_pair(),
            Op::Edit => self.edit_source(),
            Op::Refuse => {
                let (pair_id, target, source) = linked.unwrap();
                self.refuse_linked_edit(&pair_id, &target, &source);
            }
            Op::Local => {
                let (pair_id, target, _) = linked.unwrap();
                self.set_state(&pair_id, &target, HalfExceptionState::Local);
            }
            Op::Omit => {
                let (pair_id, target, _) = linked.unwrap();
                self.set_state(&pair_id, &target, HalfExceptionState::Omitted);
            }
            Op::Rejoin => {
                let (pair_id, target) = overridden.unwrap();
                self.rejoin(&pair_id, &target);
            }
            Op::Unlink => self.unlink(),
            Op::Undo => self.undo(),
            Op::Redo => self.redo(),
        }
        check_invariants(&self.doc);
    }
}

#[test]
fn seeded_operation_sequence_keeps_mirror_invariants() {
    let mut totals = [0usize; 9];
    for seed in 0..32u64 {
        let mut fuzz = Fuzz {
            rng: Rng(seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(1)),
            two_board: seed % 2 == 0,
            doc: if seed % 2 == 0 {
                two_board_fixture()
            } else {
                same_board_fixture()
            },
            history: Vec::new(),
            redo: Vec::new(),
            undone: Vec::new(),
            fresh: 1,
            counts: [0; 9],
        };
        check_invariants(&fuzz.doc);
        for _ in 0..40 {
            fuzz.step();
        }
        for (slot, count) in totals.iter_mut().zip(fuzz.counts) {
            *slot += count;
        }
    }
    assert!(
        totals.iter().all(|&count| count > 0),
        "all 9 mirror op families must exercise across 32x40 runs: {totals:?}"
    );
}
