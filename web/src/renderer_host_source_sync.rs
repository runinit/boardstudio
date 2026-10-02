//! Native source guard for the temporary private renderer lifetime snapshot.
#[test]
fn private_page_host_matches_normalized_library_host() {
    let source = include_str!("renderer_host.rs");
    const PAGE_ONLY_UPDATE_SCENE: &str = r#"    pub fn update_scene(&self, input: JsValue) -> Result<(), String> {
        if self.inner.disposed.get() || self.inner.context_lost.get() {
            return Err("Renderer is no longer active".to_owned());
        }
        call_method(&self.inner.renderer, "setScene", &[input]).map_err(js_error)?;
        schedule_frame(&self.inner).map_err(js_error)
    }
"#;
    assert_eq!(source.matches(PAGE_ONLY_UPDATE_SCENE).count(), 1);
    let without_page_unused_method = source.replace(PAGE_ONLY_UPDATE_SCENE, "");
    let normalized = without_page_unused_method.replacen("//!", "//", 3);
    assert_eq!(
        include_str!("renderer_host_page_base.rs"),
        normalized,
        "refresh the private page snapshot only with the documented page-only normalization"
    );
}
