const mode = new URL(import.meta.url).searchParams.get("bodies");
self.postMessage({ kind: "ready" });
self.onmessage = (event) => {
  if (event.data.kind !== "request") return;
  const request = JSON.parse(event.data.frame);
  const result = { revision: request.identity.revision, step: [], mesh: null, bounds: null };
  if (mode === "object") result.bodies = { length: 1 };
  const wire = {
    requestId: request.requestId,
    jobId: request.jobId,
    identity: request.identity,
    operation: request.operation,
    outcome: "completed",
    error: null,
  };
  self.postMessage({ kind: "reply", frame: JSON.stringify(wire), result });
};
