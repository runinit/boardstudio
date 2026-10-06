//! Native bundled-content builder. Recipes use Core's public document/edit/archive APIs.
#[path = "demo_projects/mod.rs"]
mod demo_projects;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    demo_projects::prepare(std::env::args().nth(1).as_deref())?;
    Ok(())
}
