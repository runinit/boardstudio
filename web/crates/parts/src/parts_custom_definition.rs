//! Scoped edits for the authored geometry fields of a project part definition.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SessionEpoch};
use boardstudio_core::model::{
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
    use crate::runtime::Runtime;
    use boardstudio_application::{AcceptedSnapshot, Scope};
    use boardstudio_core::model::{Pad, PadShape, PartDefinition, PartKind};
    use boardstudio_web_runtime::pending_edits::PendingEditResult;
    use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
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
        /// Release departed row bindings before discarding their projection handles.
        fn retain_rows(&mut self, live: &[u64], pending: &PendingEditSignals<DefinitionFieldKey>) {
            self.drafts.retain(|row, _| {
                if live.contains(row) {
                    return true;
                }
                for field in PadField::text_fields() {
                    pending.unbind_field(&DefinitionFieldKey::Pad { row: *row, field });
                }
                pending.unbind_one_shot(&DefinitionFieldKey::Pad {
                    row: *row,
                    field: PadField::Remove,
                });
                false
            });
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
        let pending = use_hook(|| PendingEditSignals::<DefinitionFieldKey>::new());
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
            pending.unbind_field(&DefinitionFieldKey::CourtyardWidth);
            pending.unbind_field(&DefinitionFieldKey::CourtyardHeight);
            let mut rows = row_drafts.borrow_mut();
            rows.retain_rows(&[], &pending);
            pending.settle(false, |_| String::new());
            *observation_owner.borrow_mut() = capture.clone();
            width.set(courtyard.0.clone());
            height.set(courtyard.1.clone());
            width_dirty.set(false);
            height_dirty.set(false);
            failure.set(None);
        }
        use_drop({
            let pending = pending.clone();
            let row_drafts = row_drafts.clone();
            move || {
                pending.unbind_field(&DefinitionFieldKey::CourtyardWidth);
                pending.unbind_field(&DefinitionFieldKey::CourtyardHeight);
                row_drafts.borrow_mut().retain_rows(&[], &pending);
                pending.settle(false, |_| String::new());
            }
        });

        // Begin a pending edit for a committed field. Admission keeps stale callbacks
        // out early; value validation explains itself inline; the resolver owns every
        // document-dependent check and runs against the accepted document at execution.
        let submit: EventHandler<DefinitionFieldEdit> = use_callback({
            let runtime = runtime.clone();
            let selection = selection;
            let capture = capture.clone();
            let pending = pending.clone();
            let row_drafts = row_drafts.clone();
            let width = width;
            let height = height;
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
                // The helper remembers the draft each field edit was submitted with, so
                // an older outcome can never clobber a newer draft.
                let submitted = match key {
                    DefinitionFieldKey::CourtyardWidth => width.peek().clone(),
                    DefinitionFieldKey::CourtyardHeight => height.peek().clone(),
                    DefinitionFieldKey::Pad { row, field } => row_drafts
                        .borrow()
                        .drafts
                        .get(&row)
                        .and_then(|drafts| {
                            field
                                .text_index()
                                .map(|index| drafts.drafts[index].peek().clone())
                        })
                        .unwrap_or_default(),
                    DefinitionFieldKey::Kind | DefinitionFieldKey::AddPad => String::new(),
                };
                // A baseline value is a real new intent while an older edit waits.
                // Otherwise compare with fresh accepted state, so retained callbacks
                // cannot mistake a stale prop value for the accepted value.
                if !pending.is_pending(&key)
                    && current
                        .document
                        .definitions
                        .iter()
                        .find(|definition| definition.id == capture.definition_id)
                        .is_some_and(|definition| {
                            field_matches_accepted(&key, &edit, &submitted, definition)
                        })
                {
                    match key {
                        DefinitionFieldKey::CourtyardWidth => width_dirty.set(false),
                        DefinitionFieldKey::CourtyardHeight => height_dirty.set(false),
                        DefinitionFieldKey::Pad { row, field } => {
                            if let Some(index) = field.text_index()
                                && let Some(mut drafts) =
                                    row_drafts.borrow().drafts.get(&row).copied()
                            {
                                drafts.dirty[index].set(false);
                            }
                        }
                        DefinitionFieldKey::Kind | DefinitionFieldKey::AddPad => {}
                    }
                    return;
                }
                let seed = match edit {
                    DefinitionEdit::AddPad => runtime.operation().0,
                    _ => 0,
                };
                let resolver = definition_field_resolver(capture.definition_id.clone(), edit, seed);
                if matches!(
                    key,
                    DefinitionFieldKey::AddPad
                        | DefinitionFieldKey::Pad {
                            field: PadField::Remove,
                            ..
                        }
                ) {
                    // One-shot controls stay quiet while their edit is queued.
                    pending.begin_one_shot(
                        &runtime,
                        key,
                        "parts-definition-field",
                        Some("part definition".into()),
                        resolver,
                    );
                } else {
                    pending.begin_field(
                        &runtime,
                        key,
                        "parts-definition-field",
                        Some("part definition".into()),
                        resolver,
                        &submitted,
                    );
                }
                failure.set(None);
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

        // The selection, document session and scope this panel's edits belong to are
        // still current; a departed owner retires its edits.
        let owner_live =
            runtime.model().accepted.as_ref().is_some_and(|current| {
                capture.owner_matches(current, runtime.scope(), selection())
            });
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
                let row_drafts = rows.row(row_key, pad);
                for field in PadField::text_fields() {
                    rows.refresh(row_key, field, pad);
                    pending.bind_field(
                        DefinitionFieldKey::Pad {
                            row: row_key,
                            field,
                        },
                        row_drafts.drafts[field.text_index().unwrap_or_default()],
                        failure,
                    );
                }
            }
        }
        pending.bind_field(DefinitionFieldKey::CourtyardWidth, width, failure);
        pending.bind_field(DefinitionFieldKey::CourtyardHeight, height, failure);

        // Settle the pending edits before rendering: pending fields keep their drafts,
        // failures restore the accepted value with the message inline, and landed or
        // retired edits drop so the fields follow the accepted document again. A row
        // whose pad vanished can still report its failure at the shared placement,
        // even after its field bindings and draft handles have been pruned.
        let results = pending.settle(owner_live, |key| {
            accepted_field_text(key, &definition, &row_keys, &row_drafts, &courtyard)
        });
        for result in &results {
            let key = match result {
                PendingEditResult::Landed { key, .. }
                | PendingEditResult::Failed { key, .. }
                | PendingEditResult::Retired { key } => key,
            };
            if !pending.is_pending(key) {
                match key {
                    DefinitionFieldKey::CourtyardWidth if *width.peek() == courtyard.0 => {
                        width_dirty.set(false);
                    }
                    DefinitionFieldKey::CourtyardHeight if *height.peek() == courtyard.1 => {
                        height_dirty.set(false);
                    }
                    DefinitionFieldKey::Pad { row, field } => {
                        if let Some(index) = field.text_index()
                            && let Some(mut drafts) = row_drafts.borrow().drafts.get(row).copied()
                            && *drafts.drafts[index].peek()
                                == accepted_field_text(
                                    key,
                                    &definition,
                                    &row_keys,
                                    &row_drafts,
                                    &courtyard,
                                )
                        {
                            drafts.dirty[index].set(false);
                        }
                    }
                    _ => {}
                }
            }
            if let PendingEditResult::Failed { message, .. } = result {
                failure.set(Some(message.clone()));
            }
        }
        // A missing row can still report its terminal failure above, but must not
        // retain draft handles or helper bindings after leaving the accepted list.
        row_drafts.borrow_mut().retain_rows(&row_keys, &pending);

        let kind = kind_name(&definition.kind);
        let on_kind = {
            let begin_kind = submit;
            move |event: FormEvent| {
                if let Some(kind) = parse_kind(&event.value()) {
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
        let add_pad_pending = pending.is_pending(&DefinitionFieldKey::AddPad);

        rsx! {
            div { class: "m1-definition-fields",
                label {
                    "Kind"
                    select {
                        aria_label: "Definition kind",
                        value: "{kind}",
                        onchange: on_kind,
                        option { value: "switch", "Switch" }
                        option { value: "controller", "Controller" }
                        option { value: "connector", "Connector" }
                        option { value: "encoder", "Encoder" }
                        option { value: "passive", "Passive" }
                        option { value: "custom", "Custom" }
                    }
                }
                fieldset { class: "m1-definition-courtyard",
                    legend { "Rectangular courtyard (mm)" }
                    label { "Width", input {
                        aria_label: "Courtyard width",
                        r#type: "number", min: "0.01", step: "0.1", value: "{width()}",
                        oninput: move |event| { width.set(event.value()); width_dirty.set(true); },
                        onblur: on_width_blur,
                        onkeydown: width_keydown,
                    } }
                    label { "Height", input {
                        aria_label: "Courtyard height",
                        r#type: "number", min: "0.01", step: "0.1", value: "{height()}",
                        oninput: move |event| { height.set(event.value()); height_dirty.set(true); },
                        onblur: on_height_blur,
                        onkeydown: height_keydown,
                    } }
                }
                div { class: "m1-definition-pad-heading",
                    strong { "Pads " small { "{definition.pads.len()}" } }
                    button { r#type: "button", disabled: kicad_locked || add_pad_pending, onclick: on_add, "+ Add pad" }
                }
                if kicad_locked {
                    p { class: "m1-definition-note", "Imported pad geometry stays linked to its original KiCad source." }
                }
                for (index, (pad, row_key)) in definition.pads.iter().zip(row_keys.iter().copied()).enumerate() {
                    PadFields {
                        key: "{owner_key}:{row_key}", index, row: row_key, pad: pad.clone(),
                        locked: kicad_locked,
                        drafts: row_drafts.borrow_mut().drafts.get(&row_key).copied(),
                        remove_pending: pending.is_pending(&DefinitionFieldKey::Pad {
                            row: row_key,
                            field: PadField::Remove,
                        }),
                        failure,
                        submit,
                    }
                }
                ul { class: "m1-definition-validation", "aria-live": "polite",
                    for issue in definition_issues(&definition) { li { "{issue}" } }
                }
                if let Some(message) = failure() {
                    p { class: "m1-definition-error", role: "alert", "{message}" }
                }
            }
        }
    }

    /// Admission compares only fresh accepted projection. Submitted-draft memory and
    /// terminal restoration remain in the helper.
    fn field_matches_accepted(
        key: &DefinitionFieldKey,
        edit: &DefinitionEdit,
        submitted: &str,
        definition: &PartDefinition,
    ) -> bool {
        match edit {
            DefinitionEdit::Kind(kind) => return *kind == definition.kind,
            DefinitionEdit::CourtyardWidth(_) => return submitted == courtyard_size(definition).0,
            DefinitionEdit::CourtyardHeight(_) => return submitted == courtyard_size(definition).1,
            DefinitionEdit::AddPad | DefinitionEdit::RemovePad { .. } => return false,
            _ => {}
        }
        let pad_id = match edit {
            DefinitionEdit::PadId { pad_id, .. }
            | DefinitionEdit::PadNumber { pad_id, .. }
            | DefinitionEdit::PadCoordinate { pad_id, .. }
            | DefinitionEdit::PadSize { pad_id, .. }
            | DefinitionEdit::PadDrill { pad_id, .. }
            | DefinitionEdit::PadShape { pad_id, .. } => pad_id,
            _ => return false,
        };
        let Some(pad) = definition.pads.iter().find(|pad| &pad.id == pad_id) else {
            return false;
        };
        if let DefinitionEdit::PadShape { shape, .. } = edit {
            return *shape == pad.shape;
        }
        matches!(key, DefinitionFieldKey::Pad { field, .. }
            if field.text_index().is_some_and(|index| RowDrafts::accepted_texts(pad)[index] == submitted))
    }

    /// The accepted text one committed field settles back to. A vanished row keeps its
    /// current draft text, so the settlement cannot invent a value for a pad that is
    /// gone; the failure message still surfaces through the shared placement.
    fn accepted_field_text(
        key: &DefinitionFieldKey,
        definition: &PartDefinition,
        row_keys: &[u64],
        row_drafts: &Rc<RefCell<PanelRows>>,
        courtyard: &(String, String),
    ) -> String {
        match *key {
            DefinitionFieldKey::Kind | DefinitionFieldKey::AddPad => String::new(),
            DefinitionFieldKey::CourtyardWidth => courtyard.0.clone(),
            DefinitionFieldKey::CourtyardHeight => courtyard.1.clone(),
            DefinitionFieldKey::Pad { row, field } => {
                let index = field.text_index().unwrap_or_default();
                match row_keys.iter().position(|existing| *existing == row) {
                    Some(position) => {
                        RowDrafts::accepted_texts(&definition.pads[position])[index].clone()
                    }
                    None => row_drafts
                        .borrow()
                        .drafts
                        .get(&row)
                        .map(|drafts| drafts.drafts[index].peek().clone())
                        .unwrap_or_default(),
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
        remove_pending: bool,
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
        let on_shape = move |event: FormEvent| {
            if let Some(shape) = parse_shape(&event.value()) {
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
            fieldset { class: "m1-definition-pad", disabled: locked,
                legend { "Pad {index + 1}" }
                div { class: "m1-definition-pad-grid",
                    label { "ID", input { aria_label: "Pad {index + 1} ID", value: "{id()}", oninput: move |event| { id.set(event.value()); id_dirty.set(true); }, onblur: on_id_blur, onkeydown: id_keydown } }
                    label { "Number", input { aria_label: "Pad {index + 1} number", value: "{number()}", oninput: move |event| { number.set(event.value()); number_dirty.set(true); }, onblur: on_number_blur, onkeydown: number_keydown } }
                    label { "X", input { aria_label: "Pad {index + 1} X", r#type: "number", step: "0.1", value: "{x()}", oninput: move |event| { x.set(event.value()); x_dirty.set(true); }, onblur: on_x_blur, onkeydown: x_keydown } }
                    label { "Y", input { aria_label: "Pad {index + 1} Y", r#type: "number", step: "0.1", value: "{y()}", oninput: move |event| { y.set(event.value()); y_dirty.set(true); }, onblur: on_y_blur, onkeydown: y_keydown } }
                    label { "Width", input { aria_label: "Pad {index + 1} width", r#type: "number", min: "0.01", step: "0.1", value: "{size_x()}", oninput: move |event| { size_x.set(event.value()); size_x_dirty.set(true); }, onblur: on_width_blur, onkeydown: width_keydown } }
                    label { "Height", input { aria_label: "Pad {index + 1} height", r#type: "number", min: "0.01", step: "0.1", value: "{size_y()}", oninput: move |event| { size_y.set(event.value()); size_y_dirty.set(true); }, onblur: on_height_blur, onkeydown: height_keydown } }
                    label { "Shape", select { aria_label: "Pad {index + 1} shape", value: "{shape_name(&pad.shape)}", onchange: on_shape,
                        option { value: "circle", "Circle" } option { value: "oval", "Oval" }
                        option { value: "rect", "Rectangle" } option { value: "roundrect", "Rounded rectangle" }
                    } }
                    label { "Drill", input { aria_label: "Pad {index + 1} drill", r#type: "number", min: "0.01", step: "0.1", placeholder: "None", value: "{drill()}", oninput: move |event| { drill.set(event.value()); drill_dirty.set(true); }, onblur: on_drill_blur, onkeydown: drill_keydown } }
                }
                button { class: "m1-definition-remove-pad", r#type: "button", disabled: remove_pending, onclick: on_remove, "Remove pad" }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DefinitionEdit {
    Kind(PartKind),
    CourtyardWidth(String),
    CourtyardHeight(String),
    AddPad,
    PadId {
        pad_id: String,
        value: String,
    },
    PadNumber {
        pad_id: String,
        value: String,
    },
    PadCoordinate {
        pad_id: String,
        axis: Axis,
        value: String,
    },
    PadSize {
        pad_id: String,
        axis: Axis,
        value: String,
    },
    PadDrill {
        pad_id: String,
        value: String,
    },
    PadShape {
        pad_id: String,
        shape: PadShape,
    },
    RemovePad {
        pad_id: String,
    },
}

/// Value-only validation the panel runs at dispatch, so an invalid draft is explained
/// inline without submitting anything. Checks that depend on the document (target
/// existence, eligibility, uniqueness) belong to the resolver.
pub fn validate_definition_edit(edit: &DefinitionEdit) -> Result<(), String> {
    match edit {
        DefinitionEdit::Kind(_)
        | DefinitionEdit::AddPad
        | DefinitionEdit::PadShape { .. }
        | DefinitionEdit::RemovePad { .. } => Ok(()),
        DefinitionEdit::CourtyardWidth(value) | DefinitionEdit::CourtyardHeight(value) => {
            parse_number(value, true, COURTYARD_NUMBERS).map(|_| ())
        }
        DefinitionEdit::PadId { value, .. } | DefinitionEdit::PadNumber { value, .. } => {
            if value.trim().is_empty() {
                Err(PAD_IDS_REQUIRED.into())
            } else {
                Ok(())
            }
        }
        DefinitionEdit::PadCoordinate { value, .. } => {
            parse_number(value, false, PAD_NUMBERS).map(|_| ())
        }
        DefinitionEdit::PadSize { value, .. } => parse_number(value, true, PAD_NUMBERS).map(|_| ()),
        DefinitionEdit::PadDrill { value, .. } => {
            if value.trim().is_empty() {
                Ok(())
            } else {
                parse_number(value, true, PAD_NUMBERS).map(|_| ())
            }
        }
    }
}

/// One resolved edit: Session fills the base revision and transaction identity when
/// the edit runs (ticket 03), so the resolver submits only the operation and targets.
pub(crate) fn replacement_commit(operation: EditOperation, target_ids: Vec<String>) -> Resolution {
    Resolution::submit(target_ids, operation)
}

/// Resolve one definition-field edit against the accepted document at execution: find
/// the definition, apply only this field to a clone of the accepted document, and
/// submit the replacement from there, so an edit queued behind another never reverts
/// it. Vanished or ineligible targets retire with a reason the user understands.
pub fn definition_field_resolver(
    definition_id: String,
    edit: DefinitionEdit,
    pad_id_seed: u64,
) -> EditResolver {
    EditResolver::new(
        "parts-definition-field",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            if !document
                .definitions
                .iter()
                .any(|item| item.id == definition_id)
            {
                return Resolution::Retire(DEFINITION_GONE.into());
            }
            if document
                .definitions
                .iter()
                .any(|item| item.id == definition_id && item.generator.is_some())
            {
                return Resolution::Retire(GENERATOR_LOCKED.into());
            }
            let mut replacement = document.as_ref().clone();
            let mut target_ids = vec![definition_id.clone()];
            if let Err(reason) = apply_definition_edit(
                &mut replacement,
                &definition_id,
                &mut target_ids,
                &edit,
                pad_id_seed,
            ) {
                return Resolution::Retire(reason);
            }
            // The clone is built from the accepted document, so equality means this
            // field already holds the committed value: resolve Unchanged, not a
            // landing heuristic (ADR-0005).
            if replacement == **document {
                return Resolution::Unchanged;
            }
            replacement_commit(
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
                target_ids,
            )
        },
    )
}

/// Apply one field edit in place. `Err` is the retire reason.
fn apply_definition_edit(
    doc: &mut ProjectDoc,
    definition_id: &str,
    target_ids: &mut Vec<String>,
    edit: &DefinitionEdit,
    pad_id_seed: u64,
) -> Result<(), String> {
    let Some(definition) = doc
        .definitions
        .iter_mut()
        .find(|item| item.id == definition_id)
    else {
        return Err(DEFINITION_GONE.into());
    };
    match edit {
        DefinitionEdit::Kind(kind) => definition.kind = kind.clone(),
        DefinitionEdit::CourtyardWidth(value) => {
            let width = parse_number(value, true, COURTYARD_NUMBERS)?;
            let (_, height, center) = courtyard_bounds(&definition.courtyard);
            authored_rectangle(definition, width, height, center);
        }
        DefinitionEdit::CourtyardHeight(value) => {
            let height = parse_number(value, true, COURTYARD_NUMBERS)?;
            let (width, _, center) = courtyard_bounds(&definition.courtyard);
            authored_rectangle(definition, width, height, center);
        }
        DefinitionEdit::AddPad => {
            imported_pads_retire(definition)?;
            let number = next_pad_number(&definition.pads);
            let id = unique_pad_id(&definition.pads, pad_id_seed);
            definition.pads.push(Pad {
                id,
                number,
                at: Vec2 { x: 0.0, y: 0.0 },
                size: Vec2 { x: 2.0, y: 2.0 },
                shape: PadShape::Circle,
                drill: None,
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            });
        }
        DefinitionEdit::PadId { pad_id, value } => {
            imported_pads_retire(definition)?;
            if !definition.pads.iter().any(|pad| pad.id == *pad_id) {
                return Err(PAD_GONE.into());
            }
            if value.trim().is_empty() {
                return Err(PAD_IDS_REQUIRED.into());
            }
            if definition
                .pads
                .iter()
                .any(|other| other.id != *pad_id && other.id == *value)
            {
                return Err(PAD_IDS_UNIQUE.into());
            }
            if let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) {
                pad.id = value.clone();
            }
            let part_ids = doc
                .parts
                .iter()
                .filter(|part| part.definition_id == definition.id)
                .map(|part| part.id.clone())
                .collect::<std::collections::BTreeSet<_>>();
            for net in &mut doc.nets {
                for pin in &mut net.pins {
                    if part_ids.contains(&pin.part_id) && pin.pad_id == *pad_id {
                        pin.pad_id = value.clone();
                    }
                }
            }
            target_ids.extend([pad_id.clone(), value.clone()]);
        }
        DefinitionEdit::PadNumber { pad_id, value } => {
            imported_pads_retire(definition)?;
            let value = value.trim().to_owned();
            if value.is_empty() {
                return Err(PAD_IDS_REQUIRED.into());
            }
            if definition
                .pads
                .iter()
                .any(|other| other.id != *pad_id && other.number == value)
            {
                return Err(PAD_NUMBERS_UNIQUE.into());
            }
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) else {
                return Err(PAD_GONE.into());
            };
            pad.number = value;
        }
        DefinitionEdit::PadCoordinate {
            pad_id,
            axis,
            value,
        } => {
            imported_pads_retire(definition)?;
            let number = parse_number(value, false, PAD_NUMBERS)?;
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) else {
                return Err(PAD_GONE.into());
            };
            match axis {
                Axis::X => pad.at.x = number,
                Axis::Y => pad.at.y = number,
            }
        }
        DefinitionEdit::PadSize {
            pad_id,
            axis,
            value,
        } => {
            imported_pads_retire(definition)?;
            let number = parse_number(value, true, PAD_NUMBERS)?;
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) else {
                return Err(PAD_GONE.into());
            };
            match axis {
                Axis::X => pad.size.x = number,
                Axis::Y => pad.size.y = number,
            }
        }
        DefinitionEdit::PadDrill { pad_id, value } => {
            imported_pads_retire(definition)?;
            let drill = if value.trim().is_empty() {
                None
            } else {
                Some(parse_number(value, true, PAD_NUMBERS)?)
            };
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) else {
                return Err(PAD_GONE.into());
            };
            pad.drill = drill;
        }
        DefinitionEdit::PadShape { pad_id, shape } => {
            imported_pads_retire(definition)?;
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) else {
                return Err(PAD_GONE.into());
            };
            pad.shape = shape.clone();
        }
        DefinitionEdit::RemovePad { pad_id } => {
            imported_pads_retire(definition)?;
            if !definition.pads.iter().any(|pad| pad.id == *pad_id) {
                return Err(PAD_GONE.into());
            }
            definition.pads.retain(|pad| pad.id != *pad_id);
            let part_ids = doc
                .parts
                .iter()
                .filter(|part| part.definition_id == definition.id)
                .map(|part| part.id.clone())
                .collect::<std::collections::BTreeSet<_>>();
            for net in &mut doc.nets {
                net.pins
                    .retain(|pin| !part_ids.contains(&pin.part_id) || pin.pad_id != *pad_id);
            }
            target_ids.push(pad_id.clone());
        }
    }
    Ok(())
}

/// Rewrite the courtyard as an authored rectangle of the given size around `center`.
fn authored_rectangle(definition: &mut PartDefinition, width: f64, height: f64, center: Vec2) {
    let half_x = width / 2.0;
    let half_y = height / 2.0;
    definition.courtyard = vec![
        Vec2 {
            x: center.x - half_x,
            y: center.y - half_y,
        },
        Vec2 {
            x: center.x + half_x,
            y: center.y - half_y,
        },
        Vec2 {
            x: center.x + half_x,
            y: center.y + half_y,
        },
        Vec2 {
            x: center.x - half_x,
            y: center.y + half_y,
        },
    ];
    let envelope = definition
        .envelope_source
        .get_or_insert_with(Default::default);
    envelope.courtyard = Some(boardstudio_core::model::EnvelopeOrigin::Authored);
}

fn imported_pads_retire(definition: &PartDefinition) -> Result<(), String> {
    if definition.kicad_source.is_some() {
        Err(IMPORTED_PADS.into())
    } else {
        Ok(())
    }
}
fn parse_number(value: &str, positive: bool, message: &str) -> Result<f64, String> {
    let number = if value.trim().is_empty() && !positive {
        0.0
    } else {
        value.parse::<f64>().unwrap_or(f64::NAN)
    };
    if !number.is_finite() || (positive && number <= 0.0) {
        Err(message.into())
    } else {
        Ok(number)
    }
}
pub(crate) fn courtyard_bounds(points: &[Vec2]) -> (f64, f64, Vec2) {
    if points.is_empty() {
        return (10.0, 6.0, Vec2 { x: 0.0, y: 0.0 });
    }
    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    (
        (max_x - min_x),
        (max_y - min_y),
        Vec2 {
            x: (min_x + max_x) / 2.0,
            y: (min_y + max_y) / 2.0,
        },
    )
}
fn next_pad_number(pads: &[Pad]) -> String {
    let mut candidate = pads.len() + 1;
    while pads.iter().any(|pad| pad.number == candidate.to_string()) {
        candidate += 1;
    }
    candidate.to_string()
}
fn unique_pad_id(pads: &[Pad], operation: u64) -> String {
    let base = format!("pad-{operation}");
    if !pads.iter().any(|pad| pad.id == base) {
        return base;
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !pads.iter().any(|pad| pad.id == candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{ProjectDoc, SceneDelta};
    use std::sync::Arc;

    fn definition(pads: serde_json::Value) -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id":"custom","name":"Custom","kind":"custom","courtyard":[],"pads":pads
        }))
        .expect("definition")
    }
    fn snapshot(definition: PartDefinition) -> AcceptedSnapshot {
        let mut document = ProjectDoc::empty("project", "Fixture");
        document.definitions.push(definition);
        snapshot_doc(document)
    }
    fn snapshot_doc(document: ProjectDoc) -> AcceptedSnapshot {
        let scene: SceneDelta = serde_json::from_value(serde_json::json!({
            "revision":0,"transactionId":"fixture","changedIds":[],"transforms":[],
            "matrixScenes":[],"contours":[],"boardContours":[],"boardReadiness":[],"findings":[],
            "readiness":{"layout":true,"outline":true,"pcb":true,"case":false}
        }))
        .expect("scene");
        AcceptedSnapshot {
            session_epoch: SessionEpoch(1),
            token: SnapshotToken(7),
            document: Arc::new(document),
            scene: Arc::new(scene),
        }
    }
    fn scope() -> Scope {
        Scope {
            session_epoch: SessionEpoch(1),
            document_id: "project".into(),
            board_id: "board".into(),
            instance_id: None,
        }
    }
    fn pad(id: &str, number: &str) -> serde_json::Value {
        serde_json::json!({ "id":id,"number":number,"at":{"x":0.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle" })
    }

    #[test]
    fn accepted_pad_rename_preserves_the_row_through_scalar_refresh() {
        let mut pads = definition(serde_json::json!([pad("a", "1"), pad("b", "2")])).pads;
        let mut rows = PadRowKeys::default();
        let before = rows.for_pads("owner", &pads);
        pads[0].id = "renamed".into();
        pads[0].at.y = 4.0;
        assert_eq!(rows.for_pads("owner", &pads), before);
    }

    #[test]
    fn pad_list_edits_preserve_survivors_and_retire_deleted_rows() {
        let mut pads = definition(serde_json::json!([pad("a", "1"), pad("b", "2")])).pads;
        let mut rows = PadRowKeys::default();
        let before = rows.for_pads("owner", &pads);
        let removed = pads.remove(0);
        assert_eq!(rows.for_pads("owner", &pads), vec![before[1]]);
        pads.push(removed);
        let restored = rows.for_pads("owner", &pads);
        assert_eq!(restored[0], before[1]);
        assert_ne!(restored[1], before[0]);
        pads.swap(0, 1);
        assert_eq!(
            rows.for_pads("owner", &pads),
            vec![restored[1], restored[0]]
        );
    }

    #[test]
    fn ambiguous_replacements_and_owner_switches_retire_pad_rows() {
        let pads = definition(serde_json::json!([pad("a", "1"), pad("b", "2")])).pads;
        let replacement = definition(serde_json::json!([pad("c", "1"), pad("d", "2")])).pads;
        let mut rows = PadRowKeys::default();
        let before = rows.for_pads("owner", &pads);
        let replaced = rows.for_pads("owner", &replacement);
        assert!(replaced.iter().all(|key| !before.contains(key)));
        let next_owner = rows.for_pads("other-owner", &replacement);
        assert!(next_owner.iter().all(|key| !replaced.contains(key)));
    }

    #[test]
    fn duplicate_pad_ids_never_share_or_transfer_a_row_lifetime() {
        let pads = definition(serde_json::json!([pad("a", "1"), pad("a", "2")])).pads;
        let mut rows = PadRowKeys::default();
        let before = rows.for_pads("owner", &pads);
        assert_ne!(before[0], before[1]);
        let refreshed = rows.for_pads("owner", &pads);
        assert!(refreshed.iter().all(|key| !before.contains(key)));
    }
    #[test]
    fn field_admission_ignores_departed_owners_but_not_newer_revisions() {
        let current = snapshot(definition(serde_json::json!([pad("a", "1")])));
        let owner_scope = scope();
        let capture = DefinitionPanelCapture::new(
            &current,
            Some(owner_scope.clone()),
            &current.document.definitions[0],
        );
        let selected = Some((Some(owner_scope.clone()), "custom".into()));
        assert!(capture.owner_matches(&current, Some(owner_scope.clone()), selected.clone()));
        assert!(
            !capture.owner_matches(
                &current,
                Some(owner_scope.clone()),
                Some((Some(owner_scope.clone()), "other".into()))
            ),
            "a stale selection cannot commit field drafts"
        );
        assert!(
            !capture.owner_matches(&current, None, selected.clone()),
            "a lost scope cannot commit field drafts"
        );

        // A newer accepted revision keeps the owner live: field edits queue freely and
        // resolve against the document accepted when they run (ADR-0005).
        let mut newer_document = current.document.as_ref().clone();
        newer_document.revision += 1;
        let newer = AcceptedSnapshot {
            token: SnapshotToken(current.token.0 + 1),
            document: Arc::new(newer_document),
            ..current.clone()
        };
        assert!(capture.owner_matches(&newer, Some(owner_scope), selected));
    }

    #[cfg(not(target_arch = "wasm32"))]
    mod native {
        use super::*;
        use boardstudio_application::{Event, TerminalOutcome};
        use boardstudio_web_runtime::{operation_outcomes::OutcomeSlot, runtime::Runtime};
        use std::rc::Rc;

        /// Drive the native test Runtime (real Session and Core, in-memory saves) and report
        /// each field edit's settlement.
        struct Driver {
            runtime: Rc<Runtime>,
            next_seed: u64,
        }

        impl Driver {
            fn open(document: ProjectDoc) -> Self {
                let runtime = Runtime::new();
                runtime.submit(Event::Open {
                    operation_id: runtime.operation(),
                    document,
                });
                assert!(runtime.model().accepted.is_some());
                Self {
                    runtime,
                    next_seed: 2,
                }
            }

            /// Submit one field edit; the pad id seed is explicit so AddPad identities are
            /// deterministic in assertions. The slot stays empty while a gate holds the edit.
            fn submit_at(&self, seed: u64, edit: DefinitionEdit) -> OutcomeSlot {
                let operation = self.runtime.operation();
                let slot = self.runtime.observe_operation(operation);
                self.runtime.submit(Event::ResolveEdit {
                    operation_id: operation,
                    label: "parts-definition-field".into(),
                    resolver: definition_field_resolver("custom".into(), edit, seed),
                });
                slot
            }

            fn resolve_at(&mut self, seed: u64, edit: DefinitionEdit) -> Vec<TerminalOutcome> {
                self.submit_at(seed, edit)
                    .borrow()
                    .iter()
                    .cloned()
                    .collect()
            }

            fn resolve(&mut self, edit: DefinitionEdit) -> Vec<TerminalOutcome> {
                let seed = self.next_seed;
                self.next_seed += 1;
                self.resolve_at(seed, edit)
            }

            /// One landed field edit: settles Completed and moves the revision by one.
            fn lands_one_edit(&mut self, edit: DefinitionEdit) {
                let before = self.accepted().revision;
                assert_eq!(
                    self.resolve(edit),
                    vec![TerminalOutcome::Completed],
                    "the field edit lands"
                );
                assert_eq!(self.accepted().revision, before + 1);
            }

            fn accepted(&self) -> ProjectDoc {
                self.runtime
                    .model()
                    .accepted
                    .expect("accepted document")
                    .document
                    .as_ref()
                    .clone()
            }

            fn custom(&self) -> PartDefinition {
                self.accepted()
                    .definitions
                    .iter()
                    .find(|definition| definition.id == "custom")
                    .unwrap()
                    .clone()
            }
        }

        #[test]
        fn every_supported_field_resolves_one_individual_accepted_edit() {
            let mut initial = definition(serde_json::json!([pad("a", "1")]));
            initial.pads[0].at.x = 1.0;
            initial.courtyard = vec![
                Vec2 { x: -5.0, y: -3.0 },
                Vec2 { x: 5.0, y: -3.0 },
                Vec2 { x: 5.0, y: 3.0 },
                Vec2 { x: -5.0, y: 3.0 },
            ];
            let mut document = ProjectDoc::empty("project", "Fixture");
            document.definitions.push(initial);
            let mut driver = Driver::open(document);

            driver.lands_one_edit(DefinitionEdit::Kind(PartKind::Connector));
            assert!(matches!(driver.custom().kind, PartKind::Connector));

            driver.lands_one_edit(DefinitionEdit::CourtyardWidth("14".into()));
            assert_eq!(
                courtyard_bounds(&driver.custom().courtyard),
                (14.0, 6.0, Vec2 { x: 0.0, y: 0.0 })
            );

            driver.lands_one_edit(DefinitionEdit::CourtyardHeight("8".into()));
            assert_eq!(courtyard_bounds(&driver.custom().courtyard).1, 8.0);

            driver.lands_one_edit(DefinitionEdit::PadNumber {
                pad_id: "a".into(),
                value: "9".into(),
            });
            assert_eq!(driver.custom().pads[0].number, "9");

            driver.lands_one_edit(DefinitionEdit::PadCoordinate {
                pad_id: "a".into(),
                axis: Axis::Y,
                value: "2".into(),
            });
            assert_eq!(driver.custom().pads[0].at.y, 2.0);

            driver.lands_one_edit(DefinitionEdit::PadSize {
                pad_id: "a".into(),
                axis: Axis::X,
                value: "3".into(),
            });
            driver.lands_one_edit(DefinitionEdit::PadSize {
                pad_id: "a".into(),
                axis: Axis::Y,
                value: "4".into(),
            });
            assert_eq!(driver.custom().pads[0].size, Vec2 { x: 3.0, y: 4.0 });

            driver.lands_one_edit(DefinitionEdit::PadDrill {
                pad_id: "a".into(),
                value: "0.5".into(),
            });
            assert_eq!(driver.custom().pads[0].drill, Some(0.5));

            driver.lands_one_edit(DefinitionEdit::PadDrill {
                pad_id: "a".into(),
                value: String::new(),
            });
            assert_eq!(driver.custom().pads[0].drill, None);

            driver.lands_one_edit(DefinitionEdit::PadShape {
                pad_id: "a".into(),
                shape: PadShape::Oval,
            });
            assert!(matches!(driver.custom().pads[0].shape, PadShape::Oval));
        }

        #[test]
        fn an_unchanged_field_edit_lands_without_moving_the_revision() {
            let mut initial = definition(serde_json::json!([pad("a", "1")]));
            initial.pads[0].at.x = 1.0;
            let mut document = ProjectDoc::empty("project", "Fixture");
            document.definitions.push(initial);
            let mut driver = Driver::open(document);
            let before = driver.accepted();

            assert_eq!(
                driver.resolve(DefinitionEdit::PadCoordinate {
                    pad_id: "a".into(),
                    axis: Axis::X,
                    value: "1".into(),
                }),
                vec![TerminalOutcome::Completed]
            );
            assert_eq!(
                driver.resolve(DefinitionEdit::Kind(PartKind::Custom)),
                vec![TerminalOutcome::Completed]
            );
            let after = driver.accepted();
            assert_eq!(after.revision, before.revision);
        }

        #[test]
        fn add_pad_keeps_free_default_and_repairs_collision_as_one_edit() {
            // Expected-red reference oracle: pads.len()+1 is 3 for [1, 3], so literal
            // reference code creates a duplicate. The reviewed correction advances to 4.
            let pads: Vec<Pad> = vec![
                serde_json::from_value(pad("a", "1")).unwrap(),
                serde_json::from_value(pad("b", "3")).unwrap(),
            ];
            let old_reference_candidate = pads.len() + 1;
            assert_eq!(old_reference_candidate.to_string(), "3");
            assert!(
                pads.iter()
                    .any(|pad| pad.number == old_reference_candidate.to_string())
            );
            assert_eq!(next_pad_number(&pads), "4");
            let ordinary = vec![
                serde_json::from_value(pad("a", "1")).unwrap(),
                serde_json::from_value(pad("b", "2")).unwrap(),
            ];
            assert_eq!(next_pad_number(&ordinary), "3");

            let mut document = ProjectDoc::empty("project", "Fixture");
            document.definitions.push(definition(serde_json::json!([
                pad("a", "1"),
                pad("b", "3")
            ])));
            let mut driver = Driver::open(document);
            assert_eq!(
                driver.resolve_at(9, DefinitionEdit::AddPad),
                vec![TerminalOutcome::Completed]
            );
            let pads = &driver.custom().pads;
            assert_eq!(pads.len(), 3);
            assert_eq!(pads[2].number, "4");
            assert_eq!(pads[2].id, "pad-9");
        }

        #[test]
        fn numeric_fields_keep_reference_blank_and_validation_semantics() {
            assert_eq!(parse_number("", false, "bad").unwrap(), 0.0);
            assert!(parse_number("", true, "bad").is_err());
            let mut document = ProjectDoc::empty("project", "Fixture");
            document
                .definitions
                .push(definition(serde_json::json!([pad("a", "1")])));
            let mut driver = Driver::open(document);
            assert_eq!(
                driver.resolve(DefinitionEdit::PadCoordinate {
                    pad_id: "a".into(),
                    axis: Axis::X,
                    value: "".into(),
                }),
                vec![TerminalOutcome::Completed]
            );
            assert_eq!(driver.custom().pads[0].at.x, 0.0);
        }

        #[test]
        fn field_edits_retire_vanished_or_ineligible_targets_with_reasons() {
            let mut document = ProjectDoc::empty("project", "Fixture");
            document.definitions.push(definition(serde_json::json!([
                pad("a", "1"),
                pad("b", "2")
            ])));
            let mut driver = Driver::open(document);
            assert_eq!(
                driver.resolve(DefinitionEdit::PadCoordinate {
                    pad_id: "missing".into(),
                    axis: Axis::X,
                    value: "1".into(),
                }),
                vec![TerminalOutcome::Rejected(PAD_GONE.into())]
            );
            assert_eq!(
                driver.resolve(DefinitionEdit::PadId {
                    pad_id: "a".into(),
                    value: "b".into(),
                }),
                vec![TerminalOutcome::Rejected(PAD_IDS_UNIQUE.into())]
            );
            assert_eq!(
                driver.resolve(DefinitionEdit::PadNumber {
                    pad_id: "a".into(),
                    value: "2".into(),
                }),
                vec![TerminalOutcome::Rejected(PAD_NUMBERS_UNIQUE.into())]
            );

            let mut generator_document = ProjectDoc::empty("project", "Fixture");
            let mut generated = definition(serde_json::json!([pad("a", "1")]));
            generated.generator = Some(boardstudio_core::model::PartGenerator {
                source: "generator/source".into(),
                version: "1".into(),
                parameters: Default::default(),
            });
            generator_document.definitions.push(generated);
            let mut driver = Driver::open(generator_document);
            assert_eq!(
                driver.resolve(DefinitionEdit::PadCoordinate {
                    pad_id: "a".into(),
                    axis: Axis::X,
                    value: "1".into(),
                }),
                vec![TerminalOutcome::Rejected(GENERATOR_LOCKED.into())]
            );

            let mut imported_document = ProjectDoc::empty("project", "Fixture");
            let mut imported = definition(serde_json::json!([pad("a", "1")]));
            imported.kicad_source = Some(boardstudio_core::model::KicadSource {
                format_version: 1,
                source: "part.kicad_mod".into(),
            });
            imported_document.definitions.push(imported);
            let mut driver = Driver::open(imported_document);
            assert_eq!(
                driver.resolve(DefinitionEdit::PadCoordinate {
                    pad_id: "a".into(),
                    axis: Axis::X,
                    value: "1".into(),
                }),
                vec![TerminalOutcome::Rejected(IMPORTED_PADS.into())]
            );

            let mut missing_document = ProjectDoc::empty("project", "Fixture");
            let mut other = definition(serde_json::json!([]));
            other.id = "other".into();
            missing_document.definitions.push(other);
            let mut driver = Driver::open(missing_document);
            assert_eq!(
                driver.resolve(DefinitionEdit::Kind(PartKind::Connector)),
                vec![TerminalOutcome::Rejected(DEFINITION_GONE.into())]
            );
        }

        #[test]
        fn pad_rename_and_removal_keep_instance_pin_relationships_scoped() {
            let mut document = ProjectDoc::empty("project", "Fixture");
            document.definitions.push(definition(serde_json::json!([
                pad("old", "1"),
                pad("keep", "2")
            ])));
            document.parts = serde_json::from_value(serde_json::json!([
                {"id":"instance-a","definitionId":"custom","reference":"U1","pose":{"at":{"x":0.0,"y":0.0},"rotation":0.0},"side":"front"},
                {"id":"instance-b","definitionId":"custom","reference":"U2","pose":{"at":{"x":1.0,"y":0.0},"rotation":0.0},"side":"front"},
                {"id":"other","definitionId":"other-definition","reference":"R1","pose":{"at":{"x":2.0,"y":0.0},"rotation":0.0},"side":"front"}
            ])).unwrap();
            document.nets = serde_json::from_value(serde_json::json!([
                {"id":"net-a","name":"A","pins":[{"partId":"instance-a","padId":"old"},{"partId":"instance-b","padId":"old"},{"partId":"other","padId":"old"}]},
                {"id":"net-b","name":"B","pins":[{"partId":"instance-a","padId":"keep"}]}
            ])).unwrap();
            let mut driver = Driver::open(document);

            driver.lands_one_edit(DefinitionEdit::PadId {
                pad_id: "old".into(),
                value: "renamed".into(),
            });
            let renamed = driver.accepted();
            assert_eq!(renamed.definitions[0].pads[0].id, "renamed");
            assert_eq!(renamed.nets[0].pins[0].pad_id, "renamed");
            assert_eq!(renamed.nets[0].pins[1].pad_id, "renamed");
            assert_eq!(renamed.nets[0].pins[2].pad_id, "old");
            assert_eq!(renamed.nets[1].pins[0].pad_id, "keep");

            driver.lands_one_edit(DefinitionEdit::RemovePad {
                pad_id: "renamed".into(),
            });
            let removed = driver.accepted();
            assert_eq!(
                removed.definitions[0]
                    .pads
                    .iter()
                    .map(|pad| pad.id.as_str())
                    .collect::<Vec<_>>(),
                vec!["keep"]
            );
            assert!(
                removed.nets[0]
                    .pins
                    .iter()
                    .all(|pin| pin.pad_id != "renamed" || pin.part_id == "other")
            );
            assert_eq!(removed.nets[1].pins[0].pad_id, "keep");
            assert_eq!(removed.parts.len(), 3);
        }

        #[test]
        fn queued_field_edits_keep_the_prior_accepted_change_while_core_is_held() {
            let mut document = ProjectDoc::empty("project", "Fixture");
            document
                .definitions
                .push(definition(serde_json::json!([pad("a", "1")])));
            let driver = Driver::open(document);

            driver.runtime.hold_next_core();
            let rename = driver.submit_at(
                2,
                DefinitionEdit::PadNumber {
                    pad_id: "a".into(),
                    value: "7".into(),
                },
            );
            assert!(driver.runtime.core_entered(), "the first edit reaches Core");
            let kind = driver.submit_at(3, DefinitionEdit::Kind(PartKind::Connector));
            assert!(rename.borrow().is_none());
            assert!(kind.borrow().is_none(), "the second edit waits for Core");

            driver.runtime.release_core();
            assert_eq!(*rename.borrow(), Some(TerminalOutcome::Completed));
            assert_eq!(*kind.borrow(), Some(TerminalOutcome::Completed));
            let custom = driver.custom();
            assert_eq!(
                custom.pads[0].number, "7",
                "the earlier accepted edit survives"
            );
            assert!(matches!(custom.kind, PartKind::Connector));
        }

        #[test]
        fn queued_field_edit_retires_when_its_pad_departs_first() {
            let mut document = ProjectDoc::empty("project", "Fixture");
            document.definitions.push(definition(serde_json::json!([
                pad("a", "1"),
                pad("b", "2")
            ])));
            let driver = Driver::open(document);

            driver.runtime.hold_next_core();
            let removal = driver.submit_at(2, DefinitionEdit::RemovePad { pad_id: "a".into() });
            assert!(driver.runtime.core_entered(), "the removal reaches Core");
            let queued = driver.submit_at(
                3,
                DefinitionEdit::PadCoordinate {
                    pad_id: "a".into(),
                    axis: Axis::X,
                    value: "4".into(),
                },
            );
            assert!(queued.borrow().is_none(), "the queued edit waits for Core");

            driver.runtime.release_core();
            assert_eq!(*removal.borrow(), Some(TerminalOutcome::Completed));
            assert_eq!(
                *queued.borrow(),
                Some(TerminalOutcome::Rejected(PAD_GONE.into()))
            );
            let pads = driver.custom().pads;
            assert_eq!(pads.len(), 1);
            assert_eq!(pads[0].id, "b");
        }
    }
}
