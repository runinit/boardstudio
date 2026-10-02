// This binary-only module compiles the existing host source unchanged, then
// adds the page viewer's private renderer operations in the same module so
// they can use RendererHost's private lifecycle state.
include!("renderer_host.rs");
include!("renderer_host_page_extensions.rs");
