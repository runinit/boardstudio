//! Scoped edits for the authored geometry fields of a project part definition.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SessionEpoch};
use boardstudio_model::model::{
    EditOperation, Pad, PadShape, PartDefinition, PartKind, ProjectDoc, Vec2,
};

pub(crate) const DEFINITION_GONE: &str = "This part definition no longer exists.";
pub(crate) const GENERATOR_LOCKED: &str =
    "Generated definitions are edited through their generator settings.";
const PAD_GONE: &str = "This pad no longer exists on the definition.";
const IMPORTED_PADS: &str = "Imported pad geometry stays linked to its original KiCad source.";
const PAD_IDS_REQUIRED: &str =
    "Pad IDs and numbers are required. Positions must be finite, sizes and drill must be positive.";
const PAD_IDS_UNIQUE: &str = "Pad IDs must be unique within the component.";
const PAD_NUMBERS_UNIQUE: &str = "Pad numbers must be unique within the component.";
const COURTYARD_NUMBERS: &str = "Courtyard dimensions must be positive numbers.";
const PAD_NUMBERS: &str = "Pad positions and dimensions must be finite numbers.";

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Default)]
struct PadRowKeys {
    owner: String,
    next_key: u64,
    rows: Vec<(String, u64)>,
}

#[cfg(any(target_arch = "wasm32", test))]
impl PadRowKeys {
    fn for_pads(&mut self, owner: &str, pads: &[Pad]) -> Vec<u64> {
        if self.owner != owner {
            self.owner = owner.to_owned();
            self.rows.clear();
        }
        // IDs identify surviving rows. Ambiguous duplicate IDs cannot share a
        // component lifetime, even in an imported definition needing repair.
        let mut keys: Vec<_> = pads
            .iter()
            .map(|pad| {
                let mut previous = self.rows.iter().filter(|(id, _)| id == &pad.id);
                let first = previous.next();
                if previous.next().is_none()
                    && pads.iter().filter(|other| other.id == pad.id).count() == 1
                {
                    first.map(|(_, key)| *key)
                } else {
                    None
                }
            })
            .collect();
        let removed: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, (_, key))| !keys.contains(&Some(*key)))
            .collect();
        let added: Vec<_> = keys
            .iter()
            .enumerate()
            .filter_map(|(index, key)| key.is_none().then_some(index))
            .collect();
        // One accepted ID change at the same position is a row rename. Keep
        // that row's other drafts; list edits and ambiguous replacements retire
        // unmatched rows instead of assigning their drafts to another pad.
        if self.rows.len() == pads.len()
            && let ([(old_index, (_, key))], [new_index]) = (removed.as_slice(), added.as_slice())
            && old_index == new_index
        {
            keys[*new_index] = Some(*key);
        }
        let keys: Vec<_> = keys
            .into_iter()
            .map(|key| {
                key.unwrap_or_else(|| {
                    let fresh = self.next_key;
                    self.next_key += 1;
                    fresh
                })
            })
            .collect();
        self.rows = pads
            .iter()
            .zip(&keys)
            .map(|(pad, key)| (pad.id.clone(), *key))
            .collect();
        keys
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) mod ui {
    use super::{
        Axis, DefinitionEdit, DefinitionPanelCapture, PadRowKeys, courtyard_bounds,
        definition_field_resolver, validate_definition_edit,
    };
    use crate::parts::editor_fields::{EditorNumberField, EditorSelect, EditorTextField};
    use crate::runtime::Runtime;
    use boardstudio_application::{AcceptedSnapshot, Scope};
    use boardstudio_model::model::{Pad, PadShape, PartDefinition, PartKind};
    use boardstudio_web_ui_shared::ui::{
        alert::{Alert, AlertVariant},
        button::{Button, ButtonVariant},
        callout::{Callout, CalloutVariant},
    };
    use dioxus::prelude::*;
    use dioxus_web::WebEventExt;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlInputElement;

    #[cfg(test)]
    thread_local! {
        static PAD_NUMBER_DRAFT_FOR_TEST: std::cell::RefCell<Option<(Signal<String>, Signal<bool>)>> = const { std::cell::RefCell::new(None) };
    }

    #[cfg(test)]
    pub fn set_pad_number_draft_for_test(value: &str) -> bool {
        PAD_NUMBER_DRAFT_FOR_TEST.with(|draft| {
            let Some((mut draft, mut dirty)) = *draft.borrow() else {
                return false;
            };
            draft.set(value.to_owned());
            dirty.set(true);
            true
        })
    }

    #[cfg(test)]
    pub fn clear_pad_number_draft_for_test() {
        PAD_NUMBER_DRAFT_FOR_TEST.with(|draft| *draft.borrow_mut() = None);
    }

    /// The draft Signals of one pad row, owned by the panel so a settlement can never
    /// write Signals whose row component has already unmounted.
    #[derive(Clone, Copy, PartialEq)]
    struct RowDrafts {
        drafts: [Signal<String>; PadField::TEXT_FIELDS],
        dirty: [Signal<bool>; PadField::TEXT_FIELDS],
    }

    impl RowDrafts {
        fn new(pad: &Pad) -> Self {
            Self {
                drafts: [
                    Signal::new(pad.id.clone()),
                    Signal::new(pad.number.clone()),
                    Signal::new(pad.at.x.to_string()),
                    Signal::new(pad.at.y.to_string()),
                    Signal::new(pad.size.x.to_string()),
                    Signal::new(pad.size.y.to_string()),
                    Signal::new(pad.drill.map(|value| value.to_string()).unwrap_or_default()),
                ],
                dirty: std::array::from_fn(|_| Signal::new(false)),
            }
        }

        fn accepted_texts(pad: &Pad) -> [String; PadField::TEXT_FIELDS] {
            [
                pad.id.clone(),
                pad.number.clone(),
                pad.at.x.to_string(),
                pad.at.y.to_string(),
                pad.size.x.to_string(),
                pad.size.y.to_string(),
                pad.drill.map(|value| value.to_string()).unwrap_or_default(),
            ]
        }
    }

    /// Every pad row's drafts plus the accepted text each draft was last refreshed
    /// from, so a draft follows only its own accepted value and unrelated accepted
    /// changes leave dirty drafts alone.
    #[derive(Default)]
    struct PanelRows {
        drafts: BTreeMap<u64, RowDrafts>,
        accepted: BTreeMap<u64, [String; PadField::TEXT_FIELDS]>,
    }

    impl PanelRows {
        fn retain_rows(&mut self, live: &[u64]) {
            self.drafts.retain(|row, _| live.contains(row));
            self.accepted.retain(|row, _| live.contains(row));
        }

        /// The draft Signals of one row, created from the accepted pad on first sight.
        fn row(&mut self, row: u64, pad: &Pad) -> RowDrafts {
            *self
                .drafts
                .entry(row)
                .or_insert_with(|| RowDrafts::new(pad))
        }

        /// Refresh clean fields while preserving user input, including a newer draft
        /// that returned to the previous accepted value while an edit waited.
        fn refresh(&mut self, row: u64, field: PadField, pad: &Pad) {
            let Some(index) = field.text_index() else {
                return;
            };
            let accepted = RowDrafts::accepted_texts(pad);
            let entry = self.accepted.entry(row).or_insert_with(|| accepted.clone());
            // Update only this field's entry so the remaining fields of the same pass
            // still see their own accepted changes.
            if entry[index] != accepted[index] {
                entry[index] = accepted[index].clone();
                if let Some(mut drafts) = self.drafts.get(&row).copied()
                    && !*drafts.dirty[index].peek()
                {
                    drafts.drafts[index].set(entry[index].clone());
                }
            }
        }
    }

    /// One panel edit per bounded logical field. Rows carry their stable row key, so a
    /// landed pad edit never settles into another row's inputs.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum DefinitionFieldKey {
        Kind,
        CourtyardWidth,
        CourtyardHeight,
        AddPad,
        Pad { row: u64, field: PadField },
    }

    /// A committed field edit, addressed to the panel that owns the edits. Top-level
    /// fields carry no row; pad-row fields carry their row's stable key.
    #[derive(Clone, Debug, PartialEq)]
    pub struct DefinitionFieldEdit {
        pub row: Option<u64>,
        pub edit: DefinitionEdit,
    }

    #[component]
    pub fn CustomDefinitionFields(
        snapshot: AcceptedSnapshot,
        scope: Option<Scope>,
        selection: Signal<Option<(Option<Scope>, String)>>,
        definition: PartDefinition,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let mut failure = use_signal(|| None::<String>);
        let capture = DefinitionPanelCapture::new(&snapshot, scope.clone(), &definition);
        let owner_identity = (scope.clone(), selection(), definition.id.clone());
        // Pad-row drafts live here so a settlement can never write a Signal whose row
        // component already unmounted; the helper's bindings point at these Signals.
        let row_drafts = use_hook(|| Rc::new(RefCell::new(PanelRows::default())));
        let courtyard = courtyard_size(&definition);
        let initial_width = courtyard.0.clone();
        let initial_height = courtyard.1.clone();
        let mut width = use_signal(move || initial_width);
        let mut height = use_signal(move || initial_height);
        let mut width_dirty = use_signal(|| false);
        let mut height_dirty = use_signal(|| false);
        let observation_owner = use_hook(|| Rc::new(RefCell::new(capture.clone())));
        if *observation_owner.borrow() != capture {
            // A replacement definition takes fresh bindings. Old observations retire
            // before the replacement can receive their terminal feedback.
            *row_drafts.borrow_mut() = PanelRows::default();
            *observation_owner.borrow_mut() = capture.clone();
            width.set(courtyard.0.clone());
            height.set(courtyard.1.clone());
            width_dirty.set(false);
            height_dirty.set(false);
            failure.set(None);
        }

        // Begin a pending edit for a committed field. Admission keeps stale callbacks
        // out early; value validation explains itself inline; the resolver owns every
        // document-dependent check and runs against the accepted document at execution.
        let submit: EventHandler<DefinitionFieldEdit> = use_callback({
            let runtime = runtime.clone();
            let selection = selection;
            let capture = capture.clone();
            let row_drafts = row_drafts.clone();
            let mut width = width;
            let mut height = height;
            let mut failure = failure;
            move |request: DefinitionFieldEdit| {
                let model = runtime.model();
                let Some(current) = model.accepted.as_ref() else {
                    return;
                };
                if !capture.owner_matches(current, runtime.scope(), selection()) {
                    return;
                }
                let edit = request.edit;
                if let Err(message) = validate_definition_edit(&edit) {
                    failure.set(Some(message));
                    return;
                }
                let key = DefinitionFieldKey::of(request.row, &edit);
                let seed = match edit {
                    DefinitionEdit::AddPad => runtime.operation().0,
                    _ => 0,
                };
                let resolver = definition_field_resolver(capture.definition_id.clone(), edit, seed);
                let result = runtime.commit("parts-definition-field", resolver);
                if let Some(previous) = current
                    .document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == capture.definition_id)
                {
                    let (accepted_width, accepted_height) = courtyard_size(previous);
                    match key {
                        DefinitionFieldKey::CourtyardWidth => {
                            width_dirty.set(false);
                            width.set(accepted_width);
                        }
                        DefinitionFieldKey::CourtyardHeight => {
                            height_dirty.set(false);
                            height.set(accepted_height);
                        }
                        DefinitionFieldKey::Pad { row, field } => {
                            let rows = row_drafts.borrow();
                            if let Some(index) = field.text_index()
                                && let Some(mut drafts) = rows.drafts.get(&row).copied()
                                && let Some(accepted) = rows.accepted.get(&row)
                            {
                                drafts.dirty[index].set(false);
                                drafts.drafts[index].set(accepted[index].clone());
                            }
                        }
                        DefinitionFieldKey::Kind | DefinitionFieldKey::AddPad => {}
                    }
                }
                failure.set(result.err());
            }
        });

        let kicad_locked = definition.kicad_source.is_some();
        let accepted_width = courtyard.0.clone();
        use_effect(use_reactive((&accepted_width,), move |(value,)| {
            if !width_dirty() {
                width.set(value);
            }
        }));
        let accepted_height = courtyard.1.clone();
        use_effect(use_reactive((&accepted_height,), move |(value,)| {
            if !height_dirty() {
                height.set(value);
            }
        }));
        use_effect(use_reactive((&owner_identity,), {
            let accepted_width = courtyard.0.clone();
            let accepted_height = courtyard.1.clone();
            let mut failure = failure;
            move |_| {
                width.set(accepted_width.clone());
                height.set(accepted_height.clone());
                width_dirty.set(false);
                height_dirty.set(false);
                failure.set(None);
            }
        }));

        let owner_key = format!("{:?}:{}", scope, definition.id);
        let pad_rows = use_hook(|| Rc::new(RefCell::new(PadRowKeys::default())));
        let row_keys = pad_rows.borrow_mut().for_pads(&owner_key, &definition.pads);
        // Subscribe to the workspace's runtime-change version: outcomes settle outside
        // Dioxus (Core replies, saves), so this read is what wakes the settle pass below
        // even when the accepted document did not change (for example a failed save).
        let version = use_context::<Signal<u64>>();
        let _ = version();

        // Refresh each visible row's drafts from its accepted text and keep the
        // helper's bindings pointed at the live Signals. A new row key starts from the
        // accepted document; a dirty draft follows only its own accepted value.
        {
            let mut rows = row_drafts.borrow_mut();
            for (pad, row_key) in definition.pads.iter().zip(row_keys.iter().copied()) {
                rows.row(row_key, pad);
                for field in PadField::text_fields() {
                    rows.refresh(row_key, field, pad);
                }
            }
        }

        row_drafts.borrow_mut().retain_rows(&row_keys);

        let kind = kind_name(&definition.kind);
        let on_kind = {
            let begin_kind = submit;
            move |next: Option<String>| {
                if let Some(value) = next
                    && let Some(kind) = parse_kind(&value)
                {
                    begin_kind.call(DefinitionFieldEdit {
                        row: None,
                        edit: DefinitionEdit::Kind(kind),
                    });
                }
            }
        };
        let on_add = {
            let begin_add = submit;
            move |_| {
                begin_add.call(DefinitionFieldEdit {
                    row: None,
                    edit: DefinitionEdit::AddPad,
                })
            }
        };
        let on_width_blur = {
            let begin_width = submit;
            move |_| {
                if !width_dirty() {
                    return;
                }
                let draft = width();
                begin_width.call(DefinitionFieldEdit {
                    row: None,
                    edit: DefinitionEdit::CourtyardWidth(draft),
                });
            }
        };
        let on_height_blur = {
            let begin_height = submit;
            move |_| {
                if !height_dirty() {
                    return;
                }
                let draft = height();
                begin_height.call(DefinitionFieldEdit {
                    row: None,
                    edit: DefinitionEdit::CourtyardHeight(draft),
                });
            }
        };
        let committed_width = courtyard.0.clone();
        let width_keydown =
            move |event| draft_keydown(event, width, width_dirty, committed_width.clone());
        let committed_height = courtyard.1.clone();
        let height_keydown =
            move |event| draft_keydown(event, height, height_dirty, committed_height.clone());

        rsx! {
            div { class: "grid gap-3",
                EditorSelect {
                    label: "Kind",
                    aria_label: Some("Definition kind".to_owned()),
                    value: kind.to_owned(),
                    options: [("switch", "Switch"), ("controller", "Controller"), ("connector", "Connector"), ("encoder", "Encoder"), ("passive", "Passive"), ("custom", "Custom")].map(|(value, text)| (value.to_owned(), text.to_owned())).to_vec(),
                    on_select: move |value: String| on_kind(Some(value)),
                }
                fieldset { class: "grid gap-2",
                    legend { "Rectangular courtyard (mm)" }
                    EditorNumberField {
                        label: "Width", value: "{width()}",
                        aria_label: "Courtyard width",
                        min: "0.01", step: "0.1",
                        oninput: move |event: FormEvent| { width.set(event.value()); width_dirty.set(true); },
                        onblur: on_width_blur,
                        onkeydown: width_keydown,
                    }
                    EditorNumberField {
                        label: "Height", value: "{height()}",
                        aria_label: "Courtyard height",
                        min: "0.01", step: "0.1",
                        oninput: move |event: FormEvent| { height.set(event.value()); height_dirty.set(true); },
                        onblur: on_height_blur,
                        onkeydown: height_keydown,
                    }
                }
                div { class: "flex flex-wrap items-center justify-between gap-2",
                    strong { "Pads " small { "{definition.pads.len()}" } }
                    Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: kicad_locked, onclick: on_add, "+ Add pad" }
                }
                if kicad_locked {
                    Callout { variant: CalloutVariant::Info, "Imported pad geometry stays linked to its original KiCad source." }
                }
                for (index, (pad, row_key)) in definition.pads.iter().zip(row_keys.iter().copied()).enumerate() {
                    PadFields {
                        key: "{owner_key}:{row_key}", index, row: row_key, pad: pad.clone(),
                        locked: kicad_locked,
                        drafts: row_drafts.borrow_mut().drafts.get(&row_key).copied(),
                        failure,
                        submit,
                    }
                }
                ul { class: "grid gap-1 text-sm", "aria-live": "polite",
                    for issue in definition_issues(&definition) { li { "{issue}" } }
                }
                if let Some(message) = failure() {
                    Alert { variant: AlertVariant::Destructive, "{message}" }
                }
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum PadField {
        Id,
        Number,
        X,
        Y,
        SizeX,
        SizeY,
        Drill,
        Shape,
        Remove,
    }

    impl PadField {
        /// The seven text inputs; the shape select and remove button settle through
        /// the same collection but carry no draft text.
        const TEXT_FIELDS: usize = 7;

        fn text_fields() -> [PadField; PadField::TEXT_FIELDS] {
            [
                PadField::Id,
                PadField::Number,
                PadField::X,
                PadField::Y,
                PadField::SizeX,
                PadField::SizeY,
                PadField::Drill,
            ]
        }

        fn text_index(&self) -> Option<usize> {
            match self {
                PadField::Id => Some(0),
                PadField::Number => Some(1),
                PadField::X => Some(2),
                PadField::Y => Some(3),
                PadField::SizeX => Some(4),
                PadField::SizeY => Some(5),
                PadField::Drill => Some(6),
                PadField::Shape | PadField::Remove => None,
            }
        }
    }

    impl DefinitionFieldKey {
        /// The key a committed edit settles under: top-level fields carry no row,
        /// pad-row fields carry their row's stable key.
        fn of(row: Option<u64>, edit: &DefinitionEdit) -> Self {
            let field = match edit {
                DefinitionEdit::PadId { .. } => PadField::Id,
                DefinitionEdit::PadNumber { .. } => PadField::Number,
                DefinitionEdit::PadCoordinate { axis: Axis::X, .. } => PadField::X,
                DefinitionEdit::PadCoordinate { axis: Axis::Y, .. } => PadField::Y,
                DefinitionEdit::PadSize { axis: Axis::X, .. } => PadField::SizeX,
                DefinitionEdit::PadSize { axis: Axis::Y, .. } => PadField::SizeY,
                DefinitionEdit::PadDrill { .. } => PadField::Drill,
                DefinitionEdit::PadShape { .. } => PadField::Shape,
                DefinitionEdit::RemovePad { .. } => PadField::Remove,
                DefinitionEdit::Kind(_) => return Self::Kind,
                DefinitionEdit::CourtyardWidth(_) => return Self::CourtyardWidth,
                DefinitionEdit::CourtyardHeight(_) => return Self::CourtyardHeight,
                DefinitionEdit::AddPad => return Self::AddPad,
            };
            match (row, edit) {
                (Some(row), _) => Self::Pad { row, field },
                _ => unreachable!("pad edits carry their row key"),
            }
        }
    }

    #[component]
    fn PadFields(
        index: usize,
        row: u64,
        pad: Pad,
        locked: bool,
        drafts: Option<RowDrafts>,
        failure: Signal<Option<String>>,
        submit: EventHandler<DefinitionFieldEdit>,
    ) -> Element {
        let Some(drafts) = drafts else {
            return rsx! {};
        };
        #[cfg(test)]
        if index == 0 {
            PAD_NUMBER_DRAFT_FOR_TEST.with(|draft| {
                let index = PadField::Number.text_index().unwrap_or_default();
                *draft.borrow_mut() = Some((drafts.drafts[index], drafts.dirty[index]));
            });
        }
        let mut id = drafts.drafts[0];
        let mut number = drafts.drafts[1];
        let mut x = drafts.drafts[2];
        let mut y = drafts.drafts[3];
        let mut size_x = drafts.drafts[4];
        let mut size_y = drafts.drafts[5];
        let mut drill = drafts.drafts[6];
        let [
            mut id_dirty,
            mut number_dirty,
            mut x_dirty,
            mut y_dirty,
            mut size_x_dirty,
            mut size_y_dirty,
            mut drill_dirty,
        ] = drafts.dirty;

        let submit_id = submit;
        let old_id = pad.id.clone();
        let committed_id = pad.id.clone();
        let on_id_blur = move |_| {
            let draft = id().trim().to_owned();
            submit_id.call(DefinitionFieldEdit {
                row: Some(row),
                edit: DefinitionEdit::PadId {
                    pad_id: old_id.clone(),
                    value: draft,
                },
            });
        };
        let id_keydown = move |event| draft_keydown(event, id, id_dirty, committed_id.clone());
        let submit_number = submit;
        let old_id = pad.id.clone();
        let committed_number = pad.number.clone();
        let on_number_blur = move |_| {
            let draft = number().trim().to_owned();
            submit_number.call(DefinitionFieldEdit {
                row: Some(row),
                edit: DefinitionEdit::PadNumber {
                    pad_id: old_id.clone(),
                    value: draft,
                },
            });
        };
        let number_keydown =
            move |event| draft_keydown(event, number, number_dirty, committed_number.clone());
        let submit_x = submit;
        let old_id = pad.id.clone();
        let committed_x = pad.at.x.to_string();
        let on_x_blur = move |_| {
            submit_x.call(DefinitionFieldEdit {
                row: Some(row),
                edit: DefinitionEdit::PadCoordinate {
                    pad_id: old_id.clone(),
                    axis: Axis::X,
                    value: x(),
                },
            });
        };
        let x_keydown = move |event| draft_keydown(event, x, x_dirty, committed_x.clone());
        let submit_y = submit;
        let old_id = pad.id.clone();
        let committed_y = pad.at.y.to_string();
        let on_y_blur = move |_| {
            submit_y.call(DefinitionFieldEdit {
                row: Some(row),
                edit: DefinitionEdit::PadCoordinate {
                    pad_id: old_id.clone(),
                    axis: Axis::Y,
                    value: y(),
                },
            });
        };
        let y_keydown = move |event| draft_keydown(event, y, y_dirty, committed_y.clone());
        let submit_width = submit;
        let old_id = pad.id.clone();
        let committed_width = pad.size.x.to_string();
        let on_width_blur = move |_| {
            submit_width.call(DefinitionFieldEdit {
                row: Some(row),
                edit: DefinitionEdit::PadSize {
                    pad_id: old_id.clone(),
                    axis: Axis::X,
                    value: size_x(),
                },
            });
        };
        let width_keydown =
            move |event| draft_keydown(event, size_x, size_x_dirty, committed_width.clone());
        let submit_height = submit;
        let old_id = pad.id.clone();
        let committed_height = pad.size.y.to_string();
        let on_height_blur = move |_| {
            submit_height.call(DefinitionFieldEdit {
                row: Some(row),
                edit: DefinitionEdit::PadSize {
                    pad_id: old_id.clone(),
                    axis: Axis::Y,
                    value: size_y(),
                },
            });
        };
        let height_keydown =
            move |event| draft_keydown(event, size_y, size_y_dirty, committed_height.clone());
        let submit_drill = submit;
        let old_id = pad.id.clone();
        let committed_drill = pad
            .drill
            .map(|number| number.to_string())
            .unwrap_or_default();
        let on_drill_blur = move |_| {
            submit_drill.call(DefinitionFieldEdit {
                row: Some(row),
                edit: DefinitionEdit::PadDrill {
                    pad_id: old_id.clone(),
                    value: drill(),
                },
            });
        };
        let drill_keydown =
            move |event| draft_keydown(event, drill, drill_dirty, committed_drill.clone());
        let submit_shape = submit;
        let old_id = pad.id.clone();
        let shape_value = shape_name(&pad.shape).to_owned();
        let on_shape = move |next: Option<String>| {
            if let Some(value) = next
                && let Some(shape) = parse_shape(&value)
            {
                submit_shape.call(DefinitionFieldEdit {
                    row: Some(row),
                    edit: DefinitionEdit::PadShape {
                        pad_id: old_id.clone(),
                        shape,
                    },
                });
            }
        };
        let submit_remove = submit;
        let remove_id = pad.id.clone();
        let on_remove = move |_| {
            submit_remove.call(DefinitionFieldEdit {
                row: Some(row),
                edit: DefinitionEdit::RemovePad {
                    pad_id: remove_id.clone(),
                },
            })
        };

        rsx! {
            fieldset { class: "grid gap-2", disabled: locked,
                legend { "Pad {index + 1}" }
                div { class: "grid grid-cols-2 gap-2",
                    EditorTextField { label: "ID", value: "{id()}", aria_label: "Pad {index + 1} ID", disabled: locked, oninput: move |event: FormEvent| { id.set(event.value()); id_dirty.set(true); }, onblur: on_id_blur, onkeydown: id_keydown }
                    EditorTextField { label: "Number", value: "{number()}", aria_label: "Pad {index + 1} number", disabled: locked, oninput: move |event: FormEvent| { number.set(event.value()); number_dirty.set(true); }, onblur: on_number_blur, onkeydown: number_keydown }
                    EditorNumberField { label: "X", value: "{x()}", aria_label: "Pad {index + 1} X", disabled: locked, step: "0.1", oninput: move |event: FormEvent| { x.set(event.value()); x_dirty.set(true); }, onblur: on_x_blur, onkeydown: x_keydown }
                    EditorNumberField { label: "Y", value: "{y()}", aria_label: "Pad {index + 1} Y", disabled: locked, step: "0.1", oninput: move |event: FormEvent| { y.set(event.value()); y_dirty.set(true); }, onblur: on_y_blur, onkeydown: y_keydown }
                    EditorNumberField { label: "Width", value: "{size_x()}", aria_label: "Pad {index + 1} width", disabled: locked, min: "0.01", step: "0.1", oninput: move |event: FormEvent| { size_x.set(event.value()); size_x_dirty.set(true); }, onblur: on_width_blur, onkeydown: width_keydown }
                    EditorNumberField { label: "Height", value: "{size_y()}", aria_label: "Pad {index + 1} height", disabled: locked, min: "0.01", step: "0.1", oninput: move |event: FormEvent| { size_y.set(event.value()); size_y_dirty.set(true); }, onblur: on_height_blur, onkeydown: height_keydown }
                    EditorSelect {
                        label: "Shape",
                        aria_label: Some(format!("Pad {} shape", index + 1)),
                        value: shape_value,
                        disabled: locked,
                        options: [("circle", "Circle"), ("oval", "Oval"), ("rect", "Rectangle"), ("roundrect", "Rounded rectangle")].map(|(value, text)| (value.to_owned(), text.to_owned())).to_vec(),
                        on_select: move |value: String| on_shape(Some(value)),
                    }
                    EditorNumberField { label: "Drill", value: "{drill()}", aria_label: "Pad {index + 1} drill", disabled: locked, min: "0.01", step: "0.1", placeholder: "None", oninput: move |event: FormEvent| { drill.set(event.value()); drill_dirty.set(true); }, onblur: on_drill_blur, onkeydown: drill_keydown }
                }
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", onclick: on_remove, "Remove pad" }
            }
        }
    }

    fn kind_name(kind: &PartKind) -> &'static str {
        match kind {
            PartKind::Switch => "switch",
            PartKind::Controller => "controller",
            PartKind::Connector => "connector",
            PartKind::Encoder => "encoder",
            PartKind::Passive => "passive",
            PartKind::Custom => "custom",
            PartKind::Utility => "custom",
        }
    }
    fn shape_name(shape: &PadShape) -> &'static str {
        match shape {
            PadShape::Circle => "circle",
            PadShape::Oval => "oval",
            PadShape::Rect => "rect",
            PadShape::Roundrect => "roundrect",
        }
    }
    fn courtyard_size(definition: &PartDefinition) -> (String, String) {
        let bounds = definition
            .courtyard
            .iter()
            .fold(None, |bounds, point| match bounds {
                None => Some((point.x, point.y, point.x, point.y)),
                Some((min_x, min_y, max_x, max_y)) => Some((
                    min_x.min(point.x),
                    min_y.min(point.y),
                    max_x.max(point.x),
                    max_y.max(point.y),
                )),
            });
        match bounds {
            Some((min_x, min_y, max_x, max_y)) => {
                ((max_x - min_x).to_string(), (max_y - min_y).to_string())
            }
            None => ("10".into(), "6".into()),
        }
    }
    fn parse_kind(value: &str) -> Option<PartKind> {
        match value {
            "switch" => Some(PartKind::Switch),
            "controller" => Some(PartKind::Controller),
            "connector" => Some(PartKind::Connector),
            "encoder" => Some(PartKind::Encoder),
            "passive" => Some(PartKind::Passive),
            "custom" => Some(PartKind::Custom),
            _ => None,
        }
    }
    fn parse_shape(value: &str) -> Option<PadShape> {
        match value {
            "circle" => Some(PadShape::Circle),
            "oval" => Some(PadShape::Oval),
            "rect" => Some(PadShape::Rect),
            "roundrect" => Some(PadShape::Roundrect),
            _ => None,
        }
    }
    fn draft_keydown(
        event: KeyboardEvent,
        mut draft: Signal<String>,
        mut dirty: Signal<bool>,
        accepted: String,
    ) {
        if event.key() == Key::Escape {
            event.prevent_default();
            draft.set(accepted);
            dirty.set(false);
        } else if event.key() == Key::Enter
            && let Some(input) = event
                .data()
                .try_as_web_event()
                .and_then(|event| event.target())
                .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
        {
            let _ = input.blur();
        }
    }
    fn definition_issues(definition: &PartDefinition) -> Vec<String> {
        let mut issues = Vec::new();
        if definition.name.trim().is_empty() {
            issues.push("Name is required.".to_owned());
        }
        let (width, height, _) = courtyard_bounds(&definition.courtyard);
        if definition.courtyard.len() < 3
            || !width.is_finite()
            || !height.is_finite()
            || width <= 0.0
            || height <= 0.0
        {
            issues.push("Courtyard must have positive width and height.".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for (index, pad) in definition.pads.iter().enumerate() {
            if pad.id.trim().is_empty() || !ids.insert(pad.id.clone()) {
                issues.push(format!("Pad {} needs a unique ID.", index + 1));
            }
            if definition.kicad_source.is_none() && pad.number.trim().is_empty() {
                issues.push(format!("Pad {} needs a number.", index + 1));
            }
            if !pad.at.x.is_finite() || !pad.at.y.is_finite() {
                issues.push(format!("Pad {} position must be finite.", index + 1));
            }
            if !pad.size.x.is_finite()
                || !pad.size.y.is_finite()
                || pad.size.x <= 0.0
                || pad.size.y <= 0.0
            {
                issues.push(format!("Pad {} size must be positive.", index + 1));
            }
            if pad
                .drill
                .is_some_and(|drill| !drill.is_finite() || drill <= 0.0)
            {
                issues.push(format!("Pad {} drill must be positive.", index + 1));
            }
        }
        issues
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
pub fn set_pad_number_draft_for_test(value: &str) -> bool {
    ui::set_pad_number_draft_for_test(value)
}

#[cfg(all(test, target_arch = "wasm32"))]
pub fn clear_pad_number_draft_for_test() {
    ui::clear_pad_number_draft_for_test();
}

#[cfg(target_arch = "wasm32")]
pub use ui::CustomDefinitionFields;

/// What a definition panel needs to know its owner is still current at dispatch:
/// the scope, the document session and the selected definition. Deliberately not a
/// snapshot token or revision — field edits queue freely and resolve against
/// whatever document is accepted when they run (ADR-0005).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionPanelCapture {
    pub scope: Option<Scope>,
    pub definition_id: String,
    session_epoch: SessionEpoch,
    document_id: String,
}

impl DefinitionPanelCapture {
    pub fn new(
        snapshot: &AcceptedSnapshot,
        scope: Option<Scope>,
        definition: &PartDefinition,
    ) -> Self {
        Self {
            scope,
            definition_id: definition.id.clone(),
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
        }
    }

    /// Owner lifetime at dispatch: the panel still belongs to this scope, document
    /// session and selected definition, so a stale callback is ignored early. A newer
    /// accepted revision is not a liveness answer — the resolver re-checks the target
    /// at execution.
    pub fn owner_matches(
        &self,
        current: &AcceptedSnapshot,
        current_scope: Option<Scope>,
        current_selection: Option<(Option<Scope>, String)>,
    ) -> bool {
        self.scope.is_some()
            && current_scope == self.scope
            && current.session_epoch == self.session_epoch
            && current.document.id == self.document_id
            && current_selection == Some((self.scope.clone(), self.definition_id.clone()))
    }
}

mod edit;
pub use edit::{Axis, DefinitionEdit, definition_field_resolver, validate_definition_edit};
pub(crate) use edit::{courtyard_bounds, replacement_commit};

#[cfg(test)]
mod tests;
