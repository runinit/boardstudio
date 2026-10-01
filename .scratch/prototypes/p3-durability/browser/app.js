import init, {
  archive_stored_project_roundtrip,
  commit_one_edit,
  import_fixture,
  load_stored_project,
  retry_pending,
  save_pending,
  undo_once,
} from "./pkg/p3_durability.js";

await init();
globalThis.p3 = {
  archiveStoredProjectRoundtrip: archive_stored_project_roundtrip,
  commitOneEdit: commit_one_edit,
  importFixture: import_fixture,
  loadStoredProject: load_stored_project,
  retryPending: retry_pending,
  savePending: save_pending,
  undoOnce: undo_once,
};
document.documentElement.dataset.p3WasmReady = "true";

if (new URL(location.href).searchParams.get("skip-sw") !== "1") {
  const registration = await navigator.serviceWorker.register("./sw-bootstrap.mjs", {
    scope: "./",
    type: "module",
  });
  await navigator.serviceWorker.ready;
  document.documentElement.dataset.p3ServiceWorkerScope = registration.scope;
}
