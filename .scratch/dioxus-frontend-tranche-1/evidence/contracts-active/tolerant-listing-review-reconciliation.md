# Proposal review reconciliation

Independent proposal review checked the exact unapplied patch against47cf655b. Its P2 documentation mismatch is corrected here by retaining the reviewed `/tmp` proposal and concrete patch side by side: Document(Box<ProjectDoc>), exact string IDs including empty/whitespace, no numeric key coercion. Existing API/write/schema behavior remains unchanged. Proposal status is UNAPPROVED / UNAPPLIED; source review and git apply --check are not API authority or compilation/runtime evidence. Actual checks follow an explicit public API decision.
