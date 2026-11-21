//! The Inspector's matrix transform fields, the values they carry and the projected field
//! context of the selected matrix, row, column or key.

use boardstudio_model::model::{
    AggregateMethod, AnchorPart, MatrixAnchor, MatrixSplayAffect, Mirror, PartDefinition, Vec2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixTransformField {
    OriginX,
    OriginY,
    MatrixRotation,
    MatrixMirror,
    RowOffsetX,
    RowOffsetY,
    RowOffsetReset,
    RowShiftX,
    RowShiftY,
    RowBend,
    RowPadding,
    ColumnOffsetX,
    ColumnOffsetY,
    ColumnOffsetReset,
    ColumnOffsetConvert,
    ColumnStagger,
    ColumnSpread,
    ColumnSplay,
    SplayOriginMode,
    SplayOriginX,
    SplayOriginY,
    SplayOriginPoint,
    SplayOriginSwingPoint,
    KeyOffsetX,
    KeyOffsetY,
    KeyShiftX,
    KeyShiftY,
    KeyBend,
    KeyRotation,
    KeyTransformReset,
    KeyEnabled,
    /// Omit one mirrored key from the matrix surface (or rejoin it). The toggle
    /// submits SetHalfException{Part, Omitted/Linked}; Core keeps the live pose.
    KeyOmit,
    KeyAssembly,
    KeyAttached,
    KeyAssembliesLocal,
    KeyAddAnchoredMatrix,
    DeleteSelection,
    MatrixDetach,
    MatrixDetachConfirm,
    MatrixAnchorPick,
    MatrixAnchorMethod,
    MatrixAnchorParts,
    MatrixFollowX,
    MatrixFollowY,
    MatrixFollowRotation,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatrixTransformValue {
    Number(f64),
    Mirror(Option<Mirror>),
    Offset(Vec2),
    CellTransform {
        offset: Vec2,
        rotation: f64,
    },
    OriginMode(bool),
    Point(Vec2),
    AnchorKey {
        matrix_id: String,
        row: u32,
        column: u32,
    },
    AnchorMethod(AggregateMethod),
    AnchorParts(Vec<AnchorPart>),
    Bool(bool),
    Text(String),
    Attached(Vec<(String, String)>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatrixTransformFields {
    Matrix {
        origin: Vec2,
        rotation: f64,
        mirror: Option<Mirror>,
        mirror_y_locked: bool,
        /// False when the matrix is anchored without rotation Follow: Core
        /// would refuse the no-op edit, so the Inspector disables the field.
        rotation_followed: bool,
        anchor: Option<MatrixAnchor>,
    },
    Row {
        row: u32,
        offset: Vec2,
        shift: Vec2,
        bend: f64,
        padding: Option<f64>,
        pitch: f64,
    },
    Column {
        column: u32,
        offset: Vec2,
        stagger: f64,
        spread: Option<f64>,
        pitch: f64,
        splay_angle: f64,
        rotation: f64,
        splay_origin: Vec2,
        custom_origin: bool,
        /// Core's verdict on `ColumnOffsetToStagger` for this column, from the accepted
        /// matrix, so the Inspector shows Core's reason instead of re-deriving the rule.
        convert: Result<(), String>,
    },
    Key {
        row: u32,
        column: u32,
        enabled: bool,
        definition_id: String,
        choices: Vec<(String, String)>,
        assemblies: Vec<(String, String)>,
        component_choices: Vec<PartDefinition>,
        mirror_target: bool,
        assemblies_local: bool,
        offset: Vec2,
        rotation: f64,
        shift: Vec2,
        bend: f64,
        /// Native-pair mirror exception state for this key, when the key
        /// resolves to a mirror-side part with a Part correspondence.
        half_exception: Option<KeyHalfException>,
    },
}

pub fn column_angle(splay: f64, rotation: f64, affect: &MatrixSplayAffect) -> f64 {
    match affect {
        MatrixSplayAffect::Column => rotation,
        MatrixSplayAffect::Following => splay,
    }
}

/// Native-pair mirror exception context for one matrix key, resolved from the
/// accepted document at projection time and re-resolved at execution time.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyHalfException {
    pub pair_id: String,
    pub mirror_id: String,
    pub omitted: bool,
}
