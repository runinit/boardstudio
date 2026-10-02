## F1 source review — `f0ac0a19...7d46cd9a`

**Scoped pass; no concrete source-level F1 spec finding.** The prior project-menu finding is fixed: demo, saved-project and import actions close the menu, and close/Escape returns focus to the summary (`web/src/presentation.rs:170-220, 504-519`). The bounded source review also found the exact six labels plus separate Export, Objects board/instance/component navigation, Inspect position editing, explicit placeholders with a Layout return path, and existing Layout/Case/archive/STEP paths. Workspace state remains app-scoped; active non-pan gestures are cancelled on workspace changes, and an unmounted numeric inspector draft is reverted. Light/Dark/System uses the browser preference key and responds to OS changes. Compact controls expose workspace and panel navigation, with horizontal overflow clipped at compact widths.

This is a source-only review. The issue’s desktop/compact/theme captures and browser interaction evidence remain for the root’s in-progress verification; this pass does not mark those gates complete.

## Final source delta — `242184509c1ec2d97ab0754dd29faa95edf0b342`

Reviewed the delta from `7d46cd9a`: compact footer and save-status bars change from `position: sticky` to `position: static`. This avoids covering history controls and creates no new F1 spec concern. **Scoped source review remains pass; no concrete finding.** Compact browser verification is still pending.
