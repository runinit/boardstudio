//! Private layer edit requests and root-filtered status for the Keymap panel.

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) enum KeymapLayerOperation {
    Add,
    Rename { layer_id: String, name: String },
    Remove { layer_id: String },
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) enum KeymapLayerFeedback {
    Pending,
    Saved,
    Failed(String),
}
