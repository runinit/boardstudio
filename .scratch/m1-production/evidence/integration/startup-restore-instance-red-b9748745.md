# Startup restore and physical-instance selector: public red capture

Captured before provider changes on the pristine b974 release at the root prefix in a fresh named browser session. Clicking **Sofle v2 copy** opened its editor and persisted project `m1-sofle-v2-copy` in isolated `boardstudio-m1-root` storage. The record has revision 3 and two physical instances (`left` and `right`). The scoped active-project localStorage key also held `m1-sofle-v2-copy`.

After reload, the app showed the Keyboard library and saved Sofle row only; there was no active editor. The same happened after an offline reload while the root service worker controlled the page. The saved copy remained present and reopened manually. This is the startup restoration red: a saved active project ID is present, but cold startup leaves the user in the library.

When the saved project was open, the accessible controls exposed a **Board** selector with **Left PCB** and **Right PCB**. There was no physical-instance selector and neither instance name appeared in visible text. The fixture source has two physical instances, so current UI board selection does not let the user choose the saved physical instance for physical-instance scoped case settings or generation.

The JSON record contains build/source/page/worker hashes, storage scope, assertions, raw AX snapshots, and screenshot paths/hashes. No product files were changed. The browser session was closed after evidence capture. This red does not assert restore, instance transition, job cancellation, or subpath green behavior.
