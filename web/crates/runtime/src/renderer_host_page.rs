// Binary-only private renderer module. `renderer_host_page_base.rs` is a
// source snapshot of renderer_host.rs at baseline 261ef5bfe1dc984f24be44c95873a684c4940968
// (SHA-256 377f45e0ad51f5979dcba152d6b14dec71ea035991ee031b370829b5ac5b06e6).
// Normalization changes the three leading inner-doc prefixes to ordinary
// comments and omits exactly the unused unchecked update_scene method.
// The wrapper uses sequence-checked scene submission instead.
// Keep the snapshot synchronized with the native renderer_host_source_sync test.
include!("renderer_host_page_base.rs");
include!("renderer_host_page_extensions.rs");
