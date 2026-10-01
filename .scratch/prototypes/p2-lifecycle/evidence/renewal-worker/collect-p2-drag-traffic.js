(() => {
  const worker = window.__p2WorkerTraffic.workers[0];
  const start = window.__p2DragStart;
  const innerBytes = (objectFrame) => new TextEncoder().encode(objectFrame.frame).byteLength;
  const request = (record) => {
    const wrapper = JSON.parse(record.frame);
    const envelope = JSON.parse(wrapper.frame);
    const core = envelope.action?.Core;
    return {
      wrapperJsonUtf8Bytes: record.bytes,
      innerFrameUtf8Bytes: innerBytes(wrapper),
      identity: envelope.id,
      kind: core?.kind,
      operationId: core?.id,
      phase: core?.command?.phase,
      targetIds: core?.command?.targetIds,
      bufferField: Object.hasOwn(wrapper, "buffer"),
    };
  };
  const reply = (record) => {
    const wrapper = JSON.parse(record.frame);
    const envelope = JSON.parse(wrapper.frame);
    const core = envelope.payload?.Core;
    return {
      wrapperJsonUtf8Bytes: record.bytes,
      innerFrameUtf8Bytes: innerBytes(wrapper),
      identity: envelope.id,
      replyKind: core?.kind,
      revision: core?.document?.revision,
      sceneRevision: core?.scene?.revision,
      documentJsonUtf8Bytes: core?.document
        ? new TextEncoder().encode(JSON.stringify(core.document)).byteLength
        : 0,
      sceneJsonUtf8Bytes: core?.scene
        ? new TextEncoder().encode(JSON.stringify(core.scene)).byteLength
        : 0,
      bufferField: Object.hasOwn(wrapper, "buffer"),
    };
  };
  const requests = worker.requests.slice(start.requestCount).map(request);
  const replies = worker.replies.slice(start.replyCount).map(reply);
  const requestBytes = requests.reduce((total, item) => total + item.innerFrameUtf8Bytes, 0);
  const replyBytes = replies.reduce((total, item) => total + item.innerFrameUtf8Bytes, 0);
  return JSON.stringify({
    measurement: "UTF-8 byte lengths of captured JSON frame strings; wrapper JSON size also shown; structured-clone wire/heap bytes are not asserted",
    fixtureSha256: "b577dd2009fffbf00489cc8d0f2ccc088861f62a7f30af42470d35c11534bdae",
    target: "matrix/main-right-keys/r0c0",
    start,
    requests,
    replies,
    totals: {
      requests: requests.length,
      replies: replies.length,
      innerRequestFrameUtf8Bytes: requestBytes,
      innerReplyFrameUtf8Bytes: replyBytes,
      wrapperRequestJsonUtf8Bytes: requests.reduce((total, item) => total + item.wrapperJsonUtf8Bytes, 0),
      wrapperReplyJsonUtf8Bytes: replies.reduce((total, item) => total + item.wrapperJsonUtf8Bytes, 0),
      transferredBinaryBytes: 0,
    },
    committedTransform: document
      .querySelector('[data-part-id="matrix/main-right-keys/r0c0"]')
      .getAttribute("transform"),
    status: [...document.querySelectorAll('[role="status"]')].map((node) => node.textContent),
  });
})()
