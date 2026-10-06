//! Private layer edit requests and root-filtered status for the Keymap panel.

#[derive(Clone, Debug, PartialEq)]
pub enum KeymapLayerOperation {
    Add,
    Rename { layer_id: String, name: String },
    Remove { layer_id: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum KeymapLayerFeedback {
    Pending,
    Saved,
    Failed(String),
}
