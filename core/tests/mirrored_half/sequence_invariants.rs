//! Operation-sequence invariants for a native mirrored pair through the public Core surface.
//!
//! Every script (scripted regressions and seeded random sequences alike) runs through one
//! checker. After each accepted commit, same-value commit, Undo, Redo, or archive reopen it
//! asserts that:
//!
//! - every Linked mirror Part is the pair-map reflection of its source (switch rule for matrix
//!   members and controllers, full-circle rule otherwise);
//! - an Omitted mirror Part is absent, and a Local one keeps its payload unless the step
//!   targeted it; a Part takes its own exception, else its key's, else its matrix's;
//! - correspondences never change, and exception overlays are exactly the ones requested;
//! - mirror matrix membership is exactly the present mirror members whose key is present;
//! - Undo and Redo restore the exact neighboring documents, a same-value commit keeps the
//!   revision and the Redo branch, and reopening a saved archive is a fixpoint;
//! - a refused request leaves the accepted document byte-identical.
use super::parent_acceptance::save_archive_and_open;
use super::*;
use std::collections::BTreeMap;

const PAIR_ID: &str = "sequence-invariant-pair";
const AXIS: f64 = 200.0;
const SOURCE_MATRIX: &str = "left-synthetic-keys";
const MIRROR_MATRIX: &str = "right-synthetic-keys";
const INDEPENDENT: [&str; 3] = ["RST", "TRRS", "U1"];
const SLOTS: [&str; 4] = ["r0c0", "r0c1", "r1c0", "r1c1"];
const CHILDREN: [&str; 2] = ["diode", "led"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Linked,
    Local,
    Omitted,
}

impl State {
    fn wire(self) -> &'static str {
        match self {
            Self::Linked => "linked",
            Self::Local => "local",
            Self::Omitted => "omitted",
        }
    }
}

/// One public request. Part ids are given on the mirror half; `source` maps them back.
#[derive(Clone, Debug)]
enum Op {
    MoveSource(String, f64, f64),
    MoveMirror(String, f64, f64),
    SetException(String, State),
    /// A same-value mechanical edit of the source instance.
    Noop,
    Undo,
    Redo,
    Reopen,
}

use Op::*;

fn independent(name: &str) -> String {
    format!("right/{name}")
}

fn key(slot: &str) -> String {
    format!("matrix/{MIRROR_MATRIX}/{slot}")
}

fn child(slot: &str, name: &str) -> String {
    format!("matrix/{MIRROR_MATRIX}/{slot}/{name}")
}

/// Independent fixture oracle. Only the four authored keys and their diode/LEDs are members.
fn fixture_slot(id: &str) -> Option<(&str, bool)> {
    for slot in SLOTS {
        if key(slot) == id {
            return Some((slot, false));
        }
        if CHILDREN.iter().any(|name| child(slot, name) == id) {
            return Some((slot, true));
        }
    }
    None
}

fn source_of(mirror_id: &str) -> String {
    mirror_id
        .replacen("right/", "left/", 1)
        .replacen(MIRROR_MATRIX, SOURCE_MATRIX, 1)
}

fn same_document(actual: &Value, expected: &Value) -> bool {
    let mut expected = expected.clone();
    expected["revision"] = actual["revision"].clone();
    *actual == expected
}

fn raw(engine: &mut CoreEngine, input: Value) -> Value {
    serde_json::from_str(&engine.request(&input.to_string())).unwrap()
}

fn halves(document: &Value) -> &Value {
    &pair(document, PAIR_ID)["halves"]
}

/// What the checker expects of the pair, maintained from accepted requests only.
struct Model {
    engine: CoreEngine,
    history: Vec<Value>,
    cursor: usize,
    overlays: BTreeMap<String, State>,
    correspondences: Value,
}

impl Model {
    fn new() -> Self {
        let source = synthetic_half::with_construction_features(runtime_supported_document());
        let (mut engine, seed) = seed(&source, "left");
        let created = create(
            &mut engine,
            &seed,
            PAIR_ID,
            "left",
            new_board("right", "right"),
            mapping(AXIS),
        )
        .unwrap()["document"]
            .clone();
        let correspondences = halves(&created)["correspondences"].clone();
        let model = Self {
            engine,
            history: vec![created],
            cursor: 0,
            overlays: BTreeMap::new(),
            correspondences,
        };
        model.check(&model.history[0], None, &[]);
        model
    }

    fn current(&self) -> &Value {
        &self.history[self.cursor]
    }

    /// A Part's own exception, else its key's, else its matrix's.
    fn effective(&self, mirror_id: &str) -> State {
        self.overlays
            .get(mirror_id)
            .or_else(|| {
                let (slot, companion) = fixture_slot(mirror_id)?;
                if companion && let Some(state) = self.overlays.get(&key(slot)) {
                    return Some(state);
                }
                self.overlays.get(MIRROR_MATRIX)
            })
            .copied()
            .unwrap_or(State::Linked)
    }

    /// Mirror matrix Parts that inherit the matrix exception.
    fn matrix_inheritors(&self) -> Vec<String> {
        SLOTS
            .iter()
            .filter(|slot| !self.overlays.contains_key(&key(slot)))
            .flat_map(|slot| {
                std::iter::once(key(slot)).chain(
                    CHILDREN
                        .iter()
                        .map(|name| child(slot, name))
                        .filter(|child| !self.overlays.contains_key(child)),
                )
            })
            .collect()
    }

    fn commit(&mut self, label: &str, targets: &[&str], operation: Value) -> Option<Value> {
        let before = serde_json::to_string(&snapshot(&mut self.engine)).unwrap();
        let input = json!({"id":label,"kind":"edit","command":{
            "baseRevision":self.current()["revision"],"transactionId":label,
            "phase":"commit","targetIds":targets,"operation":operation}});
        let reply = raw(&mut self.engine, input);
        if operation["kind"] == "move-parts" && self.effective(targets[0]) == State::Omitted {
            assert_eq!(reply["kind"], "error", "{label}");
            assert_eq!(
                reply["message"],
                format!(
                    "{:?} is omitted on its mirror half and has no geometry; rejoin it before editing it",
                    targets[0]
                )
            );
        }
        if reply["kind"] == "error" {
            assert_eq!(
                serde_json::to_string(&snapshot(&mut self.engine)).unwrap(),
                before,
                "{label}: refusal {} must not change the document",
                reply["message"]
            );
            return None;
        }
        assert_eq!(reply["kind"], "scene", "{label}: {reply}");
        let changed = reply["scene"]["changedIds"].as_array().unwrap();
        for link in self
            .correspondences
            .as_array()
            .unwrap()
            .iter()
            .filter(|link| link["kind"] == "part")
        {
            for id in [&link["sourceId"], &link["mirrorId"]] {
                let part_id = id.as_str().unwrap();
                if find_part(self.current(), part_id) != find_part(&reply["document"], part_id) {
                    assert!(
                        changed.contains(id),
                        "{label} must report {part_id}: {changed:?}"
                    );
                }
            }
        }
        Some(reply["document"].clone())
    }

    fn accept(&mut self, document: Value) {
        if document == *self.current() {
            // A same-value commit keeps revision, document and the Redo branch.
            return;
        }
        self.history.truncate(self.cursor + 1);
        self.history.push(document);
        self.cursor += 1;
    }

    /// Runs one request and returns whether Core accepted it.
    fn step(&mut self, op: &Op) -> bool {
        let label = format!("{op:?}");
        let previous = self.current().clone();
        let mut touched: Vec<String> = Vec::new();
        let accepted = match op {
            MoveSource(mirror_id, x, y) | MoveMirror(mirror_id, x, y) => {
                let id = if matches!(op, MoveSource(..)) {
                    source_of(mirror_id)
                } else {
                    touched.push(mirror_id.clone());
                    // A key moved on the mirror carries its companions, explicitly Local
                    // ones included. (Through a linked key a Local companion keeps its world
                    // pose instead; see `parent19_local_matrix_companions_*`.)
                    touched.extend(CHILDREN.map(|name| format!("{mirror_id}/{name}")));
                    mirror_id.clone()
                };
                let operation = json!({"kind":"move-parts",
                    "positions":[{"id":id,"at":{"x":x,"y":y}}]});
                let Some(document) = self.commit(&label, &[&id], operation) else {
                    return false;
                };
                assert_eq!(
                    part(&document, &id)["pose"]["at"],
                    json!({"x":x,"y":y}),
                    "{label}"
                );
                self.accept(document);
                true
            }
            SetException(mirror_id, state) => {
                let item_kind = if mirror_id == MIRROR_MATRIX {
                    "matrix"
                } else {
                    "part"
                };
                let operation = json!({"kind":"set-half-exception","pairId":PAIR_ID,
                    "itemKind":item_kind,"mirrorId":mirror_id,"state":state.wire()});
                let Some(document) = self.commit(&label, &[mirror_id], operation) else {
                    return false;
                };
                // Make Local keeps the visible item and its inheriting companions exactly.
                let keeps_payload =
                    *state == State::Local && self.effective(mirror_id) != State::Omitted;
                if !keeps_payload && mirror_id == MIRROR_MATRIX {
                    touched.extend(self.matrix_inheritors());
                } else if !keeps_payload {
                    touched.push(mirror_id.clone());
                }
                if !keeps_payload && matches!(fixture_slot(mirror_id), Some((_, false))) {
                    // Companions without an overlay of their own follow their key.
                    touched.extend(
                        CHILDREN
                            .iter()
                            .map(|name| format!("{mirror_id}/{name}"))
                            .filter(|child| !self.overlays.contains_key(child)),
                    );
                }
                match state {
                    State::Linked => self.overlays.remove(mirror_id),
                    _ => self.overlays.insert(mirror_id.clone(), *state),
                };
                self.accept(document);
                true
            }
            Undo | Redo => {
                let target = if matches!(op, Undo) {
                    self.cursor.checked_sub(1)
                } else {
                    Some(self.cursor + 1).filter(|next| *next < self.history.len())
                };
                let Some(target) = target else {
                    return false;
                };
                let action = if matches!(op, Undo) { "undo" } else { "redo" };
                let reply = raw(&mut self.engine, json!({"id":action,"kind":action}));
                assert_eq!(reply["kind"], "scene", "{label}: {reply}");
                assert!(
                    same_document(&reply["document"], &self.history[target]),
                    "{label} must restore the exact neighboring document"
                );
                self.history[target] = reply["document"].clone();
                self.cursor = target;
                self.overlays = overlays(self.current());
                touched.extend(self.overlays.keys().cloned());
                touched.extend(overlays(&previous).into_keys());
                if touched.iter().any(|id| id == MIRROR_MATRIX) {
                    touched.extend(SLOTS.map(key));
                }
                touched.extend(
                    touched
                        .clone()
                        .iter()
                        .flat_map(|id| CHILDREN.map(|name| format!("{id}/{name}"))),
                );
                true
            }
            Noop => {
                let operation = json!({"kind":"apply-mechanical-edit","edit":{
                    "target":{"kind":"instance","instanceId":"left"},
                    "change":{"set-dimension":{"field":"wall-thickness","value":2.0}}}});
                let live = snapshot(&mut self.engine);
                let document = self
                    .commit(&label, &["left/U1"], operation)
                    .expect("a same-value commit is accepted");
                assert_eq!(
                    document,
                    *self.current(),
                    "a same-value commit keeps revision and document"
                );
                assert_eq!(snapshot(&mut self.engine), live);
                true
            }
            Reopen => {
                let (engine, reopened) = save_archive_and_open(self.current());
                assert!(
                    same_document(&reopened, self.current()),
                    "reopening a saved archive is a fixpoint"
                );
                self.engine = engine;
                self.history = vec![reopened];
                self.cursor = 0;
                true
            }
        };
        self.check(self.current(), Some(&previous), &touched);
        accepted
    }

    fn check(&self, document: &Value, previous: Option<&Value>, touched: &[String]) {
        let halves = halves(document);
        assert_eq!(
            halves["correspondences"], self.correspondences,
            "correspondences are durable identities"
        );
        assert_eq!(
            halves["exceptions"].as_array().map_or(0, Vec::len),
            self.overlays.len(),
            "raw exception count"
        );
        assert_eq!(overlays(document), self.overlays, "exception overlays");
        let mut expected_members = BTreeSet::new();
        for link in self.correspondences.as_array().unwrap() {
            if link["kind"] != "part" {
                continue;
            }
            let mirror_id = link["mirrorId"].as_str().unwrap();
            let present = find_part(document, mirror_id);
            let member = fixture_slot(mirror_id);
            if let Some((slot, companion)) = member
                && present.is_some()
                && (!companion || find_part(document, &key(slot)).is_some())
            {
                expected_members.insert(mirror_id.to_owned());
            }
            match self.effective(mirror_id) {
                State::Omitted => assert!(present.is_none(), "Omitted {mirror_id} is absent"),
                State::Local => {
                    let present = present.unwrap_or_else(|| panic!("Local {mirror_id} exists"));
                    if let Some(previous) = previous
                        && !touched.iter().any(|id| id == mirror_id)
                    {
                        assert_eq!(
                            Some(present),
                            find_part(previous, mirror_id),
                            "Local {mirror_id} keeps its payload"
                        );
                    }
                }
                State::Linked => {
                    let mirror = present.unwrap_or_else(|| panic!("Linked {mirror_id} exists"));
                    let source = part(document, link["sourceId"].as_str().unwrap());
                    assert_reflection(document, source, mirror, member.is_some());
                }
            }
        }
        let mirror_matrix = document["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|matrix| matrix["id"] == MIRROR_MATRIX);
        if self.overlays.get(MIRROR_MATRIX) == Some(&State::Omitted) {
            assert!(
                mirror_matrix.is_none(),
                "an Omitted mirror matrix is absent"
            );
            return;
        }
        let actual_members: BTreeSet<String> =
            mirror_matrix.expect("the mirror matrix exists")["partIds"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| id.as_str().unwrap().to_owned())
                .collect();
        assert_eq!(actual_members, expected_members, "mirror matrix membership");
    }
}

fn overlays(document: &Value) -> BTreeMap<String, State> {
    halves(document)["exceptions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|exception| exception["kind"] == "part" || exception["kind"] == "matrix")
        .map(|exception| {
            let state = match exception["state"].as_str().unwrap() {
                "local" => State::Local,
                "omitted" => State::Omitted,
                other => panic!("unexpected exception state {other}"),
            };
            (exception["mirrorId"].as_str().unwrap().to_owned(), state)
        })
        .collect()
}

fn assert_reflection(document: &Value, source: &Value, mirror: &Value, in_matrix: bool) {
    let coordinate = |part: &Value, axis: &str| part["pose"]["at"][axis].as_f64().unwrap();
    let id = &mirror["id"];
    assert!(
        (coordinate(mirror, "x") - (AXIS - coordinate(source, "x"))).abs() < 1e-9
            && (coordinate(mirror, "y") - coordinate(source, "y")).abs() < 1e-9,
        "Linked {id} reflects {}: {} vs {}",
        source["id"],
        mirror["pose"],
        source["pose"]
    );
    let is_controller = document["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|definition| {
            definition["id"] == source["definitionId"] && definition["kind"] == "controller"
        });
    let heading = source["pose"]["rotation"].as_f64().unwrap();
    let expected = if in_matrix || is_controller {
        -heading
    } else {
        180.0 - heading
    };
    assert!(
        angles_match(mirror["pose"]["rotation"].as_f64().unwrap(), expected),
        "Linked {id} heading follows the pair rule"
    );
    assert_eq!(mirror["side"], source["side"], "Linked {id} side");
}

/// Runs a script whose every request Core must accept.
fn run_accepted(script: &[Op]) -> Model {
    let mut model = Model::new();
    for op in script {
        assert!(model.step(op), "{op:?} must be accepted");
    }
    model
}

#[test]
fn independent_exceptions_survive_edit_reopen_and_rejoin_the_latest_source() {
    for state in [State::Local, State::Omitted] {
        let target = independent("RST");
        let mut model = run_accepted(&[
            SetException(target.clone(), state),
            MoveSource(target.clone(), -18.0, 35.0),
            Reopen,
            SetException(target.clone(), State::Linked),
        ]);
        let expected = json!({"x":218.0,"y":35.0});
        assert_eq!(part(model.current(), &target)["pose"]["at"], expected);
        assert!(model.step(&Undo));
        assert!(model.step(&Redo));
        assert_eq!(part(model.current(), &target)["pose"]["at"], expected);
    }
}

#[test]
fn omitted_key_detaches_its_local_companion_through_reopen_and_rejoin() {
    let (key, diode) = (key("r0c0"), child("r0c0", "diode"));
    run_accepted(&[
        SetException(diode.clone(), State::Local),
        MoveMirror(diode, 123.0, 47.0),
        SetException(key.clone(), State::Omitted),
        Reopen,
        SetException(key, State::Linked),
        Undo,
        Redo,
    ]);
}

#[test]
fn a_same_value_commit_between_undo_and_redo_keeps_the_redo_branch() {
    let mut model = run_accepted(&[MoveSource(independent("U1"), -22.0, 4.0), Undo, Noop]);
    assert_eq!(
        part(model.current(), "left/U1")["pose"]["at"],
        json!({"x":-24.0,"y":0.0})
    );
    assert_eq!(
        part(model.current(), "right/U1")["pose"]["at"],
        json!({"x":224.0,"y":0.0})
    );
    assert!(model.step(&Redo));
    assert_eq!(
        part(model.current(), "left/U1")["pose"]["at"],
        json!({"x":-22.0,"y":4.0})
    );
    assert_eq!(
        part(model.current(), "right/U1")["pose"]["at"],
        json!({"x":222.0,"y":4.0})
    );
}

#[test]
fn a_companion_follows_its_local_or_omitted_key_for_mirror_side_edits() {
    let led = child("r1c0", "led");
    let mut model = run_accepted(&[SetException(key("r1c0"), State::Local)]);
    let source_led = part(model.current(), &source_of(&led)).clone();
    let mut expected = part(model.current(), &led).clone();
    expected["pose"]["at"] = json!({"x":-35.25,"y":-4.25});
    expected["properties"]["layoutOverride"] = json!(1);
    assert!(model.step(&MoveMirror(led.clone(), -35.25, -4.25)));
    for op in [Undo, Redo, Reopen] {
        assert!(model.step(&op));
    }
    assert_eq!(part(model.current(), &led), &expected);
    assert_eq!(
        part(model.current(), &source_of(&led)),
        &source_led,
        "a mirror-side edit of a Local key's companion stays on the mirror"
    );
    assert!(model.step(&MoveSource(led.clone(), 8.0, 16.0)));
    assert_eq!(part(model.current(), &led), &expected);
    let mut model = run_accepted(&[SetException(key("r1c0"), State::Omitted)]);
    assert!(!model.step(&MoveMirror(led, 10.0, 10.0)));
}

#[test]
fn a_key_or_companion_omitted_under_a_local_matrix_round_trips() {
    let diode = child("r0c0", "diode");
    let mut model = run_accepted(&[
        SetException(MIRROR_MATRIX.into(), State::Local),
        SetException(key("r0c0"), State::Omitted),
        Undo,
        Redo,
        Undo,
    ]);
    let before = part(model.current(), &diode).clone();
    for op in [
        SetException(diode.clone(), State::Omitted),
        SetException(diode.clone(), State::Linked),
    ] {
        assert!(model.step(&op), "{op:?} must be accepted");
    }
    assert_eq!(
        part(model.current(), &diode),
        &before,
        "a companion rejoined under a Local matrix is restored exactly"
    );
}

#[test]
fn an_omitted_mirror_refuses_mirror_side_edits_atomically() {
    let target = independent("TRRS");
    let mut model = run_accepted(&[SetException(target.clone(), State::Omitted)]);
    assert!(!model.step(&MoveMirror(target, 10.0, 10.0)));
}

/// A small deterministic generator, so a failing seed replays exactly.
struct Sequence(u64);

impl Sequence {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn pick<'a, T>(&mut self, values: &'a [T]) -> &'a T {
        &values[self.next() as usize % values.len()]
    }

    fn coordinate(&mut self) -> f64 {
        // Quarter-millimetre steps keep reflections exact in binary floating point.
        (self.next() % 400) as f64 * 0.25 - 50.0
    }

    fn op(&mut self, targets: &[String]) -> Op {
        let target = self.pick(targets).clone();
        match self.next() % 10 {
            0 | 1 => MoveSource(target, self.coordinate(), self.coordinate()),
            2 => MoveMirror(target, self.coordinate(), self.coordinate()),
            3 | 4 => SetException(
                if self.next().is_multiple_of(8) {
                    MIRROR_MATRIX.to_owned()
                } else {
                    target
                },
                *self.pick(&[State::Linked, State::Local, State::Omitted]),
            ),
            5 => Undo,
            6 | 7 => Redo,
            8 => Noop,
            _ => Reopen,
        }
    }
}

#[test]
fn seeded_operation_sequences_keep_every_invariant() {
    let targets: Vec<String> = INDEPENDENT
        .iter()
        .map(|name| independent(name))
        .chain(SLOTS.iter().map(|slot| key(slot)))
        .chain(
            SLOTS
                .iter()
                .flat_map(|slot| CHILDREN.map(|name| child(slot, name))),
        )
        .collect();
    let mut accepted = BTreeMap::<&str, usize>::new();
    for seed in 1..=16_u64 {
        let mut sequence = Sequence(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut model = Model::new();
        for _ in 0..32 {
            let op = sequence.op(&targets);
            let kind = match &op {
                MoveSource(..) => "move-source",
                MoveMirror(..) => "move-mirror",
                SetException(id, _) if id == MIRROR_MATRIX => "matrix-exception",
                SetException(_, State::Linked) => "rejoin",
                SetException(..) => "exception",
                Noop => "noop",
                Undo => "undo",
                Redo => "redo",
                Reopen => "reopen",
            };
            if model.step(&op) {
                *accepted.entry(kind).or_default() += 1;
            }
        }
    }
    for kind in [
        "move-source",
        "move-mirror",
        "rejoin",
        "exception",
        "matrix-exception",
        "noop",
        "undo",
        "redo",
        "reopen",
    ] {
        assert!(
            accepted.get(kind).copied().unwrap_or(0) >= 3,
            "seeded sequences exercise accepted {kind} steps: {accepted:?}"
        );
    }
}
