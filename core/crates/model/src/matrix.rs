pub mod kernel;
pub mod splay;
pub fn member_id(matrix: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix}/r{row}c{column}")
}

/// The generated part ID of one assembly on a matrix key.
pub fn companion_id(key_id: &str, assembly_id: &str) -> String {
    format!("{key_id}/{assembly_id}")
}
