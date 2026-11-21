//! Board-side layer name projection.
use boardstudio_model::model::Side;

pub fn resolve_board_layer(layer: &str, side: &Side) -> String {
    if matches!(side, &Side::Back) {
        if let Some(rest) = layer.strip_prefix("F.") {
            return format!("B.{rest}");
        }
        if let Some(rest) = layer.strip_prefix("B.") {
            return format!("F.{rest}");
        }
    }
    layer.to_owned()
}
