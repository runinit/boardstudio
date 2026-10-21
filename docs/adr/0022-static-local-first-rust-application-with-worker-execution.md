---
status: accepted
---

# Keep the application static, local-first and Rust-owned

The core workflow runs locally in a statically hosted browser application with browser persistence, without requiring server computation for editing and locally available generation/export. Maintained application rules live in Rust; generated JavaScript bindings and upstream runtimes are allowed, not a second authored application-policy layer. Heavy Core/CAD work runs in workers rather than on the UI thread. The user confirmed this platform boundary on 2026-10-19, accepting browser resource limits in exchange for local operation without a mandatory computation service.

Deployment updates must preserve active editors' access to their offline assets: prefer delayed worker activation over forcing an update that removes assets an old editor may still need. This trades immediate update adoption for continuity. Offline operation requires the relevant app and project assets to be available locally; this is not a promise to fetch uncached remote models offline, nor a browser-support matrix. This decision does not reinstate retired frontend-trial or rollback machinery.

Evidence: [current architecture](../architecture.md), [offline installation and activation policy](../../web/src/service_worker.rs), and [scoped offline-cache policy](../../web/crates/host/src/offline.rs). Historical rationale appears in the removed ownership ADR, `323967ff3a67b9b62db4f201b477396d62b55ccd:docs/adr/0003-rust-application-ownership.md`, §§1, 3 and 7; commit `8d481b7a8743ed7c8b62f47253e6d00b3096bca6` delays offline activation until old clients close.
