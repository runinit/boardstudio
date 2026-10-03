# Corrected assembly-document comparison — candidate 34770

**Candidate:** source `47623521`, route `http://127.0.0.1:34770/boardstudio/`, provenance SHA-256 `a4677af663c0e8286ae5e923bf1884b0230bcc6dc852a3030cc35a68d453365c`.

Using the same isolated browser profile and exact F6C.4 fixture, replayed only the full KiCad handoff download affected by the heading correction. Download retained at `/home/chris/.local/share/boardstudio/reviews/export-workspace-20261003/dioxus-full-draft-34769/34770-Sofle-v2-pcb-handoff.zip`, 1,522,633 bytes, SHA-256 `d2d3943027796403f93425e8670a3f5f6cfb81cf58c0753ed74f0ef84f054b30`.

The extracted `ASSEMBLY.md` is byte-identical to pinned React: 606 bytes, SHA-256 `76eee4b3f033ea61ec9906891eb1b26fec8c159b4c219b55cef04bc7377fcd1e`. It begins with `# left half`, then the required `# PCB wiring and assembly`, then the ready-for-routing disclaimer. The nested `Left PCB-kicad.zip` is byte-identical to React and to the 34769 full download. The 34769 receipt retains both full/draft actual-download results; this correction leg intentionally did not repeat the draft download. Therefore the corrected full handoff and shared assembly helper are checked, while a post-fix draft click is not claimed.
